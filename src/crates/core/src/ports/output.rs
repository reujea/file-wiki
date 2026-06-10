use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::models::{
    ClassifyAndProcessResult, DbStats, DocRelation, DocTypeRegistry, Document,
    EnrichResult, PreprocessResult, ProcessingSummary, RelationType, SimilarDoc,
    StoredDocSummary,
};

/// Phase 94 A3: 감사 추적 포트 (audit_trace 단일 키 기록).
///
/// JAMES 자체 진화 게이트 흡수 (Phase 91 lesson 50) — RBAC 없는 결정 추적.
/// 헥사고날 코어가 settings.db에 직접 의존하지 않도록 본 포트로 분리.
///
/// 어댑터 구현:
/// - `NullAuditAdapter`: 디폴트 no-op (lesson 14 회피 — 호출처가 어댑터 부재 시에도 동작)
/// - `SettingsAuditAdapter`: settings.db `audit_trace` 테이블에 기록
pub trait AuditPort: Send + Sync {
    /// 1줄 결정 기록. 실패는 silent (LLM/검색 본 흐름을 막지 않음).
    ///
    /// Arguments:
    /// - `trace_id`: 결정 단위 식별자 (UUID-like, `TraceId::new()`)
    /// - `stage`: "llm.classify" / "search.hybrid" / "mcp.search" / "verify.run" 등
    /// - `inputs_hash`: 입력 SHA-256 16자 prefix (`input_hash_prefix`)
    /// - `output_summary`: 결과 요약 (200자 cap, `truncate_output_summary`)
    /// - `applied_rule`: 적용된 규칙 / 실패 사유 / "success"
    fn record(
        &self,
        trace_id: &str,
        stage: &str,
        inputs_hash: Option<&str>,
        output_summary: Option<&str>,
        applied_rule: Option<&str>,
    );
}

/// 디폴트 no-op 어댑터. lesson 14 "미연결 포트는 코드 부담" 회피.
pub struct NullAuditAdapter;

impl AuditPort for NullAuditAdapter {
    fn record(&self, _trace_id: &str, _stage: &str, _inputs_hash: Option<&str>, _output_summary: Option<&str>, _applied_rule: Option<&str>) {}
}

/// LLM 가공 포트 — 분류+가공 통합
#[async_trait]
pub trait LLMPort: Send + Sync {
    /// 파일을 분류하고 유형별 형식으로 가공 (단일 호출)
    async fn classify_and_process(
        &self,
        file_path: &Path,
        registry: &DocTypeRegistry,
    ) -> Result<ClassifyAndProcessResult>;

    /// 텍스트 요약/병합 (범용)
    async fn summarize_text(&self, new_content: &str, existing: &str) -> Result<String> {
        Ok(format!("{}\n{}", existing, new_content))
    }

    /// HyDE 폴백용 가상 답변 생성 — 트리거 #6 인프라 (디폴트 비활성, lesson 30 패턴).
    ///
    /// 동작: 주어진 query에 대한 가상 답변을 한 문단(2~4문장)으로 생성. 검색에선
    /// 이 답변을 임베딩하여 dense 검색에 활용. 디폴트 구현은 query 자체를 반환 (no-op).
    /// 실 LLM 어댑터(claude_cli/anthropic/openai/gemini)에서 의미 있게 오버라이드.
    async fn generate_hypothetical(&self, query: &str) -> Result<String> {
        Ok(query.to_string())
    }

    /// 검증 실패 피드백을 포함하여 재가공 (Phase 5: 2-Pass)
    async fn reprocess_with_feedback(
        &self,
        file_path: &Path,
        registry: &DocTypeRegistry,
        _feedback: &str,
    ) -> Result<ClassifyAndProcessResult> {
        self.classify_and_process(file_path, registry).await
    }

    /// 전처리된 텍스트를 직접 분류+가공 (파이프라인 모드)
    /// 기본 구현: 임시 파일에 쓴 뒤 classify_and_process 위임
    async fn classify_and_process_text(
        &self,
        file_name: &str,
        text: &str,
        registry: &DocTypeRegistry,
    ) -> Result<ClassifyAndProcessResult> {
        let tmp_dir = std::env::temp_dir().join("file-pipeline-preprocess");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let tmp_path = tmp_dir.join(file_name).with_extension("txt");
        std::fs::write(&tmp_path, text)?;
        let result = self.classify_and_process(&tmp_path, registry).await;
        let _ = std::fs::remove_file(&tmp_path);
        result
    }

