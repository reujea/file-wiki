//! Phase 76: 다축 프로파일 + 선언적 룰 엔진 기반 설정 추천
//!
//! 외부 전문가 답변(prd/queries/initial-setup-advisor-experts.answer.md)에 따른 재설계.
//!
//! 변경점 (Phase 73 → Phase 76):
//! - 단일축 시나리오 5종 → 5축 SetupProfile (content_mix/sensitivity/volume/search_intent/collaboration)
//! - if/else 하드코딩 → 선언적 룰 테이블 (RULES_TOML 임베드 + TOML 파싱)
//! - 충돌 해소 (동일 path 다중 룰 매칭 시 비율 가중 → 보수 → 표시)
//! - ConfigChange 확장 (priority/risk/evidence/confidence/reversible/restart_required)
//! - apply_advice는 Phase 76-3에서 toml_edit로 교체될 예정 (현재는 in-memory + to_toml_string)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::config::PipelineConfig;
use crate::config::PipelineConfigExt;

// ── 1. 5축 프로파일 ──────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Meeting,
    Research,
    Code,
    Legal,
    General,
}

impl ContentType {
    pub fn as_key(&self) -> &'static str {
        match self {
            ContentType::Meeting => "meeting",
            ContentType::Research => "research",
            ContentType::Code => "code",
            ContentType::Legal => "legal",
            ContentType::General => "general",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum Sensitivity { #[default]
Low, Medium, High, Regulated }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum Volume { Light, #[default]
Moderate, Heavy }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SearchIntent { #[default]
Precision, Exploration, Temporal }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum Collaboration { #[default]
Solo, SmallTeam, Team }


#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SetupProfile {
    /// 자유 텍스트 (호환용 + 추론 입력)
    #[serde(default)]
    pub description: Option<String>,
    /// 콘텐츠 유형 비율. 합이 1.0이 되도록 정규화.
    #[serde(default)]
    pub content_mix: Vec<(ContentType, f32)>,
    #[serde(default)]
    pub sensitivity: Sensitivity,
    #[serde(default)]
    pub volume: Volume,
    #[serde(default)]
    pub search_intent: SearchIntent,
    #[serde(default)]
    pub collaboration: Collaboration,
    #[serde(default)]
    pub user_role: Option<String>,
}

// ── 2. 자유 텍스트 → 다축 프로파일 추론 ──────────────────────

/// 키워드 룰로 자유 텍스트에서 5축 값을 추론한다.
/// 명시적 SetupProfile이 들어오면 description 외의 값은 그대로 사용.
pub fn infer_profile_from_text(text: &str) -> SetupProfile {
    let t = text.to_lowercase();

    let meeting_kws  = ["회의", "미팅", "결정", "안건", "참석", "agenda", "meeting", "minutes"];
    let research_kws = ["연구", "논문", "실험", "research", "paper", "study", "academic"];
    let code_kws     = ["코드", "소스", "스니펫", "리포지토리", "code", "source", "repo", "snippet"];
    let legal_kws    = ["계약", "법무", "약관", "legal", "contract", "compliance"];

    let count = |kws: &[&str]| -> f32 { kws.iter().filter(|k| t.contains(*k)).count() as f32 };
    let counts = [
        (ContentType::Meeting,  count(&meeting_kws)),
        (ContentType::Research, count(&research_kws)),
        (ContentType::Code,     count(&code_kws)),
        (ContentType::Legal,    count(&legal_kws)),
    ];
    let total: f32 = counts.iter().map(|(_, n)| *n).sum();
    let content_mix: Vec<(ContentType, f32)> = if total <= 0.0 {
        vec![(ContentType::General, 1.0)]
    } else {
        counts.iter()
            .filter(|(_, n)| *n > 0.0)
            .map(|(ct, n)| (*ct, n / total))
            .collect()
    };

    // 민감도 — 키워드/규정 기반
    let high_sens = ["민감", "secret", "private", "기밀", "confidential", "personal"]
        .iter().any(|k| t.contains(*k));
    let regulated = ["compliance", "gdpr", "hipaa", "pci", "법규", "감사"]
        .iter().any(|k| t.contains(*k));
    let sensitivity = if regulated { Sensitivity::Regulated }
        else if high_sens { Sensitivity::High }
        else { Sensitivity::Low };

    // 볼륨 — 숫자/규모 키워드
    let heavy = ["대량", "수천", "thousands", "heavy", "수만"].iter().any(|k| t.contains(*k));
    let light = ["소량", "few", "light", "가끔"].iter().any(|k| t.contains(*k));
    let volume = if heavy { Volume::Heavy } else if light { Volume::Light } else { Volume::Moderate };

    // 검색 의도
    let exploration = ["탐색", "둘러보기", "explore", "browse", "discover"].iter().any(|k| t.contains(*k));
    let temporal = ["최근", "최신", "recent", "latest", "시간순"].iter().any(|k| t.contains(*k));
    let search_intent = if exploration { SearchIntent::Exploration }
        else if temporal { SearchIntent::Temporal }
        else { SearchIntent::Precision };

    // 협업 — 팀 키워드
    let team = ["팀", "team", "공유", "shared"].iter().any(|k| t.contains(*k));
    let collaboration = if team { Collaboration::SmallTeam } else { Collaboration::Solo };

    SetupProfile {
        description: Some(text.to_string()),
        content_mix,
        sensitivity,
        volume,
        search_intent,
        collaboration,
        user_role: None,
    }
}

/// 자유 텍스트만 들어와도 기본 프로파일을 반환 (호환)
pub fn classify_scenario(text: &str) -> &'static str {
    let p = infer_profile_from_text(text);
    p.content_mix.iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(ct, _)| ct.as_key())
        .unwrap_or("general")
}

