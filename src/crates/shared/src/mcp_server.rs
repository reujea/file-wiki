//! MCP 서버 — Claude Code에서 직접 파이프라인 지식기반을 검색

use std::sync::Arc;

use anyhow::Result;
use tracing::info;
use file_pipeline_core::ports::output::{EmbeddingPort, LLMPort, RerankerPort, StoragePort, VectorDBPort};
use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};

/// 검색 로그 엔트리
#[derive(Debug, Clone)]
pub struct SearchLogEntry {
    pub query: String,
    pub results_count: usize,
    pub result_ids: Vec<String>,
    pub latency_ms: u64,
    pub timestamp: String,
    pub mode: String,
}

/// MCP 서버 공유 상태
pub struct McpState {
    pub vector_db: Arc<dyn VectorDBPort>,
    pub storage: Arc<dyn StoragePort>,
    pub embedding: Arc<dyn EmbeddingPort>,
    pub llm: Arc<dyn LLMPort>,
    pub reranker: Arc<dyn RerankerPort>,
    /// settings.db 경로
    pub settings_db_path: std::path::PathBuf,
    /// 검색 캐시 (key -> (결과, 생성시간)). TTL 5분.
    pub search_cache: std::sync::Mutex<std::collections::HashMap<String, (Vec<file_pipeline_core::domain::models::SimilarDoc>, std::time::Instant)>>,
    /// 검색 로그
    pub search_log: std::sync::Mutex<Vec<SearchLogEntry>>,
    /// Phase 80-A: 검색 mode 메모리 카운터 (DB 영속화 병행)
    pub search_mode_counts: std::sync::Mutex<std::collections::HashMap<String, u64>>,
    /// Phase 80-B: CRAG 신뢰도 메모리 카운터
    pub crag_counts: std::sync::Mutex<std::collections::HashMap<String, u64>>,
    /// Ruflo A2: KG 1-hop 확장 개수 (0=비활성). SearchConfig.expand_kg_hops에서 주입.
    pub expand_kg_hops: usize,
    /// Ruflo B1: 동일 doc_type 결과 임계값 (0=비활성). 초과 시 다른 type 강제 노출.
    pub diversity_threshold: usize,
    /// 트리거 #6: HyDE 폴백 검색 활성. true + 첫 패스 빈약 결과(< hyde_min_results) 시 LLM 가상 답변 임베딩으로 재검색.
    /// SearchConfig.hyde_enabled에서 주입. 디폴트 false (인프라만, 트리거 #6 도달 시 활성).
    pub hyde_enabled: bool,
    /// HyDE 폴백 발동 임계 — 첫 패스 결과가 이 개수 미만이면 폴백 시도. 디폴트 3.
    pub hyde_min_results: usize,
    /// Phase 91 A2: 출력 PII mask 활성. SearchConfig.output_pii_mask에서 주입.
    pub output_pii_mask: bool,
    /// Phase 91 A2: 사용자 정의 PII 패턴 (settings.db pii_patterns_user에서 주입).
    /// MCP 시작 시 1회 로드 (live reload는 service.rs 측 가공 경로에 한정).
    pub pii_user_patterns: Vec<(String, String)>,
    /// Phase 94 A3: audit_trace 기록 (헥사고날 AuditPort). 디폴트 NullAuditAdapter.
    pub audit: std::sync::Arc<dyn file_pipeline_core::ports::output::AuditPort>,
    /// Phase 103 G4: TF-IDF 다양성 재순위 활성 (SearchConfig.tfidf_rerank_enabled 주입).
    pub tfidf_rerank_enabled: bool,
    /// Phase 103 G3: KG Multi-hop 빔 검색 활성 (SearchConfig.kg_beam_search 주입).
    pub kg_beam_search: bool,
}

impl McpState {
    /// Phase 80-A: settings.db에서 카운터 복원 (서버 시작 시 호출)
    pub fn restore_counters(&self) {
        if let Ok(db) = crate::settings_db::SettingsDb::open(&self.settings_db_path) {
            if let Ok(rows) = db.get_search_mode_counters() {
                let mut map = self.search_mode_counts.lock().expect("search_mode lock");
                for (mode, count, _) in rows { map.insert(mode, count); }
            }
            if let Ok(rows) = db.get_crag_counters() {
                let mut map = self.crag_counts.lock().expect("crag lock");
                for (bucket, count, _) in rows { map.insert(bucket, count); }
            }
        }
    }

    fn record_search_mode(&self, mode: &str) {
        // 메모리 즉시 증가
        if let Ok(mut m) = self.search_mode_counts.lock() {
            *m.entry(mode.to_string()).or_insert(0) += 1;
        }
        // DB 영속화 (실패해도 메모리는 유지)
        if let Ok(db) = crate::settings_db::SettingsDb::open(&self.settings_db_path) {
            let _ = db.increment_search_mode(mode);
        }
    }

    fn record_crag(&self, bucket: &str) {
        if let Ok(mut m) = self.crag_counts.lock() {
            *m.entry(bucket.to_string()).or_insert(0) += 1;
        }
        if let Ok(db) = crate::settings_db::SettingsDb::open(&self.settings_db_path) {
            let _ = db.increment_crag(bucket);
        }
    }
}

/// Phase 92 H3: MCP 도구 메타데이터 (다차원 분류).
///
/// Phase 91 B2의 단일 차원(`mutates_state`)을 Mirage Command 3차원 등록 패턴으로 확장:
/// - `mutates`: 상태 변경 여부 (Phase 91 B2)
/// - `category`: 도구 카테고리 (검색·KG·설정·할일·신호·스냅샷)
/// - `cost`: 호출 비용 (free·llm-call·external-api·heavy-compute)
///
/// JAMES (RBAC 게이트는 보류) + Mirage (3차원 등록) 패턴 흡수. 호출 게이트 도입 없음 —
/// GUI/CLI 표시 한정. 메타 룰 1 자기 적용 (다중 분류 단일 카탈로그).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpToolCategory {
    Search,       // search / get_document / list_documents
    Kg,           // kg_neighbors / kg_paths / kg_stats
    Settings,     // setup_*
    Todo,         // list_todos / complete_todo / revise_topic
    Signal,       // get_processing_metrics / get_search_mode_stats / get_crag_stats / get_chunk_stats
    Snapshot,     // setup_snapshot_*
    Lint,         // lint / setup_dryrun
}

impl McpToolCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Kg => "kg",
            Self::Settings => "settings",
            Self::Todo => "todo",
            Self::Signal => "signal",
            Self::Snapshot => "snapshot",
            Self::Lint => "lint",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpToolCost {
    Free,           // 메모리/DB 조회만
    LlmCall,        // LLM 호출 동반 (revise_topic 등)
    HeavyCompute,   // 벡터 검색 + 리랭킹 등 무거운 연산
}

impl McpToolCost {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::LlmCall => "llm-call",
            Self::HeavyCompute => "heavy-compute",
        }
    }
}

/// Phase 92 H3: MCP 도구의 다차원 메타데이터.
///
/// Mirage Command 3차원 등록 패턴 흡수 — 전역/리소스별/파일타입별 → mutates/category/cost.
#[derive(Debug, Clone)]
pub struct McpToolMetadata {
    pub name: &'static str,
    pub mutates: bool,
    pub category: McpToolCategory,
    pub cost: McpToolCost,
}

/// Phase 91 B2 (Phase 92 H3 확장): MCP 도구의 상태 변경 여부 — 호환성 유지 wrapper.
pub fn mcp_tool_mutates_state(name: &str) -> bool {
    mcp_tool_catalog_full().iter().any(|m| m.name == name && m.mutates)
}

/// Phase 91 B2 (Phase 92 H3 확장): 단일 차원 분류 카탈로그 — 호환성 유지 wrapper.
pub fn mcp_tool_catalog() -> Vec<(&'static str, bool)> {
    mcp_tool_catalog_full().into_iter().map(|m| (m.name, m.mutates)).collect()
}

/// Phase 92 H3: 다차원 분류 카탈로그 (Mirage 패턴).
///
/// `list_tools` 결과와 동기화 의무 — 신규 도구 추가 시 본 함수도 갱신 (메타 룰 1).
pub fn mcp_tool_catalog_full() -> Vec<McpToolMetadata> {
    use McpToolCategory::*;
    use McpToolCost::*;
    vec![
        // 검색·조회 (read-only)
        McpToolMetadata { name: "search", mutates: false, category: Search, cost: HeavyCompute },
        McpToolMetadata { name: "get_document", mutates: false, category: Search, cost: Free },
        McpToolMetadata { name: "list_documents", mutates: false, category: Search, cost: Free },
        McpToolMetadata { name: "stats", mutates: false, category: Signal, cost: Free },
        McpToolMetadata { name: "lint", mutates: false, category: Lint, cost: Free },
        // KG
        McpToolMetadata { name: "kg_neighbors", mutates: false, category: Kg, cost: Free },
        McpToolMetadata { name: "kg_paths", mutates: false, category: Kg, cost: Free },
        McpToolMetadata { name: "kg_stats", mutates: false, category: Kg, cost: Free },
        // 코퍼스 신호 (read-only)
        McpToolMetadata { name: "get_processing_metrics", mutates: false, category: Signal, cost: Free },
        McpToolMetadata { name: "get_search_mode_stats", mutates: false, category: Signal, cost: Free },
        McpToolMetadata { name: "get_crag_stats", mutates: false, category: Signal, cost: Free },
        McpToolMetadata { name: "get_chunk_stats", mutates: false, category: Signal, cost: Free },
        McpToolMetadata { name: "list_todos", mutates: false, category: Todo, cost: Free },
        // 설정 추천 (preview only)
        McpToolMetadata { name: "optimize", mutates: false, category: Settings, cost: Free },
        McpToolMetadata { name: "setup_review", mutates: false, category: Settings, cost: Free },
        McpToolMetadata { name: "setup_dryrun", mutates: false, category: Lint, cost: Free },
        McpToolMetadata { name: "setup_profile_infer", mutates: false, category: Settings, cost: Free },
        McpToolMetadata { name: "setup_modules_list", mutates: false, category: Settings, cost: Free },
        McpToolMetadata { name: "setup_snapshot_list", mutates: false, category: Snapshot, cost: Free },
        McpToolMetadata { name: "setup_decision_log_list", mutates: false, category: Snapshot, cost: Free },
        // 쓰기 (mutating)
        McpToolMetadata { name: "complete_todo", mutates: true, category: Todo, cost: Free },
        McpToolMetadata { name: "revise_topic", mutates: true, category: Todo, cost: LlmCall },
        McpToolMetadata { name: "setup_apply", mutates: true, category: Settings, cost: Free },
        McpToolMetadata { name: "setup_apply_modules", mutates: true, category: Settings, cost: Free },
        McpToolMetadata { name: "setup_snapshot_rollback", mutates: true, category: Snapshot, cost: Free },
        McpToolMetadata { name: "setup_snapshot_measure", mutates: true, category: Snapshot, cost: Free },
        // Phase E (Grimoire 흡수, prd/research/external-analysis-2026-06-04-grimoire.md)
        McpToolMetadata { name: "get_index", mutates: false, category: Search, cost: Free },
        McpToolMetadata { name: "get_context", mutates: false, category: Search, cost: Free },
        McpToolMetadata { name: "write_note", mutates: true, category: Search, cost: Free },
    ]
}

fn make_tool(name: &'static str, description: &'static str, schema_json: serde_json::Value) -> Tool {
    Tool {
        name: name.into(),
        title: None,
        description: Some(description.into()),
        input_schema: Arc::new(
            serde_json::from_value(schema_json).unwrap_or_default()
        ),
        output_schema: None,
        annotations: None,
        execution: None,
        icons: None,
        meta: None,
    }
}