    /// 기존 문서를 새 정보로 보강 (Phase C: 누적 업데이트)
    async fn enrich_existing(
        &self,
        existing_content: &str,
        new_info: &str,
        doc_types: &[String],
    ) -> Result<EnrichResult>;
}

/// Phase 92 H5: 원격 저장소 capability 메타데이터 (Mirage Resource 패턴 흡수).
///
/// 모든 어댑터가 동일한 5 메서드(upload/download/list/delete/is_configured)를 구현하지만
/// 실제 지원 범위가 다르다 (예: NotionStorageAdapter는 attach 모드에서 upload bail).
/// 본 구조체로 호출자가 사전에 capability 확인 가능.
///
/// Mirage 흡수 결정 (lesson 51, 메타 룰 16 차원 B 🟡):
/// - VFS 전체 추상화는 본질 도메인 불일치 (🔴 보류)
/// - mount tree / bash 명령 인터페이스는 보류 (🔴)
/// - **capability 메타데이터 표준화만 흡수** (🟡 → 🟢)
#[derive(Debug, Clone)]
pub struct ResourceCapabilities {
    /// 어댑터 이름 (예: "s3" / "webdav" / "network" / "notion" / "null")
    pub backend: &'static str,
    /// upload 메서드가 실제 동작하는가 (Notion attach는 bail이므로 false 가능)
    pub can_upload: bool,
    /// download 메서드가 실제 동작하는가
    pub can_download: bool,
    /// list 메서드가 실제 동작하는가
    pub can_list: bool,
    /// delete 메서드가 실제 동작하는가 (Notion은 archived=true PATCH, hard delete 미지원)
    pub can_delete: bool,
    /// 모드 분기 옵션 (Notion=["page","attach"], 다른 어댑터는 빈 배열)
    pub mode_options: &'static [&'static str],
    /// 현재 활성 모드 (mode_options 중 하나, 없으면 빈 문자열)
    pub active_mode: String,
    /// hard delete 지원 여부 (false면 archive/soft delete만)
    pub supports_hard_delete: bool,
}

impl ResourceCapabilities {
    /// 디폴트 capability — 모든 메서드 동작 + 모드 없음.
    /// S3/WebDAV/Network 같은 표준 백엔드용.
    pub fn standard(backend: &'static str) -> Self {
        Self {
            backend,
            can_upload: true,
            can_download: true,
            can_list: true,
            can_delete: true,
            mode_options: &[],
            active_mode: String::new(),
            supports_hard_delete: true,
        }
    }

    /// Null capability — 모든 메서드 no-op. NullRemoteStorage용.
    pub fn null() -> Self {
        Self {
            backend: "null",
            can_upload: false,
            can_download: false,
            can_list: false,
            can_delete: false,
            mode_options: &[],
            active_mode: String::new(),
            supports_hard_delete: false,
        }
    }
}

/// 원격 저장소 포트 (S3/WebDAV/네트워크/Notion)
#[async_trait]
pub trait RemoteStoragePort: Send + Sync {
    /// 로컬 파일을 원격 저장소에 업로드
    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()>;
    /// 원격 파일을 로컬에 다운로드
    async fn download(&self, remote_key: &str, local_path: &Path) -> Result<()>;
    /// 원격 저장소의 파일 목록 조회
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;
    /// 원격 파일 삭제
    async fn delete(&self, remote_key: &str) -> Result<()>;
    /// 원격 저장소가 설정되어 있는지
    fn is_configured(&self) -> bool;

    /// Phase 92 H5: 어댑터의 capability 메타데이터.
    ///
    /// 디폴트 구현은 `ResourceCapabilities::standard("unknown")` — 어댑터별로 오버라이드 권장.
    /// 호출자(GUI/MCP)가 사전에 지원 범위 확인 가능 — 예: Notion attach 모드의 upload를
    /// 시도하기 전에 `capabilities().can_upload`로 차단.
    ///
    /// Mirage Resource 패턴 흡수 (lesson 51). 디폴트 메서드라 기존 어댑터 호환성 유지.
    fn capabilities(&self) -> ResourceCapabilities {
        ResourceCapabilities::standard("unknown")
    }
}

/// 압축 스토리지 포트
pub trait StoragePort: Send + Sync {
    /// 파일을 zstd 압축하여 dest_dir에 저장, 압축 파일 경로 반환
    fn compress_and_store(&self, source: &Path, dest_dir: &Path) -> Result<std::path::PathBuf>;