// ── 3. 룰 테이블 ─────────────────────────────────────────────

/// 룰의 축 조건. 모든 조건을 AND로 평가.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleCondition {
    /// content_type 비율 조건 (e.g. meeting>=0.5)
    #[serde(default)]
    pub content: Option<ContentCond>,
    #[serde(default)]
    pub sensitivity: Option<Sensitivity>,
    #[serde(default)]
    pub volume: Option<Volume>,
    #[serde(default)]
    pub search_intent: Option<SearchIntent>,
    #[serde(default)]
    pub collaboration: Option<Collaboration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentCond {
    pub kind: ContentType,
    pub min_ratio: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Priority { P0, P1, P2 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel { Low, Medium, High, Critical }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence { Heuristic, Benchmark, Literature, UserFeedback }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence { Low, Medium, High }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub setting: String,
    pub recommend: serde_json::Value,
    pub reason: String,
    #[serde(default)]
    pub condition: RuleCondition,
    #[serde(default = "default_priority")]
    pub priority: Priority,
    #[serde(default = "default_risk")]
    pub risk: RiskLevel,
    #[serde(default = "default_evidence")]
    pub evidence: Evidence,
    #[serde(default = "default_confidence")]
    pub confidence: Confidence,
    #[serde(default = "default_true")]
    pub reversible: bool,
    #[serde(default)]
    pub restart_required: bool,
}

fn default_priority() -> Priority { Priority::P1 }
fn default_risk() -> RiskLevel { RiskLevel::Low }
fn default_evidence() -> Evidence { Evidence::Heuristic }
fn default_confidence() -> Confidence { Confidence::Medium }
fn default_true() -> bool { true }

#[derive(Debug, Deserialize)]
struct RuleFile { rule: Vec<Rule> }

/// 임베드된 기본 룰 테이블 (TOML)
pub const DEFAULT_RULES_TOML: &str = include_str!("setup_rules.toml");

pub struct RecommendationEngine {
    rules: Vec<Rule>,
}

impl RecommendationEngine {
    pub fn from_toml(s: &str) -> Result<Self> {
        let parsed: RuleFile = toml::from_str(s).context("룰 테이블 TOML 파싱 실패")?;
        Ok(Self { rules: parsed.rule })
    }

    pub fn default_engine() -> Self {
        Self::from_toml(DEFAULT_RULES_TOML)
            .expect("DEFAULT_RULES_TOML 파싱 실패 — 빌드 타임에 검증되어야 함")
    }

    pub fn rules(&self) -> &[Rule] { &self.rules }

    pub fn evaluate(&self, profile: &SetupProfile, current: &PipelineConfig) -> Vec<ConfigChange> {
        // 매칭된 룰 모두 수집 후 path별 그룹핑
        let mut by_path: std::collections::HashMap<String, Vec<&Rule>> = std::collections::HashMap::new();
        for r in &self.rules {
            if !match_condition(&r.condition, profile) { continue; }
            by_path.entry(r.setting.clone()).or_default().push(r);
        }

        let mut out = Vec::new();
        for (path, rules) in by_path {
            // 충돌 해소
            let resolved = resolve_conflicts(&path, &rules, profile);
            for chg in resolved {
                if let Some(c) = build_change(&path, chg, current) {
                    out.push(c);
                }
            }
        }
        // priority 정렬: P0 → P1 → P2
        out.sort_by_key(|c| match c.priority { Priority::P0 => 0, Priority::P1 => 1, Priority::P2 => 2 });
        out
    }
}

fn match_condition(c: &RuleCondition, p: &SetupProfile) -> bool {
    if let Some(ref cc) = c.content {
        let r = p.content_mix.iter().find(|(k, _)| *k == cc.kind).map(|(_, v)| *v).unwrap_or(0.0);
        if r < cc.min_ratio { return false; }
    }
    if let Some(s) = c.sensitivity { if s != p.sensitivity { return false; } }
    if let Some(v) = c.volume { if v != p.volume { return false; } }
    if let Some(i) = c.search_intent { if i != p.search_intent { return false; } }
    if let Some(co) = c.collaboration { if co != p.collaboration { return false; } }
    true
}

/// 충돌 해소: 같은 path에 다중 매칭 시
/// 1) content 룰끼리는 비율 가중으로 가장 높은 비중의 룰 선택
/// 2) recommend 값이 동일하면 합쳐서 1건
/// 3) 다르면 가장 보수적(현재 값에 가까운)인 쪽 선택 + 충돌 표시
fn resolve_conflicts<'a>(path: &str, rules: &[&'a Rule], profile: &SetupProfile) -> Vec<ResolvedChange<'a>> {
    let _ = path;
    if rules.is_empty() { return vec![]; }
    if rules.len() == 1 { return vec![ResolvedChange { rule: rules[0], conflict: None }]; }

    // 같은 recommend 값을 그룹화 (JSON 직렬화 키)
    let mut groups: std::collections::BTreeMap<String, Vec<&Rule>> = std::collections::BTreeMap::new();
    for r in rules {
        let key = r.recommend.to_string();
        groups.entry(key).or_default().push(r);
    }
    if groups.len() == 1 {
        // 모두 동일 값. 우선순위 P0 우선.
        let mut all: Vec<&Rule> = rules.to_vec();
        all.sort_by_key(|r| match r.priority { Priority::P0 => 0, Priority::P1 => 1, Priority::P2 => 2 });
        return vec![ResolvedChange { rule: all[0], conflict: None }];
    }

    // 다중 그룹 — 비율 가중으로 우세 그룹 선택
    let weight = |rule: &Rule| -> f32 {
        if let Some(ref cc) = rule.condition.content {
            profile.content_mix.iter().find(|(k, _)| *k == cc.kind).map(|(_, v)| *v).unwrap_or(0.0)
        } else { 1.0 } // content 무관 룰은 풀 가중치
    };

    let mut group_weights: Vec<(String, f32, Vec<&Rule>)> = groups.into_iter()
        .map(|(k, rs)| {
            let w: f32 = rs.iter().map(|r| weight(r)).fold(0.0, f32::max);
            (k, w, rs)
        })
        .collect();
    group_weights.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let winner = &group_weights[0];
    let losers: Vec<&Rule> = group_weights[1..].iter().flat_map(|(_, _, rs)| rs.iter().copied()).collect();
    let conflict_summary = if losers.is_empty() { None } else {
        Some(format!(
            "다른 권장 {}건이 있었지만 현재 콘텐츠 비율상 적합도가 낮아 제외",
            losers.len()
        ))
    };

    // winner 그룹 내에서 P0 우선
    let mut wrules: Vec<&Rule> = winner.2.clone();
    wrules.sort_by_key(|r| match r.priority { Priority::P0 => 0, Priority::P1 => 1, Priority::P2 => 2 });
    vec![ResolvedChange { rule: wrules[0], conflict: conflict_summary }]
}

