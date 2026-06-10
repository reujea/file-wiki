use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ── 날짜 출처 구분 (교차참조 Supersedes/Updates 판정용) ────────

/// 문서 날짜의 출처를 추적하여 교차참조 판정 정확도를 보장
#[derive(Debug, Clone, PartialEq)]
pub enum DocDate {
    /// 본문/메타데이터에서 추출한 명시적 날짜
    Explicit(String),
    /// 파일 시스템 mtime
    FileMtime(String),
    /// 날짜 불명
    Unknown,
}

impl DocDate {
    /// String 날짜를 DocDate로 변환 (빈 문자열 → Unknown)
    pub fn from_string(date: &str) -> Self {
        if date.is_empty() || date == "unknown" {
            DocDate::Unknown
        } else {
            DocDate::Explicit(date.to_string())
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            DocDate::Explicit(s) | DocDate::FileMtime(s) => s,
            DocDate::Unknown => "",
        }
    }

    /// 두 날짜가 모두 신뢰 가능(Explicit)인지
    pub fn both_reliable(a: &DocDate, b: &DocDate) -> bool {
        matches!((a, b), (DocDate::Explicit(_), DocDate::Explicit(_)))
    }
}

// ── 문서 유형 정의 (doc_types.toml에서 런타임 로드) ──────────

/// 단일 문서 유형 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTypeDef {
    pub id: String,
    pub label_ko: String,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub sections: Vec<String>,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub dedup_key: Option<String>,
    #[serde(default)]
    pub sensitive: bool,
    /// 유형별 검증 임계값 오버라이드 (None이면 글로벌 기본값)
    #[serde(default)]
    pub thresholds: Option<crate::domain::verification::VerificationThresholds>,
}

/// 문서 유형 레지스트리 — doc_types.toml 로드 결과를 보관
#[derive(Debug, Clone)]
pub struct DocTypeRegistry {
    types: Vec<DocTypeDef>,
}

impl DocTypeRegistry {
    pub fn new(types: Vec<DocTypeDef>) -> Self {
        Self { types }
    }

    /// 빈 레지스트리 (테스트/stub 용)
    pub fn empty() -> Self {
        Self { types: Vec::new() }
    }

    /// id로 유형 조회
    pub fn get(&self, id: &str) -> Option<&DocTypeDef> {
        self.types.iter().find(|t| t.id == id)
    }

    /// 전체 유형 목록
    pub fn all(&self) -> &[DocTypeDef] {
        &self.types
    }

    /// 전체 유형 목록 (가변 참조)
    pub fn all_mut(&mut self) -> &mut Vec<DocTypeDef> {
        &mut self.types
    }

    /// 특정 유형의 필수 섹션 반환 (없으면 빈 벡터)
    pub fn sections_for(&self, id: &str) -> Vec<String> {
        self.get(id)
            .map(|t| t.sections.clone())
            .unwrap_or_default()
    }

    /// 복수 유형의 필수 섹션 합집합
    pub fn sections_for_types(&self, ids: &[String]) -> Vec<String> {
        let mut all_sections = Vec::new();
        for id in ids {
            for section in self.sections_for(id) {
                if !all_sections.contains(&section) {
                    all_sections.push(section);
                }
            }
        }
        all_sections
    }

    /// 특정 유형의 LLM 프롬프트
    pub fn prompt_for(&self, id: &str) -> Option<&str> {
        self.get(id).map(|t| t.prompt.as_str())
    }

    /// 유형별 검증 임계값 (None이면 글로벌 기본값 사용)
    pub fn thresholds_for(&self, id: &str) -> Option<&crate::domain::verification::VerificationThresholds> {
        self.get(id).and_then(|t| t.thresholds.as_ref())
    }

    /// 복수 유형의 임계값 merge (가장 엄격한 값 선택)
    pub fn thresholds_for_types(&self, ids: &[String]) -> Option<crate::domain::verification::VerificationThresholds> {
        let mut result: Option<crate::domain::verification::VerificationThresholds> = None;
        for id in ids {
            if let Some(t) = self.thresholds_for(id) {
                result = Some(match result {
                    None => t.clone(),
                    Some(existing) => crate::domain::verification::VerificationThresholds {
                        structure_min: existing.structure_min.max(t.structure_min),
                        compression_min: existing.compression_min.max(t.compression_min),
                        compression_max: existing.compression_max.min(t.compression_max),
                        keyword_coverage_min: existing.keyword_coverage_min.max(t.keyword_coverage_min),
                        keyword_completeness_min: existing.keyword_completeness_min.max(t.keyword_completeness_min),
                        rouge_l_min: existing.rouge_l_min.max(t.rouge_l_min),
                        entity_preservation_min: existing.entity_preservation_min.max(t.entity_preservation_min),
                    },
                });
            }
        }
        result
    }