    /// zstd 레벨 오버라이드로 압축 (기본 구현: level 무시하고 compress_and_store 호출)
    fn compress_with_level(&self, source: &Path, dest_dir: &Path, _level: i32) -> Result<std::path::PathBuf> {
        self.compress_and_store(source, dest_dir)
    }

    /// 압축 파일을 임시 디렉토리에 해제, 임시 파일 경로 반환
    fn decompress_temp(&self, compressed: &Path) -> Result<std::path::PathBuf>;

    /// 압축 파일의 헤더만 부분 해제 (지정 줄 수까지)
    fn read_header(&self, compressed: &Path, lines: usize) -> Result<String>;
}

/// 벡터 DB 포트
pub trait VectorDBPort: Send + Sync {
    /// DB 초기화 (컬렉션/테이블 생성)
    fn init(&self) -> Result<()>;

    /// 배치 모드 시작 — persist를 지연하여 성능 최적화
    fn batch_begin(&self) {}

    /// 배치 모드 종료 + flush — 지연된 persist 실행
    fn batch_end(&self) {}

    /// 증분 flush 필요 여부 (동적 임계치 기반)
    fn should_incremental_flush(&self) -> bool { false }

    /// flushed_embeddings 반환 (refresh 전 검색 대상 확장용)
    fn get_flushed_embeddings(&self) -> Vec<(String, Vec<f32>)> { vec![] }

    /// flushed_embeddings에 추가
    fn add_flushed_embedding(&self, _doc_id: &str, _embedding: &[f32]) {}

    /// DB refresh: mmap + HNSW 재빌드 + flushed_embeddings 초기화
    fn db_refresh(&self) {}

    /// flushed_embeddings 수
    fn flushed_count(&self) -> usize { 0 }

    /// pending + flushed가 있는지
    fn has_pending_work(&self) -> bool { false }

    /// MinHash LSH 후보 조회 (활성화 시 키워드 자카드 유사 문서만 반환)
    fn minhash_candidates(&self, keywords: &[String]) -> Vec<String> { let _ = keywords; vec![] }

    /// MinHash LSH 활성화 여부 (force=true 시 강제 활성, min_docs는 자동 활성 임계치)
    fn minhash_enabled_with(&self, force: bool, min_docs: usize) -> bool {
        let _ = (force, min_docs);
        false
    }

    /// 총 문서 수 (minhash 자동 활성 판단용)
    fn doc_count(&self) -> usize { 0 }

    /// 문서 색인 (upsert)
    fn upsert(&self, doc: &Document) -> Result<()>;

    /// 유사 문서 검색
    fn search_similar(&self, embedding: &[f32], top_k: usize) -> Result<Vec<SimilarDoc>>;

    /// SHA-256 해시로 문서 검색
    fn find_by_hash(&self, hash: &str) -> Result<Option<String>>;

    /// 문서 유형 + 날짜로 검색 (doc_types 배열에서 contains 매칭)
    fn find_by_type(&self, doc_type: &str, date: &str) -> Result<Option<String>>;

    /// DB 통계
    fn stats(&self) -> Result<DbStats>;

    /// 전체 문서 목록 (재색인용)
    fn list_all(&self) -> Result<Vec<StoredDocSummary>>;

    /// 특정 문서의 유형 목록 조회
    fn get_types(&self, doc_id: &str) -> Result<Vec<String>>;

    /// 특정 문서의 유형 업데이트
    fn update_types(&self, doc_id: &str, types: Vec<String>) -> Result<()>;

    // ── 관계 그래프 (Phase B) ───────────────────────────────

    /// 두 문서 간 관계 생성 (origin=AutoSimilarity)
    fn link(&self, source_id: &str, target_id: &str, relation: RelationType) -> Result<()>;

    /// Phase 83: origin 명시 관계 생성. 기본 구현은 link() 위임 (origin 정보 손실).
    /// 어댑터에서 origin 저장이 필요하면 override.
    fn link_with_origin(
        &self,
        source_id: &str,
        target_id: &str,
        relation: RelationType,
        origin: crate::domain::models::RelationOrigin,
    ) -> Result<()> {
        let _ = origin;
        self.link(source_id, target_id, relation)
    }

    /// 특정 문서의 관련 문서 조회
    fn find_related(&self, doc_id: &str) -> Result<Vec<DocRelation>>;

    /// 문서 삭제 (DB에서 제거)
    fn delete(&self, doc_id: &str) -> Result<()> {
        let _ = doc_id;
        Ok(())
    }

