//! 파이프라인 설정 — 인프라 의존 로직 (로드/직렬화/경로 해석/DB).
//!
//! **순수 데이터 타입**(struct + Default + serde helper + UI 메타데이터 + 순수 메서드
//! `default_config`/`validate`/`needs_restart`)은 `file_pipeline_core::domain::config_models`로
//! 이전되었다 (헥사고날 도메인 분리). 본 모듈은 toml/dirs/env/fs 의존 로직만 보유하며,
//! 이전된 타입을 아래에서 `pub use`로 re-export 하므로 기존 호출처는 변경 없이 동작한다.
//!
//! **orphan rule 주의**: re-export된 타입은 core 소속이므로 shared에서 inherent impl을
//! 추가할 수 없다. 따라서 인프라 의존 메서드(`load`/`load_from_str`/`to_toml_string`/
//! `resolve_paths`/`create_all`)는 extension trait(`PipelineConfigExt`/`ResolvedPathsExt`)로
//! 제공한다. 호출처는 `use file_pipeline_shared::config::{PipelineConfigExt, ResolvedPathsExt}`로
//! 트레이트를 스코프에 들여야 한다 (shared 내부 호출처는 `crate::config::...` 경로 사용).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

// ── core/domain/config_models 순수 타입 re-export (호출처 변경 0건) ──
pub use file_pipeline_core::domain::config_models::{
    generate_credential_id, config_metadata, default_pipeline,
    ChunkingConfig, CompressionConfig, CrossRefConfig, EmbeddingConfig, FieldMeta,
    LlmConfig, LlmCredential, LoggingConfig, MemoryTierConfig, ModelsConfig,
    NotificationBatchConfig, NotificationConfig, PathsConfig, PipelineConfig,
    PreprocessingConfig, RemoteStorageConfig, RerankConfig, ResolvedPaths, RetentionConfig,
    ScheduleConfig, SearchConfig, SensitiveConfig, SlackConfig, TelegramConfig,
    VectorDbConfig, VerificationConfig,
};

// 파이프라인 정의 (core models에서 re-export — 기존 경로 호환)
pub use file_pipeline_core::domain::models::{PipelineDefinition, PipelineStep};

// ═══════════════════════════════════════════════════════════════
// 인프라 의존 메서드 — extension trait (orphan rule 회피)
// ═══════════════════════════════════════════════════════════════

/// `PipelineConfig`의 인프라 의존 메서드 (toml/env 의존).
///
/// core의 순수 타입에 inherent impl을 추가할 수 없어(orphan rule) trait로 분리.
/// 호출처에서 `use file_pipeline_shared::config::PipelineConfigExt;` 필요.
pub trait PipelineConfigExt: Sized {
    /// 설정 파일 로드 (TOML)
    fn load(path: &Path) -> Result<Self>;
    /// TOML 문자열로부터 직접 로드 (apply 후 검증용)
    fn load_from_str(s: &str) -> Result<Self>;
    /// TOML 문자열로 직렬화
    fn to_toml_string(&self) -> Result<String>;
    /// 설정 우선순위 적용 후 실제 경로 반환
    fn resolve_paths(&self, cli_base: Option<&str>) -> ResolvedPaths;
}