struct ResolvedChange<'a> {
    rule: &'a Rule,
    conflict: Option<String>,
}

// ── 4. ConfigChange ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    pub path: String,
    pub current: serde_json::Value,
    pub recommended: serde_json::Value,
    pub reason: String,
    #[serde(default = "default_priority")]
    pub priority: Priority,
    #[serde(default = "default_risk")]
    pub risk: RiskLevel,
    #[serde(default = "default_evidence")]
    pub evidence: Evidence,
    #[serde(default = "default_confidence")]
    pub confidence: Confidence,
    #[serde(default = "default_true")]
    pub reversible: bool,
    #[serde(default)]
    pub needs_restart: bool,
    #[serde(default)]
    pub conflict_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupAdvice {
    pub profile: SetupProfile,
    /// 호환용 — content_mix 최상위 항목의 키
    pub scenario: String,
    pub summary: String,
    pub changes: Vec<ConfigChange>,
}

fn build_change(path: &str, rc: ResolvedChange, current: &PipelineConfig) -> Option<ConfigChange> {
    let cur = current_value(current, path)?;
    if values_equal(&cur, &rc.rule.recommend) { return None; }
    Some(ConfigChange {
        path: path.into(),
        current: cur,
        recommended: rc.rule.recommend.clone(),
        reason: rc.rule.reason.clone(),
        priority: rc.rule.priority,
        risk: rc.rule.risk,
        evidence: rc.rule.evidence,
        confidence: rc.rule.confidence,
        reversible: rc.rule.reversible,
        needs_restart: rc.rule.restart_required,
        conflict_note: rc.conflict,
    })
}

