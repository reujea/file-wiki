//! 코퍼스 진단 엔진 — pipeline doctor, 진단 스냅샷, health check, 벤치마크 스냅샷

use crate::ports::output::VectorDBPort;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 코퍼스 진단 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusStats {
    pub timestamp: String,
    pub doc_count: usize,
    pub relations: RelationStats,
    pub per_doc_histogram: HashMap<String, usize>,
    /// outgoing+incoming 통합 degree 상위 10
    pub hub_docs_top10: Vec<HubDoc>,
    /// incoming degree 상위 10 (다른 문서가 이 문서를 참조하는 수)
    #[serde(default)]
    pub incoming_top10: Vec<HubDoc>,
    pub isolated_docs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationStats {
    pub total: usize,
    pub unique_pairs: usize,
    pub double_count_ratio: f64,
    pub by_type: HashMap<String, TypeStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeStats {
    pub count: usize,
    pub avg_per_doc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubDoc {
    pub doc_id: String,
    pub relation_count: usize,
}

/// Health check 결과
#[derive(Debug, Clone)]
pub struct HealthIssue {
    pub level: HealthLevel,
    pub message: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthLevel {
    Ok,
    Warning,
    Error,
}

impl std::fmt::Display for HealthLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthLevel::Ok => write!(f, "✅"),
            HealthLevel::Warning => write!(f, "⚠️"),
            HealthLevel::Error => write!(f, "❌"),
        }
    }
}

/// 코퍼스 분석 실행
pub fn analyze_corpus(vector_db: &dyn VectorDBPort) -> Result<CorpusStats> {
    let all = vector_db.list_all()?;
    let doc_count = all.len();

    let mut total_relations = 0usize;
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    let mut per_doc_counts: Vec<(String, usize)> = Vec::new();
    let mut incoming_counts: HashMap<String, usize> = HashMap::new();
    let mut unique_pairs: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    for doc in &all {
        let rels = vector_db.find_related(&doc.id)?;
        total_relations += rels.len();
        per_doc_counts.push((doc.id.clone(), rels.len()));
        for r in &rels {
            *type_counts.entry(format!("{}", r.relation_type)).or_default() += 1;
            // incoming degree: 이 문서가 target인 관계 카운트
            if r.target_id == doc.id {
                *incoming_counts.entry(doc.id.clone()).or_default() += 1;
            }
            let pair = if r.source_id < r.target_id {
                (r.source_id.clone(), r.target_id.clone())
            } else {
                (r.target_id.clone(), r.source_id.clone())
            };
            unique_pairs.insert(pair);
        }
    }

    let isolated = per_doc_counts.iter().filter(|(_, c)| *c == 0).count();
    let double_ratio = if !unique_pairs.is_empty() {
        total_relations as f64 / unique_pairs.len() as f64
    } else { 0.0 };

    // 히스토그램
    let mut histogram: HashMap<String, usize> = HashMap::new();
    for (_, c) in &per_doc_counts {
        let bucket = match *c {
            0..=9 => "0-9",
            10..=19 => "10-19",
            20..=29 => "20-29",
            30..=49 => "30-49",
            50..=99 => "50-99",
            _ => "100+",
        };
        *histogram.entry(bucket.to_string()).or_default() += 1;
    }

    // 허브 top 10 (total degree)
    per_doc_counts.sort_by(|a, b| b.1.cmp(&a.1));
    let hub_top10: Vec<HubDoc> = per_doc_counts.iter().take(10)
        .map(|(id, count)| HubDoc { doc_id: id.clone(), relation_count: *count })
        .collect();

    // incoming top 10
    let mut incoming_vec: Vec<(String, usize)> = incoming_counts.into_iter().collect();
    incoming_vec.sort_by(|a, b| b.1.cmp(&a.1));
    let incoming_top10: Vec<HubDoc> = incoming_vec.iter().take(10)
        .map(|(id, count)| HubDoc { doc_id: id.clone(), relation_count: *count })
        .collect();

    // 유형별 통계
    let by_type: HashMap<String, TypeStats> = type_counts.into_iter()
        .map(|(t, count)| {
            let avg = if doc_count > 0 { count as f64 / doc_count as f64 } else { 0.0 };
            (t, TypeStats { count, avg_per_doc: avg })
        })
        .collect();

    Ok(CorpusStats {
        timestamp: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        doc_count,
        relations: RelationStats {
            total: total_relations,
            unique_pairs: unique_pairs.len(),
            double_count_ratio: double_ratio,
            by_type,
        },
        per_doc_histogram: histogram,
        hub_docs_top10: hub_top10,
        incoming_top10,
        isolated_docs: isolated,
    })
}

/// Health check 판정
pub fn health_check(stats: &CorpusStats) -> Vec<HealthIssue> {
    let mut issues = Vec::new();

    // 문서 수
    issues.push(HealthIssue {
        level: HealthLevel::Ok,
        message: format!("문서 수: {}", stats.doc_count),
        action: String::new(),
    });

    // 고립 문서
    if stats.isolated_docs > stats.doc_count / 20 {
        issues.push(HealthIssue {
            level: HealthLevel::Warning,
            message: format!("고립 문서 {}건 ({}%)", stats.isolated_docs, stats.isolated_docs * 100 / stats.doc_count.max(1)),
            action: "threshold 낮추거나 임베딩 품질 확인".to_string(),
        });
    } else {
        issues.push(HealthIssue {
            level: HealthLevel::Ok,
            message: format!("고립 문서: {}건", stats.isolated_docs),
            action: String::new(),
        });
    }

    // Supersedes/Updates 0건
    let sup = stats.relations.by_type.get("supersedes").map(|t| t.count).unwrap_or(0);
    let upd = stats.relations.by_type.get("updates").map(|t| t.count).unwrap_or(0);
    if sup == 0 && upd == 0 && stats.doc_count > 50 {
        issues.push(HealthIssue {
            level: HealthLevel::Error,
            message: "Supersedes/Updates 관계 0건".to_string(),
            action: "날짜 메타데이터 누락 의심. pipeline doctor --check-dates".to_string(),
        });
    }

    // 허브 편중
    if let Some(hub) = stats.hub_docs_top10.first() {
        let hub_ratio = hub.relation_count as f64 / stats.relations.total.max(1) as f64 * 100.0;
        if hub_ratio > 5.0 {
            issues.push(HealthIssue {
                level: HealthLevel::Warning,
                message: format!("허브 문서 편중: 상위 1개가 전체 관계의 {:.1}%", hub_ratio),
                action: "mutual top-K 적용 또는 허브 문서 확인".to_string(),
            });
        }
    }

    // double_count_ratio
    if stats.relations.double_count_ratio < 1.8 && stats.relations.total > 100 {
        issues.push(HealthIssue {
            level: HealthLevel::Warning,
            message: format!("비대칭 관계 비율 높음 (ratio: {:.2})", stats.relations.double_count_ratio),
            action: "양방향 link 누락 의심".to_string(),
        });
    }

    // incoming degree 폭증
    if let Some(top) = stats.incoming_top10.first() {
        let threshold = (stats.doc_count as f64 * 0.1).max(50.0) as usize;
        if top.relation_count > threshold {
            issues.push(HealthIssue {
                level: HealthLevel::Warning,
                message: format!("incoming 허브 폭증: {} ({} incoming)", top.doc_id.chars().take(12).collect::<String>(), top.relation_count),
                action: "이 문서가 과도하게 참조됨. mutual top-K 또는 cap 조정 검토".to_string(),
            });
        }
    }

    issues
}

/// 진단 리포트 포맷
pub fn format_report(stats: &CorpusStats, issues: &[HealthIssue]) -> String {
    let mut out = String::new();
    out.push_str(&format!("\nCorpus Health Report ({})\n", stats.timestamp));
    out.push_str("─────────────────────────────────\n");

    for issue in issues {
        out.push_str(&format!("{} {}\n", issue.level, issue.message));
        if !issue.action.is_empty() {
            out.push_str(&format!("   → {}\n", issue.action));
        }
    }

    out.push_str(&format!("\n관계 총수: {} (고유 쌍: {}, ratio: {:.2})\n",
        stats.relations.total, stats.relations.unique_pairs, stats.relations.double_count_ratio));

    out.push_str("유형별:\n");
    for (t, s) in &stats.relations.by_type {
        out.push_str(&format!("  {}: {} (avg {:.1}/doc)\n", t, s.count, s.avg_per_doc));
    }

    out.push_str(&format!("히스토그램: {:?}\n", stats.per_doc_histogram));
    out.push_str(&format!("허브 top 3 (total): {:?}\n", &stats.hub_docs_top10[..stats.hub_docs_top10.len().min(3)]));

    if !stats.incoming_top10.is_empty() {
        out.push_str(&format!("허브 top 3 (incoming): {:?}\n", &stats.incoming_top10[..stats.incoming_top10.len().min(3)]));
    }

    out
}

// ═══════════════════════════════════════════════════════════════
// 벤치마크 스냅샷 — JSON 영속화 + CI 회귀 감지
// ═══════════════════════════════════════════════════════════════

/// 벤치마크 실행 결과 스냅샷 (JSON 직렬화 가능)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSnapshot {
    /// 스냅샷 메타
    pub version: u32,
    pub timestamp: String,
    pub label: String,
    pub git_hash: Option<String>,

    /// 규모
    pub doc_count: usize,

    /// 처리 성능
    pub throughput: ThroughputMetrics,

    /// per-doc 분포
    pub per_doc: Option<PerDocMetrics>,

    /// 검색 성능
    pub search: Option<SearchMetrics>,

    /// 교차참조
    pub crossref: CrossrefMetrics,

    /// 스토리지
    pub storage: Option<StorageMetrics>,

    /// 코퍼스 진단 (CorpusStats)
    pub corpus: Option<CorpusStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    pub total_secs: f64,
    pub process_secs: f64,
    pub batch_end_secs: f64,
    pub flush_secs: f64,
    pub docs_per_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerDocMetrics {
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
    /// p95/p50 비율 — 1.0에 가까울수록 균일
    pub variance_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetrics {
    pub avg_ms: f64,
    pub p95_ms: f64,
    pub queries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossrefMetrics {
    pub relation_count: usize,
    pub unique_pairs: usize,
    pub double_count_ratio: f64,
    pub isolated_docs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    pub input_bytes: u64,
    pub processed_bytes: u64,
    pub originals_bytes: u64,
    pub compression_pct: f64,
}

impl BenchmarkSnapshot {
    pub const CURRENT_VERSION: u32 = 1;

    /// 현재 git short hash 가져오기
    pub fn git_short_hash() -> Option<String> {
        let mut cmd = std::process::Command::new("git");
        cmd.args(["rev-parse", "--short", "HEAD"]);
        #[cfg(windows)]
        { use std::os::windows::process::CommandExt; cmd.creation_flags(0x08000000); }
        cmd.output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            })
    }

    /// 스냅샷을 JSON 파일로 저장
    pub fn save_to(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// JSON 파일에서 스냅샷 로드
    pub fn load_from(path: &std::path::Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    /// 디렉토리에서 특정 라벨의 최신 스냅샷 로드
    pub fn load_latest(dir: &std::path::Path, label_prefix: &str) -> Result<Option<Self>> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with(label_prefix)
                    && e.file_name().to_string_lossy().ends_with(".json")
            })
            .collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
        match entries.first() {
            Some(entry) => Ok(Some(Self::load_from(&entry.path())?)),
            None => Ok(None),
        }
    }
}

