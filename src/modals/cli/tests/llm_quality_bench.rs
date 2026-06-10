//! 실환경 LLM 품질 벤치마크
//!
//! 다양한 유형 샘플 문서 → 실제 LLM 가공 → 검증 통과율 자동 측정
//! 실행: PIPELINE_REAL_BENCH=1 cargo nextest run --test llm_quality_bench

use std::path::Path;
use std::process::Command;
use std::time::Instant;

/// 샘플 문서 생성
fn create_sample_docs(dir: &Path) -> Vec<(String, String, &'static str)> {
    let samples = vec![
        ("meeting_sample.txt", "2026년 4월 8일 프로젝트 회의록\n\n참석자: 김개발, 이기획, 박디자인\n\n결정사항:\n1. API v2 마이그레이션 4월 말까지 완료\n2. Qdrant 유지, MyDocSearch 통합 불필요\n3. 다음 스프린트에 모바일 대응 시작\n\n액션아이템:\n- 김개발: API v2 스키마 작성 (4/12까지)\n- 이기획: 사용자 피드백 정리 (4/10까지)\n- 박디자인: 모바일 와이어프레임 (4/15까지)\n\n다음 회의: 4/15(월) 14:00", "meeting"),
        ("todo_sample.txt", "오늘 할일 2026-04-08\n\n- [ ] API 리팩터링 | 4/10 | backend\n- [ ] 테스트 커버리지 80% | 4/12 | qa\n- [x] 코드 리뷰 PR #42 | 완료 | review\n- [ ] 배포 스크립트 작성 | 4/11 | devops\n- [x] 문서 업데이트 | 완료 | docs", "todo"),
        ("memo_sample.txt", "Qdrant 벡터 검색 성능 메모\n\nHNSW 인덱스에서 3000문서 기준 0.57ms 검색 속도 확인.\nInt8 양자화 적용 시 RAM 75% 절감.\nSparse vector (BM25) 추가로 키워드 검색 품질 향상.\n\n결론: 10K 규모까지는 현재 설정으로 충분.", "memo"),
        ("study_sample.txt", "Rust 소유권 학습 노트\n\n핵심개념:\n- 소유권(Ownership): 각 값은 하나의 소유자만 가짐\n- 빌림(Borrowing): &T (불변 참조), &mut T (가변 참조)\n- 생명주기(Lifetime): 참조의 유효 범위\n\n요약: Rust의 메모리 안전성은 컴파일 타임에 보장됨. GC 없이 메모리 누수 방지.\n\n모르는것: Pin<T>과 Unpin의 정확한 사용 시나리오\n\n복습포인트: Drop trait와 RAII 패턴 관계", "study"),
        ("report_sample.txt", "2026년 1분기 프로젝트 보고서\n\n목적: file-pipeline 프로젝트 진행 현황 보고\n\n현황:\n- .rs 파일: 75개\n- 코드: ~14,500줄\n- 테스트: 138개 전체 통과\n- LLM 프로바이더: 5종 + Fallback\n\n분석:\n- 아키텍처: 헥사고날 패턴 위반 0건\n- 성능: HNSW 0.57ms@3K, stub 68 docs/s@5K\n- 비용: Claude CLI 무료, 시간 비용만 발생\n\n결론: MVP 수준 달성. 실사용 검증 단계로 진입.\n\n제안: 100건 파일럿 → 1K 확장 → 10K 본격 운영", "report"),
    ];

    let mut result = Vec::new();
    for (name, content, expected_type) in &samples {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        result.push((name.to_string(), path.to_string_lossy().to_string(), *expected_type));
    }
    result
}

