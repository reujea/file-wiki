use anyhow::Result;

use super::models::{LintIssue, LintIssueType, LintReport};
use super::verification::detect_strong_claims;
use crate::ports::output::{StoragePort, VectorDBPort};

/// 지식기반 품질 검사
pub struct Linter;

impl Linter {
    pub fn lint(vector_db: &dyn VectorDBPort) -> Result<LintReport> {
        let mut report = LintReport::default();
        let all_docs = vector_db.list_all()?;

        for doc in &all_docs {
            // 고아 문서: 관계가 0개
            let relations = vector_db.find_related(&doc.id)?;
            if relations.is_empty() {
                report.orphan_docs.push(doc.id.clone());
                report.issues.push(LintIssue {
                    doc_id: doc.id.clone(),
                    issue_type: LintIssueType::Orphan,
                    description: format!("관계 없는 고아 문서: {:?}", doc.path),
                });
            }

            // 유형 없는 문서
            let types = vector_db.get_types(&doc.id)?;
            if types.is_empty() {
                report.issues.push(LintIssue {
                    doc_id: doc.id.clone(),
                    issue_type: LintIssueType::DuplicateTopic,
                    description: "유형 없는 문서".into(),
                });
            }

        }

        // 누락된 백링크: A→B 관계가 있는데 B→A가 없는 경우
        for doc in &all_docs {
            let relations = vector_db.find_related(&doc.id)?;
            for rel in &relations {
                let reverse = vector_db.find_related(&rel.target_id)?;
                let has_backlink = reverse.iter().any(|r| r.target_id == doc.id);
                if !has_backlink {
                    report.issues.push(LintIssue {
                        doc_id: doc.id.clone(),
                        issue_type: LintIssueType::MissingBacklink,
                        description: format!(
                            "{} → {} 관계에 역방향 백링크 없음",
                            doc.id, rel.target_id
                        ),
                    });
                }
            }
        }

        Ok(report)
    }

