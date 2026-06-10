use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub compression: CompressionConfig,
    #[serde(default)]
    pub vector_db: VectorDbConfig,
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    #[serde(default = "default_notification")]
    pub notification: NotificationConfig,
    #[serde(default)]
    pub verification: VerificationConfig,
    #[serde(default)]
    pub models: ModelsConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    /// 등록된 LLM 크레덴셜 목록
    #[serde(default)]
    pub credentials: Vec<LlmCredential>,
    #[serde(default)]
    pub preprocessing: PreprocessingConfig,
    #[serde(default)]
    pub sensitive: SensitiveConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    /// 파일 동시 처리 수 (기본: 4)
    #[serde(default = "default_max_workers")]
    pub max_workers: usize,
    #[serde(default)]
    pub schedule: ScheduleConfig,
    /// 고정 파이프라인 (단일)
    #[serde(default)]
    pub pipelines: PipelineDefinition,
    /// 청킹 설정
    #[serde(default)]
    pub chunking: ChunkingConfig,
    /// 리랭킹 설정
    #[serde(default)]
    pub rerank: RerankConfig,
    /// 원격 저장소 설정
    #[serde(default)]
    pub remote_storage: RemoteStorageConfig,
    /// 교차참조 설정
    #[serde(default)]
    pub crossref: CrossRefConfig,
    /// 보존 & Purge 설정
    #[serde(default)]
    pub retention: RetentionConfig,
    /// 이벤트 훅 정의 목록
    #[serde(default)]
    pub hooks: Vec<file_pipeline_core::domain::hooks::HookDefinition>,
    /// Phase 71: Memory Tier 분류 임계 (코드 상수 → config 이전)
    #[serde(default)]
    pub memory_tier: MemoryTierConfig,
    /// Phase 71: 검색 후처리 파라미터 (window/MMR/sparse/시간 가중)
    #[serde(default)]
    pub search: SearchConfig,
    /// Phase 71: 알림 배치 요약 주기
    #[serde(default)]
    pub notification_batch: NotificationBatchConfig,
}

/// Phase 71: Memory Tier 분류 임계
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryTierConfig {
    /// hot 분류 기준 (마지막 접근 후 N일 이내)
    #[serde(default = "default_hot_days")]
    pub hot_days: u32,
    /// warm 분류 기준
    #[serde(default = "default_warm_days")]
    pub warm_days: u32,
    /// cold 분류 기준 (이후는 archived)
    #[serde(default = "default_cold_days")]
    pub cold_days: u32,
}
fn default_hot_days() -> u32 { 7 }
fn default_warm_days() -> u32 { 30 }
fn default_cold_days() -> u32 { 90 }
impl Default for MemoryTierConfig {
    fn default() -> Self { Self { hot_days: 7, warm_days: 30, cold_days: 90 } }
}

/// Phase 71: 검색 후처리 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Sentence Window: 매칭 위치 ±N 줄
    #[serde(default = "default_window_lines")]
    pub window_lines: u32,
    /// MMR λ (0.0~1.0). 낮을수록 다양성, 높을수록 관련도 우선
    #[serde(default = "default_mmr_lambda")]
    pub mmr_lambda: f32,
    /// Sparse(BM25) 가중치 (Hybrid Match에서 dense 대비 비율)
    #[serde(default = "default_sparse_weight")]
    pub sparse_weight: f32,
    /// 시간 가중 비율 (recent 모드 boost, 0.10 = +10%)
    #[serde(default = "default_time_weight")]
    pub time_weight: f32,
    /// A2 (Ruflo graph hops 차용): 매칭 문서별 추가할 KG 이웃 수. 0=비활성
    #[serde(default = "default_expand_kg_hops")]
    pub expand_kg_hops: usize,
    /// B1: doc_type 다양성 강제 — 동일 doc_type N건 초과 시 다른 doc_type 1건 강제 포함. 0=비활성
    #[serde(default = "default_diversity_threshold")]
    pub diversity_threshold: usize,
    /// 트리거 #6: HyDE 폴백 검색 활성. 첫 패스 빈약 결과 시 LLM 가상 답변 임베딩으로 재검색.
    /// 디폴트 false (인프라 — lesson 30 패턴, 실사용 "검색 안 됨" 피드백 도달 시 활성).
    #[serde(default)]
    pub hyde_enabled: bool,
    /// HyDE 폴백 발동 임계 — 첫 패스 결과가 이 개수 미만이면 폴백 시도. 디폴트 3.
    #[serde(default = "default_hyde_min_results")]
    pub hyde_min_results: usize,
    /// Phase 91 A2: 출력 시점 PII mask 활성. 검색 결과 / MCP 응답의 PII를
    /// `[REDACTED:kind]`로 마스킹. 디폴트 true (안전 우선).
    #[serde(default = "default_output_pii_mask")]
    pub output_pii_mask: bool,
    /// Phase 103 G4 (GraphRAG TF-IDF 흡수): 검색 후처리 단계에 TF-IDF 다양성 재순위 추가.
    /// fastembed reranker 직후 적용. 본 단계는 본문 토큰 빈도 기반으로 결과 다양화.
    /// 디폴트 false (인프라 — lesson 30 패턴, 사용자 검색 30회+ 후 MRR before/after 측정 후 활성).
    #[serde(default)]
    pub tfidf_rerank_enabled: bool,
    /// Phase 103 G3 (GraphRAG Multi-hop 빔 검색 흡수): A2 KG hop 활성화 시 빔 탐색 적용.
    /// expand_kg_hops > 0 + 본 옵션 true 일 때 빔 폭만큼만 유지. 디폴트 false (인프라).
    #[serde(default)]
    pub kg_beam_search: bool,
}
fn default_window_lines() -> u32 { 5 }
fn default_mmr_lambda() -> f32 { 0.5 }
fn default_sparse_weight() -> f32 { 1.0 }
fn default_time_weight() -> f32 { 0.10 }
fn default_expand_kg_hops() -> usize { 0 }
fn default_diversity_threshold() -> usize { 0 }
fn default_hyde_min_results() -> usize { 3 }
fn default_output_pii_mask() -> bool { true }
impl Default for SearchConfig {
    fn default() -> Self { Self {
        window_lines: 5, mmr_lambda: 0.5, sparse_weight: 1.0, time_weight: 0.10,
        expand_kg_hops: 0, diversity_threshold: 0,
        hyde_enabled: false, hyde_min_results: 3,
        output_pii_mask: true,
        tfidf_rerank_enabled: false,
        kg_beam_search: false,
    } }
}