impl PipelineConfigExt for PipelineConfig {
    fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context(format!("설정 파일 읽기 실패: {:?}", path))?;
        toml::from_str(&content).context("TOML 파싱 실패")
    }

    fn load_from_str(s: &str) -> Result<Self> {
        toml::from_str(s).context("TOML 문자열 파싱 실패")
    }

    fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self).context("TOML 직렬화 실패")
    }

    /// 설정 우선순위 적용 후 실제 경로 반환
    ///
    /// base 결정은 `find_data_dir`에 위임 — CLI/Tauri가 같은 분기 트리(PIPELINE_BASE →
    /// cwd settings.db/toml → exe_dir → APPDATA)를 사용하도록 통일 (사이드 발견 6 해소).
    /// `config.paths.base`는 명시적 explicit이 없을 때만 적용되도록 find_data_dir 결과 위에 덮어쓴다.
    fn resolve_paths(&self, cli_base: Option<&str>) -> ResolvedPaths {
        let base = if let Some(cli) = cli_base {
            PathBuf::from(cli)
        } else if std::env::var("PIPELINE_BASE").ok().filter(|s| !s.trim().is_empty()).is_some() {
            // PIPELINE_BASE를 가장 우선 — find_data_dir도 동일 분기
            find_data_dir(None)
        } else if let Some(cfg_base) = self.paths.base.as_ref() {
            PathBuf::from(cfg_base)
        } else {
            find_data_dir(None)
        };

        let resolve = |env_var: &str, config_val: &Option<String>, subdir: &str| -> PathBuf {
            std::env::var(env_var)
                .ok()
                .map(PathBuf::from)
                .or_else(|| config_val.as_ref().map(PathBuf::from))
                .unwrap_or_else(|| base.join(subdir))
        };

        let extra_inboxes: Vec<PathBuf> = self.paths.extra_inboxes
            .iter()
            .map(PathBuf::from)
            .collect();

        ResolvedPaths {
            inbox: resolve("PIPELINE_INBOX", &self.paths.inbox, "inbox"),
            extra_inboxes,
            processed: resolve("PIPELINE_PROCESSED", &self.paths.processed, "processed"),
            originals: resolve("PIPELINE_ORIGINALS", &self.paths.originals, "originals"),
            sensitive: resolve("PIPELINE_SENSITIVE", &self.paths.sensitive, "sensitive"),
            todo: self.paths.todo.as_ref().map(PathBuf::from).unwrap_or_else(|| base.join("todo")),
            temp: base.join(".tmp"),
            logs: base.join("logs"),
            models: base.join("models"),
            // Phase 200 plugin 폴더 — 첫 실행 시 자동 생성 (plugin-architecture-2026-06-04.md §2-A)
            plugins: std::env::var("PIPELINE_PLUGINS")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| base.join("plugins")),
            base,
        }
    }
}

/// `ResolvedPaths`의 인프라 의존 메서드 (std::fs).
///
/// core의 순수 타입에 inherent impl을 추가할 수 없어(orphan rule) trait로 분리.
/// 호출처에서 `use file_pipeline_shared::config::ResolvedPathsExt;` 필요.
pub trait ResolvedPathsExt {
    /// 모든 디렉토리 생성
    fn create_all(&self) -> Result<()>;
}