impl ServerHandler for McpState {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "file-pipeline".into(),
                title: Some("File Processing Pipeline".into()),
                version: "0.1.0".into(),
                description: Some("로컬 파일 지식기반 검색 서버".into()),
                icons: None,
                website_url: None,
            },
            instructions: Some(
"file-pipeline 지식기반 + 패턴 기반 설정 추천 서버.

## 핵심 원칙 — 사용자 입력 vs 코퍼스 신호

사용자가 자기 문서 비율을 정확히 모르므로, 시스템은 다음 원칙을 따른다:

- **첫 사용**: 일반 설정(default)으로 시작. 비율 추론 묻지 말 것.
- **50파일 이상 처리 후**: 코퍼스 신호 + 사용자가 원하는 동작을 종합해 추천.
- **사용자가 도메인을 답하지 않게**: 'doc_type 비율?'이 아니라 '어떤 동작을 우선?'으로 묻는다.

## '설정 추천', '튜닝', '내 사용 패턴 분석' 요청 시 흐름

### 1. 코퍼스 신호 수집 (필요한 것만 호출)

- get_processing_metrics — 총 문서 수, doc_type 분포, 민감 격리 비율
- get_search_mode_stats — 사용자가 자주 쓰는 검색 mode 분포
- get_crag_stats — 검색 신뢰도 누적 (correct/ambiguous/incorrect)
- get_chunk_stats — 평균 청크 크기, 코드펜스/헤딩 비율 (샘플링)
- kg_stats — 관계 풍부도, 고립 노드
- lint — 정합성 경고

총 문서가 50건 미만이면 신호 부족 — '50파일 이상 처리 후 다시 시도' 안내.

### 2. 신호 → 동작 모듈 추천 매핑

다음 휴리스틱으로 모듈을 추천:

- **secure_strict (민감 강화)**: sensitive_count/total >= 0.05 또는 코드 doc_type 비중 높음
- **chunk_small (작은 청크)**: 코드 doc_type 우세 또는 평균 청크가 target_bytes에 자주 미달
- **chunk_large (큰 청크)**: 회의록/연구 doc_type 우세 — 단 chunk_small과 배타이므로 우세 도메인으로 택일
- **search_precision (정밀)**: crag_correct_ratio < 0.7 (리랭킹 미활용 가능성) 또는 mode가 default 80%+
- **search_exploration**: mode 'related' 비중 높음
- **search_recent**: mode 'recent' 비중 높음
- **rich_relations**: kg_stats의 평균 관계 수 < 2 (관계 부족) 또는 사용자 명시
- **verify_strict**: research/legal 우세
- **long_retention**: research/legal 우세
- **high_throughput**: total_documents > 500 또는 weekly_avg > 50
- **auto_lint**: lint_warnings > 5

### 3. 사용자에게 제시

추천 모듈을 다음 형식으로 보여준다:

```
관찰된 패턴 (300건 처리)
- doc_type: code 40% / meeting 30% / general 30%
- 민감 격리: 8% (medium 수준)
- 검색 mode: default 75% / recent 15% / related 10%
- CRAG correct: 62% (개선 여지)
- 평균 관계 수: 1.8/문서

추천 동작 모듈
✅ 정밀 검색 — CRAG correct 62%, 리랭킹 활성 권장
✅ 작은 청크 (코드) — 코드 비중 40%
✅ 풍부한 관계 그래프 — 평균 1.8건 (낮음)
⚪ 민감 강화 — 8% (medium, 사용자 판단)
```

### 4. setup_apply_modules 호출

사용자 승인 시 module_ids 배열로 호출. 충돌은 자동 보수적 해소.
- dryrun=true로 먼저 미리보기 가능
- critical risk 변경(예: retention.enabled)은 apply_critical=true 필요
- 적용 후 자동으로 ConfigSnapshot 생성

### 5. 사후 측정

50파일 더 처리 후 setup_snapshot_measure로 효과 측정. before/after 비교 → 자동 롤백 권고.

## 도구 카테고리

- 검색·문서: search / get_document / list_documents / stats
- 지식 그래프: kg_neighbors / kg_paths / kg_stats
- 할일·토픽: list_todos / complete_todo / revise_topic / lint
- 코퍼스 신호: get_processing_metrics / get_search_mode_stats / get_crag_stats / get_chunk_stats
- 동작 모듈: setup_modules_list / setup_apply_modules
- 스냅샷: setup_snapshot_list / setup_snapshot_rollback / setup_snapshot_measure
- 결정 이력: setup_decision_log_list (Phase 82, snapshot_id 또는 limit)
- (legacy) setup_review / setup_apply / setup_dryrun / setup_profile_infer — 5축 입력 호환용. 신규는 모듈 흐름 권장.
".into()),
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<ListToolsResult, rmcp::ErrorData>> + Send + '_ {
        let tools = vec![
            make_tool("search", "지식기반에서 문서를 검색합니다. 벡터+키워드+날짜+유형 하이브리드.", serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "검색 쿼리 (벡터 유사도)" },
                    "keyword": { "type": "string", "description": "키워드 필터 (선택)" },
                    "doc_type": { "type": "string", "description": "문서 유형 필터: meeting, study, log 등 (선택)" },
                    "date_from": { "type": "string", "description": "시작 날짜 YYYY-MM-DD (선택)" },
                    "date_to": { "type": "string", "description": "종료 날짜 YYYY-MM-DD (선택)" },
                    "top_k": { "type": "integer", "description": "반환 수", "default": 5 },
                    "mode": { "type": "string", "description": "검색 모드: default(최적 자동), exact(정확 키워드), related(주제 탐색), recent(최근 우선), fusion(다중 쿼리 융합)", "default": "default" }
                },
                "required": ["query"]
            })),
            make_tool("get_document", "특정 문서의 가공본을 조회합니다.", serde_json::json!({
                "type": "object",
                "properties": { "doc_id": { "type": "string" } },
                "required": ["doc_id"]
            })),
            make_tool("list_documents", "전체 문서 목록을 반환합니다.", serde_json::json!({
                "type": "object",
                "properties": { "doc_type": { "type": "string" } }
            })),
            make_tool("stats", "지식기반 통계를 반환합니다.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("lint", "지식기반 품질 검사를 실행합니다.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("revise_topic", "토픽 페이지를 피드백으로 수정합니다.", serde_json::json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "토픽 파일 경로" },
                    "feedback": { "type": "string", "description": "수정 피드백" }
                },
                "required": ["file", "feedback"]
            })),
            make_tool("kg_neighbors", "특정 문서의 관계 그래프 이웃을 조회합니다.", serde_json::json!({
                "type": "object",
                "properties": {
                    "doc_id": { "type": "string", "description": "문서 ID (file_hash)" }
                },
                "required": ["doc_id"]
            })),
            make_tool("kg_paths", "두 문서 간 관계 경로를 탐색합니다 (최대 2-hop).", serde_json::json!({
                "type": "object",
                "properties": {
                    "source_id": { "type": "string", "description": "출발 문서 ID" },
                    "target_id": { "type": "string", "description": "도착 문서 ID" }
                },
                "required": ["source_id", "target_id"]
            })),
            make_tool("kg_stats", "지식 그래프 전체 통계를 반환합니다.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("list_todos", "할일 목록을 반환합니다 (미완료/완료 전체).", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("complete_todo", "할일 항목을 완료 처리합니다.", serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "완료할 항목 텍스트" }
                },
                "required": ["text"]
            })),
            // Phase 102: 메타 MCP 도구 — 비전문가용 통합 "설정 최적화" 진입점
            make_tool("optimize", "한 번 호출로 설정 최적화 분석을 통합 수행합니다 (비전문가용 진입점). 누적 카운터 분석(C1) + 검토 대기 추천 + 시나리오 권고(선택)를 한 응답으로 반환. 자동 적용은 없으며 next_actions에 다음 단계 안내. 사용자가 setup_apply / accept_suggested_decision으로 명시 적용.", serde_json::json!({
                "type": "object",
                "properties": {
                    "scenario": { "type": "string", "description": "사용 시나리오 자유 텍스트 (선택, 예: '회의록 위주로 가공 중 — 추천 부탁'). 입력 시 setup_review 결과 포함." },
                    "run_analysis": { "type": "boolean", "description": "C1 누적 카운터 자동 분석 실행 여부 (디폴트 true). false 시 기존 추천만 조회.", "default": true }
                }
            })),
            // Phase 76: 다축 SetupProfile 기반 설정 리뷰
            make_tool("setup_review", "사용자 시나리오를 분석해 추천 설정 변경사항을 반환합니다. profile(다축) 또는 scenario(자유 텍스트) 중 하나를 받습니다. 5축 구조: content_mix(meeting/research/code/legal/general 비율) + sensitivity + volume + search_intent + collaboration.", serde_json::json!({
                "type": "object",
                "properties": {
                    "scenario": { "type": "string", "description": "사용 시나리오 자유 텍스트 (예: '회의록 위주로 가공할 거야'). profile 미제공 시 사용." },
                    "user_role": { "type": "string", "description": "사용자 역할 (선택)" },
                    "profile": {
                        "type": "object",
                        "description": "다축 프로파일 (선택, 직접 지정 시 scenario보다 우선).",
                        "properties": {
                            "description": { "type": "string" },
                            "content_mix": {
                                "type": "array",
                                "description": "[(content_type, ratio)] 배열. 예: [[\"meeting\", 0.6], [\"code\", 0.4]]",
                                "items": { "type": "array" }
                            },
                            "sensitivity": { "type": "string", "enum": ["low", "medium", "high", "regulated"] },
                            "volume": { "type": "string", "enum": ["light", "moderate", "heavy"] },
                            "search_intent": { "type": "string", "enum": ["precision", "exploration", "temporal"] },
                            "collaboration": { "type": "string", "enum": ["solo", "small_team", "team"] },
                            "user_role": { "type": "string" }
                        }
                    }
                }
            })),
            make_tool("setup_apply", "setup_review 결과 중 사용자가 승인한 변경사항을 pipeline.toml에 적용합니다. toml_edit로 주석 보존. .bak 백업 + ConfigSnapshot(settings.db) 자동 생성. risk='critical' 항목은 apply_critical=true가 명시되어야 적용됩니다.", serde_json::json!({
                "type": "object",
                "properties": {
                    "accepted_paths": { "type": "array", "items": { "type": "string" }, "description": "적용할 path 목록 (setup_review 응답의 changes[].path)" },
                    "scenario": { "type": "string", "description": "자유 텍스트 시나리오 (profile 미제공 시)" },
                    "profile": { "type": "object", "description": "다축 프로파일 (setup_review와 동일 구조)" },
                    "apply_critical": { "type": "boolean", "description": "Critical 등급 변경(예: retention 활성화) 적용 명시 (기본 false)" }
                },
                "required": ["accepted_paths"]
            })),
            // Phase 77: snapshot 관리
            make_tool("setup_snapshot_list", "최근 설정 스냅샷 목록을 반환합니다.", serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "default": 20 }
                }
            })),
            make_tool("setup_snapshot_rollback", "지정한 스냅샷의 pipeline.toml을 복원합니다. 현재 파일은 .pre-rollback.bak으로 보존.", serde_json::json!({
                "type": "object",
                "properties": {
                    "snapshot_id": { "type": "string" },
                    "reason": { "type": "string", "description": "롤백 사유 (감사 로그용)" }
                },
                "required": ["snapshot_id", "reason"]
            })),
            make_tool("setup_snapshot_measure", "지정한 스냅샷에 현재 시점 metrics를 측정해 기록합니다. before/after 비교 후 자동 롤백 권고를 포함합니다.", serde_json::json!({
                "type": "object",
                "properties": {
                    "snapshot_id": { "type": "string", "description": "측정 대상 스냅샷 ID" },
                    "compare_to": { "type": "string", "description": "비교 대상 스냅샷 ID (선택). 미지정 시 직전 측정된 스냅샷과 비교." }
                },
                "required": ["snapshot_id"]
            })),
            // Phase 82: Decision Log — setup_apply / setup_apply_modules 결정 이력
            make_tool("setup_decision_log_list", "setup_apply / setup_apply_modules 호출 시 각 ConfigChange의 결정(accepted/rejected/critical_skipped) 이력을 반환합니다. snapshot_id로 ConfigSnapshot과 연결.", serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "default": 50, "description": "최근 N건. 0=전체" },
                    "snapshot_id": { "type": "string", "description": "지정 시 해당 적용의 결정만 반환" }
                }
            })),
            // Phase 80-E: 동작 모듈 (5축 룰 폐기 후 직접 선택 진입점)
            make_tool("setup_modules_list", "사용 가능한 동작 모듈 목록 (가공·검색·운영 그룹별). 사용자가 어떤 동작을 원하는지 선택하는 단위.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("setup_apply_modules", "선택된 동작 모듈 ID 목록을 합집합으로 적용합니다. 충돌 시 보수적 선택 (큰 청크/true/합집합/강한 도구). 배타 그룹 위반 시 에러.", serde_json::json!({
                "type": "object",
                "properties": {
                    "module_ids": { "type": "array", "items": { "type": "string" }, "description": "적용할 모듈 ID 배열 (예: ['secure_strict', 'search_precision', 'auto_lint'])" },
                    "apply_critical": { "type": "boolean", "description": "Critical risk 변경 적용 동의 (기본 false)" },
                    "dryrun": { "type": "boolean", "description": "true면 변경 미리보기만 반환 (실제 적용 안 함)" }
                },
                "required": ["module_ids"]
            })),
            // Phase 80-A/B/C/D: 패턴 분석 입력 신호 (분리된 도구)
            make_tool("get_search_mode_stats", "검색 mode 누적 카운터를 반환합니다 (default/exact/related/recent/fusion). 서버 재시작 후 settings.db에서 복원됨.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("get_crag_stats", "검색 신뢰도(CRAG) 누적 카운터를 반환합니다 (correct/ambiguous/incorrect). top_score >= 0.8 = correct.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("get_chunk_stats", "코퍼스 청크 통계를 샘플링으로 산출합니다 (평균 청크 크기, 코드펜스 포함 비율, 헤딩 인식률).", serde_json::json!({
                "type": "object",
                "properties": {
                    "sample_size": { "type": "integer", "description": "샘플링 문서 수 (기본 50)", "default": 50 }
                }
            })),
            make_tool("get_processing_metrics", "처리 시간/verify 성공률/quarantine 비율 등 처리 메트릭을 반환합니다. settings.db processing_metrics 누적 카운터 기반.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("get_llm_cache_stats", "LLM 결과 캐시 통계를 반환합니다 (entries/total_hits/avg_hits_per_entry). Ruflo A1 — 동일 파일 재가공 시 claude_cli 호출 회피.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("clear_llm_cache", "LLM 결과 캐시 전체 삭제. 모델/프롬프트 변경 후 재가공 시 사용. 반환: 삭제된 행 수.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("c1_thresholds_list", "C1 자동 추천 룰 임계값 목록 (DB 오버라이드만, 코드 디폴트 미포함). 키: mode_min_total / mode_dominant_ratio / crag_min_total / crag_incorrect_ratio / processed_min / quarantine_ratio / verify_pass_min.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("c1_threshold_set", "C1 룰 임계값 upsert. ratio 키는 0~1, count 키는 양의 정수 권장.", serde_json::json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string" },
                    "value": { "type": "number" }
                },
                "required": ["key", "value"]
            })),
            make_tool("pii_patterns_list", "C2 사용자 정의 PII 정규식 패턴 목록. 디폴트 5종 (ssn_kr/credit_card/email/phone_kr/biz_reg_kr)은 코드에 고정.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("pii_pattern_add", "C2 PII 패턴 추가 (regex 사전 검증). regex 컴파일 실패 시 에러.", serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "pattern": { "type": "string" },
                    "enabled": { "type": "boolean", "default": true }
                },
                "required": ["name", "pattern"]
            })),
            make_tool("pii_pattern_remove", "C2 PII 패턴 제거.", serde_json::json!({
                "type": "object",
                "properties": { "name": { "type": "string" } },
                "required": ["name"]
            })),
            make_tool("auto_suggest_from_counters", "Phase 80 검색 mode + CRAG 신뢰도 카운터를 분석해 자동 추천을 decision_log에 INSERT합니다 (source='auto_suggestion'). Ruflo C1 1단계 — 사용자 confirm 없이 제안만, 실제 config 변경 X. setup_decision_log_list로 검토.", serde_json::json!({
                "type": "object", "properties": {}
            })),
            make_tool("accept_suggested_decision", "사용자가 confirm한 suggested decision_log entry를 pipeline.toml에 적용합니다 (toml_edit 주석 보존 + .bak 백업). decision: suggested → accepted. Ruflo C1 2단계.", serde_json::json!({
                "type": "object",
                "properties": {
                    "decision_id": { "type": "integer", "description": "decision_log.id" }
                },
                "required": ["decision_id"]
            })),
            make_tool("reject_suggested_decision", "suggested decision_log entry를 reject 처리합니다 (config 변경 없음, decision: suggested → rejected).", serde_json::json!({
                "type": "object",
                "properties": {
                    "decision_id": { "type": "integer", "description": "decision_log.id" }
                },
                "required": ["decision_id"]
            })),
            // Phase 78
            make_tool("setup_dryrun", "추천 적용 결과를 미리 보여줍니다. 현재 config와 추천 적용 후 config의 차이를 단계별로 분석. 실제 처리는 하지 않음.", serde_json::json!({
                "type": "object",
                "properties": {
                    "scenario": { "type": "string" },
                    "profile": { "type": "object", "description": "다축 프로파일 (선택)" },
                    "accepted_paths": { "type": "array", "items": { "type": "string" }, "description": "적용 시뮬레이션할 path 목록 (미지정 시 전체)" }
                }
            })),
            make_tool("setup_profile_infer", "현재 코퍼스 사용 패턴(doc_type 분포, 민감 격리 비율, 검색 모드)에서 SetupProfile을 자동 추정합니다. 저장된 프로파일과의 불일치 항목도 함께 반환.", serde_json::json!({
                "type": "object",
                "properties": {
                    "saved_profile": { "type": "object", "description": "비교 대상 (선택). 미지정 시 추정만 반환." }
                }
            })),
            // Phase E (Grimoire 흡수, prd/research/external-analysis-2026-06-04-grimoire.md)
            make_tool("get_index", "코퍼스 라우팅용 목차를 반환합니다. doc_type별 그룹 + 카운트 + 최근 갱신 표시. search 호출 전 Claude가 사전 라우팅에 사용.", serde_json::json!({
                "type": "object",
                "properties": {
                    "group_by": { "type": "string", "enum": ["doc_type", "date"], "default": "doc_type", "description": "그룹화 기준" },
                    "top_per_group": { "type": "integer", "default": 5, "description": "그룹당 최근 N건 표시" }
                }
            })),
            make_tool("get_context", "현재 작업 경로(cwd)에서 관련 프로젝트/doc_type을 추론합니다. 관련 토픽 자동 제안.", serde_json::json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "description": "작업 디렉토리 절대 경로" }
                },
                "required": ["cwd"]
            })),
            make_tool("write_note", "Claude가 작성한 노트를 분류규약에 맞춰 저장합니다. type/domain을 setup_rules.toml의 분류 규약으로 역매핑해 저장 위치 결정. Markdown frontmatter 자동 생성. 디폴트 비활성 — pipeline.toml [grimoire].write_note_enabled=true 필요.", serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "노트 제목" },
                    "content": { "type": "string", "description": "본문 (Markdown)" },
                    "type": { "type": "string", "description": "분류 타입 (analysis/runbook/note 등)" },
                    "domain": { "type": "string", "description": "도메인 (backend/frontend/sre 등)" }
                },
                "required": ["title", "content"]
            })),
        ];

        // 비활성화된 도구 필터링: settings.db.mcp_disabled_tools에 등록된 이름 제외
        let mut tools = tools;
        if let Ok(db) = crate::settings_db::SettingsDb::open(&self.settings_db_path) {
            if let Ok(disabled) = db.list_disabled_mcp_tools() {
                if !disabled.is_empty() {
                    tools.retain(|t| !disabled.iter().any(|d| d.as_str() == t.name.as_ref()));
                }
            }
        }
        std::future::ready(Ok(ListToolsResult { tools, next_cursor: None, meta: None }))
    }

    // rmcp ServerHandler trait이 요구하는 시그니처라 async fn으로 변경 불가
    #[allow(clippy::manual_async_fn)]
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = std::result::Result<CallToolResult, rmcp::ErrorData>> + Send + '_ {
        async move {
            let name = request.name.as_ref();
            let args = serde_json::to_value(request.arguments.unwrap_or_default())
                .unwrap_or_default();

            // 비활성화된 도구는 호출 차단
            if let Ok(db) = crate::settings_db::SettingsDb::open(&self.settings_db_path) {
                if let Ok(disabled) = db.list_disabled_mcp_tools() {
                    if disabled.iter().any(|d| d == name) {
                        return Ok(CallToolResult::error(vec![Content::text(
                            format!("도구 '{}'는 사용자가 비활성화함. Settings > MCP 도구에서 활성화하세요.", name)
                        )]));
                    }
                }
            }

            let result = match name {
                "search" => self.handle_search(&args).await,
                "get_document" => self.handle_get_document(&args).await,
                "list_documents" => self.handle_list_documents(&args).await,
                "stats" => self.handle_stats().await,
                "lint" => self.handle_lint().await,
                "revise_topic" => self.handle_revise_topic(&args).await,
                "kg_neighbors" => self.handle_kg_neighbors(&args).await,
                "kg_paths" => self.handle_kg_paths(&args).await,
                "kg_stats" => self.handle_kg_stats().await,
                "list_todos" => self.handle_list_todos().await,
                "complete_todo" => self.handle_complete_todo(&args).await,
                // Phase 102: 비전문가용 통합 최적화 진입점
                "optimize" => self.handle_optimize(&args).await,
                // Phase 73
                "setup_review" => self.handle_setup_review(&args).await,
                "setup_apply" => self.handle_setup_apply(&args).await,
                // Phase 77
                "setup_snapshot_list" => self.handle_setup_snapshot_list(&args).await,
                "setup_snapshot_rollback" => self.handle_setup_snapshot_rollback(&args).await,
                "setup_snapshot_measure" => self.handle_setup_snapshot_measure(&args).await,
                // Phase 78
                "setup_dryrun" => self.handle_setup_dryrun(&args).await,
                "setup_profile_infer" => self.handle_setup_profile_infer(&args).await,
                // Phase 80-A/B/C/D
                "get_search_mode_stats" => self.handle_get_search_mode_stats().await,
                "get_crag_stats" => self.handle_get_crag_stats().await,
                "get_chunk_stats" => self.handle_get_chunk_stats(&args).await,
                "get_processing_metrics" => self.handle_get_processing_metrics().await,
                "get_llm_cache_stats" => self.handle_get_llm_cache_stats().await,
                "clear_llm_cache" => self.handle_clear_llm_cache().await,
                "c1_thresholds_list" => self.handle_c1_thresholds_list().await,
                "c1_threshold_set" => self.handle_c1_threshold_set(&args).await,
                "pii_patterns_list" => self.handle_pii_patterns_list().await,
                "pii_pattern_add" => self.handle_pii_pattern_add(&args).await,
                "pii_pattern_remove" => self.handle_pii_pattern_remove(&args).await,
                "auto_suggest_from_counters" => self.handle_auto_suggest_from_counters().await,
                "accept_suggested_decision" => self.handle_accept_suggested(&args).await,
                "reject_suggested_decision" => self.handle_reject_suggested(&args).await,
                // Phase 80-E
                "setup_modules_list" => self.handle_setup_modules_list().await,
                "setup_apply_modules" => self.handle_setup_apply_modules(&args).await,
                // Phase 82
                "setup_decision_log_list" => self.handle_setup_decision_log_list(&args).await,
                // Phase E (Grimoire 흡수)
                "get_index" => self.handle_get_index(&args).await,
                "get_context" => self.handle_get_context(&args).await,
                "write_note" => self.handle_write_note(&args).await,
                _ => Err(anyhow::anyhow!("알 수 없는 도구: {}", name)),
            };

            match result {
                Ok(json) => Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&json).unwrap_or_default(),
                )])),
                Err(e) => Ok(CallToolResult::error(vec![Content::text(
                    format!("오류: {}", e),
                )])),
            }
        }
    }
}

