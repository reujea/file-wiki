#[cfg(test)]
mod doc_type_registry_tests {
    use crate::domain::models::{DocTypeDef, DocTypeRegistry};

    fn sample_registry() -> DocTypeRegistry {
        DocTypeRegistry::new(vec![
            DocTypeDef {
                id: "meeting".into(),
                label_ko: "회의록".into(),
                patterns: vec!["회의".into(), "meeting".into()],
                sections: vec!["결정사항".into(), "액션아이템".into(), "다음안건".into()],
                prompt: "회의록 형식".into(),
                dedup_key: None,
                sensitive: false, thresholds: None,
            },
            DocTypeDef {
                id: "todo".into(),
                label_ko: "할일".into(),
                patterns: vec!["할일".into(), "todo".into()],
                sections: vec!["긴급+중요".into(), "중요+여유".into()],
                prompt: "아이젠하워".into(),
                dedup_key: Some("date".into()),
                sensitive: false, thresholds: None,
            },
        ])
    }

    #[test]
    fn test_registry_get() {
        let reg = sample_registry();
        assert!(reg.get("meeting").is_some());
        assert!(reg.get("unknown").is_none());
    }

    #[test]
    fn test_registry_sections_for() {
        let reg = sample_registry();
        let sections = reg.sections_for("meeting");
        assert_eq!(sections.len(), 3);
        assert!(sections.contains(&"결정사항".to_string()));
    }

    #[test]
    fn test_registry_sections_for_types() {
        let reg = sample_registry();
        let sections = reg.sections_for_types(&["meeting".into(), "todo".into()]);
        assert!(sections.len() >= 5); // 3 + 2
        assert!(sections.contains(&"결정사항".to_string()));
        assert!(sections.contains(&"긴급+중요".to_string()));
    }

    #[test]
    fn test_registry_sections_for_unknown() {
        let reg = sample_registry();
        let sections = reg.sections_for("unknown_type");
        assert!(sections.is_empty());
    }

    #[test]
    fn test_empty_registry() {
        let reg = DocTypeRegistry::empty();
        assert!(reg.all().is_empty());
        assert!(reg.get("meeting").is_none());
    }

    #[test]
    fn test_hint_patterns() {
        let reg = sample_registry();
        let hints = reg.hint_patterns();
        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0].0, "meeting");
    }
}

#[cfg(test)]
mod relation_type_tests {
    use crate::domain::models::RelationType;

    #[test]
    fn test_display() {
        assert_eq!(RelationType::References.to_string(), "references");
        assert_eq!(RelationType::Updates.to_string(), "updates");
        assert_eq!(RelationType::RelatedTopic.to_string(), "related_topic");
        assert_eq!(RelationType::Supersedes.to_string(), "supersedes");
    }
}

#[cfg(test)]
mod metadata_tests {
    use crate::domain::models::Metadata;

    #[test]
    fn test_metadata_default_related_docs() {
        let json = r#"{"doc_types":["meeting"],"rationale":"test","date":"2026-04-05","summary":"s","keywords":[],"sensitive":false,"doi":null}"#;
        let meta: Metadata = serde_json::from_str(json).unwrap();
        assert!(meta.related_docs.is_empty());
    }

    #[test]
    fn test_metadata_with_related_docs() {
        let json = r#"{"doc_types":["meeting"],"rationale":"test","date":"2026-04-05","summary":"s","keywords":[],"sensitive":false,"doi":null,"related_docs":["abc","def"]}"#;
        let meta: Metadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.related_docs.len(), 2);
    }
}

#[cfg(test)]
mod compile_state_tests {
    use super::super::incremental::{CompileState, BenchmarkReport};

    #[test]
    fn test_new_file_is_changed() {
        let state = CompileState::new();
        assert!(state.is_changed("file.txt", "abc123"));
    }

    #[test]
    fn test_same_hash_not_changed() {
        let mut state = CompileState::new();
        state.record_compile("file.txt", "abc123", 1000, 500);
        assert!(!state.is_changed("file.txt", "abc123"));
    }

    #[test]
    fn test_different_hash_is_changed() {
        let mut state = CompileState::new();
        state.record_compile("file.txt", "abc123", 1000, 500);
        assert!(state.is_changed("file.txt", "def456"));
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        let mut state = CompileState::new();
        state.record_compile("a.txt", "h1", 500, 250);
        state.record_compile("b.txt", "h2", 800, 400);
        state.save(&path).unwrap();

        let loaded = CompileState::load(&path).unwrap();
        assert_eq!(loaded.entries.len(), 2);
        assert!(!loaded.is_changed("a.txt", "h1"));
        assert!(loaded.is_changed("a.txt", "wrong"));
    }