/// Claude CLI 존재 확인
fn has_claude_cli() -> bool {
    Command::new("claude").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

#[tokio::test]
async fn llm_quality_classification_accuracy() {
    if std::env::var("PIPELINE_REAL_BENCH").is_err() {
        eprintln!("SKIP: PIPELINE_REAL_BENCH=1 설정 후 실행");
        return;
    }
    if !has_claude_cli() {
        eprintln!("SKIP: Claude CLI 미설치");
        return;
    }

    let dir = tempfile::TempDir::new().unwrap();
    let samples = create_sample_docs(dir.path());

    let adapter = file_pipeline_adapters::driven::llm::claude_adapter::ClaudeCliAdapter::new();
    let registry = file_pipeline_core::domain::models::DocTypeRegistry::new(vec![
        file_pipeline_core::domain::models::DocTypeDef {
            id: "meeting".into(), label_ko: "회의록".into(),
            patterns: vec!["회의".into(), "meeting".into()],
            sections: vec!["결정사항".into(), "액션아이템".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None,
        },
        file_pipeline_core::domain::models::DocTypeDef {
            id: "todo".into(), label_ko: "할일".into(),
            patterns: vec!["할일".into(), "todo".into()],
            sections: vec!["긴급+중요".into()],
            prompt: String::new(), dedup_key: Some("date".into()), sensitive: false, thresholds: None,
        },
        file_pipeline_core::domain::models::DocTypeDef {
            id: "memo".into(), label_ko: "메모".into(),
            patterns: vec!["메모".into()],
            sections: vec!["핵심내용".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None,
        },
        file_pipeline_core::domain::models::DocTypeDef {
            id: "study".into(), label_ko: "학습노트".into(),
            patterns: vec!["학습".into(), "노트".into()],
            sections: vec!["핵심개념".into(), "요약".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None,
        },
        file_pipeline_core::domain::models::DocTypeDef {
            id: "report".into(), label_ko: "보고서".into(),
            patterns: vec!["보고서".into(), "report".into()],
            sections: vec!["목적".into(), "현황".into(), "결론".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None,
        },
    ]);

    use file_pipeline_core::ports::output::LLMPort;

    let mut correct = 0;
    let mut total = 0;
    let mut total_time = 0u128;

    eprintln!("\n=== LLM 품질 벤치마크 ===\n");

    for (name, path_str, expected_type) in &samples {
        let path = std::path::Path::new(path_str);
        let start = Instant::now();
        match adapter.classify_and_process(path, &registry).await {
            Ok(result) => {
                let elapsed = start.elapsed().as_millis();
                total_time += elapsed;
                total += 1;

                let type_match = result.doc_types.iter().any(|t| t == expected_type);
                if type_match { correct += 1; }

                // 검증
                let original = std::fs::read_to_string(path).unwrap();
                let thresholds = file_pipeline_core::domain::verification::VerificationThresholds::default();
                let verification = file_pipeline_core::domain::verification::verify_with_thresholds(
                    &original, &result.content, &[], &result.metadata.keywords, result.sections.as_ref(), &thresholds,
                );

                let status = match &verification.overall {
                    file_pipeline_core::domain::models::VerificationLevel::Pass => "PASS",
                    file_pipeline_core::domain::models::VerificationLevel::Warning(_) => "WARN",
                    file_pipeline_core::domain::models::VerificationLevel::Fail(_) => "FAIL",
                };

                eprintln!("  {} → {:?} (기대: {}) [{}] {:.0}ms 구조={:.0}% 키워드={:.0}% ROUGE={:.0}%",
                    name, result.doc_types, expected_type,
                    if type_match { "✅" } else { "❌" },
                    elapsed,
                    verification.structure_completeness * 100.0,
                    verification.keyword_coverage * 100.0,
                    verification.rouge_l_recall * 100.0,
                );
                eprintln!("    검증: {} {:?}", status, verification.details);
            }
            Err(e) => {
                total += 1;
                eprintln!("  {} → 오류: {}", name, e);
            }
        }
    }

    eprintln!("\n=== 결과 ===");
    eprintln!("분류 정확도: {}/{} ({:.0}%)", correct, total, correct as f64 / total as f64 * 100.0);
    eprintln!("평균 처리 시간: {:.0}ms", total_time as f64 / total as f64);
    eprintln!("총 시간: {:.1}초", total_time as f64 / 1000.0);

    // 최소 기준: 60% 정확도 (5종 중 3종 이상)
    assert!(correct >= 3, "분류 정확도 60% 이상: {}/{}", correct, total);
}