    /// LLM 힌트용: (id, patterns) 목록
    pub fn hint_patterns(&self) -> Vec<(&str, &[String])> {
        self.types
            .iter()
            .map(|t| (t.id.as_str(), t.patterns.as_slice()))
            .collect()
    }
}

// ── 메타데이터 ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metadata {
    pub doc_types: Vec<String>,
    pub rationale: String,
    pub date: String,
    pub summary: String,
    pub keywords: Vec<String>,
    pub sensitive: bool,
    pub doi: Option<String>,
    #[serde(default)]
    pub related_docs: Vec<String>,
    /// 파생 문서의 원본 ID 목록 (프로베넌스)
    #[serde(default)]
    pub source_doc_ids: Vec<String>,
    /// 검색 힌트 — 사용자가 이 문서를 찾을 때 입력할 만한 질문/키워드
    #[serde(default)]
    pub search_hints: Vec<String>,
    /// LLM이 추출한 엔티티 (이름:유형 쌍)
    #[serde(default)]
    pub entities: Vec<(String, String)>,
    /// 상위 제목 계층 (H1>H2>H3 경로). Phase 61 G1 — 검색 시 문맥 파악.
    /// 청킹 시 SemanticChunk.title_path에서 복사. 청크 단위가 아닌 문서 단위 Metadata에서는
    /// 첫 청크의 path 또는 빈 Vec. 기존 직렬화 호환을 위해 #[serde(default)] 적용.
    #[serde(default)]
    pub hierarchy: Vec<String>,
    /// 콘텐츠 유형. Phase 61 G7 — text/table/code/image_caption.
    /// 기본값 "text"로 기존 인덱스 호환.
    #[serde(default = "default_content_type")]
    pub content_type: String,
    /// Phase 87 wikidocs 353407: "확인 필요" 항목 — 원천 자료에서 미확인이거나 추가 검증 필요한 주장.
    /// LLM 가공 시 비워두고, 검증·lint 단계에서 채워짐. 비어 있으면 "전부 확인됨" 의미.
    #[serde(default)]
    pub needs_verification: Vec<String>,
    /// Phase 87 wikidocs 353407: "다시 물어볼 질문" — 원천 자료로 답할 수 없는 후속 질문.
    /// LLM이 가공 시 자율 생성하거나, lint가 모호한 영역 발견 시 채움. 비어 있으면 "후속 질문 없음".
    #[serde(default)]
    pub open_questions: Vec<String>,
    /// Phase 103 G1 (GraphRAG Lexical Graph Statement 노드 흡수): 사실 단위 핵심 진술.
    /// LLM이 가공 시 핵심 statements 추출. 검색 정밀도 ↑ (문서 단위 → 사실 단위). 디폴트 빈 Vec.
    /// 트리거: 가공 50파일+ + 측정 후 needs_verification 결합 활성화 (lesson 30 인프라 선구현 패턴).
    #[serde(default)]
    pub statements: Vec<String>,
    /// Adaptive Chunking 4지표 (arxiv 2603.25333 흡수, 인프라 선구현).
    /// `chunking.compute_quality` config가 true일 때 가공 시 계산. 디폴트 None.
    #[serde(default)]
    pub chunk_quality: Option<crate::domain::chunking_quality::ChunkQualityMetrics>,
}

fn default_content_type() -> String {
    "text".to_string()
}

// ── LLM 분류+가공 통합 결과 ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyAndProcessResult {
    pub doc_types: Vec<String>,
    pub rationale: String,
    pub content: String,
    pub metadata: Metadata,
    /// 구조화된 섹션 (검증용). LLM이 JSON sections 객체로 반환하면 채워짐.
    pub sections: Option<HashMap<String, Vec<String>>>,
}

