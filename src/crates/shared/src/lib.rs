//! file-pipeline-shared — 공유 라이브러리
//!
//! 모든 모달(CLI, APP, MCP)이 공유하는 config, build_service.

pub mod audit_anomaly;
pub mod auto_suggester;
pub mod settings_audit_adapter;
pub mod cached_llm;
pub mod cli;
pub mod config;
pub mod config_snapshot;
pub mod credential_store;
pub mod host_tools_cache;
pub mod mcp_server;
pub mod platform;
pub mod secrets;
pub mod settings_db;
pub mod setup_dryrun;
pub mod setup_modules;
pub mod setup_review;
pub mod test_helpers;
pub mod tray;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use file_pipeline_core::ports::output::VectorDBPort;
use tracing::{info, warn};

use file_pipeline_adapters::driven::embedding::claude_embed::ClaudeEmbeddingAdapter;
use file_pipeline_adapters::driven::embedding::openai_embed::OpenAIEmbeddingAdapter;
use file_pipeline_adapters::driven::llm::claude_adapter::ClaudeCliAdapter;
use file_pipeline_adapters::driven::notification::composite::{
    CompositeNotificationAdapter, NullNotificationAdapter,
};
use file_pipeline_adapters::driven::notification::slack_notify::SlackNotificationAdapter;
use file_pipeline_adapters::driven::notification::telegram_notify::TelegramNotificationAdapter;
use file_pipeline_adapters::driven::storage::zstd_storage::ZstdStorageAdapter;
use file_pipeline_adapters::driven::vector_db::local_store::LocalVectorStore;
use file_pipeline_adapters::driven::verification::claude_verifier::ClaudeVerificationAdapter;
use file_pipeline_adapters::stub::{
    StubDuplicateResolution, StubEmbedder, StubLlm, StubSensitiveNotification,
};
use file_pipeline_core::domain::classifier::SensitivityDetector;
use file_pipeline_core::domain::models::DocTypeRegistry;
use file_pipeline_core::service::FileProcessingService;