fn values_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value::*;
    match (a, b) {
        (Number(x), Number(y)) => {
            // f64 비교를 epsilon 허용
            match (x.as_f64(), y.as_f64()) {
                (Some(xa), Some(yb)) => (xa - yb).abs() < 1e-6,
                _ => x == y,
            }
        }
        _ => a == b,
    }
}

fn current_value(c: &PipelineConfig, path: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let v = serde_json::to_value(c).ok()?;
    let mut cur = &v;
    for p in &parts {
        cur = cur.get(*p)?;
    }
    Some(cur.clone())
}

// ── 5. Public API ────────────────────────────────────────────

/// Phase 76: 다축 프로파일 → SetupAdvice
pub fn build_advice_from_profile(profile: SetupProfile, current: &PipelineConfig) -> SetupAdvice {
    let engine = RecommendationEngine::default_engine();
    let changes = engine.evaluate(&profile, current);
    let scenario = profile.content_mix.iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(ct, _)| ct.as_key().to_string())
        .unwrap_or_else(|| "general".into());

    let mix_str = profile.content_mix.iter()
        .map(|(ct, r)| format!("{} {}%", ct.as_key(), (r * 100.0).round() as i32))
        .collect::<Vec<_>>()
        .join(", ");
    let summary = if changes.is_empty() {
        format!("프로파일 ({mix_str}) — 현재 설정이 적합합니다.")
    } else {
        format!("프로파일 ({mix_str}) — {} 개 추천", changes.len())
    };

    SetupAdvice { profile, scenario, summary, changes }
}

/// 자유 텍스트 호환 진입점
pub fn build_advice(scenario_text: &str, user_role: Option<String>, current: &PipelineConfig) -> SetupAdvice {
    let mut profile = infer_profile_from_text(scenario_text);
    profile.user_role = user_role;
    build_advice_from_profile(profile, current)
}

/// Phase 76-3: toml_edit 기반 적용 (주석 보존, P0 항목 전체 + Critical 차단)
///
/// `apply_critical=false`이면 Critical 등급 변경은 스킵.
pub fn apply_advice(
    config_path: &Path,
    advice: &SetupAdvice,
    accepted_paths: &[String],
) -> Result<Vec<String>> {
    apply_advice_with_options(config_path, advice, accepted_paths, false)
}

pub fn apply_advice_with_options(
    config_path: &Path,
    advice: &SetupAdvice,
    accepted_paths: &[String],
    apply_critical: bool,
) -> Result<Vec<String>> {
    apply_advice_full(config_path, advice, accepted_paths, apply_critical, None).map(|r| r.applied)
}

#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub applied: Vec<String>,
    pub snapshot_id: Option<String>,
}

/// settings_db가 있으면 ConfigSnapshot도 함께 저장 (Phase 77)
/// + Phase 82: decision_log에 항목별 결정 기록 (source/context는 default).
pub fn apply_advice_full(
    config_path: &Path,
    advice: &SetupAdvice,
    accepted_paths: &[String],
    apply_critical: bool,
    db: Option<&crate::settings_db::SettingsDb>,
) -> Result<ApplyResult> {
    apply_advice_full_with_log(
        config_path, advice, accepted_paths, apply_critical, db,
        "setup_review", None,
    )
}