/// Phase 71: 알림 배치 요약 주기
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationBatchConfig {
    /// 배치 요약 flush 유휴 시간 (초)
    #[serde(default = "default_batch_summary_interval")]
    pub summary_interval_secs: u64,
}
fn default_batch_summary_interval() -> u64 { 30 }
impl Default for NotificationBatchConfig {
    fn default() -> Self { Self { summary_interval_secs: 30 } }
}

/// 보존 & Purge 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// 자동 purge 활성화
    #[serde(default)]
    pub enabled: bool,
    /// 보존 기간 (일)
    #[serde(default = "default_retention_days")]
    pub days: u32,
    /// 삭제 대상 디렉토리 ("originals", "processed", "quarantine")
    #[serde(default = "default_retention_targets")]
    pub targets: Vec<String>,
    /// 자동 실행 주기 (시간)
    #[serde(default = "default_retention_interval")]
    pub interval_hours: u32,
}

fn default_retention_days() -> u32 { 90 }
fn default_retention_targets() -> Vec<String> { vec!["originals".to_string()] }
fn default_retention_interval() -> u32 { 24 }

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            days: default_retention_days(),
            targets: default_retention_targets(),
            interval_hours: default_retention_interval(),
        }
    }
}

/// 교차참조 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossRefConfig {
    /// 교차참조 활성화
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 모드: "auto" (키워드/임베딩 기반, LLM 없음) | "llm" (LLM 보강 판단)
    #[serde(default = "default_crossref_mode")]
    pub mode: String,
    /// 유사도 임계값: 이 이상이면 관계 생성 (auto 모드)
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
    /// Supersedes 판정 임계값 (같은 유형 + 이 이상이면 대체로 판단)
    #[serde(default = "default_supersedes_threshold")]
    pub supersedes_threshold: f32,
    /// 키워드 겹침 최소 수 (RelatedTopic 판정)
    #[serde(default = "default_keyword_overlap")]
    pub keyword_overlap_min: usize,
    /// 검색 후보 수 [deprecated: threshold 기반 전체 스캔으로 전환됨]
    #[serde(default = "default_crossref_top_k")]
    pub top_k: usize,
    /// outgoing cap: Supersedes 최대 수
    #[serde(default = "default_cap_supersedes")]
    pub cap_supersedes: usize,
    /// outgoing cap: Updates 최대 수
    #[serde(default = "default_cap_updates")]
    pub cap_updates: usize,
    /// outgoing cap: RelatedTopic 최대 수
    #[serde(default = "default_cap_related")]
    pub cap_related: usize,
    /// outgoing cap: References 최대 수
    #[serde(default = "default_cap_references")]
    pub cap_references: usize,
    /// incoming cap: 문서당 최대 수신 관계 (0=무제한)
    #[serde(default)]
    pub cap_incoming: usize,
    /// MinHash LSH 강제 활성화 (자동 임계치 무시)
    #[serde(default)]
    pub minhash_force_enable: bool,
    /// MinHash LSH 자동 활성 최소 문서 수
    #[serde(default = "default_minhash_min_docs")]
    pub minhash_min_docs: usize,
    /// 메타데이터 블로킹 활성화 (doc_type 또는 키워드 1개 이상 겹침 필요)
    #[serde(default)]
    pub metadata_blocking: bool,
    /// Phase 71: flush_crossref 비동기 큐 처리 주기 (초). 기본 30초 유휴 후 flush.
    #[serde(default = "default_flush_interval_secs")]
    pub flush_interval_secs: u64,
}

fn default_crossref_mode() -> String { "auto".into() }
fn default_similarity_threshold() -> f32 { 0.8 }  // Phase 64 (트리거 대기 #1): 0.7 → 0.8 상향. 관계 -57.9% (HashEmbedder 100문서 실측)
fn default_supersedes_threshold() -> f32 { 0.95 }
fn default_keyword_overlap() -> usize { 3 }
fn default_crossref_top_k() -> usize { 3 }
fn default_cap_supersedes() -> usize { 2 }
fn default_cap_updates() -> usize { 5 }
fn default_cap_related() -> usize { 20 }
fn default_cap_references() -> usize { 10 }
fn default_minhash_min_docs() -> usize { 3_000 }
fn default_flush_interval_secs() -> u64 { 30 }

impl Default for CrossRefConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: default_crossref_mode(),
            similarity_threshold: default_similarity_threshold(),
            supersedes_threshold: default_supersedes_threshold(),
            keyword_overlap_min: default_keyword_overlap(),
            top_k: default_crossref_top_k(),
            cap_supersedes: default_cap_supersedes(),
            cap_updates: default_cap_updates(),
            cap_related: default_cap_related(),
            cap_references: default_cap_references(),
            cap_incoming: 0,
            minhash_force_enable: false,
            minhash_min_docs: default_minhash_min_docs(),
            metadata_blocking: false,
            flush_interval_secs: default_flush_interval_secs(),
        }
    }
}

/// 의미 단위 청킹 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// 에이전트 청킹 시 의미 단위 사용 (false=기존 40KB 바이트 분할)
    #[serde(default = "default_true")]
    pub semantic_enabled: bool,
    /// 목표 청크 크기 (바이트, 대략 토큰*4)
    #[serde(default = "default_target_bytes")]
    pub target_bytes: usize,
    /// 최대 청크 크기
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
    /// 오버랩 문장 수
    #[serde(default = "default_overlap")]
    pub overlap_sentences: usize,
    /// 코드 펜스 보존
    #[serde(default = "default_true")]
    pub preserve_code_blocks: bool,
    /// 표 마크다운 보존 (`|...|` 표 블록 내부 절단 금지) — Phase 85 트리거 #8 인프라.
    /// 디폴트 false. 표 비중 높은 도메인 진입 시 활성화 (lesson 30 패턴).
    #[serde(default)]
    pub preserve_tables: bool,
    /// Adaptive Chunking 4지표(SC/BI/ICC/DCC) 계산 — arxiv 2603.25333 흡수, 인프라 선구현.
    /// 디폴트 false. 활성화 시 가공 단계에서 Metadata.chunk_quality에 채워짐.
    /// 트리거: 50파일+ 가공 + baseline 측정 후 활성화 (lesson 30 패턴).
    #[serde(default)]
    pub compute_quality: bool,
    /// 청킹 전략 — "semantic"(디폴트) / "fixed" / "recursive" / "adaptive".
    /// Phase B 인프라 선구현 (chunk_by_strategy 단일 진입점).
    /// Adaptive 본체는 Phase C 진입 시 활성. 현재는 semantic 위임 (호환).
    #[serde(default = "default_chunking_strategy")]
    pub strategy: String,
}

