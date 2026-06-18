mod daemon;

use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use file_pipeline_adapters::driving::terminal_resolution::TerminalDuplicateResolution;
use file_pipeline_adapters::driving::terminal_sensitive::TerminalSensitiveNotification;
use file_pipeline_core::domain::models::DocTypeRegistry;
use file_pipeline_core::ports::output::RerankerPort;
use file_pipeline_core::service::FileProcessingService;

use file_pipeline_shared::{config, build_service};
use file_pipeline_shared::config::{PipelineConfigExt, ResolvedPathsExt};
use file_pipeline_adapters::driving::tray;

#[derive(Parser)]
#[command(name = "pipeline", version, about = "File Processing Pipeline")]
struct Cli {
    /// 설정 파일 경로
    #[arg(long)]
    config: Option<String>,

    /// 데이터 루트 경로 오버라이드
    #[arg(long)]
    base: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// pipeline.toml 템플릿 생성
    Init,

    /// 솔루션 시작 (watch + batch + Tauri앱 + 주기 lint/purge + topic-merge)
    Start,

    /// 현재 적용 설정 출력
    ShowConfig,

    /// 통계 출력
    Stats,

    /// 코퍼스 진단 (관계 분포, 품질, 헬스체크)
    Doctor {
        /// JSON 스냅샷 저장 (spec/benchmarks/ 디렉토리)
        #[arg(long)]
        save: bool,
    },

    /// Obsidian 내보내기 (위키 + 그래프)
    Export {
        /// 내보내기 출력 디렉토리
        #[arg(long, default_value = "wiki")]
        output: String,
        /// Obsidian vault 경로 (지정 시 직접 복사)
        #[arg(long)]
        obsidian_vault: Option<String>,
    },

    /// 메모 직접 입력 (inbox에 텍스트 파일 생성)
    Memo {
        /// 메모 내용
        text: String,
    },

    /// 토픽 페이지 수정 요청 (사용자 피드백 → LLM 수정)
    TopicRevise {
        /// 수정할 토픽 파일 경로
        file: String,
        /// 수정 피드백
        #[arg(short, long)]
        feedback: String,
    },

    /// 할일 관리
    Todo {
        #[command(subcommand)]
        action: TodoAction,
    },

    /// 지식 그래프 조회
    Kg {
        #[command(subcommand)]
        action: KgAction,
    },

    /// 파일 가공 (inbox 배치 처리)
    Process,

    /// 검색
    Search {
        /// 검색 쿼리
        query: String,
        /// 검색 모드 (default/exact/related/recent/fusion)
        #[arg(long, default_value = "default")]
        mode: String,
        /// 문서 유형 필터
        #[arg(long, short = 't')]
        doc_type: Option<String>,
        /// 시작 날짜 (YYYY-MM-DD)
        #[arg(long)]
        after: Option<String>,
        /// 결과 수
        #[arg(long, default_value = "5")]
        top_k: usize,
        /// JSON 출력
        #[arg(long)]
        json: bool,
    },

    /// 설정 관리
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// 골든셋 관리 (검색 품질 모니터링)
    Golden {
        #[command(subcommand)]
        action: GoldenAction,
    },

    /// 벤치마크 실행
    Bench {
        /// 문서 수 (기본 100)
        #[arg(default_value = "100")]
        count: usize,
    },

    /// 기존 Qdrant 벡터를 .vec 파일로 추출 (프로바이더 변경 시)
    BackfillVec,

    /// 서비스 관리 (OS 부팅 시 자동실행)
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },
}