// ── 문서 ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Document {
    pub origin_path: PathBuf,
    pub compressed_origin: Option<PathBuf>,
    pub processed_path: Option<PathBuf>,
    pub metadata: Option<Metadata>,
    pub file_hash: String,
    pub embedding: Vec<f32>,
}

// ── 중복 처리 ────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuplicateAction {
    Skip,
    Replace,
    Merge,
    Keep,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuplicateKind {
    Exact,
    Semantic,
    TypeSpecific,
}

// ── 검색 결과 ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimilarDoc {
    pub id: String,
    pub path: PathBuf,
    pub score: f32,
    pub doc_types: Vec<String>,
    pub date: String,
    /// 상위 제목 계층 (H1>H2>H3). Phase 61 G1 — 검색 결과 UI에서 breadcrumb로 표시.
    /// 기존 인덱스 호환을 위해 #[serde(default)] 적용.
    #[serde(default)]
    pub hierarchy: Vec<String>,
}

/// VectorDB에 저장된 문서 요약 (재색인/lint용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDocSummary {
    pub id: String,
    pub path: PathBuf,
    pub doc_types: Vec<String>,
    #[serde(default)]
    pub date: String,
}

// ── DB 통계 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbStats {
    pub total_documents: u64,
    pub by_type: Vec<(String, u64)>,
    pub total_size_bytes: u64,
    pub sensitive_count: u64,
}

// ── 처리 요약 (알림용) ────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize)]
pub struct ProcessingSummary {
    pub success: u32,
    pub processing: u32,
    pub errors: u32,
    pub skipped: u32,
    pub sensitive: u32,
    pub duplicates: u32,
    pub quarantined: u32,
    /// 에러/경고 상세 (파일명 + 사유 + 대안)
    pub issues: Vec<ProcessingIssue>,
    /// 처리된 문서 유형별 카운트
    pub by_type: HashMap<String, u32>,
    /// 검증 메트릭 기록
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub verification_metrics: Vec<VerificationMetricEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessingIssue {
    pub filename: String,
    pub level: String,       // "error" / "warning"
    pub reason: String,
    pub action_taken: String, // "quarantine 이동", "2-Pass 재가공 후 성공", "스킵" 등
}

/// 검증 결과 기록 (Dashboard 메트릭용)
#[derive(Debug, Clone, Serialize)]
pub struct VerificationMetricEntry {
    pub doc_id: String,
    pub timestamp: String,
    pub structure: f64,
    pub compression: f64,
    pub keyword_coverage: f64,
    pub keyword_completeness: f64,
    pub rouge_l: f64,
    pub entity: f64,
    pub overall: String,
}

impl ProcessingSummary {
    pub fn record_success(&mut self, doc_types: &[String]) {
        self.success += 1;
        for t in doc_types {
            *self.by_type.entry(t.clone()).or_default() += 1;
        }
    }

    pub fn record_error(&mut self, filename: &str, reason: &str, action: &str) {
        self.errors += 1;
        self.issues.push(ProcessingIssue {
            filename: filename.to_string(),
            level: "error".to_string(),
            reason: reason.to_string(),
            action_taken: action.to_string(),
        });
    }

    pub fn record_warning(&mut self, filename: &str, reason: &str, action: &str) {
        self.issues.push(ProcessingIssue {
            filename: filename.to_string(),
            level: "warning".to_string(),
            reason: reason.to_string(),
            action_taken: action.to_string(),
        });
    }

    pub fn is_empty(&self) -> bool {
        self.success == 0 && self.errors == 0 && self.skipped == 0 && self.sensitive == 0
    }
}

// ── 검증 ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationLevel {
    Pass,
    Warning(String),
    Fail(String),
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub structure_completeness: f64,
    pub compression_ratio: f64,
    pub keyword_coverage: f64,
    pub keyword_completeness: f64,
    pub rouge_l_recall: f64,
    pub entity_preservation: f64,
    pub overall: VerificationLevel,
    pub details: Vec<String>,
}

// ── Sparse 임베딩 (BM25/BGE-M3 lexical, Phase 89 #10 인프라) ─