fn default_chunking_strategy() -> String { "semantic".to_string() }

fn default_target_bytes() -> usize { 1500 }
fn default_max_bytes() -> usize { 2500 }
fn default_overlap() -> usize { 2 }

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            semantic_enabled: true,
            target_bytes: 1500,
            max_bytes: 2500,
            overlap_sentences: 2,
            preserve_code_blocks: true,
            preserve_tables: false,
            compute_quality: false,
            strategy: "semantic".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rerank_provider")]
    pub provider: String,
    #[serde(default = "default_rerank_top_n")]
    pub top_n: usize,
}

fn default_rerank_provider() -> String { "fastembed".into() }
fn default_rerank_top_n() -> usize { 20 }

impl Default for RerankConfig {
    fn default() -> Self {
        Self { enabled: true, provider: default_rerank_provider(), top_n: default_rerank_top_n() }
    }
}

fn default_max_workers() -> usize { 4 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// [하위호환] 이전 retention_days → RetentionConfig.days로 이관됨
    #[serde(default, skip_serializing)]
    pub retention_days: u64,
    /// lint 실행 주기 (시간 단위, 0=비활성).
    /// Phase 87 (wikidocs 353407)에서 다층 주기로 분화:
    /// - 기본 단주기(이 필드) = 색인 정합성/상한 검사
    /// - `lint_weekly_hours` = 중복·미연결 검사 (느림)
    /// - `lint_monthly_hours` = 오래된·상충 검사 (가장 느림)
    #[serde(default = "default_lint_hours")]
    pub lint_interval_hours: u64,
    /// Phase 87 다층 lint — 주 1회 중복·미연결 검사 주기 (시간, 0=비활성, 기본 168=7일).
    #[serde(default = "default_lint_weekly_hours")]
    pub lint_weekly_hours: u64,
    /// Phase 87 다층 lint — 월 1회 오래된·상충 검사 주기 (시간, 0=비활성, 기본 720=30일).
    #[serde(default = "default_lint_monthly_hours")]
    pub lint_monthly_hours: u64,
    /// fragment 임계값 (이하 글자수는 LLM 스킵, 기본: 100)
    #[serde(default = "default_fragment_threshold")]
    pub fragment_threshold: usize,
    /// fragment 그루핑 트리거 수 (기본: 5)
    #[serde(default = "default_fragment_group_trigger")]
    pub fragment_group_trigger: usize,
    /// Ruflo C1: 자동 추천 주기 (시간 단위, 0=비활성, 기본: 4)
    #[serde(default = "default_auto_suggest_hours")]
    pub auto_suggest_interval_hours: u64,
}

fn default_fragment_threshold() -> usize { 100 }
fn default_fragment_group_trigger() -> usize { 5 }
fn default_lint_hours() -> u64 { 6 }
fn default_lint_weekly_hours() -> u64 { 168 }   // 7일
fn default_lint_monthly_hours() -> u64 { 720 }  // 30일
fn default_auto_suggest_hours() -> u64 { 4 }

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            retention_days: 0,
            lint_interval_hours: default_lint_hours(),
            lint_weekly_hours: default_lint_weekly_hours(),
            lint_monthly_hours: default_lint_monthly_hours(),
            fragment_threshold: default_fragment_threshold(),
            fragment_group_trigger: default_fragment_group_trigger(),
            auto_suggest_interval_hours: default_auto_suggest_hours(),
        }
    }
}

fn default_version() -> String { "1".into() }
fn default_notification() -> NotificationConfig {
    NotificationConfig::default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct PathsConfig {
    pub base: Option<String>,
    pub inbox: Option<String>,
    /// 추가 inbox 경로 목록. 여러 폴더를 감시하려면 여기에 추가.
    #[serde(default)]
    pub extra_inboxes: Vec<String>,
    pub processed: Option<String>,
    pub originals: Option<String>,
    pub sensitive: Option<String>,
    pub todo: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    #[serde(default = "default_zstd_level")]
    pub zstd_level: i32,
    #[serde(default = "default_ttl")]
    pub original_ttl_days: u64,
    #[serde(default = "default_true")]
    pub compress_processed: bool,
    /// 민감 문서 암호화 (향후 구현)
    #[serde(default)]
    pub encrypt_sensitive: bool,
}

fn default_zstd_level() -> i32 { 3 }
fn default_ttl() -> u64 { 30 }
fn default_true() -> bool { true }

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            zstd_level: 3,
            original_ttl_days: 30,
            compress_processed: true,
            encrypt_sensitive: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDbConfig {
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_dup_threshold")]
    pub semantic_dup_threshold: f32,
    #[serde(default = "default_top_k")]
    pub search_top_k: usize,
    /// 임베딩 벡터 차원 (기본: 1024 — fastembed BGE-M3)
    #[serde(default = "default_dim")]
    pub dim: u64,
    /// RRF prefetch 배수 (top_k * rrf_multiplier 만큼 prefetch). 기본: 3
    #[serde(default = "default_rrf_multiplier")]
    pub rrf_multiplier: usize,
}

fn default_dim() -> u64 { 1024 }
fn default_rrf_multiplier() -> usize { 3 }

fn default_backend() -> String { "sqlite".into() }
fn default_dup_threshold() -> f32 { 0.03 }
fn default_top_k() -> usize { 5 }

impl Default for VectorDbConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            semantic_dup_threshold: default_dup_threshold(),
            search_top_k: default_top_k(),
            dim: default_dim(),
            rrf_multiplier: default_rrf_multiplier(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Phase 65: fastembed 고정. UI 비노출, TOML 직접 편집은 power user only.
    #[serde(default = "default_embed_model")]
    pub default_model: String,
    /// 임베딩 입력에 추가할 instruction prefix (예: "Represent this document for retrieval:")
    #[serde(default)]
    pub instruction_prefix: Option<String>,
    /// 모델 캐시 디렉토리 (fastembed: HuggingFace 캐시 / Python ONNX legacy: model.onnx 위치)
    #[serde(default)]
    pub onnx_model_dir: Option<String>,
}

fn default_embed_model() -> String { "fastembed".into() }

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            default_model: default_embed_model(),
            instruction_prefix: None,
            onnx_model_dir: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NotificationConfig {
    #[serde(default)]
    pub telegram: Option<TelegramConfig>,
    #[serde(default)]
    pub slack: Option<SlackConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: Option<String>,
    pub chat_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlackConfig {
    pub bot_token: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub llm_hallucination_check: bool,
    #[serde(default = "default_max_retry")]
    pub max_retry: u32,
    #[serde(default = "default_on_fail")]
    pub on_fail: String,
    /// 검증 임계값 오버라이드 (None이면 코드 기본값)
    #[serde(default)]
    pub thresholds: Option<file_pipeline_core::domain::verification::VerificationThresholds>,
}

fn default_max_retry() -> u32 { 3 }
fn default_on_fail() -> String { "quarantine_with_notify".into() }

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            llm_hallucination_check: false,
            max_retry: 1,
            on_fail: default_on_fail(),
            thresholds: None,
        }
    }
}