/// claude CLI가 PATH에 있는지 확인
pub fn which_claude() -> bool {
    let mut cmd = std::process::Command::new("claude");
    cmd.arg("--version");
    hide_console_window(&mut cmd);
    cmd.output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Windows GUI 모드에서 자식 프로세스의 콘솔 창 숨김
pub fn hide_console_window(cmd: &mut std::process::Command) -> &mut std::process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// FileProcessingService 구성 (Tauri/CLI 공용)
pub fn build_service(
    cfg: &config::PipelineConfig,
    paths: &config::ResolvedPaths,
    registry: DocTypeRegistry,
) -> Result<FileProcessingService> {
    let storage = Arc::new(ZstdStorageAdapter::new(
        cfg.compression.zstd_level,
        paths.temp.clone(),
    ));

    // VectorDB — LocalVectorStore (인프로세스, mmap+Rayon+HNSW)
    // Phase 89 C-1: paths.base를 명시 전달 — `--base` CLI 옵션이 LocalVectorStore까지 전파됨.
    // resolve_data_base()는 환경변수만 보므로, build_service 진입점에서 명시 주입 필수.
    let vector_db: Arc<dyn file_pipeline_core::ports::output::VectorDBPort> = {
        info!("벡터 DB: LocalVectorStore");
        let db = Arc::new(LocalVectorStore::with_path(paths.base.join(".local-store.json")));
        db.init().expect("LocalVectorStore init");
        db
    };

    // CLAUDE_BIN 디렉토리 감지
    if let Ok(ref val) = std::env::var("CLAUDE_BIN") {
        if std::path::Path::new(val).is_dir() {
            warn!("CLAUDE_BIN이 디렉토리를 가리킴: {} — PATH에서 claude 탐색", val);
            std::env::remove_var("CLAUDE_BIN");
        }
    }

    let build_llm_provider = |name: &str, llm_cfg: &config::LlmConfig|
        -> Option<(String, Arc<dyn file_pipeline_core::ports::output::LLMPort>)>
    {
        match name {
            "anthropic_api" => {
                let key = std::env::var("ANTHROPIC_API_KEY").ok().or_else(|| llm_cfg.anthropic_api_key.clone())?;
                Some(("anthropic_api".into(), Arc::new(file_pipeline_adapters::driven::llm::anthropic_adapter::AnthropicApiAdapter::new(key, cfg.models.process_model.clone()))))
            }
            "openai_api" => {
                let key = std::env::var("OPENAI_API_KEY").ok().or_else(|| llm_cfg.openai_api_key.clone())?;
                Some(("openai_api".into(), Arc::new(file_pipeline_adapters::driven::llm::openai_llm_adapter::OpenAiLlmAdapter::new(key, llm_cfg.openai_model.clone()))))
            }
            "ollama" => {
                Some(("ollama".into(), Arc::new(file_pipeline_adapters::driven::llm::ollama_adapter::OllamaAdapter::new(llm_cfg.ollama_url.clone(), llm_cfg.ollama_model.clone()))))
            }
            "gemini" => {
                let key = std::env::var("GEMINI_API_KEY").ok().or_else(|| llm_cfg.gemini_api_key.clone())?;
                Some(("gemini".into(), Arc::new(file_pipeline_adapters::driven::llm::gemini_adapter::GeminiAdapter::new(key, llm_cfg.gemini_model.clone()))))
            }
            _ => {
                // "claude_cli" 또는 미지정/오타 → 기본 폴백
                if std::env::var("CLAUDE_BIN").is_ok() || which_claude() {
                    let adapter = ClaudeCliAdapter::new();
                    Some(("claude_cli".into(), Arc::new(adapter) as Arc<dyn file_pipeline_core::ports::output::LLMPort>))
                } else {
                    None
                }
            }
        }
    };

    let llm: Arc<dyn file_pipeline_core::ports::output::LLMPort> = {
        let mut providers: Vec<(String, Arc<dyn file_pipeline_core::ports::output::LLMPort>)> = vec![];

        // default_credential이 설정되어 있으면 해당 크레덴셜을 primary로 사용
        let used_default_cred = if let Some(ref cred_name) = cfg.llm.default_credential {
            if let Some(cred) = cfg.credentials.iter().find(|c| &c.name == cred_name) {
                if let Some(adapter) = build_llm_from_credential(cred) {
                    info!("LLM primary (credential '{}'): {}", cred_name, cred.provider);
                    providers.push((cred.provider.clone(), adapter));
                    true
                } else {
                    warn!("기본 크레덴셜 '{}' 어댑터 생성 실패 → 글로벌 provider fallback", cred_name);
                    false
                }
            } else {
                warn!("기본 크레덴셜 '{}' 없음 → 글로벌 provider fallback", cred_name);
                false
            }
        } else {
            false
        };

        if !used_default_cred {
            if let Some(p) = build_llm_provider(&cfg.llm.provider, &cfg.llm) {
                info!("LLM primary: {}", p.0);
                providers.push(p);
            }
        }
        for fb in &cfg.llm.fallback_providers {
            if let Some(p) = build_llm_provider(fb, &cfg.llm) {
                info!("LLM fallback: {}", p.0);
                providers.push(p);
            }
        }
        if providers.is_empty() {
            info!("LLM: Stub (프로바이더 없음)");
            Arc::new(StubLlm)
        } else if providers.len() == 1 {
            providers.remove(0).1
        } else {
            Arc::new(file_pipeline_adapters::driven::llm::fallback_adapter::FallbackLlmAdapter::new(providers))
        }
    };

    // 대용량 파일 에이전트 래핑 (의미 단위 청킹 설정 적용)
    let llm: Arc<dyn file_pipeline_core::ports::output::LLMPort> = {
        let mut adapter = file_pipeline_adapters::driven::llm::chunked_agent::ChunkedAgentAdapter::new(llm);
        if cfg.chunking.semantic_enabled {
            adapter = adapter.with_semantic_chunking(
                file_pipeline_core::domain::chunking::SemanticChunkConfig {
                    target_bytes: cfg.chunking.target_bytes,
                    max_bytes: cfg.chunking.max_bytes,
                    overlap_sentences: cfg.chunking.overlap_sentences,
                    preserve_code_blocks: cfg.chunking.preserve_code_blocks,
                    preserve_tables: cfg.chunking.preserve_tables,
                },
            );
        }
        Arc::new(adapter)
    };

    // Ruflo A1 — LLM 결과 캐시 (file_hash + content_hash 기반)
    let llm: Arc<dyn file_pipeline_core::ports::output::LLMPort> = if cfg.llm.llm_cache_enabled {
        info!("LLM 캐시: 활성 (settings.db llm_cache)");
        Arc::new(cached_llm::CachedLLM::new(
            llm,
            paths.base.join("settings.db"),
        ))
    } else {
        llm
    };

    // Embedding
    let embedding: Arc<dyn file_pipeline_core::ports::output::EmbeddingPort> =
        // fastembed (BGE-M3 순수 Rust) — Phase 62 우선 옵션
        if cfg.embedding.default_model == "fastembed" {
            #[cfg(feature = "fastembed")]
            {
                let cache_dir = cfg.embedding.onnx_model_dir.as_deref()
                    .filter(|s| !s.is_empty())
                    .map(std::path::PathBuf::from);
                let result = match cache_dir {
                    Some(dir) => file_pipeline_adapters::driven::embedding::fastembed_adapter::FastEmbedAdapter::with_cache_dir(dir),
                    None => file_pipeline_adapters::driven::embedding::fastembed_adapter::FastEmbedAdapter::new(),
                };
                match result {
                    Ok(adapter) => {
                        info!("임베딩: fastembed BGE-M3 (1024차원, 순수 Rust)");
                        Arc::new(adapter)
                    }
                    Err(e) => {
                        warn!("fastembed 로드 실패, Claude CLI로 폴백: {}", e);
                        if which_claude() {
                            Arc::new(ClaudeEmbeddingAdapter::new(cfg.vector_db.dim as usize))
                        } else {
                            Arc::new(StubEmbedder::new(cfg.vector_db.dim as usize))
                        }
                    }
                }
            }
            #[cfg(not(feature = "fastembed"))]
            {
                warn!("default_model='fastembed'이지만 feature 비활성. Claude CLI로 폴백 (빌드 시 --features fastembed 필요)");
                if which_claude() {
                    Arc::new(ClaudeEmbeddingAdapter::new(cfg.vector_db.dim as usize))
                } else {
                    Arc::new(StubEmbedder::new(cfg.vector_db.dim as usize))
                }
            }
        } else if cfg.embedding.default_model == "onnx" || cfg.embedding.default_model == "bge_m3" {
            // Phase 64 트리거 #11: onnx feature(Rust 네이티브 ort) 폐기. fastembed가 우선.
            // PythonOnnx legacy fallback (사용자가 명시 onnx 선택 + Python 환경 보유 시)
            let model_dir = cfg.embedding.onnx_model_dir.as_deref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("models/bge-m3"));
            match file_pipeline_adapters::driven::embedding::python_onnx_embed::PythonOnnxEmbeddingAdapter::new(
                model_dir, cfg.vector_db.dim as usize
            ) {
                Ok(adapter) => {
                    info!("임베딩: Python ONNX legacy (dim={}). 권장: default_model=fastembed로 변경", cfg.vector_db.dim);
                    Arc::new(adapter)
                }
                Err(e) => {
                    warn!("Python ONNX 폴백 실패: {}. Claude CLI로 폴백 (또는 default_model=fastembed로 변경 권장).", e);
                    if which_claude() {
                        Arc::new(ClaudeEmbeddingAdapter::new(cfg.vector_db.dim as usize))
                    } else {
                        Arc::new(StubEmbedder::new(cfg.vector_db.dim as usize))
                    }
                }
            }
        } else if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            info!("임베딩: OpenAI text-embedding-3-small");
            Arc::new(OpenAIEmbeddingAdapter::new(api_key))
        } else if which_claude() {
            info!("임베딩: Claude CLI 키워드 해시");
            Arc::new(ClaudeEmbeddingAdapter::new(cfg.vector_db.dim as usize))
        } else {
            info!("임베딩: Stub (LLM 미감지)");
            Arc::new(StubEmbedder::new(cfg.vector_db.dim as usize))
        };

    // 알림
    let notification: Arc<dyn file_pipeline_core::ports::output::NotificationPort> = {
        let mut adapters: Vec<Box<dyn file_pipeline_core::ports::output::NotificationPort>> = vec![];
        let tg_token = std::env::var("TELEGRAM_BOT_TOKEN").ok().or_else(|| {
            cfg.notification.telegram.as_ref().and_then(|t| t.bot_token.clone())
        });
        let tg_chat = std::env::var("TELEGRAM_CHAT_ID").ok().or_else(|| {
            cfg.notification.telegram.as_ref().and_then(|t| t.chat_id.clone())
        });
        if let (Some(token), Some(chat_id)) = (tg_token, tg_chat) {
            info!("알림: Telegram 활성화");
            adapters.push(Box::new(TelegramNotificationAdapter::new(token, chat_id)));
        }
        let slack_token = std::env::var("SLACK_BOT_TOKEN").ok().or_else(|| {
            cfg.notification.slack.as_ref().and_then(|s| s.bot_token.clone())
        });
        let slack_channel = std::env::var("SLACK_CHANNEL").ok().or_else(|| {
            cfg.notification.slack.as_ref().and_then(|s| s.channel.clone())
        });
        if let (Some(token), Some(channel)) = (slack_token, slack_channel) {
            info!("알림: Slack 활성화");
            adapters.push(Box::new(SlackNotificationAdapter::new(token, channel)));
        }
        if adapters.is_empty() {
            Arc::new(NullNotificationAdapter)
        } else {
            Arc::new(CompositeNotificationAdapter::new(adapters))
        }
    };

    // Tauri 모드: stdin이 없으므로 항상 Stub
    let duplicate_resolution: Arc<dyn file_pipeline_core::ports::input::DuplicateResolutionPort> =
        Arc::new(StubDuplicateResolution);
    let sensitive_notification: Arc<dyn file_pipeline_core::ports::input::SensitiveNotificationPort> =
        Arc::new(StubSensitiveNotification);

    // Phase 81: 호스트 도구 감지 결과를 settings.db에서 로드 (없으면 1회 감지 + 저장)
    let preprocessing = {
        let db_path = paths.base.join("settings.db");
        let host_tools = match settings_db::SettingsDb::open(&db_path) {
            Ok(db) => host_tools_cache::ensure_cached(&db).unwrap_or_else(|e| {
                warn!("호스트 도구 캐시 로드 실패, 즉시 감지로 폴백: {}", e);
                file_pipeline_adapters::driven::preprocessing::preprocessor::HostToolDetector::detect()
            }),
            Err(e) => {
                warn!("settings.db 열기 실패, 호스트 도구 즉시 감지: {}", e);
                file_pipeline_adapters::driven::preprocessing::preprocessor::HostToolDetector::detect()
            }
        };
        if !host_tools.is_empty() {
            info!("호스트 전처리 도구 (캐시): {}",
                host_tools.iter().map(|(_, v)| v.as_str()).collect::<Vec<_>>().join(", "));
        }
        Arc::new(
            file_pipeline_adapters::driven::preprocessing::preprocessor::CompositePreprocessor::with_tools(
                &cfg.preprocessing.pdf_tool,
                &cfg.preprocessing.ocr_tool,
                host_tools,
            )
        )
    };

    let sensitivity_detector = SensitivityDetector::new(
        cfg.sensitive.merged_keywords(),
        cfg.sensitive.merged_extensions(),
    );

    // Ruflo C2: 사용자 정의 PII 패턴 로드 (settings.db pii_patterns_user)
    let pii_user_patterns: Vec<(String, String)> = {
        let db_path = paths.base.join("settings.db");
        match settings_db::SettingsDb::open(&db_path) {
            Ok(db) => db.list_user_pii_patterns().unwrap_or_default()
                .into_iter()
                .filter(|(_, _, enabled)| *enabled)
                .map(|(n, p, _)| (n, p))
                .collect(),
            Err(_) => Vec::new(),
        }
    };

    let remote_storage: Arc<dyn file_pipeline_core::ports::output::RemoteStoragePort> = if cfg.remote_storage.enabled {
        match cfg.remote_storage.provider.as_str() {
            "network" => {
                if let Some(ref path) = cfg.remote_storage.network_path {
                    info!("원격 저장소: 네트워크 경로 ({})", path);
                    Arc::new(file_pipeline_adapters::driven::storage::network_storage::NetworkStorageAdapter::new(
                        std::path::PathBuf::from(path)
                    ))
                } else {
                    warn!("원격 저장소: 네트워크 경로 미설정 → 비활성");
                    Arc::new(file_pipeline_adapters::driven::storage::remote_null::NullRemoteStorage)
                }
            }
            "webdav" => {
                if let (Some(url), Some(user), Some(pass)) = (
                    cfg.remote_storage.webdav_url.clone(),
                    cfg.remote_storage.webdav_user.clone(),
                    cfg.remote_storage.webdav_password.clone(),
                ) {
                    info!("원격 저장소: WebDAV ({})", url);
                    Arc::new(file_pipeline_adapters::driven::storage::webdav_storage::WebDavStorageAdapter::new(
                        url, user, pass, cfg.remote_storage.webdav_prefix.clone().unwrap_or_default()
                    ))
                } else {
                    warn!("원격 저장소: WebDAV 설정 불완전 → 비활성");
                    Arc::new(file_pipeline_adapters::driven::storage::remote_null::NullRemoteStorage)
                }
            }
            "s3" => {
                if let (Some(endpoint), Some(bucket), Some(access_key), Some(secret_key)) = (
                    cfg.remote_storage.s3_endpoint.clone(),
                    cfg.remote_storage.s3_bucket.clone(),
                    cfg.remote_storage.s3_access_key.clone(),
                    cfg.remote_storage.s3_secret_key.clone(),
                ) {
                    info!("원격 저장소: S3 ({}/{})", endpoint, bucket);
                    Arc::new(file_pipeline_adapters::driven::storage::s3_storage::S3StorageAdapter::new(
                        endpoint,
                        bucket,
                        cfg.remote_storage.s3_region.clone().unwrap_or_else(|| "us-east-1".into()),
                        access_key,
                        secret_key,
                        cfg.remote_storage.s3_prefix.clone().unwrap_or_default(),
                    ))
                } else {
                    warn!("원격 저장소: S3 설정 불완전 → 비활성");
                    Arc::new(file_pipeline_adapters::driven::storage::remote_null::NullRemoteStorage)
                }
            }
            "notion" => {
                if let (Some(token), Some(parent)) = (
                    cfg.remote_storage.notion_token.clone(),
                    cfg.remote_storage.notion_parent_page_id.clone(),
                ) {
                    info!("원격 저장소: Notion (mode={}, parent={})",
                        cfg.remote_storage.notion_mode, parent);
                    Arc::new(file_pipeline_adapters::driven::storage::notion_storage::NotionStorageAdapter::new(
                        token,
                        parent,
                        &cfg.remote_storage.notion_mode,
                        cfg.remote_storage.notion_database_id.clone(),
                    ))
                } else {
                    warn!("원격 저장소: Notion 설정 불완전 (token/parent_page_id 필요) → 비활성");
                    Arc::new(file_pipeline_adapters::driven::storage::remote_null::NullRemoteStorage)
                }
            }
            _ => {
                Arc::new(file_pipeline_adapters::driven::storage::remote_null::NullRemoteStorage)
            }
        }
    } else {
        Arc::new(file_pipeline_adapters::driven::storage::remote_null::NullRemoteStorage)
    };

    let compile_state_path = paths.base.join(".compile-state.json");
    let compile_state = file_pipeline_core::domain::incremental::CompileState::load(&compile_state_path)
        .unwrap_or_default();

    // Phase 94 A3: settings.db 기반 AuditPort (헥사고날 — adapter는 shared에 위치)
    let audit: Arc<dyn file_pipeline_core::ports::output::AuditPort> =
        crate::settings_audit_adapter::SettingsAuditAdapter::shared(paths.base.join("settings.db"));

    // Phase 202 B2: plugin IPC registry — discover + audit 주입
    let mut plugin_registry =
        file_pipeline_core::plugin::PluginRegistry::new().with_audit(Arc::clone(&audit));
    match plugin_registry.discover(&paths.plugins) {
        Ok(n) => {
            if n > 0 {
                info!("plugin: {} 건 discover", n);
            }
        }
        Err(e) => {
            warn!("plugin discover 실패 (계속 진행): {}", e);
        }
    }
    let plugin_registry = Arc::new(plugin_registry);

    Ok(FileProcessingService {
        llm,
        storage,
        vector_db,
        embedding,
        notification,
        preprocessing,
        remote_storage,
        audit,
        verification: if cfg.verification.llm_hallucination_check && which_claude() {
            info!("검증: Claude CLI 환각 탐지 활성화");
            Some(Arc::new(ClaudeVerificationAdapter::new()))
        } else {
            None
        },
        duplicate_resolution,
        sensitive_notification,
        registry: Arc::new(registry),
        sensitivity_detector,
        pii_user_patterns: std::sync::RwLock::new(pii_user_patterns),
        inbox_dir: paths.inbox.clone(),
        processed_dir: paths.processed.clone(),
        originals_dir: paths.originals.clone(),
        sensitive_dir: paths.sensitive.clone(),
        todo_dir: paths.todo.clone(),
        semantic_dup_threshold: cfg.vector_db.semantic_dup_threshold,
        max_retry: cfg.verification.max_retry,
        quarantine_dir: paths.base.join("quarantine"),
        compile_state: std::sync::Mutex::new(compile_state),
        compile_state_path,
        compile_state_batch: std::sync::atomic::AtomicBool::new(false),
        global_thresholds: cfg.verification.thresholds.clone(),
        verification_enabled: cfg.verification.enabled,
        fragment_threshold: cfg.schedule.fragment_threshold,
        crossref_mode: if cfg.crossref.enabled { cfg.crossref.mode.clone() } else { "off".into() },
        crossref_similarity_threshold: cfg.crossref.similarity_threshold,
        crossref_supersedes_threshold: cfg.crossref.supersedes_threshold,
        crossref_keyword_overlap_min: cfg.crossref.keyword_overlap_min,
        crossref_top_k: cfg.crossref.top_k,
        crossref_cap_supersedes: cfg.crossref.cap_supersedes,
        crossref_cap_updates: cfg.crossref.cap_updates,
        crossref_cap_related: cfg.crossref.cap_related,
        crossref_cap_references: cfg.crossref.cap_references,
        crossref_cap_incoming: cfg.crossref.cap_incoming,
        crossref_minhash_force: cfg.crossref.minhash_force_enable,
        crossref_minhash_min_docs: cfg.crossref.minhash_min_docs,
        crossref_metadata_blocking: cfg.crossref.metadata_blocking,
        summary: std::sync::Mutex::new(Default::default()),
        progress_callback: None,
        error_log: std::sync::Mutex::new(file_pipeline_core::domain::error_log::ErrorLog::new()),
        token_usage: std::sync::Mutex::new(load_token_usage(&paths.base)),
        embed_instruction_prefix: cfg.embedding.instruction_prefix.clone(),
        crossref_queue: std::sync::Mutex::new(Vec::new()),
        crossref_last_run: std::sync::Mutex::new(None),
        crossref_interval_secs: 30,
        metrics_recorder: Some(Arc::new(SettingsDbMetricsAdapter::new(
            paths.base.join("settings.db"),
        ))),
        plugin_registry,
    })
}

