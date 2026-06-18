//! 백그라운드 파이프라인 서비스
//!
//! Tauri 앱 내에서 tokio 런타임으로 실행.
//! config 로드 → 어댑터 초기화 → inbox 감시 + 배치 + 주기 lint/purge.

use std::sync::Arc;

use anyhow::Result;
use tracing::{info, warn};

use file_pipeline_shared::config;
use file_pipeline_shared::config::{PipelineConfigExt, ResolvedPathsExt};

use crate::state::AppState;

/// 서비스 초기화 — AppState를 구성하여 반환
pub fn init_app_state() -> Result<AppState> {
    let (db, mut cfg, registry) = config::load_from_db(None)?;
    let db_path = db.path().to_path_buf();
    let paths = cfg.resolve_paths(None);
    paths.create_all()?;

    // dev 빌드 + credential 0건 시 현재 환경 기반 claude_cli credential 자동 주입 (in-memory만).
    // settings.db 미저장 — release 환경 오염 방지. 매 dev 실행 시 재주입.
    #[cfg(debug_assertions)]
    if cfg.credentials.is_empty() {
        if let Some(seed) = file_pipeline_shared::dev_seed_credential() {
            cfg.llm.default_credential = Some(seed.name.clone());
            cfg.llm.provider = seed.provider.clone();
            cfg.credentials.push(seed);
            file_pipeline_shared::write_log(
                "INFO",
                "[dev-seed] credential 0건 → claude_cli in-memory seed 주입",
            );
        }
    }

    // 초기화 상태를 로그 파일에 기록
    file_pipeline_shared::write_log("INFO", &format!("[1/4] 설정 로드 ({})", db_path.display()));
    file_pipeline_shared::write_log("INFO", &format!("[2/4] 벡터 DB: {}", cfg.vector_db.backend));
    let (progress_tx, _) = tokio::sync::broadcast::channel::<String>(256);
    let mut service = file_pipeline_shared::build_service(&cfg, &paths, registry)?;
    let tx = progress_tx.clone();
    service.progress_callback = Some(Arc::new(move |msg: &str| {
        let _ = tx.send(msg.to_string());
        // logs/pipeline.log에도 핵심 이벤트만 기록 (사이드 발견 1)
        // start/done은 INFO, error는 ERROR, step은 너무 verbose라 제외
        if msg.contains("\"event\":\"done\"") || msg.contains("\"event\":\"start\"") {
            file_pipeline_shared::write_log("INFO", msg);
        } else if msg.contains("\"event\":\"error\"") || msg.contains("\"event\":\"fragment\"") {
            file_pipeline_shared::write_log("WARN", msg);
        }
    }));
    file_pipeline_shared::write_log("INFO", &format!("[3/4] 서비스 구성 완료 (LLM: {}, inbox: {})", cfg.llm.provider, paths.inbox.display()));

    let topics_dir = paths.processed.clone();
    file_pipeline_shared::write_log("INFO", "[4/4] 초기화 완료");

    Ok(AppState {
        service: Arc::new(service),
        config: Arc::new(tokio::sync::RwLock::new(cfg)),
        settings_db_path: db_path,
        topics_dir,
        verification_metrics: Arc::new(tokio::sync::RwLock::new(vec![])),
        progress_tx: Some(progress_tx),
        watcher_active: Arc::new(std::sync::atomic::AtomicBool::new(true)),
    })
}

