//! 파이프라인 설정의 **순수 데이터 타입** (헥사고날 도메인 분리).
//!
//! 본 모듈은 toml/dirs/env 등 인프라에 의존하지 않는다. struct 정의 + Default impl +
//! serde helper free fn + 순수 메서드(`default_config`/`validate`/`needs_restart`)만 보유.
//! 파일 로드/직렬화/경로 해석 등 인프라 의존 로직은 `file_pipeline_shared::config`에
//! 잔류한다 (extension trait + free fn).

use serde::{Deserialize, Serialize};

use crate::domain::models::{PipelineDefinition, PipelineStep};

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
    pub hooks: Vec<crate::domain::hooks::HookDefinition>,
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
    pub thresholds: Option<crate::domain::verification::VerificationThresholds>,
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
    pub base: std::path::PathBuf,
    pub inbox: std::path::PathBuf,
    /// 추가 inbox 경로 목록
    pub extra_inboxes: Vec<std::path::PathBuf>,
    pub processed: std::path::PathBuf,
    pub originals: std::path::PathBuf,
    pub sensitive: std::path::PathBuf,
    pub todo: std::path::PathBuf,
    pub temp: std::path::PathBuf,
    pub logs: std::path::PathBuf,
    pub models: std::path::PathBuf,
    /// plugin binary 배치 디렉토리 (Phase 200, plugin-architecture-2026-06-04.md §2-A).
    /// 첫 실행 시 자동 생성. 비어있으면 plugin 0개로 정상 부팅.
    pub plugins: std::path::PathBuf,
}

// ── 순수 메서드 (인프라 비의존) ──────────────────────────────
// load/load_from_str/to_toml_string/resolve_paths(env+dirs)/create_all(fs)는
// 인프라 의존이므로 file_pipeline_shared::config에 extension trait로 잔류.

impl PipelineConfig {
    /// 기본 설정 생성 (순수 — 리터럴/Default만)
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

    /// 설정값 검증 (순수 — fs는 onnx 경로 존재 확인에만 사용, dirs/toml 비의존)
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn needs_restart(&self, other: &Self) -> bool {
        self.vector_db.backend != other.vector_db.backend
            || self.embedding.default_model != other.embedding.default_model
            || self.notification.telegram != other.notification.telegram
            || self.notification.slack != other.notification.slack
            || self.llm.provider != other.llm.provider
    }
}

// ── 설정 메타데이터 (Settings UI용, 순수) ────────────────────

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

// ── Phase 77: 설정 스냅샷 순수 타입 (헥사고날 도메인 분리) ────────────
//
// ConfigSnapshot/SnapshotMetrics/RollbackThresholds/RollbackEvaluation +
// evaluate_rollback는 toml/dirs/fs 비의존 순수 데이터·로직이므로 core 보유.
// create_snapshot()/rollback_snapshot() (SetupProfile + fs + PipelineConfigExt 의존)는
// file_pipeline_shared::config_snapshot에 잔류한다.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    /// 16자 hex (millis 기반)
    pub id: String,
    /// RFC3339
    pub created_at: String,
    /// pipeline.toml SHA256 (16 hex)
    pub config_hash: String,
    /// pipeline.toml 원본 (적용 전)
    pub config_backup: String,
    /// 적용 시 사용한 SetupProfile JSON (없으면 None).
    /// core는 SetupProfile 타입을 보유하지 않으므로 직렬화된 JSON 문자열로 유지 (cycle-free).
    pub profile_json: Option<String>,
    /// 적용된 path 목록
    pub applied_paths: Vec<String>,
    /// 측정된 metrics JSON (NULL이면 미측정)
    pub metrics_json: Option<String>,
    pub rolled_back: bool,
    pub rollback_reason: Option<String>,
}

impl ConfigSnapshot {
    pub fn applied_paths_json(&self) -> String {
        serde_json::to_string(&self.applied_paths).unwrap_or_else(|_| "[]".into())
    }

    pub fn metrics(&self) -> Option<SnapshotMetrics> {
        self.metrics_json.as_ref().and_then(|s| serde_json::from_str(s).ok())
    }
}

/// 측정 지표 — stats + lint + verify에서 fetch
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnapshotMetrics {
    /// 측정 시점 RFC3339
    pub measured_at: String,
    /// 측정 시점까지 처리된 문서 수
    pub files_processed: usize,
    /// verify 1-Pass 성공률 (0.0~1.0)
    pub verify_pass_rate: f32,
    /// quarantine 비율 (0.0~1.0)
    pub quarantine_rate: f32,
    /// 평균 처리 시간 (ms, 토큰 사용량 대신 stats.avg_process_ms)
    pub avg_process_time_ms: u64,
    /// lint 경고 수
    pub lint_warnings: usize,
    /// 평균 crossref 링크 수 (문서당)
    pub avg_crossref_per_doc: f32,
}