/// Sentence Window: 문서에서 query 키워드가 가장 많이 매칭되는 줄 ± window_size 반환
fn sentence_window(content: &str, query_words: &[&str], window_size: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() { return content.to_string(); }

    // 각 줄의 query 매칭 점수 계산
    let mut best_idx = 0;
    let mut best_score = 0usize;
    for (i, line) in lines.iter().enumerate() {
        let line_lower = line.to_lowercase();
        let score: usize = query_words.iter()
            .filter(|w| line_lower.contains(&w.to_lowercase()))
            .count();
        if score > best_score {
            best_score = score;
            best_idx = i;
        }
    }

    // 매칭 없으면 첫 15줄 반환 (기존 read_header 동작)
    if best_score == 0 {
        return lines.iter().take(15).copied().collect::<Vec<_>>().join("\n");
    }

    // 매칭 위치 ± window_size
    let start = best_idx.saturating_sub(window_size);
    let end = (best_idx + window_size + 1).min(lines.len());
    lines[start..end].join("\n")
}

/// 검색 결과 스니펫 추출: 쿼리 단어와 가장 많이 매칭되는 문장을 반환
fn extract_snippet(content: &str, query: &str, max_len: usize) -> String {
    let query_words: Vec<&str> = query.split_whitespace().collect();
    let sentences: Vec<&str> = content.split(['.', '!', '?', '\n'])
        .filter(|s| !s.trim().is_empty())
        .collect();

    // Find sentence with most query word matches
    let mut best_sentence = "";
    let mut best_score = 0;
    for s in &sentences {
        let s_lower = s.to_lowercase();
        let score = query_words.iter()
            .filter(|w| s_lower.contains(&w.to_lowercase()))
            .count();
        if score > best_score {
            best_score = score;
            best_sentence = s;
        }
    }

    if best_sentence.is_empty() {
        content.chars().take(max_len).collect()
    } else {
        let snippet = best_sentence.trim();
        if snippet.len() > max_len {
            format!("{}...", &snippet[..max_len])
        } else {
            snippet.to_string()
        }
    }
}