/// CI 회귀 감지 결과
#[derive(Debug)]
pub struct RegressionResult {
    pub passed: bool,
    pub checks: Vec<RegressionCheck>,
}

#[derive(Debug)]
pub struct RegressionCheck {
    pub metric: String,
    pub baseline: f64,
    pub current: f64,
    pub threshold: f64,
    pub passed: bool,
}

impl std::fmt::Display for RegressionCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let icon = if self.passed { "PASS" } else { "FAIL" };
        write!(
            f, "[{}] {} — baseline: {:.2}, current: {:.2}, threshold: {:.2}",
            icon, self.metric, self.baseline, self.current, self.threshold
        )
    }
}

/// 두 스냅샷 간 성능 회귀 검사
///
/// 기준:
/// - per-doc p95 ≤ 100ms (절대 기준)
/// - flush ≤ 30초 (절대 기준)
/// - throughput 회귀 ≤ 20% (상대 기준)
pub fn check_regression(baseline: &BenchmarkSnapshot, current: &BenchmarkSnapshot) -> RegressionResult {
    let mut checks = Vec::new();

    // 1. per-doc p95 절대 기준 (100ms)
    if let Some(ref pd) = current.per_doc {
        checks.push(RegressionCheck {
            metric: "per-doc p95 (ms)".to_string(),
            baseline: baseline.per_doc.as_ref().map(|b| b.p95_ms).unwrap_or(0.0),
            current: pd.p95_ms,
            threshold: 100.0,
            passed: pd.p95_ms <= 100.0,
        });
    }

    // 2. flush 절대 기준 (30초)
    checks.push(RegressionCheck {
        metric: "flush (secs)".to_string(),
        baseline: baseline.throughput.flush_secs,
        current: current.throughput.flush_secs,
        threshold: 30.0,
        passed: current.throughput.flush_secs <= 30.0,
    });

    // 3. throughput 상대 기준 (20% 이상 하락 시 실패)
    let tp_threshold = baseline.throughput.docs_per_sec * 0.80;
    checks.push(RegressionCheck {
        metric: "throughput (docs/sec)".to_string(),
        baseline: baseline.throughput.docs_per_sec,
        current: current.throughput.docs_per_sec,
        threshold: tp_threshold,
        passed: current.throughput.docs_per_sec >= tp_threshold,
    });

    let passed = checks.iter().all(|c| c.passed);
    RegressionResult { passed, checks }
}