    /// 특정 문서의 가공본 내용 업데이트 (Phase C)
    fn update_content(&self, doc_id: &str, new_content: &str, change_summary: &str) -> Result<()>;

    /// 특정 문서의 벡터 조회 (backfill-vec용)
    fn get_vector(&self, doc_id: &str) -> Result<Option<Vec<f32>>> {
        let _ = doc_id;
        Ok(None)
    }

    /// 특정 문서의 키워드 조회 (backfill-sparse용)
    fn get_keywords(&self, doc_id: &str) -> Result<Vec<String>> {
        let _ = doc_id;
        Ok(vec![])
    }

    /// Phase 89 N-4: 특정 문서의 메타데이터 조회 (UI 노출용).
    /// needs_verification / open_questions / search_hints / entities 등 보조 필드 접근.
    /// 기본 구현은 None — 어댑터에서 override.
    fn get_metadata(&self, doc_id: &str) -> Result<Option<crate::domain::models::Metadata>> {
        let _ = doc_id;
        Ok(None)
    }

    /// Phase 89 #10 인프라: Sparse 임베딩 영속화 (BGE-M3 lexical, 트리거 대기).
    /// 디폴트 no-op — 어댑터에서 override 시 sparse_index에 저장.
    fn upsert_sparse_embedding(
        &self,
        _doc_id: &str,
        _sparse: &crate::domain::models::SparseEmbedding,
    ) -> Result<()> {
        Ok(())
    }

    /// Phase 89 #10 인프라: Sparse-only 검색 (dot product top-k).
    /// 디폴트 빈 결과 — search_hybrid에서 활성화 시 dense + sparse RRF 결합.
    fn search_sparse(
        &self,
        _sparse: &crate::domain::models::SparseEmbedding,
        _top_k: usize,
    ) -> Result<Vec<SimilarDoc>> {
        Ok(vec![])
    }

    /// Sparse 인덱스 활성 여부 (디폴트 false).
    fn sparse_enabled(&self) -> bool { false }

    /// sparse vector 단독 업데이트 (backfill-sparse용)
    fn upsert_sparse(&self, _doc_id: &str, _keywords: &[String]) -> Result<()> {
        Ok(())
    }

    /// 엔티티 저장 (upsert: 동일 id면 doc_ids/mention_count 누적)
    fn upsert_entity(&self, entity: &crate::domain::models::Entity) -> Result<()> {
        let _ = entity;
        Ok(())
    }

    /// 엔티티 목록 조회
    fn list_entities(&self) -> Result<Vec<crate::domain::models::Entity>> {
        Ok(vec![])
    }

    /// 특정 문서의 엔티티 조회
    fn entities_for_doc(&self, doc_id: &str) -> Result<Vec<crate::domain::models::Entity>> {
        let _ = doc_id;
        Ok(vec![])
    }

    /// 전체 임베딩의 읽기 전용 스냅샷 (행렬 곱 flush용)
    fn embedding_snapshot(&self) -> Result<crate::domain::models::EmbeddingSnapshot> {
        Ok(crate::domain::models::EmbeddingSnapshot { data: vec![], ids: vec![], dim: 0 })
    }

    /// 하이브리드 검색: 벡터 유사도 + 키워드 필터 (기본 구현: 벡터만)
    fn search_hybrid(&self, embedding: &[f32], keyword: &str, top_k: usize) -> Result<Vec<SimilarDoc>> {
        // 기본: 벡터 검색 후 키워드 필터링
        let results = self.search_similar(embedding, top_k * 3)?;
        let kw_lower = keyword.to_lowercase();
        Ok(results.into_iter()
            .filter(|d| {
                d.doc_types.iter().any(|t| t.to_lowercase().contains(&kw_lower))
                || d.date.contains(&kw_lower)
            })
            .take(top_k)
            .collect())
    }
}

/// 비텍스트 전처리 포트 (PDF/이미지→텍스트 변환)
pub trait PreprocessPort: Send + Sync {
    /// 파일을 텍스트로 변환
    fn preprocess(&self, file_path: &Path) -> Result<PreprocessResult>;

    /// 파이프라인 스텝 config로 전처리 (pdf_tool/ocr_tool 오버라이드)
    fn preprocess_with_config(&self, file_path: &Path, pdf_tool: &str, ocr_tool: &str) -> Result<PreprocessResult> {
        let _ = (pdf_tool, ocr_tool);
        self.preprocess(file_path)
    }

    /// 해당 확장자를 지원하는지
    fn supports(&self, extension: &str) -> bool;
}