/// 자동 롤백 조건 (외부 전문가 답변 §4.3)
#[derive(Debug, Clone, Copy)]
pub struct RollbackThresholds {
    /// verify_pass_rate가 이전 대비 N%p 이상 떨어지면 트리거
    pub verify_pass_drop_pp: f32,
    /// quarantine_rate가 이 값 초과면 트리거
    pub quarantine_rate_max: f32,
    /// avg_process_time_ms가 이전 대비 N배 이상이면 트리거
    pub process_time_factor_max: f32,
}

impl Default for RollbackThresholds {
    fn default() -> Self {
        Self {
            verify_pass_drop_pp: 0.15,
            quarantine_rate_max: 0.10,
            process_time_factor_max: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RollbackEvaluation {
    pub should_rollback: bool,
    pub triggers: Vec<String>,
}

/// before와 after 메트릭을 비교해 롤백 권고 여부 산출 (순수 산술)
pub fn evaluate_rollback(
    before: &SnapshotMetrics,
    after: &SnapshotMetrics,
    thresholds: &RollbackThresholds,
) -> RollbackEvaluation {
    let mut triggers = Vec::new();

    let verify_drop = before.verify_pass_rate - after.verify_pass_rate;
    if verify_drop >= thresholds.verify_pass_drop_pp {
        triggers.push(format!(
            "verify_pass_rate {:.1}%p 하락 (임계 {:.1}%p)",
            verify_drop * 100.0,
            thresholds.verify_pass_drop_pp * 100.0,
        ));
    }
    if after.quarantine_rate > thresholds.quarantine_rate_max {
        triggers.push(format!(
            "quarantine_rate {:.1}% (임계 {:.1}%)",
            after.quarantine_rate * 100.0,
            thresholds.quarantine_rate_max * 100.0,
        ));
    }
    if before.avg_process_time_ms > 0 {
        let factor = after.avg_process_time_ms as f32 / before.avg_process_time_ms as f32;
        if factor >= thresholds.process_time_factor_max {
            triggers.push(format!(
                "avg_process_time_ms {:.1}배 증가 (임계 {:.1}배)",
                factor, thresholds.process_time_factor_max,
            ));
        }
    }

    RollbackEvaluation {
        should_rollback: !triggers.is_empty(),
        triggers,
    }
}

#[cfg(test)]
mod config_snapshot_tests {
    use super::*;

    #[test]
    fn test_evaluate_rollback_triggers_on_quarantine() {
        let before = SnapshotMetrics { verify_pass_rate: 0.95, quarantine_rate: 0.02, avg_process_time_ms: 100, ..Default::default() };
        let after = SnapshotMetrics { verify_pass_rate: 0.94, quarantine_rate: 0.15, avg_process_time_ms: 110, ..Default::default() };
        let ev = evaluate_rollback(&before, &after, &RollbackThresholds::default());
        assert!(ev.should_rollback);
        assert!(ev.triggers.iter().any(|t| t.contains("quarantine_rate")));
    }

    #[test]
    fn test_evaluate_rollback_triggers_on_verify_drop() {
        let before = SnapshotMetrics { verify_pass_rate: 0.95, quarantine_rate: 0.02, avg_process_time_ms: 100, ..Default::default() };
        let after = SnapshotMetrics { verify_pass_rate: 0.70, quarantine_rate: 0.05, avg_process_time_ms: 110, ..Default::default() };
        let ev = evaluate_rollback(&before, &after, &RollbackThresholds::default());
        assert!(ev.should_rollback);
        assert!(ev.triggers.iter().any(|t| t.contains("verify_pass_rate")));
    }

    #[test]
    fn test_evaluate_rollback_no_trigger() {
        let before = SnapshotMetrics { verify_pass_rate: 0.95, quarantine_rate: 0.02, avg_process_time_ms: 100, ..Default::default() };
        let after = SnapshotMetrics { verify_pass_rate: 0.94, quarantine_rate: 0.03, avg_process_time_ms: 110, ..Default::default() };
        let ev = evaluate_rollback(&before, &after, &RollbackThresholds::default());
        assert!(!ev.should_rollback);
        assert!(ev.triggers.is_empty());
    }

    #[test]
    fn test_metrics_serde_roundtrip() {
        let m = SnapshotMetrics {
            measured_at: "2026-05-07T10:00:00Z".into(),
            files_processed: 50,
            verify_pass_rate: 0.92,
            quarantine_rate: 0.03,
            avg_process_time_ms: 250,
            lint_warnings: 4,
            avg_crossref_per_doc: 2.5,
        };
        let s = serde_json::to_string(&m).unwrap();
        let r: SnapshotMetrics = serde_json::from_str(&s).unwrap();
        assert_eq!(r.files_processed, 50);
        assert_eq!(r.lint_warnings, 4);
    }
}
