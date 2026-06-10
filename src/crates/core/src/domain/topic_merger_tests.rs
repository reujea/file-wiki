use super::*;

#[test]
fn test_date_to_quarter() {
    assert_eq!(date_to_quarter("2026-01-15"), "2026-Q1");
    assert_eq!(date_to_quarter("2026-04-05"), "2026-Q2");
    assert_eq!(date_to_quarter("2026-07-20"), "2026-Q3");
    assert_eq!(date_to_quarter("2026-12-31"), "2026-Q4");
    assert_eq!(date_to_quarter(""), "unknown");
    assert_eq!(date_to_quarter("invalid"), "unknown");
}

#[test]
fn test_sanitize_filename() {
    assert_eq!(sanitize_filename("hello-world_123"), "hello-world_123");
    assert_eq!(sanitize_filename("a/b\\c:d"), "a_b_c_d");
}

#[test]
fn test_rotate_backups() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(&file, "v3").unwrap();

    let bak = file.with_extension("md.bak");
    std::fs::write(&bak, "v2").unwrap();
    rotate_backups(&file);
    assert!(file.with_extension("md.bak.1").exists());
}

#[test]
fn test_cluster_small() {
    let docs = vec![
        DocSummary { id: "a".into(), date: "2026-04-01".into(), content: "a".into(), embedding: vec![1.0, 0.0] },
        DocSummary { id: "b".into(), date: "2026-04-02".into(), content: "b".into(), embedding: vec![0.9, 0.1] },
    ];
    let clusters = cluster_by_embedding(&docs, 20);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].len(), 2);
}

#[test]
fn test_cluster_split() {
    let docs: Vec<DocSummary> = (0..5).map(|i| {
        let mut emb = vec![0.0f32; 10];
        emb[i % 10] = 1.0;
        DocSummary { id: format!("d{}", i), date: "2026-04-01".into(), content: format!("doc {}", i), embedding: emb }
    }).collect();
    let clusters = cluster_by_embedding(&docs, 2);
    assert!(clusters.len() >= 2);
    let total: usize = clusters.iter().map(|c: &Vec<usize>| c.len()).sum();
    assert_eq!(total, 5);
}

#[test]
fn test_group_by_quarter() {
    let docs = [DocSummary { id: "a".into(), date: "2026-01-10".into(), content: "".into(), embedding: vec![] },
        DocSummary { id: "b".into(), date: "2026-04-05".into(), content: "".into(), embedding: vec![] },
        DocSummary { id: "c".into(), date: "2026-01-20".into(), content: "".into(), embedding: vec![] }];
    let refs: Vec<&DocSummary> = docs.iter().collect();
    let groups = group_by_quarter(&refs);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].0, "2026-Q1");
    assert_eq!(groups[0].1.len(), 2);
}

#[test]
fn test_generate_type_index() {
    let links = vec![
        ("토픽A".to_string(), "토픽A.md".to_string(), 5usize),
        ("토픽B".to_string(), "토픽B.md".to_string(), 3usize),
    ];
    let index = generate_type_index("meeting", &links);
    assert!(index.contains("# meeting 종합"));
    assert!(index.contains("2 토픽"));
    assert!(index.contains("8 문서"));
}