/// 임베딩 포트
#[async_trait]
pub trait EmbeddingPort: Send + Sync {
    /// 벡터 차원 수
    fn dim(&self) -> usize;

    /// 단일 텍스트 임베딩
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// 모델 오버라이드 임베딩 (기본 구현: model 무시하고 embed 호출)
    async fn embed_with_model(&self, text: &str, _model: &str) -> Result<Vec<f32>> {
        self.embed(text).await
    }

    /// 배치 임베딩
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// ColBERT 토큰별 임베딩 (기본 구현: 미지원)
    /// 반환: Vec<Vec<f32>> — 각 토큰의 임베딩 벡터 (token_count × dim)
    async fn embed_colbert(&self, _text: &str) -> Result<Vec<Vec<f32>>> {
        anyhow::bail!("ColBERT 임베딩 미지원")
    }

    /// ColBERT 지원 여부
    fn supports_colbert(&self) -> bool { false }

    /// Phase 89 #10 인프라: Sparse(BGE-M3 lexical) 임베딩 (기본 구현: 미지원).
    /// 활성 어댑터(FastEmbedSparseAdapter)는 SparseEmbedding 반환.
    /// LocalVectorStore가 sparse_index에 영속화 (트리거 대기 — 디폴트 비활성).
    async fn embed_sparse(&self, _text: &str) -> Result<crate::domain::models::SparseEmbedding> {
        anyhow::bail!("sparse 임베딩 미지원")
    }

    /// Sparse 지원 여부 (디폴트 false).
    fn supports_sparse(&self) -> bool { false }
}

/// 알림 포트
#[async_trait]
pub trait NotificationPort: Send + Sync {
    /// 일반 알림
    async fn send(&self, title: &str, body: &str, level: &str) -> Result<()>;

    /// 중복 탐지 알림
    async fn send_duplicate_alert(
        &self,
        filename: &str,
        reason: &str,
        diff_summary: &str,
    ) -> Result<()>;

    /// 민감 파일 알림
    async fn send_sensitive_alert(&self, filename: &str, reason: &str) -> Result<()>;

    /// 처리 완료 알림
    async fn send_completion(
        &self,
        filename: &str,
        doc_type: &str,
        stats: &DbStats,
    ) -> Result<()>;

    /// 배치 처리 요약 알림 (대시보드 형태)
    async fn send_summary(&self, summary: &ProcessingSummary) -> Result<()>;
}

/// LLM 기반 검증 포트 (선택적)
#[async_trait]
pub trait VerificationPort: Send + Sync {
    /// 환각 탐지: (score 0~1, 설명)
    async fn detect_hallucination(
        &self,
        original: &str,
        processed: &str,
        doc_type: &str,
    ) -> Result<(f64, String)>;

    /// 완전성 확인: (score 0~1, 설명)
    async fn verify_completeness(
        &self,
        original: &str,
        processed: &str,
        doc_type: &str,
    ) -> Result<(f64, String)>;
}

/// 검색 결과 리랭킹 포트
#[async_trait]
pub trait RerankerPort: Send + Sync {
    /// 쿼리와 후보 문서 목록을 받아 관련도 순으로 재정렬
    async fn rerank(&self, query: &str, candidates: Vec<SimilarDoc>) -> Result<Vec<SimilarDoc>>;
    /// 리랭커가 활성화되어 있는지
    fn is_enabled(&self) -> bool;
}

/// Phase 82-prep: 처리 메트릭 기록 포트.
///
/// FileProcessingService가 record_success / record_error / quarantine / 시간 측정 시점에
/// 호출. 동기 + 빠른 DB 증분만 — 실패 시 silent (조용히 무시, 처리 흐름 영향 없음).
///
/// 어댑터는 shared/settings_db의 누적 카운터에 영속화. McpState는 같은 DB를 조회해
/// `verify_pass_rate` / `quarantine_rate` / `avg_process_time_ms`를 산출.
pub trait ProcessingMetricsPort: Send + Sync {
    /// 가공 성공 (verify 통과 또는 verify 비활성)
    fn record_success(&self) {}
    /// 가공 실패
    fn record_error(&self) {}
    /// quarantine 이동 (2-Pass 모두 실패)
    fn record_quarantine(&self) {}
    /// verify 결과 (true=pass, false=fail)
    fn record_verify(&self, _passed: bool) {}
    /// 단일 파일 처리 소요시간 (ms). avg 산출용 누적.
    fn record_process_time(&self, _elapsed_ms: u64) {}
}