/// 토큰 사용 기록 로드
pub fn load_token_usage(base_dir: &std::path::Path) -> file_pipeline_core::domain::models::TokenUsage {
    let path = base_dir.join(".token-usage.json");
    std::fs::read_to_string(&path).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// 토큰 사용 기록 저장
pub fn save_token_usage(base_dir: &std::path::Path, usage: &file_pipeline_core::domain::models::TokenUsage) {
    let path = base_dir.join(".token-usage.json");
    if let Ok(json) = serde_json::to_string_pretty(usage) {
        let _ = std::fs::write(&path, json);
    }
}

/// 크레덴셜 기반 LLM 어댑터 생성 (파이프라인 credential override용)
pub fn build_llm_from_credential(
    cred: &config::LlmCredential,
) -> Option<Arc<dyn file_pipeline_core::ports::output::LLMPort>> {
    match cred.provider.as_str() {
        "claude_cli" => {
            if std::env::var("CLAUDE_BIN").is_ok() || which_claude() {
                let adapter = ClaudeCliAdapter::new()
                    .with_config_dir(cred.profile_path.clone());
                Some(Arc::new(adapter))
            } else {
                None
            }
        }
        "anthropic_api" => {
            let key = cred.api_key.clone()?;
            Some(Arc::new(
                file_pipeline_adapters::driven::llm::anthropic_adapter::AnthropicApiAdapter::new(
                    key,
                    cred.model.clone().unwrap_or_else(|| "sonnet".into()),
                ),
            ))
        }
        "openai_api" => {
            let key = cred.api_key.clone()?;
            Some(Arc::new(
                file_pipeline_adapters::driven::llm::openai_llm_adapter::OpenAiLlmAdapter::new(
                    key,
                    cred.model.clone().unwrap_or_else(|| "gpt-4o".into()),
                ),
            ))
        }
        "ollama" => {
            let url = cred.url.clone().unwrap_or_else(|| "http://localhost:11434".into());
            let model = cred.model.clone().unwrap_or_else(|| "llama3".into());
            Some(Arc::new(
                file_pipeline_adapters::driven::llm::ollama_adapter::OllamaAdapter::new(url, model),
            ))
        }
        "gemini" => {
            let key = cred.api_key.clone()?;
            Some(Arc::new(
                file_pipeline_adapters::driven::llm::gemini_adapter::GeminiAdapter::new(
                    key,
                    cred.model.clone().unwrap_or_else(|| "gemini-2.0-flash".into()),
                ),
            ))
        }
        _ => None,
    }
}

/// 크레덴셜 기반 Embedding 어댑터 생성
pub fn build_embedding_from_credential(
    cred: &config::LlmCredential,
    dim: usize,
) -> Option<Arc<dyn file_pipeline_core::ports::output::EmbeddingPort>> {
    match cred.provider.as_str() {
        "openai_api" => {
            let key = cred.api_key.clone()?;
            Some(Arc::new(OpenAIEmbeddingAdapter::new(key)))
        }
        "claude_cli" => {
            if std::env::var("CLAUDE_BIN").is_ok() || which_claude() {
                Some(Arc::new(ClaudeEmbeddingAdapter::new(dim)))
            } else {
                None
            }
        }
        _ => None, // ollama, gemini, anthropic_api don't have embedding adapters yet
    }
}

/// tracing 초기화 — logging.file=true면 파일 출력, false면 콘솔 출력
///
/// - `file_enabled=true`: logs/pipeline.log에 기록 (GUI 모드 기본)
/// - `file_enabled=false`: stdout/stderr 콘솔 출력 (CLI 디버그용)
/// - `_guard` 반환: 파일 appender 가드. drop되면 flush되므로 main에서 유지해야 함.
pub fn init_tracing(
    level: &str,
    file_enabled: bool,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::fmt;

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    if file_enabled {
        // PIPELINE_BASE/CWD/exe_dir 통합 (lesson 29)
        let log_dir = config::find_data_dir(None).join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let file_appender = tracing_appender::rolling::daily(&log_dir, "pipeline.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(non_blocking).with_target(false).with_ansi(false))
            .init();
        Some(guard)
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .init();
        None
    }
}

/// 로그 파일에 메시지 기록 (data_dir/logs/) — 레거시, tracing 초기화 전 사용
///
/// 데이터 디렉토리는 `config::find_data_dir(None)`로 해석한다 — PIPELINE_BASE 환경변수,
/// CWD의 settings.db/pipeline.toml, exe_dir 순으로 탐색 (lesson 29 통합).
pub fn write_log(level: &str, message: &str) {
    let data_dir = config::find_data_dir(None);
    let log_dir = data_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("pipeline.log");
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{}] [{}] {}\n", timestamp, level, message);
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_file) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// 데이터 디렉토리에 설정 + 스켈레톤 디렉토리를 자동 생성
///
/// 데이터 디렉토리는 `config::find_data_dir(None)`로 해석 — PIPELINE_BASE 환경변수,
/// CWD의 settings.db/pipeline.toml, exe_dir 순 (lesson 29 통합).
pub fn auto_init() {
    let base_dir = config::find_data_dir(None);
    let _ = std::fs::create_dir_all(&base_dir);

    let config_path = base_dir.join("pipeline.toml");
    let is_first_run = !config_path.exists();

    // pipeline.toml 생성
    if is_first_run {
        let _ = std::fs::write(&config_path, DEFAULT_CONFIG_TEMPLATE);
    }

    // 스켈레톤 디렉토리 생성
    let dirs = ["inbox", "processed", "originals", "logs", "doc"];
    for dir in &dirs {
        let path = base_dir.join(dir);
        let _ = std::fs::create_dir_all(&path);
    }

    // 첫 실행 시 안내 메시지
    if is_first_run {
        eprintln!("========================================");
        eprintln!("  File Pipeline — 초기 설정 완료");
        eprintln!("========================================");
        eprintln!();
        eprintln!("  생성된 파일:");
        eprintln!("    {} ", config_path.display());
        eprintln!();
        eprintln!("  생성된 폴더:");
        for dir in &dirs {
            eprintln!("    {}\\{}", base_dir.display(), dir);
        }
        eprintln!();
        eprintln!("  사용법:");
        eprintln!("    1. inbox\\ 폴더에 파일을 넣으면 자동 가공됩니다");
        eprintln!("    2. pipeline.toml을 편집하여 설정을 변경할 수 있습니다");
        eprintln!("    3. 벡터 DB는 인프로세스 LocalVectorStore로 자동 시작됩니다");
        eprintln!();
        eprintln!("  주요 명령:");
        eprintln!("    pipeline.exe              GUI 모드 (Dashboard)");
        eprintln!("    pipeline.exe stats        문서 통계");
        eprintln!("    pipeline.exe memo \"텍스트\" 메모 생성");
        eprintln!("    pipeline.exe serve        MCP 서버 (Claude Code 연동)");
        eprintln!("    pipeline.exe --help       전체 도움말");
        eprintln!("========================================");
    }

    // 도움말 파일 자동 생성
    let doc_dir = base_dir.join("doc");
    let verification_guide = doc_dir.join("verification-guide.md");
    if !verification_guide.exists() {
        let _ = std::fs::write(&verification_guide, VERIFICATION_GUIDE);
    }
}

/// dev 빌드 전용 — credential 0건 시 현재 환경 기반 claude_cli credential 1건 생성.
///
/// in-memory만 — settings.db 미저장 (release 환경 오염 방지).
/// 우선순위: CLAUDE_PROFILE_PATH env → C:\dev\ide\claude\profiles\reujea → %USERPROFILE%\.claude.
/// 후보 경로 모두 부재 시 None 반환 (조용히 skip, 사용자가 GUI에서 직접 등록 가능).
#[cfg(debug_assertions)]
pub fn dev_seed_credential() -> Option<config::LlmCredential> {
    let candidates: Vec<PathBuf> = [
        std::env::var("CLAUDE_PROFILE_PATH").ok().map(PathBuf::from),
        Some(PathBuf::from(r"C:\dev\ide\claude\profiles\reujea")),
        std::env::var("USERPROFILE").ok().map(|u| PathBuf::from(u).join(".claude")),
    ]
    .into_iter()
    .flatten()
    .collect();

    let profile = candidates.into_iter().find(|p| p.exists())?;
    let profile_str = profile.to_string_lossy().to_string();

    tracing::info!("[dev-seed] claude_cli credential 자동 등록 (in-memory, profile={})", profile_str);

    Some(config::LlmCredential {
        id: "dev-seed-claude-cli".to_string(),
        name: "dev seed (claude_cli)".to_string(),
        provider: "claude_cli".to_string(),
        api_key: None,
        url: None,
        model: None,
        profile_path: Some(profile_str),
    })
}

const VERIFICATION_GUIDE: &str = r#"# 검증 시스템 가이드

## 개요

File Pipeline은 LLM이 가공한 결과의 품질을 **6가지 기준**으로 자동 검증합니다.
검증에 실패하면 피드백을 LLM에 전달하여 재가공합니다 (2-Pass).

## 검증 절차

```
1. 파일 투입 (inbox)
2. LLM이 분류 + 가공 (1차)
3. 6가지 검증 실행
   ├── PASS → 4단계로
   └── FAIL → 실패 상세를 피드백으로 LLM에 전달 → 재가공 (최대 max_retry 회)
       ├── 재검증 PASS → 4단계로
       └── 최종 FAIL → quarantine/ 폴더로 이동 + 알림 목록에 추가
4. 임베딩 생성 → 벡터 DB 색인 → 완료
```

## 6가지 검증 기준

| # | 검사 | 방법 | 기본 기준 | 실패 시 |
|---|------|------|----------|---------|
| 1 | **구조 완전성** | LLM이 반환한 sections JSON 키 확인 | 50% 이상 | FAIL |
| 2 | **압축률** | 가공본/원본 길이 비율 | 5~150% | WARNING |
| 3 | **키워드 커버리지** | LLM이 추출한 키워드가 원본에 있는지 (환각 탐지) | 50% 이상 | FAIL |
| 4 | **키워드 완전성** | 원본의 핵심 키워드가 가공본에 보존되었는지 (누락 탐지) | 30% 이상 | WARNING |
| 5 | **ROUGE-L** | 원본과 가공본의 LCS 기반 유사도 | 10% 이상 | FAIL |
| 6 | **개체 보존** | 날짜, 금액, 숫자, 이메일, URL이 보존되었는지 | 50% 이상 | WARNING |

## 검증 결과

- **PASS**: 모든 기준 통과 → 정상 색인
- **WARNING**: 경고 기준 미달 (압축률, 키워드 완전성, 개체 보존) → 색인하되 알림
- **FAIL**: 핵심 기준 미달 (구조, 키워드 커버리지, ROUGE-L) → 재가공 시도

## 2-Pass 피드백 재가공

1차 검증 실패 시, 실패 상세 (어떤 기준이 왜 실패했는지)를 LLM에 피드백으로 전달합니다.
LLM은 이 피드백을 바탕으로 가공 결과를 개선합니다.

예시:
```
FAIL: 구조 완전성 0% (기준 50%)
FAIL: 키워드 커버리지 30% (기준 50%)
→ LLM에게: "결정사항/액션아이템 섹션이 누락됨, 키워드 meeting/agenda가 원본에 없음"
→ 재가공 후 재검증
```

## 임계값 커스터마이징

### 글로벌 (pipeline.toml)
```toml
[verification.thresholds]
structure_min = 0.5
compression_min = 0.05
compression_max = 1.5
keyword_coverage_min = 0.5
keyword_completeness_min = 0.3
rouge_l_min = 0.1
entity_preservation_min = 0.5
```

### 문서 유형별 (doc_types.toml)
```toml
[[types]]
id = "meeting"
[types.thresholds]
structure_min = 0.3    # 회의록은 구조가 느슨해도 허용
compression_max = 2.0  # 회의록은 가공 후 길어질 수 있음
```

## quarantine 폴더

최종 검증 실패 문서는 `quarantine/` 폴더로 이동됩니다.
Dashboard의 에러 목록에서 실패 사유와 함께 확인할 수 있습니다.
수동으로 수정 후 inbox에 다시 넣으면 재가공됩니다.
"#;

const DEFAULT_CONFIG_TEMPLATE: &str = r#"version = "1"

# ══════════════════════════════════════════════════════════
# File Pipeline 설정
# ══════════════════════════════════════════════════════════
# inbox/ 폴더에 파일을 넣으면 자동으로 분류·가공·색인됩니다.
# 이 파일은 첫 실행 시 자동 생성됩니다. 필요한 부분만 수정하세요.

max_workers = 4

# ── 경로 설정 ──────────────────────────────────────────────
# 비어있으면 바이너리 디렉토리 기준 자동 생성
[paths]
# extra_inboxes = ["D:\\Downloads\\docs"]

# ── 압축 ──────────────────────────────────────────────────
[compression]
zstd_level = 3
original_ttl_days = 30

# ── 벡터 DB ───────────────────────────────────────────────
[vector_db]
# LocalVectorStore: 인프로세스 벡터 DB (mmap + Rayon + HNSW)
# 외부 서버 불필요. 20K+ 문서까지 지원. (Phase 65: Qdrant dead config 제거)
backend = "sqlite"
# fastembed BGE-M3 = 1024차원
dim = 1024
semantic_dup_threshold = 0.03
search_top_k = 5

# ── LLM ───────────────────────────────────────────────────
[llm]
# "claude_cli" = Claude Code CLI (PATH에 claude 필요)
# "ollama" / "anthropic_api" / "openai_api" / "gemini"
provider = "claude_cli"
# fallback_providers = ["ollama"]

# ── 검증 ──────────────────────────────────────────────────
# LLM 가공 결과를 6가지 기준으로 자동 검증. 실패 시 피드백 후 재가공.
[verification]
enabled = true
max_retry = 3
on_fail = "quarantine_with_notify"

[verification.thresholds]
structure_min = 0.3
compression_min = 0.05
compression_max = 1.5
keyword_coverage_min = 0.3
keyword_completeness_min = 0.2
rouge_l_min = 0.05
entity_preservation_min = 0.3

# ── 청킹 ─────────────────────────────────────────────────
# 대용량 파일 분할 방식. semantic_enabled=true면 마크다운 구조 인식.
[chunking]
semantic_enabled = true
target_bytes = 1500      # ~375 토큰
max_bytes = 2500         # ~625 토큰
overlap_sentences = 2    # 청크 간 오버랩 문장 수
preserve_code_blocks = true
preserve_tables = false   # 표 마크다운 보존 (트리거 #8 인프라, 디폴트 비활성)

# ── 스케줄 ────────────────────────────────────────────────
[schedule]
retention_days = 30

# ── 민감 문서 ─────────────────────────────────────────────
# 키워드가 파일명/내용에 포함되면 sensitive/ 폴더로 격리 (LLM 전송 안 함)
[sensitive]
keywords = ["비밀번호", "password", "secret", "private_key", "api_key", "token", "credential"]
extensions = [".pem", ".key", ".p12", ".pfx", ".keystore"]

# ── 로깅 ──────────────────────────────────────────────────
[logging]
level = "info"

# ══════════════════════════════════════════════════════════
# 파이프라인 정의 — 고정 단일 파이프라인
# ══════════════════════════════════════════════════════════

[pipelines]

[[pipelines.steps]]
type = "preprocess"
pdf_tool = "none"
ocr_tool = "none"

[[pipelines.steps]]
type = "llm"

[[pipelines.steps]]
type = "verify"
enabled = true

# ── 리랭킹 (검색 결과를 Claude CLI로 관련도 재정렬) ───────
[rerank]
enabled = false
# provider = "claude_cli"
# top_n = 20

# ── 외부 저장소 (가공본/원본 자동 백업) ───────────────────
[remote_storage]
enabled = false
# provider: "network" | "webdav" | "s3"
# provider = "network"
# network_path = "\\\\NAS\\share\\file-pipeline"

# provider = "webdav"
# webdav_url = "https://nextcloud.example.com/remote.php/dav/files/user/"
# webdav_user = ""
# webdav_password = ""

"#;

// ═══════════════════════════════════════════════════════════════
// Phase 82-prep: ProcessingMetricsPort 어댑터 (settings.db 누적)
// ═══════════════════════════════════════════════════════════════

/// settings.db `processing_metrics` 테이블에 누적하는 어댑터.
///
/// DB 락 대기·실패는 silent (`let _ = ...`). 처리 흐름에 영향을 주지 않는다.
/// service.rs의 record 지점에서 호출되며, 매 호출마다 1-row UPSERT 1건이다.
pub struct SettingsDbMetricsAdapter {
    db_path: PathBuf,
}

impl SettingsDbMetricsAdapter {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    fn add(&self, key: &str, delta: i64) {
        if let Ok(db) = settings_db::SettingsDb::open(&self.db_path) {
            let _ = db.add_processing_metric(key, delta);
        }
    }
}

impl file_pipeline_core::ports::output::ProcessingMetricsPort for SettingsDbMetricsAdapter {
    fn record_success(&self) { self.add("success", 1); }
    fn record_error(&self) { self.add("errors", 1); }
    fn record_quarantine(&self) { self.add("quarantined", 1); }
    fn record_verify(&self, passed: bool) {
        self.add(if passed { "verified_pass" } else { "verified_fail" }, 1);
    }
    fn record_process_time(&self, elapsed_ms: u64) {
        let ms = elapsed_ms.min(i64::MAX as u64) as i64;
        self.add("total_time_ms", ms);
        self.add("counted_for_time", 1);
    }
}