/// Sparse 임베딩 벡터 — 활성 토큰 인덱스 + 가중치.
///
/// BGE-M3 sparse 또는 BM25 가중치를 도메인-중립 형식으로 보존.
/// 어댑터(fastembed_sparse 등)는 자체 타입에서 본 형식으로 변환.
/// LocalVectorStore의 `sparse_index` (HashMap<doc_id, SparseEmbedding>)에 영속화.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SparseEmbedding {
    /// 활성 토큰 인덱스 (vocab 위치, 정렬 권장).
    pub indices: Vec<u32>,
    /// 각 인덱스의 가중치.
    pub values: Vec<f32>,
}

impl SparseEmbedding {
    pub fn is_empty(&self) -> bool { self.indices.is_empty() }
    pub fn len(&self) -> usize { self.indices.len() }

    /// Dot product — sorted indices 가정.
    pub fn dot(&self, other: &SparseEmbedding) -> f32 {
        let (mut i, mut j) = (0, 0);
        let mut score = 0.0_f32;
        while i < self.indices.len() && j < other.indices.len() {
            match self.indices[i].cmp(&other.indices[j]) {
                std::cmp::Ordering::Equal => {
                    score += self.values[i] * other.values[j];
                    i += 1; j += 1;
                }
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
            }
        }
        score
    }
}

// ── 임베딩 스냅샷 (zero-copy 행렬 곱용) ─────────────────────

/// 전체 임베딩의 읽기 전용 스냅샷 — flush_crossref에서 zero-copy 접근
pub struct EmbeddingSnapshot {
    /// contiguous float32 데이터 (doc0_dim0..doc0_dimN, doc1_dim0..doc1_dimN, ...)
    pub data: Vec<f32>,
    /// 문서 ID 배열 (data 내 순서와 일치)
    pub ids: Vec<String>,
    /// 벡터 차원
    pub dim: usize,
}

impl EmbeddingSnapshot {
    /// 특정 문서의 임베딩 슬라이스
    pub fn get(&self, idx: usize) -> &[f32] {
        &self.data[idx * self.dim..(idx + 1) * self.dim]
    }

    /// 문서 수
    pub fn len(&self) -> usize { self.ids.len() }

    pub fn is_empty(&self) -> bool { self.ids.is_empty() }
}

// ── 관계 그래프 (Phase B) ────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocRelation {
    pub source_id: String,
    pub target_id: String,
    pub relation_type: RelationType,
    /// 관계 신뢰도 (0.0~1.0)
    #[serde(default)]
    pub confidence: f32,
    /// 관계 컨텍스트 (왜 이 관계가 생겼는지)
    #[serde(default)]
    pub context: String,
    /// 관계 생성 시점
    #[serde(default)]
    pub created_at: String,
    /// Phase 83: 관계 origin — 사용자 명시 / 자동 유사도 / LLM 추론 / lint 자동수정 등
    #[serde(default)]
    pub origin: RelationOrigin,
}

/// Phase 83: 관계가 어떻게 생성되었는지
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum RelationOrigin {
    /// 자동 임베딩 유사도 (현재 crossref 기본)
    #[default]
    AutoSimilarity,
    /// 사용자가 마크다운에 [[xxx]] 위키링크로 명시
    UserWikilink,
    /// LLM이 가공 시 references 필드로 명시 추출
    LlmExtracted,
    /// 사용자가 UI에서 수동 추가
    UserManual,
    /// Lint 자동 수정 제안 적용
    LintAutoFix,
}


impl RelationOrigin {
    pub fn label_ko(&self) -> &'static str {
        match self {
            RelationOrigin::AutoSimilarity => "자동_유사도",
            RelationOrigin::UserWikilink => "사용자_위키링크",
            RelationOrigin::LlmExtracted => "LLM_추출",
            RelationOrigin::UserManual => "사용자_수동",
            RelationOrigin::LintAutoFix => "Lint_자동수정",
        }
    }
    /// 신뢰도 가중치 — 명시적 origin이 높음
    pub fn weight(&self) -> f32 {
        match self {
            RelationOrigin::UserManual => 1.0,
            RelationOrigin::UserWikilink => 0.95,
            RelationOrigin::LlmExtracted => 0.85,
            RelationOrigin::LintAutoFix => 0.7,
            RelationOrigin::AutoSimilarity => 0.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum RelationType {
    References,
    ReferencedBy,
    Updates,
    #[default]
    RelatedTopic,
    Supersedes,
    /// Phase 103 G2 (GraphRAG 의미 관계 흡수): LLM 자유 추출 의미 관계.
    /// 예: ("uses", "depends_on", "implements"). 디폴트 미사용 (lesson 30 인프라 선구현).
    /// 트리거: KG 관계 평균 <2 + 도메인 다양성 확보 + LLM 프롬프트 semantic_relations 활성화 시.
    Semantic(String),
}


impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationType::References => write!(f, "references"),
            RelationType::ReferencedBy => write!(f, "referenced_by"),
            RelationType::Updates => write!(f, "updates"),
            RelationType::RelatedTopic => write!(f, "related_topic"),
            RelationType::Supersedes => write!(f, "supersedes"),
            RelationType::Semantic(verb) => write!(f, "semantic:{}", verb),
        }
    }
}