#[derive(Subcommand)]
enum ServiceAction {
    /// OS 서비스 등록 (부팅 시 자동시작)
    Install {
        /// Task Scheduler 사용 (관리자 권한 불필요)
        #[arg(long)]
        task_scheduler: bool,
    },
    /// 서비스 시작
    Start,
    /// 서비스 중지
    Stop,
    /// 서비스 상태
    Status,
    /// 로그 출력
    Logs,
    /// 서비스 제거
    Uninstall,
    /// 감시 대상 디렉토리 추가
    Add {
        /// 추가할 inbox 디렉토리 경로
        path: String,
    },
    /// 감시 대상 디렉토리 제거
    Remove {
        /// 제거할 inbox 디렉토리 경로
        path: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// 설정 값 조회
    Get {
        /// section.key 형식 (예: compression.zstd_level)
        key: String,
    },
    /// 설정 값 변경
    Set {
        /// section.key 형식
        key: String,
        /// 값
        value: String,
    },
}

#[derive(Subcommand)]
enum GoldenAction {
    /// 골든셋 쌍 추가
    Add {
        /// 검색 쿼리
        query: String,
        /// 기대 문서 ID
        doc_id: String,
    },
    /// 골든셋 목록
    List,
    /// MRR 평가
    Eval,
}

#[derive(Subcommand)]
enum TodoAction {
    /// 미완료 할일 목록
    List,
    /// 할일 완료 처리
    Done {
        /// todo ID
        id: String,
    },
    /// 할일 스킵
    Skip {
        /// todo ID
        id: String,
        /// 사유
        #[arg(long)]
        reason: Option<String>,
    },
    /// 할일 재오픈
    Reopen {
        /// todo ID
        id: String,
    },
    /// 수동 할일 추가
    Add {
        /// 제목
        title: String,
        /// 카테고리
        #[arg(long, default_value = "manual")]
        category: String,
        /// 기한
        #[arg(long)]
        due: Option<String>,
    },
}

#[derive(Subcommand)]
enum KgAction {
    /// 문서의 관계 이웃 조회
    Neighbors {
        /// 문서 ID
        doc_id: String,
    },
    /// 두 문서 간 경로 탐색
    Paths {
        /// 시작 문서 ID
        source: String,
        /// 대상 문서 ID
        target: String,
    },
    /// 그래프 전체 통계
    Stats,
}

fn init_logging(level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // auto-init: pipeline.toml 없으면 자동 생성
    let config_path = config::find_config_path(cli.config.as_deref());
    if !config_path.exists() {
        let default_cfg = config::PipelineConfig::default_config();
        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&config_path, default_cfg.to_toml_string().unwrap_or_default());
        eprintln!("[init] pipeline.toml 생성: {}", config_path.display());
    }

    let cfg = config::find_and_load_config(cli.config.as_deref())?;

    init_logging(&cfg.logging.level);

    let paths = cfg.resolve_paths(cli.base.as_deref());

    // 디렉토리 자동 생성 (inbox/processed/originals 등)
    if let Err(e) = paths.create_all() {
        eprintln!("[init] 디렉토리 생성 실패: {}", e);
    }