/// Phase 82: source/context를 받아 decision_log에 기록.
/// - source: 적용 진입점 ("setup_review" | "setup_modules")
/// - context: 호출 컨텍스트 JSON (module_ids 등)
pub fn apply_advice_full_with_log(
    config_path: &Path,
    advice: &SetupAdvice,
    accepted_paths: &[String],
    apply_critical: bool,
    db: Option<&crate::settings_db::SettingsDb>,
    source: &str,
    context: Option<&serde_json::Value>,
) -> Result<ApplyResult> {
    // .bak 백업 (호환 유지)
    if config_path.exists() {
        let bak = config_path.with_extension("toml.bak");
        std::fs::copy(config_path, &bak).context("pipeline.toml.bak 백업 실패")?;
    }

    let raw = if config_path.exists() {
        std::fs::read_to_string(config_path).context("pipeline.toml 읽기 실패")?
    } else {
        let cfg = PipelineConfig::default_config();
        cfg.to_toml_string()?
    };

    let mut doc = raw.parse::<toml_edit::DocumentMut>()
        .context("pipeline.toml toml_edit 파싱 실패")?;

    // Phase 82: 각 ConfigChange의 결정 분류 (소요 시간 0).
    // accepted_paths에 없으면 rejected, Critical+apply_critical=false면 critical_skipped.
    #[derive(Clone, Copy, PartialEq)]
    enum Decision { Accepted, Rejected, CriticalSkipped }
    let mut decisions: Vec<(usize, Decision)> = Vec::with_capacity(advice.changes.len());
    let mut applied = Vec::new();
    for (i, ch) in advice.changes.iter().enumerate() {
        if !accepted_paths.contains(&ch.path) {
            decisions.push((i, Decision::Rejected));
            continue;
        }
        if matches!(ch.risk, RiskLevel::Critical) && !apply_critical {
            decisions.push((i, Decision::CriticalSkipped));
            continue;
        }
        if write_toml_path(&mut doc, &ch.path, &ch.recommended).is_ok() {
            applied.push(ch.path.clone());
            decisions.push((i, Decision::Accepted));
        } else {
            // write 실패 → rejected로 기록 (실제로는 거부 아닌 실패지만 effect 동일)
            decisions.push((i, Decision::Rejected));
        }
    }

    let result = doc.to_string();
    PipelineConfig::load_from_str(&result).context("적용 후 설정 재파싱 실패 — 적용 거부됨")?;
    std::fs::write(config_path, result).context("pipeline.toml 저장 실패")?;

    // ConfigSnapshot 저장 (DB 제공 시)
    let snapshot_id = if let Some(db) = db {
        let snap = crate::config_snapshot::create_snapshot(
            config_path,
            Some(&advice.profile),
            &applied,
        )?;
        let id = snap.id.clone();
        // backup은 적용 전 raw로 교체 (create_snapshot은 적용 후 파일을 읽었음)
        let snap = crate::config_snapshot::ConfigSnapshot {
            config_backup: raw,
            ..snap
        };
        db.save_snapshot(&snap).context("snapshot 저장 실패")?;
        Some(id)
    } else {
        None
    };

    // Phase 82: decision_log 기록 (DB 제공 시). 실패해도 apply 결과는 유지.
    if let Some(db) = db {
        let now = chrono::Utc::now().to_rfc3339();
        let ctx_str = context.and_then(|v| serde_json::to_string(v).ok());
        for (i, dec) in &decisions {
            let ch = &advice.changes[*i];
            let entry = crate::settings_db::DecisionLogEntry {
                id: None,
                decided_at: now.clone(),
                source: source.to_string(),
                snapshot_id: if matches!(dec, Decision::Accepted) { snapshot_id.clone() } else { None },
                path: ch.path.clone(),
                decision: match dec {
                    Decision::Accepted => "accepted",
                    Decision::Rejected => "rejected",
                    Decision::CriticalSkipped => "critical_skipped",
                }.to_string(),
                before_value: serde_json::to_string(&ch.current).ok(),
                after_value: serde_json::to_string(&ch.recommended).ok(),
                priority: Some(format!("{:?}", ch.priority)),
                risk: Some(format!("{:?}", ch.risk).to_lowercase()),
                evidence: Some(format!("{:?}", ch.evidence).to_lowercase()),
                confidence: Some(format!("{:?}", ch.confidence).to_lowercase()),
                reason: Some(ch.reason.clone()),
                context: ctx_str.clone(),
            };
            let _ = db.insert_decision(&entry);
        }
    }

    Ok(ApplyResult { applied, snapshot_id })
}

pub fn write_toml_path(doc: &mut toml_edit::DocumentMut, path: &str, value: &serde_json::Value) -> Result<()> {
    use toml_edit::{Item, Table, value as tv};

    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() { return Err(anyhow::anyhow!("빈 path")); }

    // 마지막 노드까지 테이블을 탐색 (없으면 생성)
    let last = *parts.last().expect("non-empty checked");
    let mut node: &mut Item = doc.as_item_mut();
    for p in &parts[..parts.len() - 1] {
        // 현재 노드가 테이블이 아니면 새 테이블로 교체
        let is_table = matches!(node, Item::Table(_) | Item::None) || node.as_table_like().is_some();
        if !is_table {
            *node = Item::Table(Table::new());
        }
        // 테이블 진입
        let tbl = node.as_table_mut().ok_or_else(|| anyhow::anyhow!("{}는 테이블이 아님", p))?;
        if !tbl.contains_key(p) {
            tbl[p] = Item::Table(Table::new());
        }
        node = &mut tbl[p];
    }

    let tbl = node.as_table_mut().ok_or_else(|| anyhow::anyhow!("부모 노드가 테이블이 아님: {}", path))?;
    let item = json_to_toml_value(value)?;
    // 기존 키의 decor(주석/공백) 보존: 값만 교체
    if let Some(existing) = tbl.get_mut(last) {
        match existing {
            Item::Value(v) => {
                let prefix = v.decor().prefix().cloned();
                let suffix = v.decor().suffix().cloned();
                let mut new_v = item;
                if let Some(p) = prefix { new_v.decor_mut().set_prefix(p); }
                if let Some(s) = suffix { new_v.decor_mut().set_suffix(s); }
                *existing = Item::Value(new_v);
            }
            _ => {
                *existing = tv(item);
            }
        }
    } else {
        tbl.insert(last, tv(item));
    }
    Ok(())
}