// ── 엔티티 (KG 노드) ─────────────────────────────────────────

/// 추출된 엔티티 (사람, 조직, 장소, 개념 등)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// 엔티티 고유 ID (정규화된 이름의 해시)
    pub id: String,
    /// 표시 이름
    pub name: String,
    /// 엔티티 유형
    pub entity_type: EntityType,
    /// 출현 문서 목록 (file_hash)
    pub doc_ids: Vec<String>,
    /// 출현 횟수
    pub mention_count: u32,
    /// 첫 출현 날짜
    pub first_seen: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Person,
    Organization,
    Place,
    Date,
    Amount,
    Concept,
    Technology,
    Project,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Person => write!(f, "person"),
            EntityType::Organization => write!(f, "organization"),
            EntityType::Place => write!(f, "place"),
            EntityType::Date => write!(f, "date"),
            EntityType::Amount => write!(f, "amount"),
            EntityType::Concept => write!(f, "concept"),
            EntityType::Technology => write!(f, "technology"),
            EntityType::Project => write!(f, "project"),
        }
    }
}

// ── 누적 업데이트 (Phase C) ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecord {
    pub date: String,
    pub source_file: String,
    pub change_summary: String,
}

#[derive(Debug, Clone, Default)]
pub struct EnrichResult {
    pub updated_content: String,
    pub change_summary: String,
    pub should_update: bool,
}

// ── Lint 결과 (Phase E) ─────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct LintReport {
    pub orphan_docs: Vec<String>,
    pub stale_docs: Vec<String>,
    pub issues: Vec<LintIssue>,
}

#[derive(Debug, Clone)]
pub struct LintIssue {
    pub doc_id: String,
    pub issue_type: LintIssueType,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintIssueType {
    Orphan,
    Stale,
    MissingBacklink,
    DuplicateTopic,
    Contradiction,
    /// Phase 88 (wikidocs 353407 근거 점검): 단정 표현 검출 — 약화 권고 후보
    StrongClaim,
}

// ── 전처리 결과 ──

#[derive(Debug, Clone, Default)]
pub struct PreprocessResult {
    pub text: String,
    pub images: Vec<PathBuf>,
    pub tables: Vec<String>,
}

// ── 재색인 보고서 ────────────────────────────────────���───────

#[derive(Debug, Clone, Default)]
pub struct ReindexReport {
    pub total_scanned: u64,
    pub types_changed: u64,
    pub errors: Vec<String>,
}

// ── 파이프라인 정의 ─────────────────────────────────────────

/// 파일 유형별 커스텀 처리 파이프라인
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineDefinition {
    /// 고정 파이프라인 스텝 (Preprocess → LLM → Verify)
    #[serde(default)]
    pub steps: Vec<PipelineStep>,
    /// 후처리 LLM 크레덴셜 (Todo 병합, 교차참조, 토픽 병합에 사용). 미지정 시 기본 LLM.
    #[serde(default)]
    pub postprocess_credential: Option<String>,
}

fn default_true() -> bool { true }

/// 파이프라인 스텝
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PipelineStep {
    #[serde(rename = "preprocess")]
    Preprocess {
        #[serde(default = "default_none")]
        pdf_tool: String,
        #[serde(default = "default_none")]
        ocr_tool: String,
    },
    #[serde(rename = "llm")]
    Llm {
        #[serde(default)]
        credential: Option<String>,
    },
    #[serde(rename = "verify")]
    Verify {
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default)]
        thresholds: Option<crate::domain::verification::VerificationThresholds>,
        /// 검증 LLM 크레덴셜 (2-Pass 재가공 시 사용). 미지정 시 기본 LLM.
        #[serde(default)]
        credential: Option<String>,
    },
    #[serde(rename = "embedding")]
    Embedding {
        #[serde(default)]
        model: Option<String>,
        /// 임베딩 크레덴셜. 미지정 시 기본 임베딩.
        #[serde(default)]
        credential: Option<String>,
    },
    #[serde(rename = "storage")]
    Storage {
        #[serde(default = "default_zstd_level")]
        zstd_level: i32,
    },
}