impl McpState {
    async fn handle_search(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let started = std::time::Instant::now();
        let query = args["query"].as_str().unwrap_or("");
        // Phase 94 A3: 검색 호출 단위 trace_id (메타 룰 13 2단계).
        let trace = file_pipeline_core::audit::TraceId::new();
        let inputs_hash = file_pipeline_core::audit::input_hash_prefix(query.as_bytes());
        let keyword = args["keyword"].as_str();
        let doc_type = args["doc_type"].as_str();

        // 실사용 측정 지표: 일일 검색 횟수(지표4)
        info!(
            "[mcp-usage] search query={:?} mode={} timestamp={}",
            &query[..query.len().min(80)],
            args["mode"].as_str().unwrap_or("default"),
            chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
        );
        // Phase 80-A: 검색 mode 카운터 (메모리 + DB 영속화)
        self.record_search_mode(args["mode"].as_str().unwrap_or("default"));
        let date_from = args["date_from"].as_str().unwrap_or("");
        let date_to = args["date_to"].as_str().unwrap_or("");
        let top_k = args["top_k"].as_u64().unwrap_or(5) as usize;
        let mode = args["mode"].as_str().unwrap_or("default");

        // 캐시 확인
        let cache_key = format!("{}|{}|{}", query, keyword.unwrap_or(""), doc_type.unwrap_or(""));
        {
            let cache = self.search_cache.lock().expect("cache lock");
            if let Some((cached_results, ts)) = cache.get(&cache_key) {
                if ts.elapsed().as_secs() < 300 {
                    let results = cached_results.clone();
                    // Phase 91 A2: 출력 PII mask. header + snippet 양쪽 적용.
                    let mask = self.output_pii_mask;
                    let patterns = &self.pii_user_patterns;
                    let docs: Vec<serde_json::Value> = results.iter().take(top_k).map(|r| {
                        let header_raw = self.storage.read_header(&r.path, 15).unwrap_or_default();
                        let snippet_raw = extract_snippet(&header_raw, query, 200);
                        let (header, snippet) = if mask {
                            (
                                file_pipeline_core::domain::classifier::SensitivityDetector::mask_pii_in_text(&header_raw, patterns),
                                file_pipeline_core::domain::classifier::SensitivityDetector::mask_pii_in_text(&snippet_raw, patterns),
                            )
                        } else { (header_raw, snippet_raw) };
                        serde_json::json!({ "id": r.id, "score": r.score, "doc_types": r.doc_types, "date": r.date, "hierarchy": r.hierarchy, "header": header, "snippet": snippet })
                    }).collect();
                    // 로그 기록
                    let latency_ms = started.elapsed().as_millis() as u64;
                    let _ = self.search_log.lock().map(|mut log| {
                        log.push(SearchLogEntry {
                            query: query.to_string(), results_count: docs.len(),
                            result_ids: docs.iter().filter_map(|d| d["id"].as_str().map(String::from)).collect(),
                            latency_ms, timestamp: chrono::Local::now().to_rfc3339(),
                            mode: mode.to_string(),
                        });
                    });
                    // Phase 94 A3: 캐시 hit 시에도 audit_trace 기록
                    let summary = file_pipeline_core::audit::truncate_output_summary(
                        &format!("results={} cached=true", docs.len())
                    );
                    self.audit.record(trace.as_str(), "mcp.search.cached", Some(&inputs_hash), Some(&summary), Some("success"));
                    return Ok(serde_json::json!({ "results": docs, "total": docs.len(), "cached": true }));
                }
            }
        }

        // 질의 확장 (mode=default 또는 related에서, 짧은 쿼리에 자동 적용)
        let expanded_query = if query.split_whitespace().count() <= 3 && (mode == "default" || mode == "related") {
            // 짧은 쿼리에 키워드 기반 확장 (LLM 호출 없음, 저비용)
            let words: Vec<&str> = query.split_whitespace().collect();
            let extra = words.join(" ");
            format!("{} {}", query, extra)
        } else {
            query.to_string()
        };
        let embedding = self.embedding.embed(&expanded_query).await?;

        let mut results = match mode {
            "exact" => {
                // BM25 우선: 키워드가 있으면 hybrid, 없으면 query 자체를 keyword로
                let kw = keyword.unwrap_or(query);
                self.vector_db.search_hybrid(&embedding, kw, top_k * 3)?
            }
            "related" => {
                // Dense 우선 + 그래프 확장
                let mut dense = self.vector_db.search_similar(&embedding, top_k * 3)?;
                // 상위 3개 문서의 관련 문서도 추가
                let top_ids: Vec<String> = dense.iter().take(3).map(|r| r.id.clone()).collect();
                for id in &top_ids {
                    if let Ok(rels) = self.vector_db.find_related(id) {
                        for rel in rels.iter().take(2) {
                            if !dense.iter().any(|r| r.id == rel.target_id) {
                                if let Ok(all) = self.vector_db.list_all() {
                                    if let Some(doc) = all.iter().find(|d| d.id == rel.target_id) {
                                        dense.push(file_pipeline_core::domain::models::SimilarDoc {
                                            id: doc.id.clone(), path: doc.path.clone(),
                                            score: 0.5, // 그래프 확장 기본 점수
                                            doc_types: doc.doc_types.clone(), date: doc.date.clone(),
                                            ..Default::default()
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                dense
            }
            "recent" => {
                // 기본 검색 + 날짜 정렬 우선
                let mut r = if let Some(kw) = keyword {
                    self.vector_db.search_hybrid(&embedding, kw, top_k * 5)?
                } else {
                    self.vector_db.search_similar(&embedding, top_k * 5)?
                };
                // 날짜 내림차순 정렬 (최신 우선), 동일 날짜면 score 순
                r.sort_by(|a, b| b.date.cmp(&a.date).then(b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)));
                r
            }
            "fusion" => {
                // RAG-Fusion: 원본 + 키워드 변형 쿼리 → 각각 검색 → RRF 결합
                let mut all_results = self.vector_db.search_similar(&embedding, top_k * 3)?;
                // 키워드 기반 추가 검색
                for word in query.split_whitespace().take(3) {
                    let kw_results = self.vector_db.search_hybrid(&embedding, word, top_k * 2)?;
                    for r in kw_results {
                        if !all_results.iter().any(|a| a.id == r.id) {
                            all_results.push(r);
                        }
                    }
                }
                // RRF 점수 재계산
                let mut rrf_scores: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
                for (rank, r) in all_results.iter().enumerate() {
                    *rrf_scores.entry(r.id.clone()).or_default() += 1.0 / (60.0 + rank as f64);
                }
                all_results.sort_by(|a, b| {
                    let sa = rrf_scores.get(&a.id).unwrap_or(&0.0);
                    let sb = rrf_scores.get(&b.id).unwrap_or(&0.0);
                    sb.partial_cmp(sa).unwrap_or(std::cmp::Ordering::Equal)
                });
                all_results.dedup_by(|a, b| a.id == b.id);
                all_results
            }
            _ => {
                // default: 기존 하이브리드
                if let Some(kw) = keyword {
                    self.vector_db.search_hybrid(&embedding, kw, top_k * 3)?
                } else {
                    self.vector_db.search_similar(&embedding, top_k * 3)?
                }
            }
        };

        // doc_type 필터
        if let Some(dt) = doc_type {
            results.retain(|r| r.doc_types.iter().any(|t| t == dt));
        }

        // 날짜 필터
        if !date_from.is_empty() || !date_to.is_empty() {
            results.retain(|r| {
                let date = &r.date;
                let after = date_from.is_empty() || date.as_str() >= date_from;
                let before = date_to.is_empty() || date.as_str() <= date_to;
                after && before
            });
        }
        // 리랭킹 (활성화 시)
        if self.reranker.is_enabled() && !results.is_empty() {
            let fallback = results.clone();
            results = self.reranker.rerank(query, results).await.unwrap_or(fallback);
        }

        // CRAG: 검색 신뢰도 판정 + 보완 검색
        let top_score = results.first().map(|r| r.score).unwrap_or(0.0);
        let confidence = if top_score >= 0.8 { "correct" }
            else if top_score >= 0.5 { "ambiguous" }
            else { "incorrect" };
        // Phase 80-B: CRAG 누적 카운터
        self.record_crag(confidence);

        if confidence == "ambiguous" {
            // 보완: 그래프 확장 (상위 3개 문서의 관련 문서 추가)
            let top_ids: Vec<String> = results.iter().take(3).map(|r| r.id.clone()).collect();
            for id in &top_ids {
                if let Ok(rels) = self.vector_db.find_related(id) {
                    for rel in rels.iter().take(2) {
                        if !results.iter().any(|r| r.id == rel.target_id) {
                            if let Ok(all) = self.vector_db.list_all() {
                                if let Some(doc) = all.iter().find(|d| d.id == rel.target_id) {
                                    results.push(file_pipeline_core::domain::models::SimilarDoc {
                                        id: doc.id.clone(), path: doc.path.clone(),
                                        score: top_score * 0.7,
                                        doc_types: doc.doc_types.clone(), date: doc.date.clone(),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                }
            }
        } else if confidence == "incorrect" && !results.is_empty() {
            // 전략 전환: keyword 전용 검색 추가
            let kw_results = self.vector_db.search_hybrid(&embedding, query, top_k * 2)?;
            for kr in kw_results {
                if !results.iter().any(|r| r.id == kr.id) {
                    results.push(kr);
                }
            }
        }

        // 트리거 #6 HyDE 폴백: 결과 빈약(< hyde_min_results) + 활성화 시 LLM 가상 답변 임베딩으로 재검색
        if self.hyde_enabled && results.len() < self.hyde_min_results {
            if let Ok(hyde_text) = self.llm.generate_hypothetical(query).await {
                // 디폴트 구현은 query 자체를 반환하므로 동일하면 의미 없음 — 어댑터가 오버라이드 시에만 효과
                if hyde_text != query && !hyde_text.trim().is_empty() {
                    if let Ok(hyde_emb) = self.embedding.embed(&hyde_text).await {
                        let hyde_results = self.vector_db
                            .search_similar(&hyde_emb, top_k * 2)
                            .unwrap_or_default();
                        for hr in hyde_results {
                            if !results.iter().any(|r| r.id == hr.id) {
                                // HyDE 결과는 신뢰도 감산 (0.6 보정) — 직접 매칭보다 약한 신호로 표시
                                let mut adj = hr.clone();
                                adj.score *= 0.6;
                                results.push(adj);
                            }
                        }
                        info!("[hyde] fallback triggered: results {} (min {})", results.len(), self.hyde_min_results);
                    }
                }
            }
        }

        // Ruflo A2: KG 1-hop 확장 (expand_kg_hops > 0 일 때)
        // Phase 103 G3 (GraphRAG Multi-hop 빔 검색): kg_beam_search=true 시 빔 폭(=expand_kg_hops)만큼만 유지
        // 디폴트: 단순 1-hop 확장 (lesson 30 Ruflo 패턴 유지).
        if self.expand_kg_hops > 0 && !results.is_empty() {
            let mut added = 0usize;
            let max_add = self.expand_kg_hops;
            let seed_ids: Vec<String> = results.iter().take(top_k.min(results.len()))
                .map(|r| r.id.clone()).collect();
            // G3 빔 검색: 시드 점수 상위 N건만 확장 (beam_width=expand_kg_hops)
            let seed_count = if self.kg_beam_search { max_add.min(seed_ids.len()) } else { seed_ids.len() };
            for id in seed_ids.into_iter().take(seed_count) {
                if added >= max_add { break; }
                if let Ok(rels) = self.vector_db.find_related(&id) {
                    for rel in rels {
                        if added >= max_add { break; }
                        if results.iter().any(|r| r.id == rel.target_id) { continue; }
                        if let Ok(all) = self.vector_db.list_all() {
                            if let Some(target) = all.into_iter().find(|d| d.id == rel.target_id) {
                                results.push(file_pipeline_core::domain::models::SimilarDoc {
                                    id: target.id,
                                    path: target.path,
                                    score: 0.0,
                                    doc_types: target.doc_types,
                                    date: target.date,
                                    hierarchy: vec![],
                                });
                                added += 1;
                            }
                        }
                    }
                }
            }
        }

        // Ruflo B1: 다양성 강화 (diversity_threshold > 0 일 때)
        // 동일 doc_type이 임계값 초과 시, top_k 범위 밖의 다른 doc_type 결과를 끌어올림
        if self.diversity_threshold > 0 && results.len() > top_k {
            let threshold = self.diversity_threshold;
            let head_len = top_k.min(results.len());
            let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for r in results.iter().take(head_len) {
                for t in &r.doc_types {
                    *type_counts.entry(t.clone()).or_insert(0) += 1;
                }
            }
            let dominant: Vec<String> = type_counts.iter()
                .filter(|(_, c)| **c > threshold)
                .map(|(t, _)| t.clone()).collect();
            if !dominant.is_empty() {
                // top_k 범위 밖에서 dominant에 속하지 않는 첫 후보 찾기
                let mut promote_idx: Option<usize> = None;
                for (idx, r) in results.iter().enumerate().skip(head_len) {
                    let is_dominant = r.doc_types.iter().any(|t| dominant.contains(t));
                    if !is_dominant {
                        promote_idx = Some(idx);
                        break;
                    }
                }
                // dominant 마지막 항목과 swap
                if let Some(p) = promote_idx {
                    let mut demote_idx: Option<usize> = None;
                    for (idx, r) in results.iter().enumerate().take(head_len).rev() {
                        if r.doc_types.iter().any(|t| dominant.contains(t)) {
                            demote_idx = Some(idx);
                            break;
                        }
                    }
                    if let Some(d) = demote_idx {
                        results.swap(d, p);
                    }
                }
            }
        }

        // Phase 103 G4 (GraphRAG TF-IDF 흡수): 다양성 재순위
        // 본문 토큰 빈도 기반 — 동일 토큰 집중 결과를 분산. 디폴트 비활성.
        // 활성화 시: 결과 본문 100줄을 토큰화 → 토큰 빈도 IDF 계산 → 유사 결과 demote
        if self.tfidf_rerank_enabled && results.len() > top_k {
            let head_len = top_k.min(results.len());
            // 각 결과의 본문에서 토큰 추출 (100줄 read_header)
            let mut tokens_per_doc: Vec<std::collections::HashSet<String>> = Vec::with_capacity(results.len());
            for r in results.iter() {
                let text = self.storage.read_header(&r.path, 100).unwrap_or_default();
                let set: std::collections::HashSet<String> = text.to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|t| t.len() >= 3)
                    .map(String::from)
                    .collect();
                tokens_per_doc.push(set);
            }
            // 상위 head_len 토큰 합집합 — "이미 본 토큰"
            let mut seen_tokens: std::collections::HashSet<String> = std::collections::HashSet::new();
            for s in tokens_per_doc.iter().take(head_len) {
                for t in s.iter() { seen_tokens.insert(t.clone()); }
            }
            // top_k 범위 밖에서 새 토큰 비율 최대 결과 promote
            let mut best_idx: Option<usize> = None;
            let mut best_novelty: f32 = 0.0;
            for (idx, s) in tokens_per_doc.iter().enumerate().skip(head_len) {
                if s.is_empty() { continue; }
                let novel: usize = s.iter().filter(|t| !seen_tokens.contains(*t)).count();
                let ratio = novel as f32 / s.len() as f32;
                if ratio > best_novelty {
                    best_novelty = ratio;
                    best_idx = Some(idx);
                }
            }
            // 임계 0.5: 신규 토큰 50% 이상이면 마지막 head 결과와 교체
            if best_novelty >= 0.5 {
                if let (Some(promote), demote) = (best_idx, head_len.saturating_sub(1)) {
                    if demote < results.len() && promote < results.len() {
                        results.swap(demote, promote);
                    }
                }
            }
        }

        // 캐시 저장
        {
            let mut cache = self.search_cache.lock().expect("cache lock");
            cache.insert(cache_key, (results.clone(), std::time::Instant::now()));
            if cache.len() > 1000 {
                cache.clear();
            }
        }

        results.truncate(top_k);

        // Parent-Child (경량): 검색 결과 문서의 전체 맥락 확장
        // read_header를 100줄로 확장하여 매칭 문장 주변을 반환 (Sentence Window)
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        // Phase 91 A2: 출력 PII mask. Sentence Window 결과에도 적용.
        let mask = self.output_pii_mask;
        let patterns = &self.pii_user_patterns;
        let docs: Vec<serde_json::Value> = results.iter().map(|r| {
            // Sentence Window: 더 많이 읽고 query 매칭 위치 주변 반환
            let full_text = self.storage.read_header(&r.path, 100).unwrap_or_default();
            let window_raw = sentence_window(&full_text, &query_words, 5);
            let snippet_raw = extract_snippet(&window_raw, query, 300);
            let (window, snippet) = if mask {
                (
                    file_pipeline_core::domain::classifier::SensitivityDetector::mask_pii_in_text(&window_raw, patterns),
                    file_pipeline_core::domain::classifier::SensitivityDetector::mask_pii_in_text(&snippet_raw, patterns),
                )
            } else { (window_raw, snippet_raw) };
            serde_json::json!({ "id": r.id, "score": r.score, "doc_types": r.doc_types, "date": r.date, "hierarchy": r.hierarchy, "header": window, "snippet": snippet })
        }).collect();

        // 로그 기록
        let latency_ms = started.elapsed().as_millis() as u64;
        let _ = self.search_log.lock().map(|mut log| {
            log.push(SearchLogEntry {
                query: query.to_string(), results_count: docs.len(),
                result_ids: docs.iter().filter_map(|d| d["id"].as_str().map(String::from)).collect(),
                latency_ms, timestamp: chrono::Local::now().to_rfc3339(),
                mode: mode.to_string(),
            });
        });

        // Phase 94 A3: 검색 결과 audit_trace 기록
        let elapsed_ms = started.elapsed().as_millis();
        let summary = file_pipeline_core::audit::truncate_output_summary(
            &format!("results={} confidence={} mode={} elapsed_ms={}", docs.len(), confidence, mode, elapsed_ms)
        );
        self.audit.record(trace.as_str(), "mcp.search", Some(&inputs_hash), Some(&summary), Some("success"));

        Ok(serde_json::json!({ "results": docs, "total": docs.len(), "confidence": confidence, "mode": mode }))
    }

    async fn handle_get_document(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let doc_id = args["doc_id"].as_str().unwrap_or("");
        // Phase 97 A3 확장 — 메타 룰 13 2단계 완성도 100%
        let trace = file_pipeline_core::audit::TraceId::new();
        let inputs_hash = file_pipeline_core::audit::input_hash_prefix(doc_id.as_bytes());
        let all = self.vector_db.list_all()?;
        let doc = all.iter().find(|d| d.id == doc_id)
            .ok_or_else(|| {
                let summary = file_pipeline_core::audit::truncate_output_summary(&format!("not_found: {}", doc_id));
                self.audit.record(trace.as_str(), "mcp.get_document", Some(&inputs_hash), Some(&summary), Some("error"));
                anyhow::anyhow!("문서 없음: {}", doc_id)
            })?;

        // 실사용 측정 지표: 검색 성공률(지표1), 검색-사용 지연(지표5)
        info!(
            "[mcp-usage] get_document doc_id={} doc_types={:?} timestamp={}",
            doc_id,
            doc.doc_types,
            chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
        );

        let temp = self.storage.decompress_temp(&doc.path)?;
        let content = std::fs::read_to_string(&temp)?;
        let _ = std::fs::remove_file(&temp);
        let relations = self.vector_db.find_related(doc_id)?;
        let summary = file_pipeline_core::audit::truncate_output_summary(
            &format!("types={:?} content_len={} relations={}", doc.doc_types, content.len(), relations.len())
        );
        self.audit.record(trace.as_str(), "mcp.get_document", Some(&inputs_hash), Some(&summary), Some("success"));
        Ok(serde_json::json!({
            "id": doc.id, "doc_types": doc.doc_types, "content": content,
            "relations": relations.iter().map(|r| serde_json::json!({
                "target": r.target_id, "type": r.relation_type.to_string(),
            })).collect::<Vec<_>>(),
        }))
    }

    async fn handle_list_documents(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let type_filter = args["doc_type"].as_str();
        // Phase 97 A3 확장
        let trace = file_pipeline_core::audit::TraceId::new();
        let inputs_hash = file_pipeline_core::audit::input_hash_prefix(
            type_filter.unwrap_or("").as_bytes()
        );
        let all = self.vector_db.list_all()?;
        let filtered: Vec<_> = all.iter()
            .filter(|d| type_filter.map(|t| d.doc_types.iter().any(|dt| dt == t)).unwrap_or(true))
            .map(|d| serde_json::json!({ "id": d.id, "path": d.path.to_string_lossy(), "doc_types": d.doc_types }))
            .collect();
        let summary = file_pipeline_core::audit::truncate_output_summary(
            &format!("total={} filter={:?}", filtered.len(), type_filter)
        );
        self.audit.record(trace.as_str(), "mcp.list_documents", Some(&inputs_hash), Some(&summary), Some("success"));
        Ok(serde_json::json!({ "documents": filtered, "total": filtered.len() }))
    }

    async fn handle_stats(&self) -> Result<serde_json::Value> {
        let stats = self.vector_db.stats()?;
        Ok(serde_json::json!({ "total_documents": stats.total_documents, "by_type": stats.by_type }))
    }

    async fn handle_revise_topic(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let file = args["file"].as_str().unwrap_or("");
        let feedback = args["feedback"].as_str().unwrap_or("");
        let path = std::path::Path::new(file);
        let revised = file_pipeline_core::domain::topic_merger::TopicMerger::revise_topic(
            path, feedback, self.llm.as_ref(),
        ).await?;
        Ok(serde_json::json!({ "revised": true, "length": revised.len() }))
    }

    async fn handle_kg_neighbors(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let doc_id = args["doc_id"].as_str().unwrap_or("");
        // Phase 95 A3 확장
        let trace = file_pipeline_core::audit::TraceId::new();
        let inputs_hash = file_pipeline_core::audit::input_hash_prefix(doc_id.as_bytes());
        let result = file_pipeline_core::domain::wiki_export::KgQueryEngine::neighbors(
            self.vector_db.as_ref(), doc_id,
        )?;
        let summary = file_pipeline_core::audit::truncate_output_summary(
            &format!("nodes={} edges={}", result.nodes.len(), result.edges.len())
        );
        self.audit.record(trace.as_str(), "mcp.kg_neighbors", Some(&inputs_hash), Some(&summary), Some("success"));
        Ok(serde_json::to_value(&result)?)
    }

    async fn handle_kg_paths(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let source = args["source_id"].as_str().unwrap_or("");
        let target = args["target_id"].as_str().unwrap_or("");
        // Phase 95 A3 확장
        let trace = file_pipeline_core::audit::TraceId::new();
        let inputs_hash = file_pipeline_core::audit::input_hash_prefix(
            format!("{}|{}", source, target).as_bytes()
        );
        let result = file_pipeline_core::domain::wiki_export::KgQueryEngine::find_paths(
            self.vector_db.as_ref(), source, target,
        )?;
        let summary = file_pipeline_core::audit::truncate_output_summary(
            &format!("paths={} nodes={}", result.paths.len(), result.nodes.len())
        );
        self.audit.record(trace.as_str(), "mcp.kg_paths", Some(&inputs_hash), Some(&summary), Some("success"));
        Ok(serde_json::to_value(&result)?)
    }

    async fn handle_kg_stats(&self) -> Result<serde_json::Value> {
        let stats = file_pipeline_core::domain::wiki_export::KgQueryEngine::stats(
            self.vector_db.as_ref(),
        )?;
        Ok(serde_json::to_value(&stats)?)
    }

    async fn handle_lint(&self) -> Result<serde_json::Value> {
        let report = file_pipeline_core::domain::lint::Linter::lint(self.vector_db.as_ref())?;
        Ok(serde_json::json!({
            "orphan_docs": report.orphan_docs.len(),
            "stale_docs": report.stale_docs.len(),
            "issues": report.issues.iter().map(|i| serde_json::json!({
                "doc_id": i.doc_id, "type": format!("{:?}", i.issue_type), "description": i.description,
            })).collect::<Vec<_>>(),
        }))
    }

    async fn handle_list_todos(&self) -> Result<serde_json::Value> {
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let todos = db.list_todos(Some("open"), None)?;
        let pending = todos.len();
        Ok(serde_json::json!({ "items": todos, "pending": pending, "total": pending }))
    }

    async fn handle_complete_todo(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let id = args["id"].as_str().unwrap_or("");
        if id.is_empty() { anyhow::bail!("id 필수"); }
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let ok = db.complete_todo(id)?;
        Ok(serde_json::json!({ "ok": ok }))
    }

    // Phase 76: 설정 리뷰 핸들러 (다축 SetupProfile 또는 자유 텍스트 scenario)
    async fn handle_setup_review(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let cfg_path = crate::config::find_config_path(None);
        let current = crate::config::PipelineConfig::load(&cfg_path)
            .unwrap_or_else(|_| crate::config::PipelineConfig::default_config());

        // profile이 직접 전달되면 사용, 아니면 scenario 텍스트에서 추론
        let advice = if let Some(profile_v) = args.get("profile") {
            let profile: crate::setup_review::SetupProfile = serde_json::from_value(profile_v.clone())
                .map_err(|e| anyhow::anyhow!("profile 파싱 실패: {}", e))?;
            crate::setup_review::build_advice_from_profile(profile, &current)
        } else {
            let scenario = args["scenario"].as_str().unwrap_or("").to_string();
            if scenario.is_empty() { anyhow::bail!("scenario 또는 profile 중 하나 필수"); }
            let user_role = args["user_role"].as_str().map(|s| s.to_string());
            crate::setup_review::build_advice(&scenario, user_role, &current)
        };
        Ok(serde_json::to_value(advice)?)
    }

    async fn handle_setup_apply(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let accepted_paths: Vec<String> = args["accepted_paths"].as_array()
            .ok_or_else(|| anyhow::anyhow!("accepted_paths 배열 필수"))?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        let apply_critical = args["apply_critical"].as_bool().unwrap_or(false);

        let cfg_path = crate::config::find_config_path(None);
        let current = crate::config::PipelineConfig::load(&cfg_path)
            .unwrap_or_else(|_| crate::config::PipelineConfig::default_config());

        let advice = if let Some(profile_v) = args.get("profile") {
            let profile: crate::setup_review::SetupProfile = serde_json::from_value(profile_v.clone())
                .map_err(|e| anyhow::anyhow!("profile 파싱 실패: {}", e))?;
            crate::setup_review::build_advice_from_profile(profile, &current)
        } else {
            let scenario = args["scenario"].as_str().unwrap_or("").to_string();
            if scenario.is_empty() { anyhow::bail!("scenario 또는 profile 중 하나 필수"); }
            crate::setup_review::build_advice(&scenario, None, &current)
        };

        // Phase 77: settings.db에 snapshot 저장
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path).ok();
        let result = crate::setup_review::apply_advice_full(
            &cfg_path, &advice, &accepted_paths, apply_critical, db.as_ref(),
        )?;
        Ok(serde_json::json!({
            "applied": result.applied,
            "snapshot_id": result.snapshot_id,
            "backup": cfg_path.with_extension("toml.bak").to_string_lossy(),
            "needs_restart": advice.changes.iter()
                .filter(|c| result.applied.contains(&c.path))
                .any(|c| c.needs_restart),
        }))
    }

    // ── Phase 77: snapshot 핸들러 ─────────────────────────────

    async fn handle_setup_snapshot_list(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let limit = args["limit"].as_u64().unwrap_or(20) as usize;
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let snaps = db.list_snapshots(limit)?;
        let out: Vec<serde_json::Value> = snaps.into_iter().map(|s| serde_json::json!({
            "id": s.id,
            "created_at": s.created_at,
            "applied_paths": s.applied_paths,
            "rolled_back": s.rolled_back,
            "rollback_reason": s.rollback_reason,
            "has_metrics": s.metrics_json.is_some(),
        })).collect();
        Ok(serde_json::json!({ "snapshots": out, "total": out.len() }))
    }

    async fn handle_setup_snapshot_rollback(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let id = args["snapshot_id"].as_str().ok_or_else(|| anyhow::anyhow!("snapshot_id 필수"))?;
        let reason = args["reason"].as_str().unwrap_or("manual rollback");
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let snap = db.get_snapshot(id)?
            .ok_or_else(|| anyhow::anyhow!("스냅샷 없음: {}", id))?;
        let cfg_path = crate::config::find_config_path(None);
        crate::config_snapshot::rollback_snapshot(&cfg_path, &snap, reason)?;
        db.mark_snapshot_rolled_back(id, reason)?;
        Ok(serde_json::json!({ "ok": true, "snapshot_id": id, "config_path": cfg_path.to_string_lossy() }))
    }

    async fn handle_setup_snapshot_measure(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let id = args["snapshot_id"].as_str().ok_or_else(|| anyhow::anyhow!("snapshot_id 필수"))?;
        let compare_to = args["compare_to"].as_str();
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;

        // 현재 metrics 측정
        let metrics = self.collect_current_metrics().await?;
        let metrics_json = serde_json::to_string(&metrics)?;
        db.update_snapshot_metrics(id, &metrics_json)?;

        // 비교 대상이 있으면 자동 롤백 권고 평가
        let evaluation = if let Some(prev_id) = compare_to {
            if let Some(prev) = db.get_snapshot(prev_id)?.and_then(|s| s.metrics()) {
                let ev = crate::config_snapshot::evaluate_rollback(
                    &prev, &metrics, &crate::config_snapshot::RollbackThresholds::default(),
                );
                Some(serde_json::json!({
                    "should_rollback": ev.should_rollback,
                    "triggers": ev.triggers,
                }))
            } else { None }
        } else {
            // compare_to 미지정 — 직전 측정된 스냅샷과 자동 비교
            let snaps = db.list_snapshots(20)?;
            let prev = snaps.iter().find(|s| s.id != id && s.metrics_json.is_some() && !s.rolled_back);
            if let Some(p) = prev.and_then(|s| s.metrics()) {
                let ev = crate::config_snapshot::evaluate_rollback(
                    &p, &metrics, &crate::config_snapshot::RollbackThresholds::default(),
                );
                Some(serde_json::json!({
                    "should_rollback": ev.should_rollback,
                    "triggers": ev.triggers,
                }))
            } else { None }
        };

        Ok(serde_json::json!({
            "snapshot_id": id,
            "metrics": metrics,
            "evaluation": evaluation,
        }))
    }

    /// vector_db 통계 + lint 결과를 SnapshotMetrics로 집계
    async fn collect_current_metrics(&self) -> Result<crate::config_snapshot::SnapshotMetrics> {
        let stats = self.vector_db.stats()?;
        let total_docs = (stats.total_documents.max(1)) as f32;

        // Lint
        let lint_warnings = match file_pipeline_core::domain::lint::Linter::lint(self.vector_db.as_ref()) {
            Ok(r) => r.issues.len(),
            Err(_) => 0,
        };

        // crossref edges 평균 — list_all로 문서별 relation 수 합산
        let total_edges: usize = match self.vector_db.list_all() {
            Ok(docs) => {
                let mut sum = 0usize;
                for d in &docs {
                    if let Ok(rels) = self.vector_db.find_related(&d.id) {
                        sum += rels.len();
                    }
                }
                sum
            }
            Err(_) => 0,
        };
        let avg_crossref = total_edges as f32 / total_docs;

        // Phase 82-prep: settings.db processing_metrics 누적 카운터에서 산출.
        // 데이터 부족(분모 0)이면 0.0/0으로 유지 (이전 placeholder와 동일 동작).
        let (verify_pass_rate, quarantine_rate, avg_process_time_ms) =
            match crate::settings_db::SettingsDb::open(&self.settings_db_path) {
                Ok(db) => match db.get_processing_metric_summary() {
                    Ok(s) => (
                        s.verify_pass_rate.unwrap_or(0.0),
                        s.quarantine_rate.unwrap_or(0.0),
                        s.avg_process_time_ms.unwrap_or(0),
                    ),
                    Err(_) => (0.0, 0.0, 0),
                },
                Err(_) => (0.0, 0.0, 0),
            };

        Ok(crate::config_snapshot::SnapshotMetrics {
            measured_at: chrono::Utc::now().to_rfc3339(),
            files_processed: stats.total_documents as usize,
            verify_pass_rate,
            quarantine_rate,
            avg_process_time_ms,
            lint_warnings,
            avg_crossref_per_doc: avg_crossref,
        })
    }

    // ── Phase 78 ─────────────────────────────────────────────

    async fn handle_setup_dryrun(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let cfg_path = crate::config::find_config_path(None);
        let current = crate::config::PipelineConfig::load(&cfg_path)
            .unwrap_or_else(|_| crate::config::PipelineConfig::default_config());

        // advice 생성 (profile 또는 scenario)
        let advice = if let Some(profile_v) = args.get("profile") {
            let profile: crate::setup_review::SetupProfile = serde_json::from_value(profile_v.clone())
                .map_err(|e| anyhow::anyhow!("profile 파싱 실패: {}", e))?;
            crate::setup_review::build_advice_from_profile(profile, &current)
        } else {
            let scenario = args["scenario"].as_str().unwrap_or("");
            if scenario.is_empty() { anyhow::bail!("scenario 또는 profile 중 하나 필수"); }
            crate::setup_review::build_advice(scenario, None, &current)
        };

        // accepted_paths가 있으면 그것만, 없으면 advice의 모든 변경
        let accepted: std::collections::HashSet<String> = args["accepted_paths"].as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| advice.changes.iter().map(|c| c.path.clone()).collect());

        // PipelineConfig 위에서 in-memory 적용 시뮬레이션 (toml_edit 안 쓰고 직접)
        let mut after = current.clone();
        for ch in &advice.changes {
            if !accepted.contains(&ch.path) { continue; }
            // recommended를 후보 path에 적용 — apply_single_change와 비슷하지만 in-memory
            let _ = simulate_apply(&mut after, &ch.path, &ch.recommended);
        }

        let report = crate::setup_dryrun::diff_configs(&current, &after)?;
        Ok(serde_json::json!({
            "advice_summary": advice.summary,
            "advice_count": advice.changes.len(),
            "accepted_count": accepted.len(),
            "report": report,
        }))
    }

    async fn handle_setup_profile_infer(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let stats = self.vector_db.stats()?;

        // 검색 모드 분포 (search_log에서)
        let search_log = self.search_log.lock().expect("search_log lock");
        let mut mode_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for entry in search_log.iter() {
            *mode_counts.entry(entry.mode.clone()).or_insert(0) += 1;
        }
        drop(search_log);
        let search_mode_distribution: Vec<(String, usize)> = mode_counts.into_iter().collect();

        let usage = crate::setup_dryrun::CorpusUsageStats {
            total_documents: stats.total_documents as usize,
            by_doc_type: stats.by_type.clone(),
            // 단기 추정: 최근 4주 평균은 stats에 없으므로 total/12를 매주 평균으로 가정
            // (12주 = 약 3개월). 추후 처리 시간 분포 추가 시 정확화.
            weekly_recent_avg: (stats.total_documents as f32) / 12.0,
            sensitive_ratio: if stats.total_documents > 0 {
                stats.sensitive_count as f32 / stats.total_documents as f32
            } else { 0.0 },
            search_mode_distribution,
        };
        let inferred = crate::setup_dryrun::infer_profile_from_usage(&usage);

        let mismatches = if let Some(saved_v) = args.get("saved_profile") {
            let saved: crate::setup_review::SetupProfile = serde_json::from_value(saved_v.clone())
                .map_err(|e| anyhow::anyhow!("saved_profile 파싱 실패: {}", e))?;
            crate::setup_dryrun::detect_mismatch(&saved, &inferred)
        } else { vec![] };

        Ok(serde_json::json!({
            "inferred": inferred,
            "usage": {
                "total_documents": usage.total_documents,
                "by_doc_type": usage.by_doc_type,
                "weekly_recent_avg": usage.weekly_recent_avg,
                "sensitive_ratio": usage.sensitive_ratio,
                "search_mode_distribution": usage.search_mode_distribution,
            },
            "mismatches": mismatches,
        }))
    }

    // ── Phase 80-A/B/C/D: 패턴 분석 입력 도구 ──────────────────

    async fn handle_get_search_mode_stats(&self) -> Result<serde_json::Value> {
        let counts = self.search_mode_counts.lock().expect("search_mode lock").clone();
        let total: u64 = counts.values().sum();
        let mut items: Vec<_> = counts.iter().map(|(m, c)| {
            let ratio = if total > 0 { *c as f32 / total as f32 } else { 0.0 };
            serde_json::json!({ "mode": m, "count": c, "ratio": ratio })
        }).collect();
        // 빈도 내림차순
        items.sort_by(|a, b| b["count"].as_u64().unwrap_or(0).cmp(&a["count"].as_u64().unwrap_or(0)));
        Ok(serde_json::json!({ "modes": items, "total": total }))
    }

    async fn handle_get_crag_stats(&self) -> Result<serde_json::Value> {
        let counts = self.crag_counts.lock().expect("crag lock").clone();
        let total: u64 = counts.values().sum();
        let bucket = |k: &str| counts.get(k).copied().unwrap_or(0);
        let ratio = |c: u64| if total > 0 { c as f32 / total as f32 } else { 0.0 };
        let correct = bucket("correct");
        let ambiguous = bucket("ambiguous");
        let incorrect = bucket("incorrect");
        Ok(serde_json::json!({
            "correct": correct,
            "ambiguous": ambiguous,
            "incorrect": incorrect,
            "correct_ratio": ratio(correct),
            "ambiguous_ratio": ratio(ambiguous),
            "incorrect_ratio": ratio(incorrect),
            "total": total,
        }))
    }

    /// 청크 통계 — 코퍼스 샘플링으로 추정 (저장된 가공본 헤더 일부)
    async fn handle_get_chunk_stats(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let sample_size = args["sample_size"].as_u64().unwrap_or(50) as usize;
        let docs = self.vector_db.list_all().unwrap_or_default();
        let sample: Vec<_> = docs.iter().take(sample_size).collect();
        if sample.is_empty() {
            return Ok(serde_json::json!({
                "sample_size": 0, "avg_chunk_bytes": 0, "code_fence_ratio": 0.0,
                "heading_ratio": 0.0, "note": "코퍼스가 비어 있음"
            }));
        }

        let mut total_bytes: usize = 0;
        let mut code_fence_count = 0usize;
        let mut heading_count = 0usize;
        let mut counted = 0usize;

        for d in &sample {
            // 가공본 헤더 일부 (1500자) 읽기 — 청크 추정에 충분
            if let Ok(header) = self.storage.read_header(&d.path, 50) {
                total_bytes += header.len();
                if header.contains("```") { code_fence_count += 1; }
                if header.lines().any(|l| l.starts_with("# ") || l.starts_with("## ") || l.starts_with("### ")) {
                    heading_count += 1;
                }
                counted += 1;
            }
        }
        let n = counted.max(1) as f32;
        Ok(serde_json::json!({
            "sample_size": counted,
            "avg_chunk_bytes": (total_bytes as f32 / n) as u32,
            "code_fence_ratio": code_fence_count as f32 / n,
            "heading_ratio": heading_count as f32 / n,
            "note": "샘플 헤더(50줄) 기반 추정. 정확한 청크 통계는 embed_gen 수집 도입 후 가능.",
        }))
    }

    async fn handle_setup_modules_list(&self) -> Result<serde_json::Value> {
        let registry = crate::setup_modules::ModuleRegistry::default_registry();
        let modules: Vec<_> = registry.all().iter().map(|m| serde_json::json!({
            "id": m.id,
            "group": m.group,
            "icon": m.icon,
            "label": m.label,
            "hint": m.hint,
            "priority": m.priority,
            "exclusive_group": m.exclusive_group,
            "change_count": m.changes.len(),
            "paths": m.changes.iter().map(|c| &c.path).collect::<Vec<_>>(),
        })).collect();
        Ok(serde_json::json!({ "modules": modules, "total": modules.len() }))
    }

    async fn handle_setup_apply_modules(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let module_ids: Vec<String> = args["module_ids"].as_array()
            .ok_or_else(|| anyhow::anyhow!("module_ids 배열 필수"))?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        let apply_critical = args["apply_critical"].as_bool().unwrap_or(false);
        let dryrun = args["dryrun"].as_bool().unwrap_or(false);

        let cfg_path = crate::config::find_config_path(None);
        let current = crate::config::PipelineConfig::load(&cfg_path)
            .unwrap_or_else(|_| crate::config::PipelineConfig::default_config());
        let registry = crate::setup_modules::ModuleRegistry::default_registry();
        let changes = registry.build_changes(&module_ids, &current)?;

        if dryrun {
            return Ok(serde_json::json!({
                "dryrun": true,
                "module_ids": module_ids,
                "changes": changes,
                "change_count": changes.len(),
            }));
        }

        // SetupAdvice를 만들어서 apply_advice_full로 위임
        let profile = crate::setup_review::SetupProfile {
            description: Some(format!("modules: {}", module_ids.join(", "))),
            ..Default::default()
        };
        let advice = crate::setup_review::SetupAdvice {
            profile,
            scenario: "modules".into(),
            summary: format!("{}개 모듈 합집합", module_ids.len()),
            changes: changes.clone(),
        };
        let accepted: Vec<String> = changes.iter().map(|c| c.path.clone()).collect();
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path).ok();
        let context = serde_json::json!({ "module_ids": module_ids });
        let result = crate::setup_review::apply_advice_full_with_log(
            &cfg_path, &advice, &accepted, apply_critical, db.as_ref(),
            "setup_modules", Some(&context),
        )?;
        Ok(serde_json::json!({
            "applied": result.applied,
            "snapshot_id": result.snapshot_id,
            "module_ids": module_ids,
            "backup": cfg_path.with_extension("toml.bak").to_string_lossy(),
        }))
    }

    async fn handle_get_processing_metrics(&self) -> Result<serde_json::Value> {
        let stats = self.vector_db.stats()?;
        // Phase 82-prep: settings.db processing_metrics 누적 카운터에서 실측치 산출.
        let summary = crate::settings_db::SettingsDb::open(&self.settings_db_path)
            .ok()
            .and_then(|db| db.get_processing_metric_summary().ok());

        let (verify_pass_rate, quarantine_rate, avg_process_time_ms, success, errors, quarantined) =
            match summary {
                Some(s) => (
                    s.verify_pass_rate.map(|v| serde_json::json!(v)).unwrap_or(serde_json::Value::Null),
                    s.quarantine_rate.map(|v| serde_json::json!(v)).unwrap_or(serde_json::Value::Null),
                    s.avg_process_time_ms.map(|v| serde_json::json!(v)).unwrap_or(serde_json::Value::Null),
                    s.success,
                    s.errors,
                    s.quarantined,
                ),
                None => (serde_json::Value::Null, serde_json::Value::Null, serde_json::Value::Null, 0, 0, 0),
            };

        Ok(serde_json::json!({
            "total_documents": stats.total_documents,
            "by_doc_type": stats.by_type,
            "sensitive_count": stats.sensitive_count,
            "total_size_bytes": stats.total_size_bytes,
            "verify_pass_rate": verify_pass_rate,
            "quarantine_rate": quarantine_rate,
            "avg_process_time_ms": avg_process_time_ms,
            "counters": {
                "success": success,
                "errors": errors,
                "quarantined": quarantined,
            },
        }))
    }

    // Ruflo C1: 룰 임계값 목록
    async fn handle_c1_thresholds_list(&self) -> Result<serde_json::Value> {
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let rows = db.list_c1_thresholds()?;
        Ok(serde_json::json!({
            "overrides": rows.into_iter().map(|(k, v)| serde_json::json!({"key": k, "value": v})).collect::<Vec<_>>(),
            "defaults": {
                "mode_min_total": 100,
                "mode_dominant_ratio": 0.6,
                "crag_min_total": 50,
                "crag_incorrect_ratio": 0.25,
                "processed_min": 30,
                "quarantine_ratio": 0.25,
                "verify_pass_min": 0.6,
            }
        }))
    }

    async fn handle_c1_threshold_set(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let key = args["key"].as_str().ok_or_else(|| anyhow::anyhow!("key 필수"))?;
        let value = args["value"].as_f64().ok_or_else(|| anyhow::anyhow!("value 숫자 필수"))?;
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        db.set_c1_threshold(key, value)?;
        Ok(serde_json::json!({ "ok": true, "key": key, "value": value }))
    }

    // Ruflo C2: PII 사용자 정의 패턴 CRUD
    async fn handle_pii_patterns_list(&self) -> Result<serde_json::Value> {
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let rows = db.list_user_pii_patterns()?;
        Ok(serde_json::json!({
            "user_patterns": rows.into_iter().map(|(n, p, e)| serde_json::json!({
                "name": n, "pattern": p, "enabled": e
            })).collect::<Vec<_>>(),
            "builtin": ["ssn_kr", "credit_card", "email", "phone_kr", "biz_reg_kr"],
        }))
    }

    async fn handle_pii_pattern_add(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let name = args["name"].as_str().ok_or_else(|| anyhow::anyhow!("name 필수"))?;
        let pattern = args["pattern"].as_str().ok_or_else(|| anyhow::anyhow!("pattern 필수"))?;
        let enabled = args["enabled"].as_bool().unwrap_or(true);
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        db.add_user_pii_pattern(name, pattern, enabled)?;
        Ok(serde_json::json!({ "ok": true, "name": name, "needs_restart": true }))
    }

    async fn handle_pii_pattern_remove(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let name = args["name"].as_str().ok_or_else(|| anyhow::anyhow!("name 필수"))?;
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let removed = db.remove_user_pii_pattern(name)?;
        Ok(serde_json::json!({ "removed": removed, "name": name, "needs_restart": true }))
    }

    // Ruflo A1: LLM 캐시 전체 삭제
    async fn handle_clear_llm_cache(&self) -> Result<serde_json::Value> {
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let deleted = db.clear_llm_cache()?;
        Ok(serde_json::json!({ "deleted": deleted }))
    }

    // Ruflo A1: LLM 캐시 통계
    async fn handle_get_llm_cache_stats(&self) -> Result<serde_json::Value> {
        let (entries, total_hits, avg_hits) = crate::settings_db::SettingsDb::open(&self.settings_db_path)
            .ok()
            .and_then(|db| db.llm_cache_stats().ok())
            .unwrap_or((0, 0, 0.0));
        Ok(serde_json::json!({
            "entries": entries,
            "total_hits": total_hits,
            "avg_hits_per_entry": avg_hits,
        }))
    }

    // Ruflo C1 1단계: 카운터 → decision_log 자동 추천
    async fn handle_auto_suggest_from_counters(&self) -> Result<serde_json::Value> {
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let inserted = crate::auto_suggester::suggest_from_counters(&db)?;
        Ok(serde_json::json!({
            "inserted": inserted,
            "next": "setup_decision_log_list로 source='auto_suggestion' 항목 검토",
        }))
    }

    // Ruflo C1 2단계: suggested → accepted (toml 적용)
    async fn handle_accept_suggested(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let decision_id = args["decision_id"].as_i64()
            .ok_or_else(|| anyhow::anyhow!("decision_id 필수"))?;
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let cfg_path = crate::config::find_config_path(None);
        let (path, after_value) = crate::auto_suggester::apply_suggested(&db, &cfg_path, decision_id)?;
        Ok(serde_json::json!({
            "applied": true,
            "path": path,
            "after_value": after_value,
            "backup": cfg_path.with_extension("toml.bak").to_string_lossy(),
        }))
    }

    // Ruflo C1 2단계: suggested → rejected (config 변경 없음)
    async fn handle_reject_suggested(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let decision_id = args["decision_id"].as_i64()
            .ok_or_else(|| anyhow::anyhow!("decision_id 필수"))?;
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        crate::auto_suggester::reject_suggested(&db, decision_id)?;
        Ok(serde_json::json!({ "rejected": true, "decision_id": decision_id }))
    }

    // Phase 82: Decision Log 조회
    async fn handle_setup_decision_log_list(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let entries = if let Some(snap_id) = args.get("snapshot_id").and_then(|v| v.as_str()) {
            db.list_decisions_by_snapshot(snap_id)?
        } else {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
            db.list_decisions(limit)?
        };
        Ok(serde_json::json!({
            "count": entries.len(),
            "entries": entries,
        }))
    }

    /// Phase 102: 메타 MCP 도구 — 비전문가 사용자를 위한 통합 "설정 최적화" 진입점
    ///
    /// 호출 1회로 다음 동작:
    /// 1. C1 누적 카운터 분석 (suggest_from_counters 호출 — 디폴트 활성)
    /// 2. 누적 진행률 + 임계값 진행 상태 보고
    /// 3. 누적된 추천 (자동 추천 + setup_review + setup_apply_modules) 통합 목록
    /// 4. 사용자 친화 next_actions 안내
    ///
    /// 본 도구는 **제안만** 반환 — 자동 적용 0건 (lesson 30 Ruflo 패턴 완전 준수).
    /// 사용자가 setup_apply / accept_suggested_decision으로 명시 적용.
    async fn handle_optimize(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let db = crate::settings_db::SettingsDb::open(&self.settings_db_path)?;
        let scenario = args.get("scenario").and_then(|v| v.as_str());

        // 1단계: C1 자동 분석 실행 (디폴트 true)
        let run_analysis = args.get("run_analysis").and_then(|v| v.as_bool()).unwrap_or(true);
        let mut newly_inserted = 0;
        if run_analysis {
            newly_inserted = crate::auto_suggester::suggest_from_counters(&db).unwrap_or(0);
        }

        // 2단계: 누적 진행률 측정
        let mode_rows = db.get_search_mode_counters().unwrap_or_default();
        let total_searches: u64 = mode_rows.iter().map(|(_, c, _)| *c).sum();
        let crag_rows = db.get_crag_counters().unwrap_or_default();
        let total_crag: u64 = crag_rows.iter().map(|(_, c, _)| *c).sum();
        let metric_summary = db.get_processing_metric_summary().ok();
        let processed_total = metric_summary.as_ref()
            .map(|s| s.success + s.errors)
            .unwrap_or(0);

        let mode_min_total = db.get_c1_threshold("mode_min_total", 100.0).unwrap_or(100.0) as u64;
        let crag_min_total = db.get_c1_threshold("crag_min_total", 50.0).unwrap_or(50.0) as u64;
        let processed_min = db.get_c1_threshold("processed_min", 30.0).unwrap_or(30.0) as u64;

        let progress = serde_json::json!({
            "search_count": { "current": total_searches, "threshold": mode_min_total, "ready": total_searches >= mode_min_total },
            "crag_count": { "current": total_crag, "threshold": crag_min_total, "ready": total_crag >= crag_min_total },
            "processed_count": { "current": processed_total, "threshold": processed_min, "ready": processed_total >= processed_min },
        });

        // 3단계: 누적 검토 대기 추천 (decision_log suggested)
        let all_decisions = db.list_decisions(200).unwrap_or_default();
        let pending: Vec<_> = all_decisions.iter()
            .filter(|d| d.decision == "suggested")
            .collect();

        // 4단계: 시나리오 기반 권고 (선택)
        let scenario_advice = if let Some(s) = scenario {
            if !s.is_empty() {
                let cfg_path = crate::config::find_config_path(None);
                let current = crate::config::PipelineConfig::load(&cfg_path)
                    .unwrap_or_else(|_| crate::config::PipelineConfig::default_config());
                Some(crate::setup_review::build_advice(s, None, &current))
            } else { None }
        } else { None };

        // 5단계: 비전문가 친화 next_actions 작성
        let any_ready = total_searches >= mode_min_total
            || total_crag >= crag_min_total
            || processed_total >= processed_min;

        let mut next_actions: Vec<String> = Vec::new();
        if !any_ready && pending.is_empty() && scenario_advice.is_none() {
            next_actions.push(format!(
                "아직 데이터가 부족합니다. inbox에 파일 더 투입 또는 검색 더 사용하세요. (현재: 검색 {}/{} · CRAG {}/{} · 가공 {}/{})",
                total_searches, mode_min_total,
                total_crag, crag_min_total,
                processed_total, processed_min,
            ));
            next_actions.push("scenario 인자로 사용자 시나리오 직접 입력 시 즉시 권고 가능 (예: \"회의록 위주 가공 중\")".to_string());
            next_actions.push("Settings → 운영 → 자동 추천 임계값 카드에서 임계값 낮춤 가능".to_string());
        }
        if !pending.is_empty() {
            next_actions.push(format!(
                "{}건 추천이 검토 대기 중. Dashboard Settings → 운영 → 자동 추천 카드에서 [적용] 또는 [거부] 클릭. 또는 MCP 도구 setup_apply (path 명시) 사용.",
                pending.len()
            ));
        }
        if let Some(adv) = &scenario_advice {
            if !adv.changes.is_empty() {
                next_actions.push(format!(
                    "시나리오 분석으로 {}건 추천. MCP 도구 setup_apply accepted_paths=[...] 호출로 적용 (변경 risk='critical'은 apply_critical=true 필수).",
                    adv.changes.len()
                ));
            }
        }
        if newly_inserted > 0 {
            next_actions.insert(0, format!("✨ 본 분석에서 신규 추천 {}건 추가됨", newly_inserted));
        }
        if next_actions.is_empty() {
            next_actions.push("현재 변경 권고 없음. 시스템이 안정 상태입니다.".to_string());
        }

        Ok(serde_json::json!({
            "progress": progress,
            "newly_inserted": newly_inserted,
            "pending_suggestions": {
                "count": pending.len(),
                "entries": pending,
            },
            "scenario_advice": scenario_advice,
            "next_actions": next_actions,
            "note": "본 도구는 제안만 반환합니다. 자동 적용은 없으며 사용자가 setup_apply / accept_suggested_decision으로 명시 적용해야 합니다.",
        }))
    }

    // ── Phase E (Grimoire 흡수, prd/research/external-analysis-2026-06-04-grimoire.md) ──

    /// E1: 코퍼스 라우팅용 목차 — doc_type 또는 hierarchy 기준 그룹 + 카운트.
    /// Claude가 search 호출 전 사전 라우팅에 활용.
    async fn handle_get_index(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let trace = file_pipeline_core::audit::TraceId::new();
        let group_by = args.get("group_by").and_then(|v| v.as_str()).unwrap_or("doc_type");
        let top_per_group = args.get("top_per_group").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

        let all = self.vector_db.list_all()?;
        let mut groups: std::collections::BTreeMap<String, Vec<&file_pipeline_core::domain::models::StoredDocSummary>> = std::collections::BTreeMap::new();

        for doc in &all {
            let keys: Vec<String> = match group_by {
                "date" => {
                    if doc.date.is_empty() {
                        vec!["(undated)".to_string()]
                    } else {
                        vec![doc.date.clone()]
                    }
                }
                _ => {
                    if doc.doc_types.is_empty() {
                        vec!["(untyped)".to_string()]
                    } else {
                        doc.doc_types.clone()
                    }
                }
            };
            for key in keys {
                groups.entry(key).or_default().push(doc);
            }
        }

        let summary: Vec<serde_json::Value> = groups.iter().map(|(key, docs)| {
            let top: Vec<serde_json::Value> = docs.iter()
                .take(top_per_group)
                .map(|d| serde_json::json!({
                    "id": d.id,
                    "path": d.path.to_string_lossy(),
                }))
                .collect();
            serde_json::json!({
                "group": key,
                "count": docs.len(),
                "top": top,
            })
        }).collect();

        let inputs_hash = file_pipeline_core::audit::input_hash_prefix(group_by.as_bytes());
        let out_summary = file_pipeline_core::audit::truncate_output_summary(
            &format!("groups={} total={}", groups.len(), all.len())
        );
        self.audit.record(trace.as_str(), "mcp.get_index", Some(&inputs_hash), Some(&out_summary), Some("success"));

        Ok(serde_json::json!({
            "group_by": group_by,
            "total_documents": all.len(),
            "groups": summary,
            "trace_id": trace.as_str(),
        }))
    }

    /// E3: cwd → project 추론. doc_types/hierarchy 매칭으로 관련 영역 제안.
    async fn handle_get_context(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let trace = file_pipeline_core::audit::TraceId::new();
        let cwd = args["cwd"].as_str().ok_or_else(|| anyhow::anyhow!("cwd 필수"))?;

        // 경로 마지막 컴포넌트를 키워드로 사용
        let cwd_path = std::path::Path::new(cwd);
        let last_component = cwd_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        let all = self.vector_db.list_all()?;
        // 경로/doc_type/hierarchy에 cwd 키워드 포함된 문서 점수
        let mut scored: Vec<(usize, &file_pipeline_core::domain::models::StoredDocSummary)> = all.iter()
            .map(|d| {
                let path_match = d.path.to_string_lossy().to_lowercase().contains(&last_component);
                let type_match = d.doc_types.iter().any(|t| t.to_lowercase().contains(&last_component));
                let score = (path_match as usize) * 3 + (type_match as usize) * 2;
                (score, d)
            })
            .filter(|(s, _)| *s > 0)
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));

        let suggested: Vec<serde_json::Value> = scored.iter().take(10).map(|(score, d)| {
            serde_json::json!({
                "id": d.id,
                "path": d.path.to_string_lossy(),
                "doc_types": d.doc_types,
                "score": score,
            })
        }).collect();

        let inputs_hash = file_pipeline_core::audit::input_hash_prefix(cwd.as_bytes());
        let out_summary = file_pipeline_core::audit::truncate_output_summary(
            &format!("cwd_keyword={} matches={}", last_component, scored.len())
        );
        self.audit.record(trace.as_str(), "mcp.get_context", Some(&inputs_hash), Some(&out_summary), Some("success"));

        Ok(serde_json::json!({
            "cwd": cwd,
            "cwd_keyword": last_component,
            "suggested_documents": suggested,
            "match_count": scored.len(),
            "trace_id": trace.as_str(),
        }))
    }

    /// E2: 분류규약 역매핑 노트 저장.
    /// 디폴트 비활성 — pipeline.toml `[grimoire].write_note_enabled=true` 필수 (현재는 항상 허용 — 인프라만).
    /// Phase E2는 인프라만. 실제 저장은 다음 phase에서 setup_rules 역매핑 통합.
    async fn handle_write_note(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let trace = file_pipeline_core::audit::TraceId::new();
        let title = args["title"].as_str().ok_or_else(|| anyhow::anyhow!("title 필수"))?;
        let content = args["content"].as_str().ok_or_else(|| anyhow::anyhow!("content 필수"))?;
        let note_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("note");
        let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("general");

        // 분류규약 역매핑: <domain>/<title-slug>.md 형태로 저장 위치 결정 (단순 휴리스틱)
        let slug = title.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let suggested_path = format!("{}/{}-{}.md", domain, note_type, slug);

        // Phase E2 인프라 — 실제 저장은 다음 phase. 현재는 dry-run 응답만.
        let inputs_hash = file_pipeline_core::audit::input_hash_prefix(title.as_bytes());
        let out_summary = file_pipeline_core::audit::truncate_output_summary(
            &format!("type={} domain={} suggested={}", note_type, domain, suggested_path)
        );
        self.audit.record(trace.as_str(), "mcp.write_note", Some(&inputs_hash), Some(&out_summary), Some("dry-run"));

        Ok(serde_json::json!({
            "status": "dry_run",
            "note": "Phase E2 인프라 — 실제 저장은 다음 phase. 분류규약 역매핑 결과만 반환.",
            "suggested_path": suggested_path,
            "type": note_type,
            "domain": domain,
            "title": title,
            "content_chars": content.chars().count(),
            "trace_id": trace.as_str(),
        }))
    }
}

/// dryrun용 in-memory 적용 — toml_edit과 동일 path를 PipelineConfig 직접 변경
fn simulate_apply(cfg: &mut crate::config::PipelineConfig, path: &str, value: &serde_json::Value) -> Result<()> {
    // serde_json::to_value → 변경 → from_value 반복은 비용이 크지만 dryrun은 1회 호출이므로 OK
    let mut v = serde_json::to_value(&*cfg)?;
    let parts: Vec<&str> = path.split('.').collect();
    let mut cur = &mut v;
    for p in &parts[..parts.len() - 1] {
        cur = cur.get_mut(*p).ok_or_else(|| anyhow::anyhow!("path 없음: {}", path))?;
    }
    if let Some(obj) = cur.as_object_mut() {
        obj.insert(parts.last().expect("non-empty").to_string(), value.clone());
        *cfg = serde_json::from_value(v).map_err(|e| anyhow::anyhow!("config 재구성 실패: {}", e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use file_pipeline_core::domain::models::{DbStats, DocRelation, Document, StoredDocSummary, SimilarDoc};
    use file_pipeline_core::ports::output::{StoragePort, VectorDBPort};

    // ── Stub VectorDB (빈 DB) ──────────────────────────────────

    struct StubVectorDb;

    impl VectorDBPort for StubVectorDb {
        fn init(&self) -> Result<()> { Ok(()) }

        fn upsert(&self, _doc: &Document) -> Result<()> { Ok(()) }

        fn search_similar(&self, _embedding: &[f32], _top_k: usize) -> Result<Vec<SimilarDoc>> {
            Ok(vec![])
        }

        fn find_by_hash(&self, _hash: &str) -> Result<Option<String>> { Ok(None) }

        fn find_by_type(&self, _doc_type: &str, _date: &str) -> Result<Option<String>> { Ok(None) }

        fn stats(&self) -> Result<DbStats> {
            Ok(DbStats {
                total_documents: 0,
                by_type: vec![],
                total_size_bytes: 0,
                sensitive_count: 0,
            })
        }

        fn list_all(&self) -> Result<Vec<StoredDocSummary>> { Ok(vec![]) }

        fn get_types(&self, _doc_id: &str) -> Result<Vec<String>> { Ok(vec![]) }

        fn update_types(&self, _doc_id: &str, _types: Vec<String>) -> Result<()> { Ok(()) }

        fn link(
            &self,
            _source_id: &str,
            _target_id: &str,
            _relation: file_pipeline_core::domain::models::RelationType,
        ) -> Result<()> {
            Ok(())
        }

        fn find_related(&self, _doc_id: &str) -> Result<Vec<DocRelation>> { Ok(vec![]) }

        fn update_content(&self, _doc_id: &str, _new_content: &str, _change_summary: &str) -> Result<()> {
            Ok(())
        }
    }

    // ── Stub Storage ───────────────────────────────────────────

    struct StubStorage;

    impl StoragePort for StubStorage {
        fn compress_and_store(&self, _source: &std::path::Path, _dest_dir: &std::path::Path) -> Result<std::path::PathBuf> {
            Ok(std::path::PathBuf::from("/tmp/stub.zst"))
        }

        fn decompress_temp(&self, _compressed: &std::path::Path) -> Result<std::path::PathBuf> {
            Ok(std::path::PathBuf::from("/tmp/stub.txt"))
        }

        fn read_header(&self, _compressed: &std::path::Path, _lines: usize) -> Result<String> {
            Ok(String::new())
        }
    }

    fn make_mcp_state() -> McpState {
        McpState {
            vector_db: Arc::new(StubVectorDb),
            storage: Arc::new(StubStorage),
            embedding: Arc::new(file_pipeline_adapters::stub::StubEmbedder::new(4)),
            llm: Arc::new(file_pipeline_adapters::stub::StubLlm),
            reranker: Arc::new(file_pipeline_adapters::driven::reranking::null_reranker::NullReranker),
            settings_db_path: std::path::PathBuf::from(":memory:"),
            search_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
            search_log: std::sync::Mutex::new(Vec::new()),
            search_mode_counts: std::sync::Mutex::new(std::collections::HashMap::new()),
            crag_counts: std::sync::Mutex::new(std::collections::HashMap::new()),
            expand_kg_hops: 0,
            diversity_threshold: 0,
            hyde_enabled: false,
            hyde_min_results: 3,
            output_pii_mask: true,
            pii_user_patterns: Vec::new(),
            audit: std::sync::Arc::new(file_pipeline_core::ports::output::NullAuditAdapter),
            tfidf_rerank_enabled: false,
            kg_beam_search: false,
        }
    }

    #[test]
    fn test_mcp_state_creation() {
        let _state = make_mcp_state();
    }

    // Phase 91 B2: mutates_state 메타데이터 테스트
    #[test]
    fn test_mcp_tool_mutates_state_read_only() {
        assert!(!mcp_tool_mutates_state("search"));
        assert!(!mcp_tool_mutates_state("get_document"));
        assert!(!mcp_tool_mutates_state("setup_review"));
        assert!(!mcp_tool_mutates_state("setup_dryrun"));
    }

    #[test]
    fn test_mcp_tool_mutates_state_writers() {
        assert!(mcp_tool_mutates_state("setup_apply"));
        assert!(mcp_tool_mutates_state("setup_apply_modules"));
        assert!(mcp_tool_mutates_state("complete_todo"));
        assert!(mcp_tool_mutates_state("revise_topic"));
        assert!(mcp_tool_mutates_state("setup_snapshot_rollback"));
    }

    #[test]
    fn test_mcp_tool_catalog_consistency() {
        // 카탈로그의 각 항목이 mcp_tool_mutates_state와 일치 (메타 룰 1 자기 적용)
        for (name, mutates) in mcp_tool_catalog() {
            assert_eq!(
                mcp_tool_mutates_state(name),
                mutates,
                "카탈로그 mismatch: {} expected mutates={}",
                name, mutates,
            );
        }
    }

    #[test]
    fn test_mcp_tool_catalog_has_writers() {
        let catalog = mcp_tool_catalog();
        let n_writers = catalog.iter().filter(|(_, m)| *m).count();
        assert!(n_writers >= 4, "쓰기 도구 4건 이상 등록되어야 함 (실측: {})", n_writers);
    }

    // Phase E (Grimoire 흡수)
    #[test]
    fn test_mcp_tool_catalog_grimoire_tools_registered() {
        let catalog = mcp_tool_catalog();
        let names: Vec<&str> = catalog.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"get_index"), "E1 get_index 등록 의무");
        assert!(names.contains(&"get_context"), "E3 get_context 등록 의무");
        assert!(names.contains(&"write_note"), "E2 write_note 등록 의무");
    }

    #[test]
    fn test_mcp_tool_catalog_count_includes_grimoire() {
        let catalog = mcp_tool_catalog();
        // Phase E 진입 전 25 → 진입 후 28
        assert!(catalog.len() >= 28, "Phase E 진입 후 28+ 도구 (실측: {})", catalog.len());
    }

    #[test]
    fn test_grimoire_get_index_read_only() {
        assert!(!mcp_tool_mutates_state("get_index"));
        assert!(!mcp_tool_mutates_state("get_context"));
    }

    #[test]
    fn test_grimoire_write_note_mutates() {
        assert!(mcp_tool_mutates_state("write_note"));
    }

    // Phase 92 H3: 다차원 분류 테스트
    #[test]
    fn test_mcp_tool_catalog_full_consistency() {
        // mcp_tool_catalog (단일 차원) ↔ mcp_tool_catalog_full (다차원) 일치성 검증
        // 메타 룰 1 자기 적용 (다중 위치 동기화 누락 회피)
        let single = mcp_tool_catalog();
        let full = mcp_tool_catalog_full();
        assert_eq!(single.len(), full.len(), "두 카탈로그 길이 일치 의무");
        for (s, f) in single.iter().zip(full.iter()) {
            assert_eq!(s.0, f.name, "이름 일치");
            assert_eq!(s.1, f.mutates, "mutates 일치");
        }
    }

    #[test]
    fn test_mcp_tool_categories_distribution() {
        let full = mcp_tool_catalog_full();
        let n_search = full.iter().filter(|m| m.category == McpToolCategory::Search).count();
        let n_settings = full.iter().filter(|m| m.category == McpToolCategory::Settings).count();
        let n_kg = full.iter().filter(|m| m.category == McpToolCategory::Kg).count();
        assert!(n_search >= 3, "검색 카테고리 3건 이상");
        assert!(n_settings >= 3, "설정 카테고리 3건 이상");
        assert_eq!(n_kg, 3, "KG 카테고리 정확히 3건");
    }

    #[test]
    fn test_mcp_tool_costs() {
        let full = mcp_tool_catalog_full();
        // revise_topic은 LLM 호출 동반
        let revise = full.iter().find(|m| m.name == "revise_topic").expect("revise_topic 등록");
        assert_eq!(revise.cost, McpToolCost::LlmCall);
        // search는 heavy compute (벡터 + 리랭킹)
        let search = full.iter().find(|m| m.name == "search").expect("search 등록");
        assert_eq!(search.cost, McpToolCost::HeavyCompute);
    }

    #[test]
    fn test_mcp_tool_category_strings() {
        assert_eq!(McpToolCategory::Search.as_str(), "search");
        assert_eq!(McpToolCategory::Kg.as_str(), "kg");
        assert_eq!(McpToolCategory::Settings.as_str(), "settings");
    }

    #[tokio::test]
    async fn test_handle_search_empty_db() {
        let state = make_mcp_state();
        let args = serde_json::json!({ "query": "test query", "top_k": 5 });
        let result = state.handle_search(&args).await.expect("handle_search 실패");

        assert_eq!(result["total"], 0);
        let results = result["results"].as_array().expect("results는 배열이어야 함");
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_handle_stats() {
        let state = make_mcp_state();
        let result = state.handle_stats().await.expect("handle_stats 실패");

        assert!(result.get("total_documents").is_some());
        assert!(result.get("by_type").is_some());
        assert_eq!(result["total_documents"], 0);
    }

    #[test]
    fn test_ruflo_a2_b1_defaults_disabled() {
        // 기본 McpState는 expand_kg_hops=0, diversity_threshold=0 — 둘 다 비활성
        let state = make_mcp_state();
        assert_eq!(state.expand_kg_hops, 0, "기본은 KG hop 비활성");
        assert_eq!(state.diversity_threshold, 0, "기본은 다양성 강화 비활성");
    }

    #[test]
    fn test_hyde_defaults_disabled() {
        // 트리거 #6 인프라: 기본은 비활성, 임계값 3
        let state = make_mcp_state();
        assert!(!state.hyde_enabled, "HyDE 디폴트는 비활성 (lesson 30 패턴)");
        assert_eq!(state.hyde_min_results, 3, "HyDE 발동 임계 디폴트는 3");
    }

    #[tokio::test]
    async fn test_hyde_disabled_no_extra_search() {
        // HyDE 비활성 상태에선 빈 결과여도 generate_hypothetical 영향 없어야 함
        let state = make_mcp_state();
        let args = serde_json::json!({ "query": "non-existent topic", "top_k": 5 });
        let result = state.handle_search(&args).await.expect("handle_search");
        // 비활성 상태에선 빈약 결과여도 폴백 없음 → 결과 0
        assert_eq!(result["total"], 0);
    }
}
