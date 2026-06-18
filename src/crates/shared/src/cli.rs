//! CLI 커맨드 핸들러 — Tauri 단일 바이너리에서 재사용
//!
//! `pipeline stats`, `pipeline memo` 등 비-GUI 커맨드를 처리.
//! Tauri main.rs에서 인자 분기 후 호출.

use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::{config, build_service};
use crate::config::{PipelineConfigExt, ResolvedPathsExt};

#[derive(Parser, Default)]
#[command(name = "pipeline", version, about = "File Processing Pipeline")]
pub struct Cli {
    /// 설정 파일 경로
    #[arg(long)]
    pub config: Option<String>,

    /// 데이터 루트 경로 오버라이드
    #[arg(long)]
    pub base: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// pipeline.toml 템플릿 생성
    Init,

    /// 솔루션 시작 (GUI 모드 — Dashboard + 백그라운드 서비스)
    Start,

    /// 현재 적용 설정 출력
    ShowConfig,

    /// 통계 출력
    Stats,

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

    /// 기존 Qdrant 벡터를 .vec 파일로 추출 (프로바이더 변경 시)
    BackfillVec,

    /// inbox 파일 일괄 가공 (CLI 배치 모드, GUI 없이 처리 후 종료)
    Batch,
}

#[derive(Subcommand)]
pub enum TodoAction {
    /// 미완료 할일 목록
    List,
    /// 할일 완료 처리
    Done {
        /// 완료할 항목 텍스트 (부분 일치)
        text: String,
    },
}

#[derive(Subcommand)]
pub enum KgAction {
    /// 문서의 관계 이웃 조회
    Neighbors { doc_id: String },
    /// 두 문서 간 경로 탐색
    Paths { source: String, target: String },
    /// 그래프 전체 통계
    Stats,
}

/// CLI 커맨드가 GUI가 아닌지 판별 (Start 또는 None은 GUI)
pub fn is_gui_command(cli: &Cli) -> bool {
    matches!(cli.command, None | Some(Commands::Start))
}