/// AI 모델 분업 설정 (ThinkingOS 패턴)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    /// 분류용 모델 (저가, 빠름)
    #[serde(default = "default_classify_model")]
    pub classify_model: String,
    /// 가공용 모델 (고급, 정확)
    #[serde(default = "default_process_model")]
    pub process_model: String,
    /// 검증용 모델
    #[serde(default = "default_verify_model")]
    pub verify_model: String,
}

fn default_classify_model() -> String { "sonnet".into() }
fn default_process_model() -> String { "sonnet".into() }
fn default_verify_model() -> String { "sonnet".into() }

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            classify_model: default_classify_model(),
            process_model: default_process_model(),
            verify_model: default_verify_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// LLM 프로바이더: claude_cli / anthropic_api / openai_api / ollama / gemini
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    #[serde(default)]
    pub anthropic_api_key: Option<String>,
    #[serde(default)]
    pub openai_api_key: Option<String>,
    #[serde(default)]
    pub gemini_api_key: Option<String>,
    /// Ollama 서버 URL (기본: http://localhost:11434)
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,
    /// Ollama 모델명 (기본: llama3)
    #[serde(default = "default_ollama_model")]
    pub ollama_model: String,
    /// OpenAI Chat 모델명 (기본: gpt-4o)
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
    /// Gemini 모델명 (기본: gemini-2.0-flash)
    #[serde(default = "default_gemini_model")]
    pub gemini_model: String,
    /// Fallback 프로바이더 체인 (primary 실패 시 순차 시도)
    #[serde(default)]
    pub fallback_providers: Vec<String>,
    /// 기본 크레덴셜 이름. 설정 시 해당 크레덴셜의 provider/key를 기본으로 사용.
    #[serde(default)]
    pub default_credential: Option<String>,
    /// Ruflo A1 — LLM 결과 캐시 활성화 (settings.db llm_cache 테이블).
    /// 동일 file_hash + content_hash 재호출 시 캐시된 결과 반환.
    #[serde(default = "default_llm_cache_enabled")]
    pub llm_cache_enabled: bool,
    /// Ruflo A1 — LLM 캐시 최대 엔트리 수 (0=무제한). 초과 시 LRU(last_hit_at 가장 오래된 것)로 정리.
    #[serde(default = "default_llm_cache_max_entries")]
    pub llm_cache_max_entries: u64,
}

fn default_llm_provider() -> String { "claude_cli".into() }
fn default_llm_cache_enabled() -> bool { true }
fn default_llm_cache_max_entries() -> u64 { 10_000 }
fn default_ollama_url() -> String { "http://localhost:11434".into() }
fn default_ollama_model() -> String { "llama3".into() }
fn default_openai_model() -> String { "gpt-4o".into() }
fn default_gemini_model() -> String { "gemini-2.0-flash".into() }

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            anthropic_api_key: None,
            openai_api_key: None,
            gemini_api_key: None,
            ollama_url: default_ollama_url(),
            ollama_model: default_ollama_model(),
            openai_model: default_openai_model(),
            gemini_model: default_gemini_model(),
            fallback_providers: vec![],
            default_credential: None,
            llm_cache_enabled: default_llm_cache_enabled(),
            llm_cache_max_entries: default_llm_cache_max_entries(),
        }
    }
}

/// LLM 프로바이더 크레덴셜
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCredential {
    /// 고유 ID (UUID). 미설정 시 자동 생성.
    #[serde(default = "generate_uuid")]
    pub id: String,
    /// 크레덴셜 이름 (사용자 지정, 예: "회사 OpenAI", "개인 Claude")
    pub name: String,
    /// 프로바이더 유형: claude_cli / ollama / anthropic_api / openai_api / gemini
    pub provider: String,
    /// API 키 (anthropic_api, openai_api, gemini)
    #[serde(default)]
    pub api_key: Option<String>,
    /// 서버 URL (ollama)
    #[serde(default)]
    pub url: Option<String>,
    /// 모델명 (ollama, openai_api, gemini)
    #[serde(default)]
    pub model: Option<String>,
    /// Claude CLI 프로필 경로 (claude_cli 전용). CLAUDE_CONFIG_DIR로 전달.
    #[serde(default)]
    pub profile_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreprocessingConfig {
    #[serde(default = "default_pdf_tool")]
    pub pdf_tool: String,
    #[serde(default = "default_ocr_tool")]
    pub ocr_tool: String,
    /// DOCX 전처리 도구: "auto" | "pandoc" | "python" | "libreoffice" | "none"
    #[serde(default = "default_auto_tool")]
    pub docx_tool: String,
    /// XLSX 전처리 도구: "auto" | "pandoc" | "python" | "libreoffice" | "none"
    #[serde(default = "default_auto_tool")]
    pub xlsx_tool: String,
    /// PPTX 전처리 도구: "auto" | "pandoc" | "libreoffice" | "none"
    #[serde(default = "default_auto_tool")]
    pub pptx_tool: String,
    #[serde(default = "default_auto_merge_threshold")]
    pub auto_merge_threshold: usize,
    #[serde(default = "default_max_topic_chars")]
    pub max_topic_chars: usize,
}

fn generate_uuid() -> String {
    generate_credential_id()
}

pub fn generate_credential_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let n: u64 = t.as_nanos() as u64;
    format!("{:016x}", n)
}