fn default_none() -> String { "none".into() }
fn default_zstd_level() -> i32 { 3 }

/// 토큰 사용 기록
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub call_count: u64,
    pub by_role: HashMap<String, TokenRoleUsage>,
    pub by_credential: HashMap<String, TokenRoleUsage>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenRoleUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub call_count: u64,
}

impl TokenUsage {
    pub fn record(&mut self, role: &str, credential: &str, input: u64, output: u64) {
        self.input_tokens += input;
        self.output_tokens += output;
        self.call_count += 1;
        let role_entry = self.by_role.entry(role.into()).or_default();
        role_entry.input_tokens += input;
        role_entry.output_tokens += output;
        role_entry.call_count += 1;
        if !credential.is_empty() {
            let cred_entry = self.by_credential.entry(credential.into()).or_default();
            cred_entry.input_tokens += input;
            cred_entry.output_tokens += output;
            cred_entry.call_count += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Metadata serde 라운드트립 ──

    #[test]
    fn test_metadata_serde_roundtrip() {
        let meta = Metadata {
            doc_types: vec!["meeting".into(), "report".into()],
            rationale: "회의록+보고서".into(),
            date: "2026-04-14".into(),
            summary: "테스트 요약".into(),
            keywords: vec!["k8s".into(), "배포".into()],
            sensitive: false,
            doi: Some("10.1234/test".into()),
            related_docs: vec!["doc_001".into()],
            source_doc_ids: vec!["src_001".into()],
            search_hints: vec!["k8s 배포 방법".into()],
            entities: vec![],
            hierarchy: vec!["Phase 61 검증".into(), "메타데이터 라운드트립".into()],
            content_type: "text".into(),
            needs_verification: vec!["배포 절차 최신성".into()],
            open_questions: vec!["롤백 시 데이터 보존 정책?".into()],
            statements: vec![],
            chunk_quality: None,
        };
        let json = serde_json::to_string(&meta).expect("serialize");
        let restored: Metadata = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.doc_types, meta.doc_types);
        assert_eq!(restored.date, meta.date);
        assert_eq!(restored.doi, meta.doi);
        assert_eq!(restored.related_docs, meta.related_docs);
        assert_eq!(restored.source_doc_ids, meta.source_doc_ids);
        assert_eq!(restored.hierarchy, meta.hierarchy);
        assert_eq!(restored.content_type, "text");
    }

    #[test]
    fn test_metadata_serde_backward_compat() {
        // Phase 61 이전 인덱스가 hierarchy/content_type 없이 직렬화돼 있어도 deserialize 성공해야 함
        let legacy_json = r#"{"doc_types":["meeting"],"rationale":"r","date":"d","summary":"s","keywords":[],"sensitive":false,"doi":null}"#;
        let restored: Metadata = serde_json::from_str(legacy_json).expect("legacy deserialize");
        assert_eq!(restored.doc_types, vec!["meeting"]);
        assert_eq!(restored.hierarchy, Vec::<String>::new());
        assert_eq!(restored.content_type, "text"); // default 적용
    }

    #[test]
    fn test_metadata_default_optional_fields() {
        let json = r#"{"doc_types":["etc"],"rationale":"r","date":"2026-01-01","summary":"s","keywords":[],"sensitive":false}"#;
        let meta: Metadata = serde_json::from_str(json).expect("deserialize");
        assert_eq!(meta.doi, None);
        assert!(meta.related_docs.is_empty());
        assert!(meta.source_doc_ids.is_empty());
    }

    // ── ProcessingSummary ──

    #[test]
    fn test_summary_record_success() {
        let mut summary = ProcessingSummary::default();
        assert!(summary.is_empty());
        summary.record_success(&["meeting".into(), "report".into()]);
        assert_eq!(summary.success, 1);
        assert_eq!(summary.by_type.get("meeting"), Some(&1));
        assert_eq!(summary.by_type.get("report"), Some(&1));
        assert!(!summary.is_empty());
    }

    #[test]
    fn test_summary_record_error() {
        let mut summary = ProcessingSummary::default();
        summary.record_error("test.txt", "검증 실패", "quarantine");
        assert_eq!(summary.errors, 1);
        assert_eq!(summary.issues.len(), 1);
        assert_eq!(summary.issues[0].level, "error");
        assert_eq!(summary.issues[0].filename, "test.txt");
    }

    #[test]
    fn test_summary_record_warning() {
        let mut summary = ProcessingSummary::default();
        summary.record_warning("memo.txt", "압축률 낮음", "2-Pass 재가공");
        assert_eq!(summary.issues.len(), 1);
        assert_eq!(summary.issues[0].level, "warning");
        assert_eq!(summary.issues[0].action_taken, "2-Pass 재가공");
    }

    // ── PipelineStep serde ──

    #[test]
    fn test_pipeline_step_llm_serde() {
        let step = PipelineStep::Llm { credential: Some("cred_001".into()) };
        let json = serde_json::to_string(&step).expect("serialize");
        assert!(json.contains("\"type\":\"llm\""));
        let restored: PipelineStep = serde_json::from_str(&json).expect("deserialize");
        match restored {
            PipelineStep::Llm { credential } => assert_eq!(credential, Some("cred_001".into())),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_pipeline_step_storage_default() {
        let json = r#"{"type":"storage"}"#;
        let step: PipelineStep = serde_json::from_str(json).expect("deserialize");
        match step {
            PipelineStep::Storage { zstd_level } => assert_eq!(zstd_level, 3),
            _ => panic!("wrong variant"),
        }
    }

    // ── PipelineDefinition ──

    #[test]
    fn test_pipeline_definition_defaults() {
        let json = r#"{"steps":[]}"#;
        let pd: PipelineDefinition = serde_json::from_str(json).expect("deserialize");
        assert!(pd.steps.is_empty());
        assert!(pd.postprocess_credential.is_none());
    }

    // ── DocTypeRegistry thresholds merge ──

    #[test]
    fn test_registry_thresholds_merge_strictest() {
        use crate::domain::verification::VerificationThresholds;
        let types = vec![
            DocTypeDef {
                id: "meeting".into(), label_ko: "회의록".into(),
                patterns: vec![], sections: vec![], prompt: String::new(),
                dedup_key: None, sensitive: false,
                thresholds: Some(VerificationThresholds {
                    structure_min: 0.5, compression_min: 0.05, compression_max: 1.5,
                    keyword_coverage_min: 0.6, keyword_completeness_min: 0.3,
                    rouge_l_min: 0.1, entity_preservation_min: 0.5,
                }),
            },
            DocTypeDef {
                id: "report".into(), label_ko: "보고서".into(),
                patterns: vec![], sections: vec![], prompt: String::new(),
                dedup_key: None, sensitive: false,
                thresholds: Some(VerificationThresholds {
                    structure_min: 0.7, compression_min: 0.1, compression_max: 1.2,
                    keyword_coverage_min: 0.5, keyword_completeness_min: 0.4,
                    rouge_l_min: 0.15, entity_preservation_min: 0.6,
                }),
            },
        ];
        let registry = DocTypeRegistry::new(types);
        let merged = registry.thresholds_for_types(&["meeting".into(), "report".into()])
            .expect("should have merged thresholds");
        // merge는 가장 엄격한 값 선택
        assert!((merged.structure_min - 0.7).abs() < 0.001); // max(0.5, 0.7)
        assert!((merged.compression_max - 1.2).abs() < 0.001); // min(1.5, 1.2)
        assert!((merged.keyword_completeness_min - 0.4).abs() < 0.001); // max(0.3, 0.4)
    }

    // ── VerificationLevel ──

    #[test]
    fn test_verification_level_equality() {
        assert_eq!(VerificationLevel::Pass, VerificationLevel::Pass);
        assert_ne!(VerificationLevel::Pass, VerificationLevel::Fail("x".into()));
        assert_ne!(
            VerificationLevel::Warning("a".into()),
            VerificationLevel::Warning("b".into())
        );
    }
}