/// 백그라운드 작업 시작 — BackgroundRef 사용 (별도 스레드용)
pub async fn start_background_tasks_standalone(bg: crate::state::BackgroundRef) -> Result<()> {
    file_pipeline_shared::write_log("INFO", "백그라운드 서비스 시작");
    let cfg = bg.config.read().await.clone();
    let paths = cfg.resolve_paths(None);
    file_pipeline_shared::write_log("INFO", &format!("inbox: {}, processed: {}", paths.inbox.display(), paths.processed.display()));

    // 파이프라인 정의 로드
    let pipeline = cfg.pipelines.clone();

    // 크레덴셜 → LLM 어댑터 매핑 빌드
    let credential_llms: std::collections::HashMap<String, Arc<dyn file_pipeline_core::ports::output::LLMPort>> =
        cfg.credentials.iter().filter_map(|cred| {
            file_pipeline_shared::build_llm_from_credential(cred)
                .map(|llm| (cred.name.clone(), llm))
        }).collect();
    if !credential_llms.is_empty() {
        info!("크레덴셜 LLM 어댑터 {} 개 빌드", credential_llms.len());
    }

    // 1. 초기 배치 처리
    {
        let batch_watcher = file_pipeline_adapters::driving::watcher::FileWatcher::new(
            paths.inbox.clone(),
            Arc::clone(&bg.service),
        ).with_max_workers(cfg.max_workers)
         .with_pipeline(pipeline.clone())
         .with_credential_llms(credential_llms.clone());
        match batch_watcher.batch_process().await {
            Ok(stats) => {
                let msg = format!("초기 배치: 완료 {} / 실패 {} / 스킵 {}", stats.done, stats.failed, stats.pending);
                info!("{}", msg);
                file_pipeline_shared::write_log("INFO", &msg);
            }
            Err(e) => {
                let msg = format!("초기 배치 실패: {}", e);
                warn!("{}", msg);
                file_pipeline_shared::write_log("ERROR", &msg);
            }
        }
    }

    // 2. 주기 lint (다층: 매일=색인 정합성 / 주1회=강한 주장 / 월1회=토픽 모순)
    if cfg.schedule.lint_interval_hours > 0 {
        let svc = Arc::clone(&bg.service);
        let interval_h = cfg.schedule.lint_interval_hours;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_h * 3600)).await;
                match file_pipeline_core::domain::lint::Linter::lint(svc.vector_db.as_ref()) {
                    Ok(report) => {
                        if !report.issues.is_empty() {
                            info!("lint: {} 이슈 발견", report.issues.len());
                        }
                    }
                    Err(e) => warn!("lint 실패: {}", e),
                }
            }
        });
    }

    // 2-W. 주 1회 lint — 강한 주장 검출 (Phase 87 wikidocs 353407, max_per_doc=5)
    if cfg.schedule.lint_weekly_hours > 0 {
        let svc = Arc::clone(&bg.service);
        let interval_h = cfg.schedule.lint_weekly_hours;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_h * 3600)).await;
                match file_pipeline_core::domain::lint::Linter::lint_strong_claims(
                    svc.vector_db.as_ref(),
                    svc.storage.as_ref(),
                    5,
                ) {
                    Ok(issues) if !issues.is_empty() => {
                        info!("[lint-weekly] 강한 주장 {}건 발견", issues.len());
                    }
                    Ok(_) => tracing::debug!("[lint-weekly] 이슈 없음"),
                    Err(e) => warn!("[lint-weekly] 실패: {}", e),
                }
            }
        });
    }

    // 2-M. 월 1회 lint — 토픽 디렉토리 모순 마크 검사
    if cfg.schedule.lint_monthly_hours > 0 {
        let topics_dir = paths.processed.clone();
        let interval_h = cfg.schedule.lint_monthly_hours;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_h * 3600)).await;
                match file_pipeline_core::domain::lint::Linter::lint_topics(&topics_dir) {
                    Ok(issues) if !issues.is_empty() => {
                        info!("[lint-monthly] 토픽 모순 {}건 발견", issues.len());
                    }
                    Ok(_) => tracing::debug!("[lint-monthly] 이슈 없음"),
                    Err(e) => warn!("[lint-monthly] 실패: {}", e),
                }
            }
        });
    }

    // 3. Ruflo C1: 자동 추천 주기 트리거 + A1 LLM 캐시 LRU GC
    if cfg.schedule.auto_suggest_interval_hours > 0 {
        let interval_h = cfg.schedule.auto_suggest_interval_hours;
        let db_path = paths.base.join("settings.db");
        let cache_max = cfg.llm.llm_cache_max_entries;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_h * 3600)).await;
                match file_pipeline_shared::settings_db::SettingsDb::open(&db_path) {
                    Ok(db) => {
                        match file_pipeline_shared::auto_suggester::suggest_from_counters(&db) {
                            Ok(n) if n > 0 => info!("[c1-periodic] 자동 추천 {}건 INSERT", n),
                            Ok(_) => tracing::debug!("[c1-periodic] 임계값 미달"),
                            Err(e) => warn!("[c1-periodic] 자동 추천 실패: {}", e),
                        }
                        // A1 LRU GC — max_entries 초과 시 가장 오래된 것부터 삭제.
                        // 결과는 llm_cache_gc_log에 기록하여 GUI stat 카드가 마지막 GC 시각/건수 노출.
                        match db.gc_llm_cache_to(cache_max) {
                            Ok(n) => {
                                if n > 0 { info!("[a1-gc] LLM 캐시 LRU 삭제 {}건", n); }
                                else { tracing::debug!("[a1-gc] 캐시 크기 정상"); }
                                let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
                                let _ = db.record_llm_cache_gc(&now, n as i64);
                            }
                            Err(e) => warn!("[a1-gc] LRU GC 실패: {}", e),
                        }

                        // Phase 94 H1: audit_trace 이상 패턴 주기 분석 (메타 룰 13 3단계 진척).
                        // 결과는 로그 + GUI Verification 탭 anomaly-report-card에서 사용자가 확인.
                        let thresholds = file_pipeline_shared::audit_anomaly::AnomalyThresholds::default();
                        match file_pipeline_shared::audit_anomaly::analyze_recent_audit(&db, &thresholds) {
                            Ok(report) if report.has_anomaly() => {
                                warn!("[h1-anomaly] 이상 신호 {}건 (검토 권고 — 자동 롤백 아님)", report.signals.len());
                                for s in &report.signals {
                                    warn!("[h1-anomaly] {} / stage={}: {}", s.kind, s.stage, s.summary);
                                }
                            }
                            Ok(report) => tracing::debug!("[h1-anomaly] 정상 ({}건 분석)", report.examined_events),
                            Err(e) => warn!("[h1-anomaly] 분석 실패: {}", e),
                        }
                    }
                    Err(e) => warn!("[c1-periodic] settings.db 열기 실패: {}", e),
                }
            }
        });
    }

    // 4. inbox 실시간 감시 (메인 + extra)
    let watcher = file_pipeline_adapters::driving::watcher::FileWatcher::new(
        paths.inbox.clone(),
        Arc::clone(&bg.service),
    )
    .with_max_workers(cfg.max_workers)
    .with_extra_inboxes(paths.extra_inboxes.clone())
    .with_pipeline(pipeline)
    .with_credential_llms(credential_llms.clone());
    watcher.watch().await?;

    Ok(())
}