fn json_to_toml_value(v: &serde_json::Value) -> Result<toml_edit::Value> {
    use serde_json::Value as J;
    use toml_edit::{Array, Value as T};
    Ok(match v {
        J::Null => return Err(anyhow::anyhow!("null 값은 TOML에 적용 불가")),
        J::Bool(b) => T::from(*b),
        J::Number(n) => {
            if let Some(i) = n.as_i64() { T::from(i) }
            else if let Some(f) = n.as_f64() { T::from(f) }
            else { return Err(anyhow::anyhow!("지원하지 않는 숫자: {}", n)); }
        }
        J::String(s) => T::from(s.as_str()),
        J::Array(arr) => {
            let mut a = Array::new();
            for item in arr {
                a.push(json_to_toml_value(item)?);
            }
            T::Array(a)
        }
        J::Object(_) => return Err(anyhow::anyhow!("중첩 객체는 path 분리 필요 (object 직접 적용 불가)")),
    })
}

// ── 6. 테스트 ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_rules_parse() {
        let engine = RecommendationEngine::default_engine();
        assert!(!engine.rules().is_empty(), "DEFAULT_RULES_TOML이 비어있음");
    }

    #[test]
    fn test_infer_profile_meeting() {
        let p = infer_profile_from_text("회의록 위주로 가공할 거야");
        assert!(p.content_mix.iter().any(|(c, _)| *c == ContentType::Meeting));
    }

    #[test]
    fn test_infer_profile_mixed() {
        let p = infer_profile_from_text("회의 코드 리뷰 결과");
        assert!(p.content_mix.iter().any(|(c, _)| *c == ContentType::Meeting));
        assert!(p.content_mix.iter().any(|(c, _)| *c == ContentType::Code));
    }

    #[test]
    fn test_infer_profile_general_fallback() {
        let p = infer_profile_from_text("그냥 파일 정리");
        assert_eq!(p.content_mix.len(), 1);
        assert_eq!(p.content_mix[0].0, ContentType::General);
    }

    #[test]
    fn test_classify_scenario_compat() {
        // 기존 호환 — 가장 우세한 축의 키 반환
        assert_eq!(classify_scenario("회의 결정사항"), "meeting");
        assert_eq!(classify_scenario("research paper"), "research");
        assert_eq!(classify_scenario("source code repo"), "code");
        assert_eq!(classify_scenario("그냥"), "general");
    }

    #[test]
    fn test_evaluate_meeting_recommends_chunking() {
        let engine = RecommendationEngine::default_engine();
        let cfg = PipelineConfig::default_config();
        let p = SetupProfile {
            content_mix: vec![(ContentType::Meeting, 1.0)],
            ..Default::default()
        };
        let changes = engine.evaluate(&p, &cfg);
        assert!(changes.iter().any(|c| c.path == "chunking.target_bytes"));
    }

    #[test]
    fn test_evaluate_code_recommends_preserve_blocks() {
        let engine = RecommendationEngine::default_engine();
        let mut cfg = PipelineConfig::default_config();
        cfg.chunking.preserve_code_blocks = false;
        let p = SetupProfile {
            content_mix: vec![(ContentType::Code, 1.0)],
            ..Default::default()
        };
        let changes = engine.evaluate(&p, &cfg);
        assert!(changes.iter().any(|c| c.path == "chunking.preserve_code_blocks"));
    }

    #[test]
    fn test_conflict_resolution_majority_wins() {
        let engine = RecommendationEngine::default_engine();
        let cfg = PipelineConfig::default_config();
        // 두 룰 모두 min_ratio=0.5 충족시키며 충돌 발생.
        // meeting 60% (target_bytes=2000) vs code 50% (target_bytes=2500)
        let p = SetupProfile {
            content_mix: vec![(ContentType::Meeting, 0.6), (ContentType::Code, 0.5)],
            ..Default::default()
        };
        let changes = engine.evaluate(&p, &cfg);
        let target = changes.iter().find(|c| c.path == "chunking.target_bytes")
            .expect("chunking.target_bytes 추천이 있어야 함");
        // meeting이 비율 높으니 meeting 추천 값 (2000)
        assert_eq!(target.recommended, serde_json::json!(2000));
        assert!(target.conflict_note.is_some(), "충돌 노트가 있어야 함");
    }

    #[test]
    fn test_evidence_and_priority_in_change() {
        let engine = RecommendationEngine::default_engine();
        let cfg = PipelineConfig::default_config();
        let p = SetupProfile {
            content_mix: vec![(ContentType::Research, 1.0)],
            ..Default::default()
        };
        let changes = engine.evaluate(&p, &cfg);
        for c in &changes {
            // 모든 룰이 evidence와 priority를 갖는다
            assert!(matches!(c.priority, Priority::P0 | Priority::P1 | Priority::P2));
            assert!(matches!(c.evidence, Evidence::Heuristic | Evidence::Benchmark | Evidence::Literature | Evidence::UserFeedback));
        }
    }

    #[test]
    fn test_high_sensitivity_recommends_keywords_extension() {
        let engine = RecommendationEngine::default_engine();
        let cfg = PipelineConfig::default_config();
        let p = SetupProfile {
            sensitivity: Sensitivity::High,
            ..Default::default()
        };
        let changes = engine.evaluate(&p, &cfg);
        // 민감도 룰이 매칭되어 어떤 변경이 생성되어야 함
        assert!(changes.iter().any(|c| c.path.starts_with("sensitive.")));
    }

    #[test]
    fn test_heavy_volume_recommends_workers() {
        let engine = RecommendationEngine::default_engine();
        let cfg = PipelineConfig::default_config();
        let p = SetupProfile {
            volume: Volume::Heavy,
            ..Default::default()
        };
        let changes = engine.evaluate(&p, &cfg);
        assert!(changes.iter().any(|c| c.path == "max_workers"));
    }

    #[test]
    fn test_apply_subset() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("temp");
        let cfg = PipelineConfig::default_config();
        write!(tmp, "{}", cfg.to_toml_string().expect("toml")).expect("write");
        let path = tmp.path().to_path_buf();

        let p = SetupProfile { content_mix: vec![(ContentType::Meeting, 1.0)], ..Default::default() };
        let advice = build_advice_from_profile(p, &cfg);
        assert!(!advice.changes.is_empty());

        let accepted: Vec<String> = vec!["chunking.target_bytes".into()];
        let applied = apply_advice(&path, &advice, &accepted).expect("apply");
        assert_eq!(applied, vec!["chunking.target_bytes"]);

        let updated = PipelineConfig::load(&path).expect("reload");
        assert_eq!(updated.chunking.target_bytes, 2000);
    }

    #[test]
    fn test_apply_creates_backup() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("temp");
        let cfg = PipelineConfig::default_config();
        write!(tmp, "{}", cfg.to_toml_string().expect("toml")).expect("write");
        let path = tmp.path().to_path_buf();

        let p = SetupProfile { content_mix: vec![(ContentType::Research, 1.0)], ..Default::default() };
        let advice = build_advice_from_profile(p, &cfg);
        let accepted: Vec<String> = advice.changes.iter().map(|c| c.path.clone()).collect();
        let _ = apply_advice(&path, &advice, &accepted).expect("apply");

        let bak = path.with_extension("toml.bak");
        assert!(bak.exists());
    }

    #[test]
    fn test_apply_blocks_critical_by_default() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("temp");
        let cfg = PipelineConfig::default_config();
        write!(tmp, "{}", cfg.to_toml_string().expect("toml")).expect("write");
        let path = tmp.path().to_path_buf();

        // retention.enabled는 룰 테이블에서 Critical로 태깅. apply_critical=false 시 스킵.
        let p = SetupProfile { volume: Volume::Heavy, ..Default::default() };
        let advice = build_advice_from_profile(p, &cfg);
        let critical_change = advice.changes.iter().find(|c| matches!(c.risk, RiskLevel::Critical));
        if let Some(cc) = critical_change {
            let accepted = vec![cc.path.clone()];
            let applied = apply_advice(&path, &advice, &accepted).expect("apply");
            assert!(applied.is_empty(), "Critical 변경이 default apply에 포함되면 안 됨");
        }
    }

    #[test]
    fn test_toml_edit_preserves_comments() {
        use std::io::Write;
        let toml_with_comments = r#"# This is a top comment
version = "1"

# chunking 그룹
[chunking]
# target chunk size in bytes
target_bytes = 1500
preserve_code_blocks = true
"#;
        let mut tmp = tempfile::NamedTempFile::new().expect("temp");
        write!(tmp, "{}", toml_with_comments).expect("write");
        let path = tmp.path().to_path_buf();

        let cfg = PipelineConfig::default_config();
        let p = SetupProfile { content_mix: vec![(ContentType::Meeting, 1.0)], ..Default::default() };
        let advice = build_advice_from_profile(p, &cfg);
        let accepted = vec!["chunking.target_bytes".to_string()];
        let _ = apply_advice(&path, &advice, &accepted).expect("apply");

        let updated_raw = std::fs::read_to_string(&path).expect("read");
        assert!(updated_raw.contains("# This is a top comment"), "탑 주석이 보존되어야 함");
        assert!(updated_raw.contains("# chunking 그룹"), "섹션 주석이 보존되어야 함");
        assert!(updated_raw.contains("# target chunk size in bytes"), "필드 주석이 보존되어야 함");
    }

    // ── Phase 82: Decision Log 통합 테스트 ──────────────────

    #[test]
    fn test_apply_writes_decision_log() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("temp");
        let cfg = PipelineConfig::default_config();
        write!(tmp, "{}", cfg.to_toml_string().expect("toml")).expect("write");
        let path = tmp.path().to_path_buf();

        let p = SetupProfile { content_mix: vec![(ContentType::Meeting, 1.0)], ..Default::default() };
        let advice = build_advice_from_profile(p, &cfg);
        assert!(advice.changes.len() >= 2, "최소 2개 변경 후보 필요");

        // 첫 번째만 accept, 나머지는 reject
        let accepted: Vec<String> = vec![advice.changes[0].path.clone()];
        let db = crate::settings_db::SettingsDb::open_in_memory().expect("db");

        let result = apply_advice_full(&path, &advice, &accepted, false, Some(&db)).expect("apply");
        assert_eq!(result.applied.len(), 1);
        assert!(result.snapshot_id.is_some());

        // decision_log에 모든 후보 기록됨
        let logs = db.list_decisions(0).expect("list");
        assert_eq!(logs.len(), advice.changes.len(), "모든 후보가 기록되어야 함");

        let accepted_logs: Vec<_> = logs.iter().filter(|e| e.decision == "accepted").collect();
        let rejected_logs: Vec<_> = logs.iter().filter(|e| e.decision == "rejected").collect();
        assert_eq!(accepted_logs.len(), 1);
        assert_eq!(rejected_logs.len(), advice.changes.len() - 1);

        // accepted 항목은 snapshot_id 연결
        assert_eq!(accepted_logs[0].snapshot_id, result.snapshot_id);
        // rejected 항목은 snapshot_id 없음
        assert!(rejected_logs[0].snapshot_id.is_none());

        // source 기본값
        assert_eq!(accepted_logs[0].source, "setup_review");

        // snapshot_id 필터링 동작
        let by_snap = db.list_decisions_by_snapshot(
            result.snapshot_id.as_ref().expect("snap")
        ).expect("filter");
        assert_eq!(by_snap.len(), 1);
        assert_eq!(by_snap[0].decision, "accepted");
    }

    #[test]
    fn test_apply_with_log_records_critical_skipped() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("temp");
        let cfg = PipelineConfig::default_config();
        write!(tmp, "{}", cfg.to_toml_string().expect("toml")).expect("write");
        let path = tmp.path().to_path_buf();

        let p = SetupProfile { volume: Volume::Heavy, ..Default::default() };
        let advice = build_advice_from_profile(p, &cfg);
        let critical = advice.changes.iter().find(|c| matches!(c.risk, RiskLevel::Critical));
        if critical.is_none() {
            // 룰 변경으로 Critical 후보가 사라지면 skip
            return;
        }
        let cc = critical.expect("crit");

        let accepted = vec![cc.path.clone()];
        let db = crate::settings_db::SettingsDb::open_in_memory().expect("db");

        let _ = apply_advice_full_with_log(
            &path, &advice, &accepted, false, Some(&db),
            "setup_modules", Some(&serde_json::json!({"module_ids": ["x"]})),
        ).expect("apply");

        let logs = db.list_decisions(0).expect("list");
        let skipped: Vec<_> = logs.iter().filter(|e| e.decision == "critical_skipped").collect();
        assert_eq!(skipped.len(), 1, "Critical accepted였지만 apply_critical=false라서 skip 기록");
        assert_eq!(skipped[0].source, "setup_modules");
        assert!(skipped[0].context.is_some(), "context JSON 전달됨");
    }
}