    /// 토픽 디렉토리에서 모순 마크 탐지
    pub fn lint_topics(topics_dir: &std::path::Path) -> Result<Vec<LintIssue>> {
        let mut issues = Vec::new();

        if !topics_dir.exists() {
            return Ok(issues);
        }

        fn scan_dir(dir: &std::path::Path, issues: &mut Vec<LintIssue>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        scan_dir(&path, issues);
                    } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            for (i, line) in content.lines().enumerate() {
                                if line.contains("⚠️") && line.to_lowercase().contains("모순") {
                                    issues.push(LintIssue {
                                        doc_id: path.to_string_lossy().to_string(),
                                        issue_type: LintIssueType::Contradiction,
                                        description: format!("{}줄: {}", i + 1, line.trim()),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        scan_dir(topics_dir, &mut issues);
        Ok(issues)
    }

    /// Phase 88 (wikidocs 353407 근거 점검): 가공본에서 단정 표현을 검출하여 약화 권고.
    ///
    /// 각 문서의 가공본을 storage에서 복원 → `detect_strong_claims` 호출 → LintIssue 생성.
    /// 디폴트로 매 문서마다 최대 5개 강한 주장만 보고(긴 문서 대응).
    /// 호출 비용: O(N * 평균 문서 크기). 대용량 코퍼스에선 lint_weekly_hours에서 호출 권장.
    pub fn lint_strong_claims(
        vector_db: &dyn VectorDBPort,
        storage: &dyn StoragePort,
        max_per_doc: usize,
    ) -> Result<Vec<LintIssue>> {
        let mut issues = Vec::new();
        let all_docs = vector_db.list_all()?;

        for doc in &all_docs {
            // 가공본 본문을 storage에서 복원 (zstd 압축 해제)
            let temp_path = match storage.decompress_temp(&doc.path) {
                Ok(p) => p,
                Err(_) => continue, // 가공본 없거나 손상 → 스킵
            };
            let content = std::fs::read_to_string(&temp_path).unwrap_or_default();
            let _ = std::fs::remove_file(&temp_path);

            if content.is_empty() {
                continue;
            }

            let hits = detect_strong_claims(&content);
            for snippet in hits.into_iter().take(max_per_doc.max(1)) {
                issues.push(LintIssue {
                    doc_id: doc.id.clone(),
                    issue_type: LintIssueType::StrongClaim,
                    description: snippet,
                });
            }
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::*;
    use crate::ports::output::VectorDBPort;
    use std::path::PathBuf;

    /// 테스트용 VectorDB Stub
    struct LintTestVectorDb {
        docs: Vec<StoredDocSummary>,
        relations: std::collections::HashMap<String, Vec<DocRelation>>,
        types: std::collections::HashMap<String, Vec<String>>,
    }

    impl LintTestVectorDb {
        fn new() -> Self {
            Self { docs: vec![], relations: std::collections::HashMap::new(), types: std::collections::HashMap::new() }
        }

        fn add_doc(&mut self, id: &str, date: &str, doc_types: Vec<String>) {
            self.docs.push(StoredDocSummary {
                id: id.into(), path: PathBuf::from(format!("{}.zst", id)),
                doc_types: doc_types.clone(), date: date.into(),
            });
            self.types.insert(id.into(), doc_types);
        }

        fn add_relation(&mut self, source: &str, target: &str) {
            self.relations.entry(source.into()).or_default().push(DocRelation {
                source_id: source.into(), target_id: target.into(),
                relation_type: RelationType::References,
                confidence: 0.0, context: String::new(), created_at: String::new(),
                origin: Default::default(),
            });
        }
    }

    impl VectorDBPort for LintTestVectorDb {
        fn init(&self) -> anyhow::Result<()> { Ok(()) }
        fn upsert(&self, _doc: &Document) -> anyhow::Result<()> { Ok(()) }
        fn search_similar(&self, _embedding: &[f32], _top_k: usize) -> anyhow::Result<Vec<SimilarDoc>> { Ok(vec![]) }
        fn find_by_hash(&self, _hash: &str) -> anyhow::Result<Option<String>> { Ok(None) }
        fn find_by_type(&self, _doc_type: &str, _date: &str) -> anyhow::Result<Option<String>> { Ok(None) }
        fn stats(&self) -> anyhow::Result<DbStats> { Ok(DbStats::default()) }
        fn list_all(&self) -> anyhow::Result<Vec<StoredDocSummary>> { Ok(self.docs.clone()) }
        fn get_types(&self, doc_id: &str) -> anyhow::Result<Vec<String>> {
            Ok(self.types.get(doc_id).cloned().unwrap_or_default())
        }
        fn update_types(&self, _doc_id: &str, _types: Vec<String>) -> anyhow::Result<()> { Ok(()) }
        fn link(&self, _source_id: &str, _target_id: &str, _relation: RelationType) -> anyhow::Result<()> { Ok(()) }
        fn find_related(&self, doc_id: &str) -> anyhow::Result<Vec<DocRelation>> {
            Ok(self.relations.get(doc_id).cloned().unwrap_or_default())
        }
        fn update_content(&self, _doc_id: &str, _new_content: &str, _change_summary: &str) -> anyhow::Result<()> { Ok(()) }
    }

    #[test]
    fn test_lint_orphan_detection() {
        let mut db = LintTestVectorDb::new();
        db.add_doc("orphan_doc", "2026-04-14", vec!["meeting".into()]);
        // 관계 없음 → orphan
        let report = Linter::lint(&db).expect("lint");
        assert!(report.orphan_docs.contains(&"orphan_doc".to_string()));
        assert!(report.issues.iter().any(|i| i.issue_type == LintIssueType::Orphan));
    }

    // test_lint_stale_detection 제거됨: lint stale 검사는 Phase 55에서 삭제됨
    // (lint_stale_days 설정 + Linter::stale 분기 제거)

    #[test]
    fn test_lint_missing_backlink() {
        let mut db = LintTestVectorDb::new();
        db.add_doc("doc_a", "2026-04-14", vec!["meeting".into()]);
        db.add_doc("doc_b", "2026-04-14", vec!["report".into()]);
        // A→B 있지만 B→A 없음
        db.add_relation("doc_a", "doc_b");
        // doc_b에는 관계 추가하지 않음

        let report = Linter::lint(&db).expect("lint");
        assert!(report.issues.iter().any(|i| i.issue_type == LintIssueType::MissingBacklink));
    }

    #[test]
    fn test_lint_no_type() {
        let mut db = LintTestVectorDb::new();
        db.docs.push(StoredDocSummary {
            id: "untyped".into(), path: PathBuf::from("untyped.zst"),
            doc_types: vec![], date: "2026-04-14".into(),
        });
        db.types.insert("untyped".into(), vec![]); // 유형 없음

        let report = Linter::lint(&db).expect("lint");
        assert!(report.issues.iter().any(|i|
            i.doc_id == "untyped" && i.issue_type == LintIssueType::DuplicateTopic
        ));
    }

    #[test]
    fn test_lint_topics_contradiction() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let topic_file = tmp.path().join("test_topic.md");
        std::fs::write(&topic_file, "## 주제\n정상 내용\n⚠️ 모순 발견: A와 B가 충돌\n").expect("write");

        let issues = Linter::lint_topics(tmp.path()).expect("lint_topics");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, LintIssueType::Contradiction);
        assert!(issues[0].description.contains("모순"));
    }

    // ── Phase 88: lint_strong_claims 테스트 ──

    use crate::ports::output::StoragePort;

    /// 가공본 텍스트를 HashMap에 보관하는 in-memory stub. lint_strong_claims는
    /// `decompress_temp`로 본문을 읽으므로 임시 파일 경로를 반환하도록 구현.
    struct StrongClaimStorage {
        contents: std::collections::HashMap<String, String>,
        tmp_dir: tempfile::TempDir,
    }
    impl StrongClaimStorage {
        fn new() -> Self {
            Self { contents: std::collections::HashMap::new(), tmp_dir: tempfile::tempdir().expect("tmp") }
        }
        fn put(&mut self, doc_id: &str, content: &str) {
            self.contents.insert(doc_id.into(), content.into());
        }
    }
    impl StoragePort for StrongClaimStorage {
        fn compress_and_store(&self, _src: &std::path::Path, _dest: &std::path::Path) -> anyhow::Result<PathBuf> {
            Ok(PathBuf::new())
        }
        fn decompress_temp(&self, compressed: &std::path::Path) -> anyhow::Result<PathBuf> {
            // doc.path는 "{doc_id}.zst" 형태로 add_doc에서 설정됨
            let key = compressed.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let content = self.contents.get(key).cloned().unwrap_or_default();
            let f = self.tmp_dir.path().join(format!("{}.tmp", key));
            std::fs::write(&f, content)?;
            Ok(f)
        }
        fn read_header(&self, _compressed: &std::path::Path, _lines: usize) -> anyhow::Result<String> {
            Ok(String::new())
        }
    }

    #[test]
    fn test_lint_strong_claims_detects_marker_sentence() {
        let mut db = LintTestVectorDb::new();
        db.add_doc("doc_strong", "2026-05-15", vec!["meeting".into()]);
        let mut storage = StrongClaimStorage::new();
        storage.put("doc_strong", "이 방법은 확실히 빠릅니다. 다른 방법도 가능성이 있습니다.");

        let issues = Linter::lint_strong_claims(&db, &storage, 5).expect("lint_strong_claims");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, LintIssueType::StrongClaim);
        assert!(issues[0].description.contains("확실히"));
    }

    #[test]
    fn test_lint_strong_claims_skip_empty() {
        let mut db = LintTestVectorDb::new();
        db.add_doc("doc_empty", "2026-05-15", vec!["meeting".into()]);
        let storage = StrongClaimStorage::new(); // content 없음 → 빈 본문

        let issues = Linter::lint_strong_claims(&db, &storage, 5).expect("lint_strong_claims");
        assert!(issues.is_empty(), "빈 본문은 스킵");
    }

    #[test]
    fn test_lint_strong_claims_max_per_doc() {
        let mut db = LintTestVectorDb::new();
        db.add_doc("doc_many", "2026-05-15", vec!["report".into()]);
        let mut storage = StrongClaimStorage::new();
        storage.put("doc_many",
            "반드시 A. 항상 B. 절대 C. 모든 D. 완벽히 E. 결코 F.");

        // max_per_doc=3 → 3건만
        let issues = Linter::lint_strong_claims(&db, &storage, 3).expect("lint_strong_claims");
        assert_eq!(issues.len(), 3, "max_per_doc 상한 적용");
    }
}