/// 백그라운드 작업 시작 (inbox 감시 + 스케줄러)
#[allow(dead_code)]
pub async fn start_background_tasks(state: &AppState) -> Result<()> {
    let cfg = state.config.read().await.clone();
    let paths = cfg.resolve_paths(None);

    let pipelines2 = cfg.pipelines.clone();

    let credential_llms: std::collections::HashMap<String, Arc<dyn file_pipeline_core::ports::output::LLMPort>> =
        cfg.credentials.iter().filter_map(|cred| {
            file_pipeline_shared::build_llm_from_credential(cred)
                .map(|llm| (cred.name.clone(), llm))
        }).collect();

    // 1. 초기 배치 처리
    {
        let batch_watcher = file_pipeline_adapters::driving::watcher::FileWatcher::new(
            paths.inbox.clone(),
            Arc::clone(&state.service),
        ).with_max_workers(cfg.max_workers)
         .with_pipeline(pipelines2.clone())
         .with_credential_llms(credential_llms.clone());
        match batch_watcher.batch_process().await {
            Ok(stats) => {
                let msg = format!("초기 배치: 완료 {} / 실패 {} / 스킵 {}", stats.done, stats.failed, stats.pending);
                info!("{}", msg);
                file_pipeline_shared::write_log("INFO", &msg);
            }
            Err(e) => {
                let msg = format!("초기 배치 실패: {}", e);
                warn!("{}", msg);
                file_pipeline_shared::write_log("ERROR", &msg);
            }
        }
    }

    // 2. 주기 lint 스케줄러 (다층)
    if cfg.schedule.lint_interval_hours > 0 {
        let svc = Arc::clone(&state.service);
        let interval_h = cfg.schedule.lint_interval_hours;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_h * 3600)).await;
                match file_pipeline_core::domain::lint::Linter::lint(svc.vector_db.as_ref()) {
                    Ok(report) => {
                        if !report.issues.is_empty() {
                            info!("lint: {} 이슈 발견", report.issues.len());
                        }
                    }
                    Err(e) => warn!("lint 실패: {}", e),
                }
            }
        });
    }

    if cfg.schedule.lint_weekly_hours > 0 {
        let svc = Arc::clone(&state.service);
        let interval_h = cfg.schedule.lint_weekly_hours;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_h * 3600)).await;
                match file_pipeline_core::domain::lint::Linter::lint_strong_claims(
                    svc.vector_db.as_ref(),
                    svc.storage.as_ref(),
                    5,
                ) {
                    Ok(issues) if !issues.is_empty() => {
                        info!("[lint-weekly] 강한 주장 {}건 발견", issues.len());
                    }
                    Ok(_) => tracing::debug!("[lint-weekly] 이슈 없음"),
                    Err(e) => warn!("[lint-weekly] 실패: {}", e),
                }
            }
        });
    }

    if cfg.schedule.lint_monthly_hours > 0 {
        let topics_dir = paths.processed.clone();
        let interval_h = cfg.schedule.lint_monthly_hours;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_h * 3600)).await;
                match file_pipeline_core::domain::lint::Linter::lint_topics(&topics_dir) {
                    Ok(issues) if !issues.is_empty() => {
                        info!("[lint-monthly] 토픽 모순 {}건 발견", issues.len());
                    }
                    Ok(_) => tracing::debug!("[lint-monthly] 이슈 없음"),
                    Err(e) => warn!("[lint-monthly] 실패: {}", e),
                }
            }
        });
    }

    // 4. inbox 실시간 감시
    let watcher = file_pipeline_adapters::driving::watcher::FileWatcher::new(
        paths.inbox.clone(),
        Arc::clone(&state.service),
    ).with_max_workers(cfg.max_workers)
     .with_pipeline(pipelines2)
     .with_credential_llms(credential_llms.clone());
    watcher.watch().await?;

    Ok(())
}