fn default_pdf_tool() -> String { "none".into() }
fn default_ocr_tool() -> String { "none".into() }
fn default_auto_tool() -> String { "none".into() }
fn default_auto_merge_threshold() -> usize { 5 }
fn default_max_topic_chars() -> usize { 10000 }

impl Default for PreprocessingConfig {
    fn default() -> Self {
        Self {
            pdf_tool: default_pdf_tool(),
            ocr_tool: default_ocr_tool(),
            docx_tool: default_auto_tool(),
            xlsx_tool: default_auto_tool(),
            pptx_tool: default_auto_tool(),
            auto_merge_threshold: 5,
            max_topic_chars: 10000,
        }
    }
}

// ── 파이프라인 정의 (core에서 re-export) ──────────────────────

pub use file_pipeline_core::domain::models::{PipelineDefinition, PipelineStep};

/// 기본 파이프라인 생성
pub fn default_pipeline() -> PipelineDefinition {
    PipelineDefinition {
        steps: vec![
            PipelineStep::Preprocess { pdf_tool: "none".into(), ocr_tool: "none".into() },
            PipelineStep::Llm { credential: None },
            PipelineStep::Verify { enabled: true, thresholds: None, credential: None },
        ],
        postprocess_credential: None,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SensitiveConfig {
    /// 민감 키워드 목록 (파일 내용에 포함 시 민감 파일로 분류)
    #[serde(default)]
    pub keywords: Vec<String>,
    /// 민감 확장자 목록 (해당 확장자 파일은 민감 파일로 분류)
    #[serde(default)]
    pub extensions: Vec<String>,
    /// (하위 호환) 기존 extra_keywords → keywords로 병합
    #[serde(default, skip_serializing)]
    extra_keywords: Vec<String>,
    /// (하위 호환) 기존 custom_keywords → keywords로 병합
    #[serde(default, skip_serializing)]
    custom_keywords: Vec<String>,
    /// (하위 호환) 기존 extra_extensions → extensions로 병합
    #[serde(default, skip_serializing)]
    extra_extensions: Vec<String>,
}

impl SensitiveConfig {
    /// 하위 호환 필드를 병합한 keywords 반환
    pub fn merged_keywords(&self) -> Vec<String> {
        let mut result = self.keywords.clone();
        result.extend(self.extra_keywords.iter().cloned());
        result.extend(self.custom_keywords.iter().cloned());
        result.sort();
        result.dedup();
        result
    }
    /// 하위 호환 필드를 병합한 extensions 반환
    pub fn merged_extensions(&self) -> Vec<String> {
        let mut result = self.extensions.clone();
        result.extend(self.extra_extensions.iter().cloned());
        result.sort();
        result.dedup();
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_true")]
    pub file: bool,
    #[serde(default = "default_true")]
    pub console: bool,
    #[serde(default = "default_max_mb")]
    pub max_mb: u64,
}

fn default_log_level() -> String { "info".into() }
fn default_max_mb() -> u64 { 100 }

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: true,
            console: true,
            max_mb: 100,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStorageConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_remote_provider")]
    pub provider: String,
    // Network path
    #[serde(default)]
    pub network_path: Option<String>,
    // WebDAV
    #[serde(default)]
    pub webdav_url: Option<String>,
    #[serde(default)]
    pub webdav_user: Option<String>,
    #[serde(default)]
    pub webdav_password: Option<String>,
    #[serde(default)]
    pub webdav_prefix: Option<String>,
    // S3
    #[serde(default)]
    pub s3_endpoint: Option<String>,
    #[serde(default)]
    pub s3_bucket: Option<String>,
    #[serde(default)]
    pub s3_region: Option<String>,
    #[serde(default)]
    pub s3_access_key: Option<String>,
    #[serde(default)]
    pub s3_secret_key: Option<String>,
    #[serde(default)]
    pub s3_prefix: Option<String>,
    // Notion (Phase 90)
    #[serde(default)]
    pub notion_token: Option<String>,
    #[serde(default)]
    pub notion_parent_page_id: Option<String>,
    /// "attach" — zst 파일을 Notion 페이지의 첨부파일로 업로드 (S3/WebDAV 호환)
    /// "page" — 가공본 텍스트를 Notion 마크다운 페이지로 변환 후 자식 페이지 생성
    #[serde(default = "default_notion_mode")]
    pub notion_mode: String,
    /// page 모드에서 사용. 미지정 시 parent_page_id에 직접 자식 페이지 생성
    #[serde(default)]
    pub notion_database_id: Option<String>,
}

fn default_remote_provider() -> String { "network".into() }
fn default_notion_mode() -> String { "attach".into() }

impl Default for RemoteStorageConfig {
    fn default() -> Self {
        Self {
            enabled: false, provider: default_remote_provider(),
            network_path: None,
            webdav_url: None, webdav_user: None, webdav_password: None, webdav_prefix: None,
            s3_endpoint: None, s3_bucket: None, s3_region: None, s3_access_key: None, s3_secret_key: None, s3_prefix: None,
            notion_token: None, notion_parent_page_id: None, notion_mode: default_notion_mode(), notion_database_id: None,
        }
    }
}

/// 설정 해석 후 실제 경로들
#[derive(Debug, Clone)]
pub struct ResolvedPaths {
    pub base: PathBuf,
    pub inbox: PathBuf,
    /// 추가 inbox 경로 목록
    pub extra_inboxes: Vec<PathBuf>,
    pub processed: PathBuf,
    pub originals: PathBuf,
    pub sensitive: PathBuf,
    pub todo: PathBuf,
    pub temp: PathBuf,
    pub logs: PathBuf,
    pub models: PathBuf,
    /// plugin binary 배치 디렉토리 (Phase 200, plugin-architecture-2026-06-04.md §2-A).
    /// 첫 실행 시 자동 생성. 비어있으면 plugin 0개로 정상 부팅.
    pub plugins: PathBuf,
}

impl PipelineConfig {
    /// 설정 파일 로드 (TOML)
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context(format!("설정 파일 읽기 실패: {:?}", path))?;
        toml::from_str(&content).context("TOML 파싱 실패")
    }

    /// TOML 문자열로부터 직접 로드 (apply 후 검증용)
    pub fn load_from_str(s: &str) -> Result<Self> {
        toml::from_str(s).context("TOML 문자열 파싱 실패")
    }

    /// 기본 설정 생성
    pub fn default_config() -> Self {
        Self {
            version: "1".into(),
            paths: PathsConfig::default(),
            compression: CompressionConfig::default(),
            vector_db: VectorDbConfig::default(),
            embedding: EmbeddingConfig::default(),
            notification: NotificationConfig::default(),
            verification: VerificationConfig::default(),
            models: ModelsConfig::default(),
            llm: LlmConfig::default(),
            credentials: vec![],
            preprocessing: PreprocessingConfig::default(),
            sensitive: SensitiveConfig::default(),
            logging: LoggingConfig::default(),
            max_workers: default_max_workers(),
            schedule: ScheduleConfig::default(),
            pipelines: default_pipeline(),
            chunking: ChunkingConfig::default(),
            remote_storage: RemoteStorageConfig::default(),
            rerank: RerankConfig::default(),
            crossref: CrossRefConfig::default(),
            retention: RetentionConfig::default(),
            hooks: vec![],
            memory_tier: MemoryTierConfig::default(),
            search: SearchConfig::default(),
            notification_batch: NotificationBatchConfig::default(),
        }
    }

    /// TOML 문자열로 직렬화
    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self).context("TOML 직렬화 실패")
    }

    /// 설정 우선순위 적용 후 실제 경로 반환
    ///
    /// base 결정은 `find_data_dir`에 위임 — CLI/Tauri가 같은 분기 트리(PIPELINE_BASE →
    /// cwd settings.db/toml → exe_dir → APPDATA)를 사용하도록 통일 (사이드 발견 6 해소).
    /// `config.paths.base`는 명시적 explicit이 없을 때만 적용되도록 find_data_dir 결과 위에 덮어쓴다.
    pub fn resolve_paths(&self, cli_base: Option<&str>) -> ResolvedPaths {
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

impl ResolvedPaths {
    /// 모든 디렉토리 생성
    pub fn create_all(&self) -> Result<()> {
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

// ── 설정 메타데이터 (Settings UI용) ──────────────────────────

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct FieldMeta {
    pub description: &'static str,
    pub field_type: &'static str,
    pub default_value: &'static str,
    pub requires_restart: bool,
}

macro_rules! field {
    ($desc:expr, $ty:expr, $default:expr, restart) => {
        FieldMeta { description: $desc, field_type: $ty, default_value: $default, requires_restart: true }
    };
    ($desc:expr, $ty:expr, $default:expr) => {
        FieldMeta { description: $desc, field_type: $ty, default_value: $default, requires_restart: false }
    };
}

#[allow(dead_code)]
pub fn config_metadata() -> Vec<(&'static str, Vec<(&'static str, FieldMeta)>)> {
    vec![
        ("compression", vec![
            ("zstd_level", field!("Zstandard 압축 레벨 (1=빠름, 22=최대압축)", "integer", "3")),
            ("original_ttl_days", field!("원본 파일 보관 일수 (0=무제한)", "integer", "30")),
            ("compress_processed", field!("가공본도 zstd 압축 적용", "boolean", "true")),
        ]),
        // Phase 65: vector_db dead config 제거 (qdrant_url / collection / dim / auto_start)
        // semantic_dup_threshold + search_top_k는 search 그룹으로 이동 가치 있으나 유지
        ("vector_db", vec![
            ("semantic_dup_threshold", field!("의미 중복 판별 기준. 낮을수록 엄격하게 중복 판정합니다.", "select:0.01=거의 동일한 문서만|0.03=높은 유사도 (권장)|0.05=보통 유사도|0.10=느슨한 유사도|0.20=넓은 범위 중복", "0.03")),
            ("search_top_k", field!("검색 결과 최대 반환 수", "integer", "5")),
        ]),
        // Ruflo A2/B1 토글 (검색 후처리)
        ("search", vec![
            ("expand_kg_hops", field!("KG 1-hop 확장 개수 (0=비활성). 검색 결과 seed의 find_related 호출로 관련 문서를 score=0.0으로 append. 5K 코퍼스 측정 후 권장값 결정.", "integer", "0")),
            ("diversity_threshold", field!("동일 doc_type 결과 임계값 (0=비활성). 초과 시 범위 밖 non-dominant 결과와 swap. 단순 swap (full MMR 아님).", "integer", "0")),
            ("hyde_enabled", field!("HyDE 폴백 검색 활성. 첫 패스 결과 부족(< hyde_min_results) 시 LLM 가상 답변 임베딩으로 재검색 (트리거 #6 인프라, 디폴트 비활성).", "boolean", "false")),
            ("hyde_min_results", field!("HyDE 폴백 발동 임계. 첫 패스 결과가 이 개수 미만이면 LLM 폴백 시도.", "integer", "3")),
        ]),
        // Phase 65: embedding은 fastembed 고정. UI 비노출 (TOML 직접 편집은 power user only).
        // ("embedding", vec![]),
        ("verification", vec![
            ("enabled", field!("문서 가공 후 품질 검증 활성화", "boolean", "true")),
            ("llm_hallucination_check", field!("LLM 환각 탐지 (Claude CLI 추가 호출)", "boolean", "false")),
            ("max_retry", field!("검증 실패 시 재가공 최대 횟수", "integer", "1")),
            ("on_fail", field!("최종 실패 시 처리 (skip_with_notify / quarantine)", "string", "skip_with_notify")),
        ]),
        ("verification.thresholds", vec![
            ("structure_min", field!("LLM 가공 결과의 구조 완전성 최소 비율. sections 키 기반 검사.", "float", "0.5")),
            ("compression_min", field!("LLM 가공 결과의 최소 길이 비율 (가공본/원본). zstd와 무관.", "float", "0.05")),
            ("compression_max", field!("LLM 가공 결과의 최대 길이 비율 (가공본/원본). zstd와 무관.", "float", "1.5")),
            ("keyword_coverage_min", field!("키워드 커버리지 최소 비율 (환각 탐지)", "float", "0.5")),
            ("keyword_completeness_min", field!("키워드 완전성 최소 비율 (누락 탐지)", "float", "0.3")),
            ("rouge_l_min", field!("ROUGE-L 최소 점수", "float", "0.1")),
            ("entity_preservation_min", field!("개체(날짜/금액/URL) 보존 최소 비율", "float", "0.5")),
        ]),
        ("models", vec![
            ("classify_model", field!("분류용 LLM 모델", "string", "sonnet")),
            ("process_model", field!("가공용 LLM 모델", "string", "sonnet")),
            ("verify_model", field!("검증용 LLM 모델", "string", "sonnet")),
        ]),
        ("llm", vec![
            ("provider", field!("LLM 프로바이더 (claude_cli / anthropic_api / openai_api / ollama / gemini)", "string", "claude_cli", restart)),
            ("anthropic_api_key", field!("Anthropic API 키", "secret", "", restart)),
            ("openai_api_key", field!("OpenAI API 키 (LLM용)", "secret", "", restart)),
            ("gemini_api_key", field!("Google Gemini API 키", "secret", "", restart)),
            ("ollama_url", field!("Ollama 서버 URL", "string", "http://localhost:11434", restart)),
            ("ollama_model", field!("Ollama 모델명", "string", "llama3", restart)),
            ("openai_model", field!("OpenAI Chat 모델명", "string", "gpt-4o", restart)),
            ("gemini_model", field!("Gemini 모델명", "string", "gemini-2.0-flash", restart)),
            ("llm_cache_enabled", field!("LLM 결과 캐시 활성화 (Ruflo A1). 동일 파일 재가공 시 claude_cli 호출 회피.", "boolean", "true")),
            ("llm_cache_max_entries", field!("LLM 캐시 최대 엔트리 (0=무제한). 초과 시 last_hit_at 오래된 순으로 LRU 삭제.", "integer", "10000")),
        ]),
        ("preprocessing", vec![
            ("pdf_tool", field!("PDF 변환 도구. none=Claude CLI 직접, marker=마크다운(pip install marker-pdf), pymupdf4llm=빠른 변환(pip install pymupdf4llm)", "select:none=변환 안 함|marker=Marker (pip install marker-pdf)|pymupdf4llm=PyMuPDF4LLM (pip install pymupdf4llm)", "none")),
            ("ocr_tool", field!("OCR 도구. none=비활성, tesseract=로컬(apt install tesseract-ocr), claude_vision=Claude Vision", "select:none=비활성|tesseract=Tesseract (apt install tesseract-ocr)|claude_vision=Claude Vision", "none")),
            ("docx_tool", field!("DOCX 변환 도구. auto=자동 감지, pandoc=범용(https://pandoc.org), python=python-docx(pip install python-docx), libreoffice=LibreOffice(https://libreoffice.org)", "select:auto=자동 감지|pandoc=Pandoc (https://pandoc.org)|python=python-docx (pip install python-docx)|libreoffice=LibreOffice (https://libreoffice.org)|none=비활성", "auto")),
            ("xlsx_tool", field!("XLSX 변환 도구. auto=자동 감지, pandoc=범용, python=openpyxl(pip install openpyxl), libreoffice=LibreOffice", "select:auto=자동 감지|pandoc=Pandoc|python=openpyxl (pip install openpyxl)|libreoffice=LibreOffice|none=비활성", "auto")),
            ("pptx_tool", field!("PPTX 변환 도구. auto=자동 감지, pandoc=범용, libreoffice=LibreOffice", "select:auto=자동 감지|pandoc=Pandoc|libreoffice=LibreOffice|none=비활성", "auto")),
            // Phase 65: 토픽 자동 병합 — UI 노출 (코드는 [preprocessing] 섹션 유지)
            ("auto_merge_threshold", field!("자동 토픽 병합 트리거 문서 수", "integer", "5")),
            ("max_topic_chars", field!("토픽 요약 최대 글자 수", "integer", "10000")),
        ]),
        ("sensitive", vec![
            ("keywords", field!("민감 키워드 목록. 파일 내용에 이 키워드가 포함되면 민감 파일로 분류되어 별도 보관됩니다. 예: 비밀번호, 주민등록번호, 계좌번호. 기본 키워드(password, secret, private_key 등) 외에 추가할 키워드를 입력하세요.", "string_array", "")),
            ("extensions", field!("민감 확장자 목록. 해당 확장자 파일은 내용과 무관하게 민감 파일로 분류됩니다. 예: .pem, .key, .pfx. 기본 확장자(.env, .pem, .key 등) 외에 추가할 확장자를 입력하세요.", "string_array", "")),
        ]),
        ("logging", vec![
            ("level", field!("로그 레벨", "select:trace=Trace (전체)|debug=Debug (상세)|info=Info (일반)|warn=Warn (경고만)|error=Error (에러만)", "info")),
            ("file", field!("파일 로그 출력", "boolean", "true")),
            ("console", field!("콘솔 로그 출력", "boolean", "true")),
            ("max_mb", field!("로그 파일 최대 크기 (MB)", "integer", "100")),
        ]),
        ("schedule", vec![
            ("lint_interval_hours", field!("기본 lint 주기 — 색인 정합성/상한 검사 (시간, 0=비활성). Phase 87 다층 lint 도입 (wikidocs 353407 매일 색인 확인).", "integer", "6")),
            ("lint_weekly_hours", field!("주 1회 lint — 중복·미연결 문서 검사 (시간, 0=비활성, 기본 168=7일).", "integer", "168")),
            ("lint_monthly_hours", field!("월 1회 lint — 오래된·상충 정보 검사 (시간, 0=비활성, 기본 720=30일).", "integer", "720")),
            ("auto_suggest_interval_hours", field!("자동 추천 주기 (시간, 0=비활성). 누적 카운터 분석 → decision_log 자동 INSERT.", "integer", "4")),
        ]),
        ("paths", vec![
            ("extra_inboxes", field!("추가 감시 폴더 경로. 기본 inbox 외에 감시할 절대 경로 목록. + 추가 버튼으로 행을 추가하고 각 행에 한 경로씩 입력. 등록된 모든 폴더에 동일한 파이프라인이 적용됩니다.", "string_array", "")),
        ]),
        ("max_workers", vec![
            ("max_workers", field!("동시 처리 워커 수", "integer", "4", restart)),
        ]),
        // Phase 65: 리랭커는 fastembed 고정. provider 드롭다운 제거.
        ("rerank", vec![
            ("enabled", field!("리랭킹 활성화 (BGE-Reranker-v2-M3 Cross-Encoder, 로컬)", "boolean", "true")),
            ("top_n", field!("리랭킹 상위 N개 결과", "integer", "20")),
        ]),
        ("chunking", vec![
            ("semantic_enabled", field!("의미 단위 청킹 사용. false일 경우 기존 40KB 바이트 분할", "boolean", "true")),
            ("target_bytes", field!("목표 청크 크기 (바이트). 대략 토큰수×4", "integer", "1500")),
            ("max_bytes", field!("최대 청크 크기 (바이트). 초과 시 강제 분할", "integer", "2500")),
            ("overlap_sentences", field!("청크 간 오버랩 문장 수. 맥락 유지를 위한 중첩", "integer", "2")),
            ("preserve_code_blocks", field!("코드 펜스(```) 블록을 분할하지 않고 보존", "boolean", "true")),
            ("preserve_tables", field!("표 마크다운(`|...|`) 블록을 분할하지 않고 보존. 표 비중 높은 도메인에서 활성화 권장 (트리거 #8 인프라)", "boolean", "false")),
        ]),
        ("crossref", vec![
            ("enabled", field!("교차참조 활성화", "boolean", "true")),
            ("mode", field!("교차참조 모드", "select:auto=자동 (LLM 없음)|llm=LLM 보강|off=비활성", "auto")),
            ("similarity_threshold", field!("유사도 임계값. 이 이상이면 관계 생성. Phase 64 디폴트 0.8 (관계 노이즈 감소). HashEmbedder/BGE-M3 모두 0.8 권장", "float", "0.8")),
            ("supersedes_threshold", field!("Supersedes 판정 임계값 (같은 유형 + 이 이상이면 대체)", "float", "0.95")),
            ("keyword_overlap_min", field!("RelatedTopic 최소 키워드 겹침 수", "integer", "3")),
            ("cap_supersedes", field!("outgoing Supersedes 최대 수 (문서당)", "integer", "2")),
            ("cap_updates", field!("outgoing Updates 최대 수 (문서당)", "integer", "5")),
            ("cap_related", field!("outgoing RelatedTopic 최대 수 (문서당)", "integer", "20")),
            ("cap_references", field!("outgoing References 최대 수 (문서당)", "integer", "10")),
            ("cap_incoming", field!("incoming 관계 최대 수 (0=무제한)", "integer", "0")),
            ("minhash_force_enable", field!("MinHash LSH 강제 활성 (자동 임계치 무시)", "boolean", "false")),
            ("minhash_min_docs", field!("MinHash LSH 자동 활성 최소 문서 수", "integer", "3000")),
            ("metadata_blocking", field!("메타데이터 블로킹 (doc_type 또는 키워드 1개 이상 겹침 필요)", "boolean", "false")),
            ("flush_interval_secs", field!("flush_crossref 비동기 큐 처리 주기 (초)", "integer", "30")),
        ]),
        // Phase 71: Memory Tier 분류 임계
        ("memory_tier", vec![
            ("hot_days", field!("hot 분류 기준 (마지막 접근 후 N일 이내)", "integer", "7")),
            ("warm_days", field!("warm 분류 기준", "integer", "30")),
            ("cold_days", field!("cold 분류 기준 (이후는 archived)", "integer", "90")),
        ]),
        // Phase 71: 검색 후처리 파라미터
        ("search", vec![
            ("window_lines", field!("Sentence Window: 매칭 위치 ±N 줄", "integer", "5")),
            ("mmr_lambda", field!("MMR λ (0.0~1.0). 낮을수록 다양성, 높을수록 관련도 우선", "float", "0.5")),
            ("sparse_weight", field!("Sparse(BM25) 가중치 (Hybrid Match에서 dense 대비 비율)", "float", "1.0")),
            ("time_weight", field!("시간 가중 비율 (recent 모드 boost, 0.10 = +10%)", "float", "0.10")),
        ]),
        // Phase 71: 알림 배치 요약 주기
        ("notification_batch", vec![
            ("summary_interval_secs", field!("배치 요약 flush 유휴 시간 (초)", "integer", "30")),
        ]),
    ]
}

#[allow(dead_code)]
impl PipelineConfig {
    /// 설정값 검증
    pub fn validate(&self) -> std::result::Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.compression.zstd_level < 1 || self.compression.zstd_level > 22 {
            errors.push("compression.zstd_level: 1~22 범위여야 합니다".into());
        }
        // Phase 65: Qdrant 제거 후 sqlite만 유효 (LocalVectorStore = sqlite backend label)
        let valid_backends = ["sqlite"];
        if !valid_backends.contains(&self.vector_db.backend.as_str()) {
            errors.push(format!("vector_db.backend: {:?} 중 하나여야 합니다", valid_backends));
        }
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            errors.push(format!("logging.level: {:?} 중 하나여야 합니다", valid_levels));
        }

        // 파이프라인 검증 (단일)
        {
            let has_llm = self.pipelines.steps.iter().any(|s| matches!(s, PipelineStep::Llm { .. }));
            if !has_llm && !self.pipelines.steps.is_empty() {
                errors.push("pipelines: LLM 스텝이 최소 1개 필요합니다".to_string());
            }
        }

        // ONNX legacy 모델 경로 검증 (Python ONNX 어댑터 사용 시)
        if self.embedding.default_model == "onnx" || self.embedding.default_model == "bge_m3" {
            if let Some(ref dir) = self.embedding.onnx_model_dir {
                let dir_path = std::path::Path::new(dir);
                if !dir_path.join("model.onnx").exists() {
                    errors.push(format!("embedding.onnx_model_dir: model.onnx 파일을 찾을 수 없습니다: {}/model.onnx", dir));
                }
                if !dir_path.join("tokenizer.json").exists() {
                    errors.push(format!("embedding.onnx_model_dir: tokenizer.json 파일을 찾을 수 없습니다: {}/tokenizer.json", dir));
                }
            }
        }

        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    /// 재시작 필요 여부 판별 (Phase 65: dead config 제거)
    pub fn needs_restart(&self, other: &Self) -> bool {
        self.vector_db.backend != other.vector_db.backend
            || self.embedding.default_model != other.embedding.default_model
            || self.notification.telegram != other.notification.telegram
            || self.notification.slack != other.notification.slack
            || self.llm.provider != other.llm.provider
    }
}

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
    let db = crate::settings_db::SettingsDb::open_or_migrate(&data_dir)?;
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