    match cli.command {
        Commands::Init => {
            let default_cfg = config::PipelineConfig::default_config();
            let toml_str = default_cfg.to_toml_string()?;
            let out_path = "pipeline.toml";
            std::fs::write(out_path, &toml_str)?;
            println!("pipeline.toml 생성 완료");
        }

        Commands::ShowConfig => {
            println!("=== 설정 (version={}) ===", cfg.version);
            println!("{}", cfg.to_toml_string()?);
            println!("\n=== 경로 ===");
            println!("base      : {}", paths.base.display());
            println!("inbox     : {}", paths.inbox.display());
            println!("processed : {}", paths.processed.display());
            println!("originals : {}", paths.originals.display());
            println!("sensitive : {}", paths.sensitive.display());
            println!("todo      : {}", paths.todo.display());
            println!("temp      : {}", paths.temp.display());
            println!("logs      : {}", paths.logs.display());

            let doc_types_path = config::resolve_doc_types_path(&paths);
            println!("doc_types : {}", doc_types_path.display());
        }

        Commands::Start => {
            // auto-init: pipeline.toml 없으면 자동 생성
            let config_path = config::find_config_path(cli.config.as_deref());
            if !config_path.exists() {
                let default_cfg = config::PipelineConfig::default_config();
                std::fs::write(&config_path, default_cfg.to_toml_string()?)?;
                info!("pipeline.toml 자동 생성: {}", config_path.display());
            }

            paths.create_all()?;

            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service_cli(&cfg, &paths, registry)?;
            let service = Arc::new(service);

            info!("=== file-pipeline 시작 ===");

            // 시스템 트레이 (Windows)
            let _tray = match tray::windows_tray::TrayManager::new() {
                Ok(t) => {
                    info!("시스템 트레이 활성화");
                    Some(t)
                }
                Err(e) => {
                    info!("시스템 트레이 비활성: {}", e);
                    None
                }
            };

            // 트레이 이벤트 폴링 (백그라운드)
            if let Some(ref _tray_ref) = _tray {
                let inbox_dir = paths.inbox.clone();
                let tray_quit = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let tray_quit_clone = tray_quit.clone();
                // 주의: tray 이벤트는 메인 스레드에서만 수신 가능 (Windows 제약)
                // Dashboard 서버가 메인 스레드를 점유하므로, 트레이 Quit은 프로세스 종료로 처리
                let _ = (tray_quit, tray_quit_clone, inbox_dir); // 향후 트레이 이벤트 처리용
            }

            // 파이프라인 정의 로드
            let pipeline = cfg.pipelines.clone();

            // 크레덴셜 → LLM 어댑터 매핑
            let credential_llms: std::collections::HashMap<String, Arc<dyn file_pipeline_core::ports::output::LLMPort>> =
                cfg.credentials.iter().filter_map(|cred| {
                    file_pipeline_shared::build_llm_from_credential(cred)
                        .map(|llm| (cred.name.clone(), llm))
                }).collect();

            // 1. 배치 처리 (기존 inbox 파일)
            {
                let batch_watcher = file_pipeline_adapters::driving::watcher::FileWatcher::new(
                    paths.inbox.clone(), Arc::clone(&service),
                ).with_max_workers(cfg.max_workers)
                 .with_pipeline(pipeline.clone())
                 .with_credential_llms(credential_llms.clone());
                match batch_watcher.batch_process().await {
                    Ok(stats) => {
                        info!("초기 배치: 완료 {} / 실패 {} / 스킵 {}", stats.done, stats.failed, stats.pending);
                        // 배치 완료 후 교차참조 일괄 처리
                        match batch_watcher.flush_crossref() {
                            Ok(n) if n > 0 => info!("초기 교차참조: {} 건 처리", n),
                            Err(e) => warn!("초기 교차참조 실패: {}", e),
                            _ => {}
                        }
                    }
                    Err(e) => warn!("초기 배치 실패: {}", e),
                }
            }

            // 2. Tauri 앱은 별도 프로세스 (tauri/ 디렉토리)
            // pipeline start는 백그라운드 서비스로 동작
            info!("서비스 시작 — Tauri 앱: tauri/ 디렉토리에서 별도 실행");

            // 3. 주기 lint 스케줄러 (다층: 매일/주1회/월1회, wikidocs 353407)
            if cfg.schedule.lint_interval_hours > 0 {
                let lint_svc = Arc::clone(&service);
                let lint_h = cfg.schedule.lint_interval_hours;
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(lint_h * 3600)).await;
                        if let Ok(report) = file_pipeline_core::domain::lint::Linter::lint(lint_svc.vector_db.as_ref()) {
                            if !report.issues.is_empty() {
                                info!("주기 lint: {} 이슈", report.issues.len());
                            }
                        }
                    }
                });
            }

            if cfg.schedule.lint_weekly_hours > 0 {
                let lint_svc = Arc::clone(&service);
                let interval_h = cfg.schedule.lint_weekly_hours;
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(interval_h * 3600)).await;
                        match file_pipeline_core::domain::lint::Linter::lint_strong_claims(
                            lint_svc.vector_db.as_ref(),
                            lint_svc.storage.as_ref(),
                            5,
                        ) {
                            Ok(issues) if !issues.is_empty() => {
                                info!("[lint-weekly] 강한 주장 {}건 발견", issues.len());
                            }
                            Ok(_) => tracing::debug!("[lint-weekly] 이슈 없음"),
                            Err(e) => tracing::warn!("[lint-weekly] 실패: {}", e),
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
                            Err(e) => tracing::warn!("[lint-monthly] 실패: {}", e),
                        }
                    }
                });
            }

            // 4. doc_types.toml 변경 감시 → AutoReindexer
            let doc_types_watch_path = doc_types_path.clone();
            let reindex_service = Arc::clone(&service);
            tokio::spawn(async move {
                use notify::{RecommendedWatcher, RecursiveMode, Watcher, EventKind};
                let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
                let tx_clone = tx.clone();
                let mut dt_watcher = match RecommendedWatcher::new(
                    move |res: Result<notify::Event, notify::Error>| {
                        if let Ok(event) = res {
                            if matches!(event.kind, EventKind::Modify(_)) {
                                let _ = tx_clone.blocking_send(());
                            }
                        }
                    },
                    notify::Config::default(),
                ) { Ok(w) => w, Err(_) => return };
                if dt_watcher.watch(&doc_types_watch_path, RecursiveMode::NonRecursive).is_err() { return; }
                while rx.recv().await.is_some() {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    while rx.try_recv().is_ok() {}
                    if let Ok(new_registry) = config::load_doc_type_registry(&doc_types_watch_path) {
                        let _ = file_pipeline_core::domain::auto_reindexer::AutoReindexer::reindex_all(
                            reindex_service.vector_db.as_ref(), reindex_service.llm.as_ref(),
                            reindex_service.storage.as_ref(), &new_registry,
                        ).await;
                    }
                }
            });

            // 5. inbox watch (실시간 감시)
            let watch_service = Arc::clone(&service);
            let watch_inbox = paths.inbox.clone();
            let watch_topics = paths.base.join("topics");
            let max_workers = cfg.max_workers;
            let watch_pipeline = pipeline;
            tokio::spawn(async move {
                let watcher = file_pipeline_adapters::driving::watcher::FileWatcher::new(
                    watch_inbox, watch_service,
                ).with_max_workers(max_workers)
                 .with_auto_merge(watch_topics, 5)
                 .with_pipeline(watch_pipeline)
                 .with_credential_llms(credential_llms);
                if let Err(e) = watcher.watch().await {
                    tracing::error!("watch 오류: {}", e);
                }
            });

            // 6. 메인 스레드 대기 (서비스 유지)
            info!("file-pipeline 서비스 실행 중 (Ctrl+C로 종료)");
            tokio::signal::ctrl_c().await?;
            info!("종료 신호 수신 — 서비스 중지");
        }

        Commands::Export { output, obsidian_vault } => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service_cli(&cfg, &paths, registry)?;

            let output_path = std::path::Path::new(&output);
            let report = file_pipeline_core::domain::wiki_export::WikiExporter::export(
                service.vector_db.as_ref(),
                service.storage.as_ref(),
                output_path,
            )?;
            println!("=== 위키 내보내기 완료 ===");
            println!("총 문서: {}", report.total);
            println!("내보냄: {}", report.exported);
            if !report.errors.is_empty() {
                println!("오류: {} 건", report.errors.len());
                for e in &report.errors {
                    println!("  - {}", e);
                }
            }
            println!("출력: {}", output_path.display());

            // Obsidian vault로 복사
            if let Some(ref vault) = obsidian_vault {
                let vault_path = std::path::Path::new(vault);
                let dest = vault_path.join("file-pipeline");
                if let Err(e) = copy_dir_recursive(output_path, &dest) {
                    println!("Obsidian vault 복사 실패: {}", e);
                } else {
                    println!("Obsidian vault 동기화: {}", dest.display());
                }
            }
        }

        Commands::Memo { text } => {
            paths.create_all()?;
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let filename = format!("memo_{}.txt", timestamp);
            let memo_path = paths.inbox.join(&filename);
            std::fs::write(&memo_path, &text)?;
            println!("메모 저장: {} ({} 자)", memo_path.display(), text.len());
            println!("inbox에 저장됨 — watch 중이면 자동 처리됩니다.");
        }

        Commands::TopicRevise { file, feedback } => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service_cli(&cfg, &paths, registry)?;

            let topic_path = std::path::Path::new(&file);
            if !topic_path.exists() {
                anyhow::bail!("토픽 파일 없음: {}", file);
            }

            println!("토픽 수정 요청: {}", file);
            println!("피드백: {}", feedback);

            let revised = file_pipeline_core::domain::topic_merger::TopicMerger::revise_topic(
                topic_path,
                &feedback,
                service.llm.as_ref(),
            )
            .await?;

            println!("=== 수정 완료 ===");
            println!("{}", &revised[..revised.len().min(500)]);
            if revised.len() > 500 {
                println!("... ({} 자)", revised.len());
            }
        }

        Commands::Stats => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service_cli(&cfg, &paths, registry)?;
            let stats = service.vector_db.stats()?;
            println!("=== 통계 ===");
            println!("총 문서 수: {}", stats.total_documents);
            for (doc_type, count) in &stats.by_type {
                println!("  {}: {}", doc_type, count);
            }
        }

        Commands::Doctor { save } => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service_cli(&cfg, &paths, registry)?;

            use file_pipeline_core::domain::diagnostics;
            let stats = diagnostics::analyze_corpus(service.vector_db.as_ref())?;
            let issues = diagnostics::health_check(&stats);
            let report = diagnostics::format_report(&stats, &issues);
            println!("{}", report);

            if save {
                let bench_dir = std::path::Path::new("spec/benchmarks");
                let _ = std::fs::create_dir_all(bench_dir);
                let filename = format!("{}.json", chrono::Local::now().format("%Y-%m-%d"));
                let path = bench_dir.join(&filename);
                let json = serde_json::to_string_pretty(&stats)?;
                std::fs::write(&path, json)?;
                println!("스냅샷 저장: {}", path.display());
            }
        }

        Commands::Todo { action } => {
            let data_dir = config::find_data_dir(cli.config.as_deref());
            let db = file_pipeline_shared::settings_db::SettingsDb::open(&data_dir.join("settings.db"))?;
            match action {
                TodoAction::List => {
                    let todos = db.list_todos(Some("open"), None)?;
                    if todos.is_empty() {
                        println!("할일 없음");
                    } else {
                        for t in &todos {
                            let cat = t["category"].as_str().unwrap_or("-");
                            let title = t["title"].as_str().unwrap_or("");
                            let date = t["created_at"].as_str().unwrap_or("").chars().take(10).collect::<String>();
                            let id = t["id"].as_str().unwrap_or("");
                            println!("  [{}] {} — {} ({})", cat, title, date, id);
                        }
                        println!("\n총 {} 건 (open)", todos.len());
                    }
                }
                TodoAction::Done { id } => {
                    if db.complete_todo(&id)? {
                        println!("완료: {}", id);
                    } else {
                        println!("항목 없음 또는 이미 완료: {}", id);
                    }
                }
                TodoAction::Skip { id, reason } => {
                    if db.skip_todo(&id, reason.as_deref())? {
                        println!("스킵: {}", id);
                    } else {
                        println!("항목 없음: {}", id);
                    }
                }
                TodoAction::Reopen { id } => {
                    if db.reopen_todo(&id)? {
                        println!("재오픈: {}", id);
                    } else {
                        println!("항목 없음: {}", id);
                    }
                }
                TodoAction::Add { title, category, due } => {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(format!("manual:{}", title.to_lowercase()).as_bytes());
                    let fp = hex::encode(hasher.finalize());
                    match db.add_todo(file_pipeline_shared::settings_db::NewTodo {
                        title: &title, category: &category, doc_id: None, doc_description: None,
                        fingerprint: &fp, source_line: None, source_text: None,
                        due_date: due.as_deref(),
                    })? {
                        Some(id) => println!("추가: {} ({})", title, id),
                        None => println!("이미 존재: {}", title),
                    }
                }
            }
        }

        Commands::Kg { action } => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service_cli(&cfg, &paths, registry)?;

            match action {
                KgAction::Neighbors { doc_id } => {
                    match file_pipeline_core::domain::wiki_export::KgQueryEngine::neighbors(
                        service.vector_db.as_ref(), &doc_id,
                    ) {
                        Ok(result) => {
                            println!("=== {} 이웃 ===", doc_id);
                            let json = serde_json::to_string_pretty(&result)?;
                            println!("{}", json);
                        }
                        Err(e) => println!("오류: {}", e),
                    }
                }
                KgAction::Paths { source, target } => {
                    match file_pipeline_core::domain::wiki_export::KgQueryEngine::find_paths(
                        service.vector_db.as_ref(), &source, &target,
                    ) {
                        Ok(result) => {
                            println!("=== {} → {} 경로 ===", source, target);
                            let json = serde_json::to_string_pretty(&result)?;
                            println!("{}", json);
                        }
                        Err(e) => println!("오류: {}", e),
                    }
                }
                KgAction::Stats => {
                    match file_pipeline_core::domain::wiki_export::KgQueryEngine::stats(
                        service.vector_db.as_ref(),
                    ) {
                        Ok(stats) => {
                            println!("=== 지식 그래프 통계 ===");
                            let json = serde_json::to_string_pretty(&stats)?;
                            println!("{}", json);
                        }
                        Err(e) => println!("오류: {}", e),
                    }
                }
            }
        }

        Commands::Process => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service_cli(&cfg, &paths, registry)?;
            let service = Arc::new(service);

            let watcher = file_pipeline_adapters::driving::watcher::FileWatcher::new(
                paths.inbox.clone(), Arc::clone(&service),
            ).with_max_workers(cfg.max_workers)
             .with_pipeline(cfg.pipelines.clone());

            service.vector_db.batch_begin();
            service.compile_state_batch_begin();
            match watcher.batch_process().await {
                Ok(stats) => println!("가공 완료: done={}, failed={}, pending={}", stats.done, stats.failed, stats.pending),
                Err(e) => println!("가공 실패: {}", e),
            }
            service.vector_db.batch_end();
            service.compile_state_batch_end();
            let _ = service.flush_crossref();
            if service.vector_db.has_pending_work() {
                service.vector_db.db_refresh();
            }
        }

        Commands::Search { query, mode, doc_type, after, top_k, json } => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service_cli(&cfg, &paths, registry)?;

            let embedding = service.embedding.embed(&query).await?;
            let mut results = match mode.as_str() {
                "exact" => service.vector_db.search_hybrid(&embedding, &query, top_k * 3)?,
                _ => service.vector_db.search_similar(&embedding, top_k * 3)?,
            };

            // 필터
            if let Some(ref dt) = doc_type {
                results.retain(|r| r.doc_types.iter().any(|t| t == dt));
            }
            if let Some(ref after_date) = after {
                results.retain(|r| r.date.as_str() >= after_date.as_str());
            }

            // 리랭킹
            if cfg.rerank.enabled && !results.is_empty() {
                if let Ok(reranked) = file_pipeline_adapters::driven::rerank::claude_reranker::ClaudeReranker::new(cfg.rerank.top_n)
                    .rerank(&query, results.clone()).await {
                    results = reranked;
                }
            }

            results.truncate(top_k);

            if json {
                let out: Vec<serde_json::Value> = results.iter().map(|r| {
                    serde_json::json!({"id": r.id, "score": r.score, "doc_types": r.doc_types, "date": r.date})
                }).collect();
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else if results.is_empty() {
                println!("검색 결과 없음");
            } else {
                for (i, r) in results.iter().enumerate() {
                    let header = service.storage.read_header(&r.path, 3).unwrap_or_default();
                    let preview = header.lines().next().unwrap_or("").trim();
                    println!("  {}. [{}] {:.3} {} — {}",
                        i + 1, r.doc_types.join(","), r.score, r.date, preview);
                }
            }
        }

        Commands::Config { action } => {
            match action {
                ConfigAction::Get { key } => {
                    let parts: Vec<&str> = key.splitn(2, '.').collect();
                    if parts.len() != 2 {
                        println!("형식: section.key (예: compression.zstd_level)");
                    } else {
                        let data_dir = config::find_data_dir(cli.config.as_deref());
                        let db = file_pipeline_shared::settings_db::SettingsDb::open(&data_dir.join("settings.db"))?;
                        match db.get_config(parts[0], parts[1])? {
                            Some(val) => println!("{}", val),
                            None => println!("(미설정)"),
                        }
                    }
                }
                ConfigAction::Set { key, value } => {
                    let parts: Vec<&str> = key.splitn(2, '.').collect();
                    if parts.len() != 2 {
                        println!("형식: section.key value (예: compression.zstd_level 5)");
                    } else {
                        let data_dir = config::find_data_dir(cli.config.as_deref());
                        let db = file_pipeline_shared::settings_db::SettingsDb::open(&data_dir.join("settings.db"))?;
                        db.set_config(parts[0], parts[1], &value)?;
                        println!("설정 변경: {}.{} = {}", parts[0], parts[1], value);
                    }
                }
            }
        }

        Commands::Golden { action } => {
            let data_dir = config::find_data_dir(cli.config.as_deref());
            let db = file_pipeline_shared::settings_db::SettingsDb::open(&data_dir.join("settings.db"))?;

            match action {
                GoldenAction::Add { query, doc_id } => {
                    db.add_golden_pair(&query, &doc_id, "manual")?;
                    println!("골든셋 추가: \"{}\" → {}", query, doc_id);
                }
                GoldenAction::List => {
                    let pairs = db.list_golden_set()?;
                    if pairs.is_empty() {
                        println!("골든셋 비어있음");
                    } else {
                        for (q, id) in &pairs {
                            println!("  \"{}\" → {}", q, id);
                        }
                        println!("총 {} 쌍", pairs.len());
                    }
                }
                GoldenAction::Eval => {
                    let pairs = db.list_golden_set()?;
                    if pairs.is_empty() {
                        println!("골든셋 비어있음. `pipeline golden add` 로 추가하세요.");
                        return Ok(());
                    }
                    paths.create_all()?;
                    let doc_types_path = config::resolve_doc_types_path(&paths);
                    let registry = config::load_doc_type_registry(&doc_types_path)?;
                    let service = build_service_cli(&cfg, &paths, registry)?;

                    let mut rr_sum = 0.0f64;
                    let mut count = 0;
                    for (query, expected_id) in &pairs {
                        let emb = service.embedding.embed(query).await?;
                        let results = service.vector_db.search_similar(&emb, 20)?;
                        let rank = results.iter().position(|r| r.id == *expected_id);
                        let rr = match rank {
                            Some(r) => 1.0 / (r + 1) as f64,
                            None => 0.0,
                        };
                        rr_sum += rr;
                        count += 1;
                        let rank_str = rank.map(|r| format!("#{}", r + 1)).unwrap_or("miss".into());
                        println!("  [{}] \"{}\" → {}", rank_str, query, expected_id);
                    }
                    let mrr = if count > 0 { rr_sum / count as f64 } else { 0.0 };
                    println!("\nMRR@20 = {:.3} ({} 쌍)", mrr, count);
                }
            }
        }

        Commands::Bench { count } => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service_cli(&cfg, &paths, registry)?;

            println!("벤치마크: {} 문서 stub 가공", count);
            let base = tempfile::TempDir::new()?;
            let inbox = base.path().join("inbox");
            std::fs::create_dir_all(&inbox)?;

            for i in 0..count {
                let name = format!("bench_{:05}.txt", i);
                let content = format!("벤치마크 문서 #{} 2026년 4월 테스트 키워드A 키워드B DOC-{:05}", i, i);
                std::fs::write(inbox.join(&name), &content)?;
            }

            let start = std::time::Instant::now();
            service.vector_db.batch_begin();
            let files: Vec<_> = std::fs::read_dir(&inbox)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .collect();
            for f in &files {
                let _ = service.process_file(f).await;
            }
            service.vector_db.batch_end();
            let _ = service.flush_crossref();
            let secs = start.elapsed().as_secs_f64();

            let stats = service.vector_db.stats()?;
            println!("결과: {} 문서, {:.1}초, {:.1} docs/s",
                stats.total_documents, secs, count as f64 / secs);
        }

        Commands::BackfillVec => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service_cli(&cfg, &paths, registry)?;

            let all = service.vector_db.list_all()?;
            let mut saved = 0usize;
            for doc in &all {
                let vec_path = doc.path.with_extension("vec");
                if vec_path.exists() {
                    continue;
                }
                if let Ok(Some(vector)) = service.vector_db.get_vector(&doc.id) {
                    if let Err(e) = file_pipeline_core::domain::vec_io::save_vec(&vec_path, &vector) {
                        warn!("vec 저장 실패 {}: {}", doc.id, e);
                    } else {
                        saved += 1;
                    }
                }
            }
            println!("backfill-vec 완료: {}/{} 저장", saved, all.len());
        }

        Commands::Service { action } => {
            let cmd = match action {
                ServiceAction::Install { task_scheduler } => {
                    daemon::DaemonCommand::Install {
                        use_task_scheduler: task_scheduler,
                    }
                }
                ServiceAction::Start => daemon::DaemonCommand::Start,
                ServiceAction::Stop => daemon::DaemonCommand::Stop,
                ServiceAction::Status => daemon::DaemonCommand::Status,
                ServiceAction::Logs => daemon::DaemonCommand::Logs,
                ServiceAction::Uninstall => daemon::DaemonCommand::Uninstall,
                ServiceAction::Add { path } => {
                    println!("감시 대상 추가: {}", path);
                    // pipeline.toml paths.extra_inbox에 추가
                    println!("[TODO] pipeline.toml에 extra_inbox 경로 추가 기능 — 추후 구현");
                    return Ok(());
                }
                ServiceAction::Remove { path } => {
                    println!("감시 대상 제거: {}", path);
                    println!("[TODO] pipeline.toml에서 extra_inbox 경로 제거 기능 — 추후 구현");
                    return Ok(());
                }
            };
            daemon::execute(cmd)?;
        }
    }

    Ok(())
}

/// 디렉토리 재귀 복사 (Obsidian vault 동기화용)
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// CLI 전용 build_service — 터미널 모드일 때 실제 TerminalResolution 사용
fn build_service_cli(
    cfg: &config::PipelineConfig,
    paths: &config::ResolvedPaths,
    registry: DocTypeRegistry,
) -> Result<FileProcessingService> {
    let mut service = build_service(cfg, paths, registry)?;

    // CLI: stdin이 터미널이면 대화형 UI 활성화
    if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        service.duplicate_resolution = Arc::new(TerminalDuplicateResolution);
        service.sensitive_notification = Arc::new(TerminalSensitiveNotification);
    }

    Ok(service)
}