impl ResolvedPathsExt for ResolvedPaths {
    fn create_all(&self) -> Result<()> {
        // quarantine은 ResolvedPaths 필드는 없지만 base.join("quarantine")으로 사용됨
        // (사이드 발견 3): 검증 실패 시 lazy 생성 의존 제거
        let quarantine = self.base.join("quarantine");
        for dir in [
            &self.base,
            &self.inbox,
            &self.processed,
            &self.originals,
            &self.sensitive,
            &self.todo,
            &quarantine,
            &self.temp,
            &self.logs,
            &self.models,
            // Phase 200 plugin 폴더 — 비어있으면 plugin 0개로 정상 부팅 (plugin-architecture-2026-06-04.md §2-A)
            &self.plugins,
        ] {
            std::fs::create_dir_all(dir)
                .context(format!("디렉토리 생성 실패: {:?}", dir))?;
        }
        for extra in &self.extra_inboxes {
            std::fs::create_dir_all(extra)
                .context(format!("extra inbox 생성 실패: {:?}", extra))?;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════
// 데이터 디렉토리 / 설정 경로 탐색 (dirs/env/fs/toml/DB 의존)
// ═══════════════════════════════════════════════════════════════

/// 바이너리가 있는 디렉토리 반환
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 데이터 디렉토리 결정 (settings.db 위치)
/// 탐색 순서: 명시경로 → cwd(settings.db) → exe 디렉토리 → %APPDATA%\FilePipeline → cwd
pub fn find_data_dir(explicit_path: Option<&str>) -> PathBuf {
    if let Some(path) = explicit_path {
        return PathBuf::from(path);
    }
    // 0. PIPELINE_BASE 환경변수 (lesson 29 — CLI ↔ Tauri 통합)
    if let Ok(base) = std::env::var("PIPELINE_BASE") {
        if !base.trim().is_empty() {
            return PathBuf::from(base);
        }
    }
    // 1. 현재 디렉토리에 settings.db 존재
    let local = Path::new("settings.db");
    if local.exists() {
        return PathBuf::from(".");
    }
    // 2. 현재 디렉토리에 pipeline.toml 존재 (마이그레이션 대상)
    let local_toml = Path::new("pipeline.toml");
    if local_toml.exists() {
        return PathBuf::from(".");
    }
    // 3. 바이너리 디렉토리
    let exe = exe_dir();
    if exe.join("settings.db").exists() || exe.join("pipeline.toml").exists() {
        return exe;
    }
    // 4. %APPDATA%\FilePipeline
    if let Some(config_dir) = dirs::config_dir() {
        let app_dir = config_dir.join("FilePipeline");
        if app_dir.join("settings.db").exists() || app_dir.join("pipeline.toml").exists() {
            return app_dir;
        }
    }
    // 기본: PIPELINE_BASE 미설정 + 기존 DB 미존재 → exe 디렉토리
    exe_dir()
}

/// SettingsDb에서 설정 + 레지스트리를 로드하는 통합 함수
/// TOML 파일이 있으면 자동 마이그레이션
/// 프롬프트도 DB에서 로드하여 adapters에 주입
pub fn load_from_db(explicit_data_dir: Option<&str>) -> Result<(
    crate::settings_db::SettingsDb,
    PipelineConfig,
    file_pipeline_core::domain::models::DocTypeRegistry,
)> {
    let data_dir = find_data_dir(explicit_data_dir);
    std::fs::create_dir_all(&data_dir)?;
    let db = crate::settings_db::open_or_migrate(&data_dir)?;
    let config = db.to_pipeline_config()?;
    let registry = db.to_doc_type_registry()?;

    // DB에서 프롬프트 로드 → adapters에 주입
    let classify = db.get_prompt("classify").ok().flatten();
    let reprocess = db.get_prompt("reprocess_suffix").ok().flatten();
    let summarize_text = db.get_prompt("summarize_text").ok().flatten();
    if classify.is_some() || reprocess.is_some() || summarize_text.is_some() {
        file_pipeline_adapters::driven::llm::prompts::inject_prompts(
            classify.as_deref(),
            reprocess.as_deref(),
            summarize_text.as_deref(),
        );
    }

    Ok((db, config, registry))
}

/// 설정 파일 경로 탐색 (로드 없이)
/// 탐색 순서: 명시경로 → cwd → 바이너리 디렉토리 → %APPDATA% → cwd(기본 생성 위치)
pub fn find_config_path(explicit_path: Option<&str>) -> PathBuf {
    if let Some(path) = explicit_path {
        return PathBuf::from(path);
    }
    // 1. 현재 작업 디렉토리
    let local = Path::new("pipeline.toml");
    if local.exists() {
        return local.to_path_buf();
    }
    // 2. 바이너리가 있는 디렉토리
    let exe = exe_dir().join("pipeline.toml");
    if exe.exists() {
        return exe;
    }
    // 3. %APPDATA%\FilePipeline
    if let Some(config_dir) = dirs::config_dir() {
        let app_config = config_dir.join("FilePipeline").join("pipeline.toml");
        if app_config.exists() {
            return app_config;
        }
    }
    // 기본: 바이너리 디렉토리에 생성
    exe_dir().join("pipeline.toml")
}

/// doc_types.toml에서 DocTypeRegistry 로드
pub fn load_doc_type_registry(path: &Path) -> Result<file_pipeline_core::domain::models::DocTypeRegistry> {
    use file_pipeline_core::domain::models::{DocTypeDef, DocTypeRegistry};

    #[derive(Deserialize)]
    struct DocTypesFile {
        #[serde(default)]
        types: Vec<DocTypeDef>,
    }

    if !path.exists() {
        // Phase 89 C-3: doc_types.toml 옵션화. settings.db의 doc_types 테이블이 단일 진실원
        // (첫 실행 시 17 기본 유형 자동 마이그레이션). 본 파일은 외부 편집 진입점.
        // 파일 미존재 시 settings.db에서 로드 — find_data_dir 사용해 동일 분기 트리.
        let data_dir = find_data_dir(None);
        if let Ok(db) = crate::settings_db::SettingsDb::open(&data_dir.join("settings.db")) {
            if let Ok(registry) = db.to_doc_type_registry() {
                if !registry.all().is_empty() {
                    tracing::info!("doc_types: settings.db에서 {} 유형 로드 (파일 없음)", registry.all().len());
                    return Ok(registry);
                }
            }
        }
        tracing::debug!("doc_types.toml 없음 + settings.db 미접근: 빈 레지스트리 사용");
        return Ok(DocTypeRegistry::empty());
    }

    let content = std::fs::read_to_string(path)
        .context(format!("doc_types.toml 읽기 실패: {:?}", path))?;
    let file: DocTypesFile = toml::from_str(&content)
        .context("doc_types.toml 파싱 실패")?;

    tracing::info!("doc_types.toml 로드: {} 유형", file.types.len());
    Ok(DocTypeRegistry::new(file.types))
}

/// doc_types.toml 경로 결정
/// 탐색 순서: cwd → 바이너리 디렉토리 → base 디렉토리
pub fn resolve_doc_types_path(paths: &ResolvedPaths) -> PathBuf {
    // 1. 현재 작업 디렉토리
    let local = Path::new("doc_types.toml");
    if local.exists() {
        return local.to_path_buf();
    }
    // 2. 바이너리가 있는 디렉토리
    let exe = exe_dir().join("doc_types.toml");
    if exe.exists() {
        return exe;
    }
    // 3. base 디렉토리
    let base = paths.base.join("doc_types.toml");
    if base.exists() {
        return base;
    }
    // 기본: 바이너리 디렉토리
    exe_dir().join("doc_types.toml")
}

/// 설정 파일 탐색 순서대로 로드 시도
/// 탐색 순서: 명시경로 → cwd → 바이너리 디렉토리 → %APPDATA% → 기본값
pub fn find_and_load_config(explicit_path: Option<&str>) -> Result<PipelineConfig> {
    if let Some(path) = explicit_path {
        return PipelineConfig::load(Path::new(path));
    }
    let local = Path::new("pipeline.toml");
    if local.exists() {
        return PipelineConfig::load(local);
    }
    let exe = exe_dir().join("pipeline.toml");
    if exe.exists() {
        return PipelineConfig::load(&exe);
    }
    if let Some(config_dir) = dirs::config_dir() {
        let app_config = config_dir.join("FilePipeline").join("pipeline.toml");
        if app_config.exists() {
            return PipelineConfig::load(&app_config);
        }
    }
    Ok(PipelineConfig::default_config())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_expected_values() {
        let cfg = PipelineConfig::default_config();
        assert_eq!(cfg.compression.zstd_level, 3);
        assert_eq!(cfg.max_workers, 4);
        // Phase 65: vector_db backend default qdrant→sqlite (Qdrant 제거 후 LocalVectorStore 단일)
        assert_eq!(cfg.vector_db.backend, "sqlite");
        // Phase 65: 임베딩 default fastembed 고정
        assert_eq!(cfg.embedding.default_model, "fastembed");
        // Phase 65: 리랭킹 default true + fastembed
        assert!(cfg.rerank.enabled);
        assert_eq!(cfg.rerank.provider, "fastembed");
        assert_eq!(cfg.llm.provider, "claude_cli");
        assert_eq!(cfg.verification.max_retry, 1);
        assert_eq!(cfg.pipelines.steps.len(), 3); // Preprocess + Llm + Verify
        assert!(cfg.credentials.is_empty());
        assert_eq!(cfg.version, "1");
        assert!(cfg.verification.enabled);
    }

    #[test]
    fn test_toml_roundtrip() {
        let original = PipelineConfig::default_config();
        let toml_str = original.to_toml_string().expect("직렬화 실패");
        let parsed: PipelineConfig = toml::from_str(&toml_str).expect("역직렬화 실패");

        assert_eq!(parsed.compression.zstd_level, original.compression.zstd_level);
        assert_eq!(parsed.max_workers, original.max_workers);
        assert_eq!(parsed.vector_db.backend, original.vector_db.backend);
        assert_eq!(parsed.llm.provider, original.llm.provider);
        assert_eq!(parsed.verification.max_retry, original.verification.max_retry);
        assert_eq!(parsed.embedding.default_model, original.embedding.default_model);
        assert_eq!(parsed.models.classify_model, original.models.classify_model);
    }

    #[test]
    fn test_partial_toml_parsing() {
        let partial = r#"
[compression]
zstd_level = 5
"#;
        let cfg: PipelineConfig = toml::from_str(partial).expect("부분 TOML 파싱 실패");
        assert_eq!(cfg.compression.zstd_level, 5);
        // 나머지는 기본값
        assert_eq!(cfg.max_workers, 4);
        assert_eq!(cfg.llm.provider, "claude_cli");
        // Phase 65: default backend qdrant→sqlite
        assert_eq!(cfg.vector_db.backend, "sqlite");
        assert!(cfg.verification.enabled);
    }

    #[test]
    fn test_pipeline_definition_serde() {
        use file_pipeline_core::domain::verification::VerificationThresholds;

        let pd = PipelineDefinition {
            steps: vec![
                PipelineStep::Preprocess {
                    pdf_tool: "marker".into(),
                    ocr_tool: "tesseract".into(),
                },
                PipelineStep::Llm {
                    credential: Some("my-claude".into()),
                },
                PipelineStep::Verify {
                    enabled: true,
                    thresholds: Some(VerificationThresholds {
                        structure_min: 0.6,
                        compression_min: 0.1,
                        compression_max: 2.0,
                        keyword_coverage_min: 0.4,
                        keyword_completeness_min: 0.3,
                        rouge_l_min: 0.15,
                        entity_preservation_min: 0.6,
                    }),
                    credential: Some("my-verify-llm".into()),
                },
                PipelineStep::Embedding {
                    model: Some("bge_m3".into()),
                    credential: Some("my-embed-cred".into()),
                },
                PipelineStep::Storage {
                    zstd_level: 5,
                },
            ],
            postprocess_credential: Some("my-postprocess".into()),
        };

        let toml_str = toml::to_string_pretty(&pd).expect("PipelineDefinition 직렬화 실패");
        let parsed: PipelineDefinition =
            toml::from_str(&toml_str).expect("PipelineDefinition 역직렬화 실패");

        assert_eq!(parsed.steps.len(), 5);
        assert_eq!(
            parsed.postprocess_credential.as_deref(),
            Some("my-postprocess")
        );

        // 각 스텝 검증
        match &parsed.steps[0] {
            PipelineStep::Preprocess { pdf_tool, ocr_tool } => {
                assert_eq!(pdf_tool, "marker");
                assert_eq!(ocr_tool, "tesseract");
            }
            other => panic!("expected Preprocess, got {:?}", other),
        }
        match &parsed.steps[1] {
            PipelineStep::Llm { credential } => {
                assert_eq!(credential.as_deref(), Some("my-claude"));
            }
            other => panic!("expected Llm, got {:?}", other),
        }
        match &parsed.steps[2] {
            PipelineStep::Verify {
                enabled,
                thresholds,
                credential,
            } => {
                assert!(*enabled);
                assert!(thresholds.is_some());
                let t = thresholds.as_ref().expect("thresholds");
                assert!((t.structure_min - 0.6).abs() < f64::EPSILON);
                assert_eq!(credential.as_deref(), Some("my-verify-llm"));
            }
            other => panic!("expected Verify, got {:?}", other),
        }
        match &parsed.steps[3] {
            PipelineStep::Embedding { model, credential } => {
                assert_eq!(model.as_deref(), Some("bge_m3"));
                assert_eq!(credential.as_deref(), Some("my-embed-cred"));
            }
            other => panic!("expected Embedding, got {:?}", other),
        }
        match &parsed.steps[4] {
            PipelineStep::Storage { zstd_level } => {
                assert_eq!(*zstd_level, 5);
            }
            other => panic!("expected Storage, got {:?}", other),
        }
    }

    #[test]
    fn test_credential_serde() {
        let cred = LlmCredential {
            id: "test-id-001".into(),
            name: "회사 OpenAI".into(),
            provider: "openai_api".into(),
            api_key: Some("sk-test-key-12345".into()),
            url: Some("https://api.openai.com".into()),
            model: Some("gpt-4o".into()),
            profile_path: Some("/home/user/.claude".into()),
        };

        let toml_str = toml::to_string_pretty(&cred).expect("LlmCredential 직렬화 실패");
        let parsed: LlmCredential =
            toml::from_str(&toml_str).expect("LlmCredential 역직렬화 실패");

        assert_eq!(parsed.id, "test-id-001");
        assert_eq!(parsed.name, "회사 OpenAI");
        assert_eq!(parsed.provider, "openai_api");
        assert_eq!(parsed.api_key.as_deref(), Some("sk-test-key-12345"));
        assert_eq!(parsed.url.as_deref(), Some("https://api.openai.com"));
        assert_eq!(parsed.model.as_deref(), Some("gpt-4o"));
        assert_eq!(parsed.profile_path.as_deref(), Some("/home/user/.claude"));
    }

    #[test]
    fn test_load_from_db_creates_defaults() {
        let dir = tempfile::TempDir::new().expect("tmpdir");
        let (db, cfg, registry) = load_from_db(Some(dir.path().to_str().expect("path"))).expect("load_from_db");
        assert_eq!(cfg.compression.zstd_level, 3);
        assert!(db.path().exists() || db.path().to_str() == Some(":memory:"));
        // registry는 비어있을 수 있음 (기본 doc_types 없이 시작)
        let _ = registry;
    }

    #[test]
    fn test_load_from_db_migrates_toml() {
        let dir = tempfile::TempDir::new().expect("tmpdir");

        // pipeline.toml 생성
        let config = PipelineConfig::default_config();
        let toml_str = config.to_toml_string().expect("toml");
        std::fs::write(dir.path().join("pipeline.toml"), &toml_str).expect("write");

        let (_db, cfg, _registry) = load_from_db(Some(dir.path().to_str().expect("path"))).expect("load_from_db");
        assert_eq!(cfg.compression.zstd_level, 3);

        // TOML 파일이 .bak으로 이동됨
        assert!(!dir.path().join("pipeline.toml").exists());
        assert!(dir.path().join("pipeline.toml.bak").exists());
    }

    #[test]
    fn test_find_data_dir_with_explicit_path() {
        let dir = tempfile::TempDir::new().expect("tmpdir");
        let result = find_data_dir(Some(dir.path().to_str().expect("path")));
        assert_eq!(result, dir.path());
    }
}