/// 스냅샷 파일명 생성 (정렬 가능: label_YYYYMMDD_HHMMSS.json)
pub fn snapshot_filename(label: &str) -> String {
    let now = chrono::Local::now();
    format!("{}_{}.json", label, now.format("%Y%m%d_%H%M%S"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stats(doc_count: usize, total_rels: usize, isolated: usize) -> CorpusStats {
        let mut by_type = HashMap::new();
        by_type.insert("supersedes".into(), TypeStats { count: 10, avg_per_doc: 0.1 });
        by_type.insert("updates".into(), TypeStats { count: 20, avg_per_doc: 0.2 });
        by_type.insert("related_topic".into(), TypeStats { count: total_rels / 2, avg_per_doc: 1.0 });
        CorpusStats {
            timestamp: "2026-04-22T12:00:00".into(),
            doc_count,
            relations: RelationStats {
                total: total_rels,
                unique_pairs: total_rels / 2,
                double_count_ratio: 2.0,
                by_type,
            },
            per_doc_histogram: HashMap::new(),
            hub_docs_top10: vec![
                HubDoc { doc_id: "hub1".into(), relation_count: 50 },
            ],
            incoming_top10: vec![
                HubDoc { doc_id: "target1".into(), relation_count: 30 },
            ],
            isolated_docs: isolated,
        }
    }

    #[test]
    fn test_health_check_ok() {
        let stats = make_stats(100, 500, 0);
        let issues = health_check(&stats);
        let errors: Vec<_> = issues.iter().filter(|i| i.level == HealthLevel::Error).collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_health_check_isolated_warning() {
        let stats = make_stats(100, 500, 20); // 20% 고립
        let issues = health_check(&stats);
        let warnings: Vec<_> = issues.iter().filter(|i| i.level == HealthLevel::Warning && i.message.contains("고립")).collect();
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_health_check_incoming_hub_warning() {
        let mut stats = make_stats(100, 500, 0);
        stats.incoming_top10 = vec![
            HubDoc { doc_id: "mega_hub".into(), relation_count: 200 }, // 100*0.1=10 < 200
        ];
        let issues = health_check(&stats);
        let hub_warnings: Vec<_> = issues.iter().filter(|i| i.message.contains("incoming")).collect();
        assert!(!hub_warnings.is_empty());
    }

    #[test]
    fn test_health_check_asymmetry() {
        let mut stats = make_stats(100, 500, 0);
        stats.relations.double_count_ratio = 1.2; // < 1.8 → 비대칭
        let issues = health_check(&stats);
        let asym: Vec<_> = issues.iter().filter(|i| i.message.contains("비대칭")).collect();
        assert!(!asym.is_empty());
    }

    #[test]
    fn test_format_report_not_empty() {
        let stats = make_stats(10, 50, 1);
        let issues = health_check(&stats);
        let report = format_report(&stats, &issues);
        assert!(report.contains("Corpus Health Report"));
        assert!(report.contains("관계 총수"));
    }

    #[test]
    fn test_snapshot_filename_format() {
        let name = snapshot_filename("scale_100");
        assert!(name.starts_with("scale_100_"));
        assert!(name.ends_with(".json"));
    }

    #[test]
    fn test_benchmark_snapshot_roundtrip() {
        let snapshot = BenchmarkSnapshot {
            version: BenchmarkSnapshot::CURRENT_VERSION,
            timestamp: "2026-04-22".into(),
            label: "test".into(),
            git_hash: Some("abc".into()),
            doc_count: 100,
            throughput: ThroughputMetrics {
                total_secs: 10.0, process_secs: 8.0,
                batch_end_secs: 0.5, flush_secs: 1.5, docs_per_sec: 10.0,
            },
            per_doc: None, search: None,
            crossref: CrossrefMetrics { relation_count: 0, unique_pairs: 0, double_count_ratio: 0.0, isolated_docs: 0 },
            storage: None, corpus: None,
        };
        let json = serde_json::to_string(&snapshot).expect("serialize");
        let loaded: BenchmarkSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(loaded.doc_count, 100);
    }

    #[test]
    fn test_check_regression_pass() {
        let baseline = BenchmarkSnapshot {
            version: 1, timestamp: "".into(), label: "".into(), git_hash: None,
            doc_count: 100,
            throughput: ThroughputMetrics {
                total_secs: 10.0, process_secs: 8.0, batch_end_secs: 0.5, flush_secs: 1.5, docs_per_sec: 10.0,
            },
            per_doc: Some(PerDocMetrics { avg_ms: 80.0, p50_ms: 75.0, p95_ms: 90.0, max_ms: 100.0, variance_ratio: 1.2 }),
            search: None, crossref: CrossrefMetrics { relation_count: 0, unique_pairs: 0, double_count_ratio: 0.0, isolated_docs: 0 },
            storage: None, corpus: None,
        };
        let result = check_regression(&baseline, &baseline);
        assert!(result.passed);
    }

    #[test]
    fn test_check_regression_throughput_fail() {
        let baseline = BenchmarkSnapshot {
            version: 1, timestamp: "".into(), label: "".into(), git_hash: None,
            doc_count: 100,
            throughput: ThroughputMetrics {
                total_secs: 10.0, process_secs: 8.0, batch_end_secs: 0.5, flush_secs: 1.5, docs_per_sec: 10.0,
            },
            per_doc: None, search: None,
            crossref: CrossrefMetrics { relation_count: 0, unique_pairs: 0, double_count_ratio: 0.0, isolated_docs: 0 },
            storage: None, corpus: None,
        };
        let mut degraded = baseline.clone();
        degraded.throughput.docs_per_sec = 5.0; // 50% 하락 → 실패
        let result = check_regression(&baseline, &degraded);
        assert!(!result.passed);
    }
}