    #[test]
    fn test_stats_recalc() {
        let mut state = CompileState::new();
        state.record_compile("a.txt", "h1", 1000, 500);
        state.record_compile("b.txt", "h2", 2000, 600);
        assert_eq!(state.stats.total_files, 2);
        assert_eq!(state.stats.total_input_chars, 3000);
        assert_eq!(state.stats.total_output_chars, 1100);
    }

    #[test]
    fn test_token_estimation() {
        assert_eq!(BenchmarkReport::estimate_tokens(1000), 500);
        assert_eq!(BenchmarkReport::estimate_tokens(0), 0);
    }

    #[test]
    fn test_cost_estimation() {
        let cost = BenchmarkReport::estimate_cost(1_000_000, 0);
        assert!((cost - 3.0).abs() < 0.01);
        let cost = BenchmarkReport::estimate_cost(0, 1_000_000);
        assert!((cost - 15.0).abs() < 0.01);
    }
}

#[cfg(test)]
mod processing_summary_tests {
    use super::super::models::ProcessingSummary;

    #[test]
    fn test_empty() {
        let s = ProcessingSummary::default();
        assert!(s.is_empty());
    }

    #[test]
    fn test_record_success() {
        let mut s = ProcessingSummary::default();
        s.record_success(&["meeting".into(), "todo".into()]);
        s.record_success(&["meeting".into()]);
        assert_eq!(s.success, 2);
        assert_eq!(*s.by_type.get("meeting").unwrap(), 2);
        assert_eq!(*s.by_type.get("todo").unwrap(), 1);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_record_error() {
        let mut s = ProcessingSummary::default();
        s.record_error("bad.txt", "구조 실패", "quarantine");
        assert_eq!(s.errors, 1);
        assert_eq!(s.issues.len(), 1);
        assert_eq!(s.issues[0].level, "error");
        assert_eq!(s.issues[0].action_taken, "quarantine");
    }

    #[test]
    fn test_record_warning() {
        let mut s = ProcessingSummary::default();
        s.record_warning("w.txt", "압축률", "2-Pass");
        assert_eq!(s.issues.len(), 1);
        assert_eq!(s.issues[0].level, "warning");
    }

    #[test]
    fn test_mixed_counts() {
        let mut s = ProcessingSummary::default();
        s.record_success(&["meeting".into()]);
        s.duplicates = 2;
        s.sensitive = 1;
        s.quarantined = 1;
        s.skipped = 3;
        assert!(!s.is_empty());
        assert_eq!(s.success, 1);
        assert_eq!(s.duplicates, 2);
    }

    #[test]
    fn test_verification_metrics() {
        use super::super::models::VerificationMetricEntry;
        let mut s = ProcessingSummary::default();
        s.verification_metrics.push(VerificationMetricEntry {
            doc_id: "test".into(), timestamp: "2026-04-08".into(),
            structure: 0.8, compression: 0.5, keyword_coverage: 0.9,
            keyword_completeness: 0.7, rouge_l: 0.3, entity: 1.0,
            overall: "pass".into(),
        });
        assert_eq!(s.verification_metrics.len(), 1);
    }
}

#[cfg(test)]
mod sensitivity_extended_tests {
    use super::super::classifier::SensitivityDetector;
    use std::path::Path;

    #[test]
    fn test_sensitive_dir_path() {
        let d = SensitivityDetector::default();
        assert!(d.is_sensitive(Path::new("/data/sensitive/report.txt")).0);
        assert!(d.is_sensitive(Path::new("/data/민감/file.txt")).0);
    }

    #[test]
    fn test_custom_keywords() {
        let d = SensitivityDetector::new(vec!["프로젝트X".into()], vec![]);
        assert!(d.is_sensitive(Path::new("프로젝트X_기밀.txt")).0);
        assert!(!d.is_sensitive(Path::new("일반문서.txt")).0);
    }

    #[test]
    fn test_extension_plus_keyword() {
        let d = SensitivityDetector::default();
        // .pdf + 경로에 "계약" → 민감
        assert!(d.is_sensitive(Path::new("/계약서/file.pdf")).0);
        // .pdf만으로는 민감 아님
        assert!(!d.is_sensitive(Path::new("report.pdf")).0);
    }
}