/// CLI 모드 실행 (비-GUI 커맨드)
pub async fn execute(cli: Cli) -> Result<()> {
    // auto-init: pipeline.toml 없으면 자동 생성 + 디렉토리 생성
    let config_path = config::find_config_path(cli.config.as_deref());
    let first_run = !config_path.exists();
    if first_run {
        let default_cfg = config::PipelineConfig::default_config();
        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&config_path, default_cfg.to_toml_string().unwrap_or_default());
        eprintln!("[init] pipeline.toml 생성: {}", config_path.display());
    }

    let cfg = config::find_and_load_config(cli.config.as_deref())?;
    let paths = cfg.resolve_paths(cli.base.as_deref());

    // 디렉토리가 없으면 자동 생성 (매 실행 시 확인)
    if let Err(e) = paths.create_all() {
        eprintln!("[init] 디렉토리 생성 실패: {}", e);
    }

    if first_run {
        eprintln!("[init] 준비 완료. inbox에 파일을 넣고 `pipeline process`를 실행하세요.");
        eprintln!("       inbox: {}", paths.inbox.display());
    }

    match cli.command.expect("GUI commands should not reach here") {
        Commands::Start => unreachable!("Start is GUI mode"),

        Commands::Init => {
            let default_cfg = config::PipelineConfig::default_config();
            let toml_str = default_cfg.to_toml_string()?;
            let path = config::find_config_path(None);
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&path, &toml_str)?;
            println!("pipeline.toml 생성: {}", path.display());
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

        Commands::Stats => {
            use file_pipeline_core::ports::output::VectorDBPort;
            // 경량 경로: 벡터DB만 로드 (LLM/임베딩 초기화 스킵)
            // Phase 89 C-1: paths.base 명시 전달 (`--base` CLI 옵션 전파)
            let db = file_pipeline_adapters::driven::vector_db::local_store::LocalVectorStore::with_path(
                paths.base.join(".local-store.json"),
            );
            db.init().expect("LocalVectorStore init");
            let vector_db: Arc<dyn VectorDBPort> = Arc::new(db);
            let stats = vector_db.stats()?;
            println!("=== 통계 ===");
            println!("총 문서 수: {}", stats.total_documents);
            for (doc_type, count) in &stats.by_type {
                println!("  {}: {}", doc_type, count);
            }
        }

        Commands::Export { output, obsidian_vault } => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service(&cfg, &paths, registry)?;
            let output_path = std::path::Path::new(&output);
            let report = file_pipeline_core::domain::wiki_export::WikiExporter::export(
                service.vector_db.as_ref(), service.storage.as_ref(), output_path,
            )?;
            println!("=== 위키 내보내기 완료 ===");
            println!("총 문서: {} | 내보냄: {}", report.total, report.exported);
            if !report.errors.is_empty() {
                for e in &report.errors { println!("  오류: {}", e); }
            }
            if let Some(ref vault) = obsidian_vault {
                let dest = std::path::Path::new(vault).join("file-pipeline");
                copy_dir_recursive(output_path, &dest)?;
                println!("Obsidian vault 동기화: {}", dest.display());
            }
        }

        Commands::Memo { text } => {
            paths.create_all()?;
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let filename = format!("memo_{}.txt", timestamp);
            let memo_path = paths.inbox.join(&filename);
            std::fs::write(&memo_path, &text)?;
            println!("메모 저장: {} ({} 자)", memo_path.display(), text.len());
        }

        Commands::TopicRevise { file, feedback } => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service(&cfg, &paths, registry)?;
            let topic_path = std::path::Path::new(&file);
            if !topic_path.exists() { anyhow::bail!("토픽 파일 없음: {}", file); }
            let revised = file_pipeline_core::domain::topic_merger::TopicMerger::revise_topic(
                topic_path, &feedback, service.llm.as_ref(),
            ).await?;
            println!("=== 수정 완료 ===");
            println!("{}", &revised[..revised.len().min(500)]);
        }

        Commands::Todo { action } => {
            // 신규 todo 시스템 (Phase 53) — settings.db todos 테이블
            match action {
                TodoAction::List => println!("(신규 todo 시스템 — Phase 53)"),
                TodoAction::Done { text } => println!("(신규 todo 시스템 — Phase 53: {})", text),
            }
        }

        Commands::Kg { action } => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service(&cfg, &paths, registry)?;
            match action {
                KgAction::Neighbors { doc_id } => {
                    let result = file_pipeline_core::domain::wiki_export::KgQueryEngine::neighbors(
                        service.vector_db.as_ref(), &doc_id,
                    )?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                KgAction::Paths { source, target } => {
                    let result = file_pipeline_core::domain::wiki_export::KgQueryEngine::find_paths(
                        service.vector_db.as_ref(), &source, &target,
                    )?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                KgAction::Stats => {
                    let stats = file_pipeline_core::domain::wiki_export::KgQueryEngine::stats(
                        service.vector_db.as_ref(),
                    )?;
                    println!("{}", serde_json::to_string_pretty(&stats)?);
                }
            }
        }

        Commands::BackfillVec => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service(&cfg, &paths, registry)?;
            let all = service.vector_db.list_all()?;
            let mut saved = 0usize;
            for doc in &all {
                let vec_path = doc.path.with_extension("vec");
                if vec_path.exists() { continue; }
                if let Ok(Some(vector)) = service.vector_db.get_vector(&doc.id) {
                    if file_pipeline_core::domain::vec_io::save_vec(&vec_path, &vector).is_ok() {
                        saved += 1;
                    }
                }
            }
            println!("backfill-vec: {}/{}", saved, all.len());
        }

        Commands::Batch => {
            paths.create_all()?;
            let doc_types_path = config::resolve_doc_types_path(&paths);
            let registry = config::load_doc_type_registry(&doc_types_path)?;
            let service = build_service(&cfg, &paths, registry)?;
            let service = Arc::new(service);

            let pipeline = cfg.pipelines.clone();
            let credential_llms: std::collections::HashMap<String, Arc<dyn file_pipeline_core::ports::output::LLMPort>> =
                cfg.credentials.iter().filter_map(|cred| {
                    crate::build_llm_from_credential(cred).map(|llm| (cred.name.clone(), llm))
                }).collect();

            let watcher = file_pipeline_adapters::driving::watcher::FileWatcher::new(
                paths.inbox.clone(), Arc::clone(&service),
            ).with_max_workers(cfg.max_workers)
             .with_pipeline(pipeline)
             .with_credential_llms(credential_llms);

            let stats = watcher.batch_process().await?;
            println!("배치 완료: 성공 {} / 실패 {} / 대기 {}", stats.done, stats.failed, stats.pending);

            // 요약 알림
            let _ = service.flush_summary().await;
        }
    }

    Ok(())
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
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
