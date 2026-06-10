//! Tauri commands — REST API 완전 대체
//!
//! 기존 /api/* 엔드포인트를 tauri::command로 변환.
//! 프론트엔드(JS)에서 invoke("command_name", args)로 호출.

use serde::Deserialize;
use tauri::State;

use crate::state::AppState;

// ── 통계 / 헬스 ─────────────────────────────────────────────

#[tauri::command]
pub async fn get_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let stats = state.service.vector_db.stats().unwrap_or_default();
    Ok(serde_json::json!({
        "total_documents": stats.total_documents,
        "by_type": stats.by_type,
    }))
}

// ── 검색 ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub keyword: Option<String>,
    pub doc_type: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub top_k: Option<usize>,
}

#[tauri::command]
pub async fn search(state: State<'_, AppState>, params: SearchParams) -> Result<serde_json::Value, String> {
    let started = std::time::Instant::now();
    let top_k = params.top_k.unwrap_or(5);
    // Phase 95 A3 확장: Tauri search 호출 trace.
    let trace = file_pipeline_core::audit::TraceId::new();
    let inputs_hash = file_pipeline_core::audit::input_hash_prefix(params.query.as_bytes());
    let embedding = state.service.embedding.embed(&params.query).await
        .map_err(|e| e.to_string())?;

    let mut results = if let Some(ref kw) = params.keyword {
        state.service.vector_db.search_hybrid(&embedding, kw, top_k * 3).unwrap_or_default()
    } else {
        state.service.vector_db.search_similar(&embedding, top_k * 3).unwrap_or_default()
    };

    if let Some(ref dt) = params.doc_type {
        results.retain(|r| r.doc_types.iter().any(|t| t == dt));
    }
    if let Some(ref from) = params.date_from {
        results.retain(|r| r.date.as_str() >= from.as_str());
    }
    if let Some(ref to) = params.date_to {
        results.retain(|r| r.date.as_str() <= to.as_str());
    }
    results.truncate(top_k);

    // Phase 91 A2: 출력 PII mask. header(미리보기 텍스트)에 적용. config.search.output_pii_mask
    // 디폴트 true. 사용자 정의 패턴(pii_user_patterns)도 함께 적용.
    let mask_enabled = state.config.read().await.search.output_pii_mask;
    let user_patterns: Vec<(String, String)> = if mask_enabled {
        state.service.pii_user_patterns.read().expect("pii_user_patterns lock").clone()
    } else { Vec::new() };

    let docs: Vec<serde_json::Value> = results.iter().map(|r| {
        let header_raw = state.service.storage.read_header(&r.path, 10).unwrap_or_default();
        let header = if mask_enabled {
            file_pipeline_core::domain::classifier::SensitivityDetector::mask_pii_in_text(&header_raw, &user_patterns)
        } else { header_raw };
        serde_json::json!({
            "id": r.id, "score": r.score,
            "doc_types": r.doc_types, "date": r.date,
            "header": header,
        })
    }).collect();

    // Phase 95 A3 확장: Tauri search audit_trace 기록
    let summary = file_pipeline_core::audit::truncate_output_summary(
        &format!("results={} elapsed_ms={}", docs.len(), started.elapsed().as_millis())
    );
    state.service.audit.record(trace.as_str(), "tauri.search", Some(&inputs_hash), Some(&summary), Some("success"));

    Ok(serde_json::json!({"results": docs, "total": docs.len()}))
}

/// 검색 흐름을 Dense / Sparse(BM25) / Hybrid(RRF) / Filtered 단계별로 노출.
/// 각 단계마다 top_k 결과를 반환하므로 검색 시뮬레이션 UI가 단계별 비교 가능.

// ── 문서 목록 / 상세 ────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListParams {
    pub doc_type: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[tauri::command]
pub async fn list_documents(state: State<'_, AppState>, params: ListParams) -> Result<serde_json::Value, String> {
    let all = state.service.vector_db.list_all().unwrap_or_default();
    let filtered: Vec<_> = all.iter()
        .filter(|d| params.doc_type.as_ref()
            .map(|t| d.doc_types.iter().any(|dt| dt == t))
            .unwrap_or(true))
        .map(|d| serde_json::json!({
            "id": d.id, "path": d.path.to_string_lossy(),
            "doc_types": d.doc_types, "date": d.date,
        }))
        .collect();
    let total = filtered.len();
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).min(100);
    let total_pages = (total + per_page - 1) / per_page;
    let paginated: Vec<_> = filtered.into_iter().skip((page - 1) * per_page).take(per_page).collect();
    Ok(serde_json::json!({
        "documents": paginated, "total": total,
        "page": page, "per_page": per_page, "total_pages": total_pages,
    }))
}

#[tauri::command]
pub async fn get_document(state: State<'_, AppState>, doc_id: String) -> Result<serde_json::Value, String> {
    let all = state.service.vector_db.list_all().unwrap_or_default();
    let doc = match all.iter().find(|d| d.id == doc_id) {
        Some(d) => d,
        None => return Ok(serde_json::json!({"error": "문서 없음"})),
    };
    let content = match state.service.storage.decompress_temp(&doc.path) {
        Ok(temp) => {
            let c = std::fs::read_to_string(&temp).unwrap_or_default();
            let _ = std::fs::remove_file(&temp);
            c
        }
        Err(e) => return Ok(serde_json::json!({"error": e.to_string()})),
    };
    let relations = state.service.vector_db.find_related(&doc_id).unwrap_or_default();
    let meta = state.service.vector_db.get_metadata(&doc_id).ok().flatten();
    Ok(serde_json::json!({
        "id": doc.id, "doc_types": doc.doc_types, "content": content,
        "needs_verification": meta.as_ref().map(|m| m.needs_verification.clone()).unwrap_or_default(),
        "open_questions": meta.as_ref().map(|m| m.open_questions.clone()).unwrap_or_default(),
        "summary": meta.as_ref().map(|m| m.summary.clone()).unwrap_or_default(),
        "keywords": meta.as_ref().map(|m| m.keywords.clone()).unwrap_or_default(),
        "relations": relations.iter().map(|r| serde_json::json!({
            "target": r.target_id, "type": r.relation_type.to_string(),
        })).collect::<Vec<_>>(),
    }))
}

/// Phase 89 N-4: lint_strong_claims 결과를 UI에 즉시 노출.
/// max_per_doc 5 고정. 호출 비용: O(N * 평균 문서 크기) — Verification 탭 "주간 검토" 카드에서 호출.
#[tauri::command]
pub async fn get_lint_strong_claims(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    match file_pipeline_core::domain::lint::Linter::lint_strong_claims(
        state.service.vector_db.as_ref(),
        state.service.storage.as_ref(),
        5,
    ) {
        Ok(issues) => Ok(serde_json::json!({
            "total": issues.len(),
            "issues": issues.iter().map(|i| serde_json::json!({
                "doc_id": i.doc_id,
                "description": i.description,
            })).collect::<Vec<_>>(),
        })),
        Err(e) => Err(e.to_string()),
    }
}

// ── Lint ──────────────────────────────────────────────────────

// ── Knowledge Graph ──────────────────────────────────────────

#[tauri::command]
pub async fn kg_neighbors(state: State<'_, AppState>, doc_id: String) -> Result<serde_json::Value, String> {
    match file_pipeline_core::domain::wiki_export::KgQueryEngine::neighbors(
        state.service.vector_db.as_ref(), &doc_id,
    ) {
        Ok(result) => Ok(serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => Ok(serde_json::json!({"error": e.to_string()})),
    }
}

#[tauri::command]
pub async fn kg_paths(state: State<'_, AppState>, source: String, target: String) -> Result<serde_json::Value, String> {
    match file_pipeline_core::domain::wiki_export::KgQueryEngine::find_paths(
        state.service.vector_db.as_ref(), &source, &target,
    ) {
        Ok(result) => Ok(serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => Ok(serde_json::json!({"error": e.to_string()})),
    }
}

#[tauri::command]
pub async fn kg_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    match file_pipeline_core::domain::wiki_export::KgQueryEngine::stats(state.service.vector_db.as_ref()) {
        Ok(stats) => Ok(serde_json::to_value(&stats).unwrap_or_default()),
        Err(e) => Ok(serde_json::json!({"error": e.to_string()})),
    }
}

// ── 교차참조 통계 ───────────────────────────────────────────

#[tauri::command]
pub async fn get_crossref_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let all = state.service.vector_db.list_all().map_err(|e| e.to_string())?;
    let mut total_relations = 0u32;
    let mut by_type: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for doc in &all {
        let relations = state.service.vector_db.find_related(&doc.id).unwrap_or_default();
        total_relations += relations.len() as u32;
        for rel in &relations {
            *by_type.entry(rel.relation_type.to_string()).or_default() += 1;
        }
    }

    Ok(serde_json::json!({
        "total_documents": all.len(),
        "total_relations": total_relations,
        "by_type": by_type,
    }))
}

// ── 검증 메트릭 ──────────────────────────────────────────────

#[tauri::command]
pub async fn get_verification_metrics(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let metrics = state.verification_metrics.read().await;
    let total = metrics.len();
    let pass = metrics.iter().filter(|m| m.overall == "pass").count();
    let fail = metrics.iter().filter(|m| m.overall == "fail").count();
    let warn = metrics.iter().filter(|m| m.overall == "warning").count();
    let recent: Vec<_> = metrics.iter().rev().take(50).cloned().collect();
    Ok(serde_json::json!({
        "total": total, "pass": pass, "fail": fail, "warning": warn,
        "recent": recent,
    }))
}

// ── 진행률 ───────────────────────────────────────────────────

#[tauri::command]
pub async fn get_progress(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mut events = vec![];
    if let Some(ref tx) = state.progress_tx {
        let mut rx = tx.subscribe();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
            if events.len() >= 50 { break; }
        }
    }
    Ok(serde_json::json!({"events": events}))
}

// ── 큐 / 에러 ───────────────────────────────────────────────

#[tauri::command]
pub async fn get_queue(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let queue_path = state.topics_dir.parent().unwrap_or(&state.topics_dir).join(".work-queue.json");
    match file_pipeline_core::domain::work_queue::WorkQueue::load(&queue_path) {
        Ok(queue) => {
            let stats = queue.stats();
            let items: Vec<serde_json::Value> = queue.items().iter().map(|(name, item)| {
                let status_str = match &item.status {
                    file_pipeline_core::domain::work_queue::WorkStatus::Pending => "대기".to_string(),
                    file_pipeline_core::domain::work_queue::WorkStatus::Processing => "처리중".to_string(),
                    file_pipeline_core::domain::work_queue::WorkStatus::Done => "완료".to_string(),
                    file_pipeline_core::domain::work_queue::WorkStatus::Modified => "변경됨".to_string(),
                    file_pipeline_core::domain::work_queue::WorkStatus::Deleted => "삭제됨".to_string(),
                    file_pipeline_core::domain::work_queue::WorkStatus::Failed { reason, retries } =>
                        format!("실패 ({}회, {})", retries, reason),
                };
                serde_json::json!({
                    "name": name,
                    "path": item.path.to_string_lossy(),
                    "status": status_str,
                    "size_kb": item.size_bytes / 1024,
                    "size_bytes": item.size_bytes,
                    "is_large": item.is_large,
                    "created_at": item.created_at,
                    "updated_at": item.updated_at,
                })
            }).collect();
            Ok(serde_json::json!({
                "stats": serde_json::to_value(&stats).unwrap_or_default(),
                "items": items,
            }))
        }
        Err(_) => Ok(serde_json::json!({"stats": {"total": 0, "pending": 0, "done": 0}, "items": []})),
    }
}

/// 특정 파일명을 포함하는 pipeline.log 라인 추출 (Processing 탭 row 클릭 시 표시).
/// - pipeline.log (현재) + pipeline.log.{date} (롤링) 양쪽 스캔
/// - 파일명을 그대로 substring 매칭 (대소문자 구분, 한글/공백 그대로)
/// - 최대 max_lines (기본 200) 반환, 시간 오름차순
#[tauri::command]
pub async fn get_file_log(
    file_name: String,
    max_lines: Option<usize>,
) -> Result<serde_json::Value, String> {
    let limit = max_lines.unwrap_or(200);
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let logs_dir = data_dir.join("logs");

    let mut entries: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(read) = std::fs::read_dir(&logs_dir) {
        for e in read.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with("pipeline.log") {
                entries.push(p);
            }
        }
    }
    // 오래된 → 최신 순으로 읽기 (롤링은 .{date} 접미사가 큰 게 최신)
    entries.sort();

    let needle = file_name.as_str();
    let mut matched: Vec<String> = Vec::new();
    for path in &entries {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                if line.contains(needle) {
                    matched.push(line.to_string());
                    if matched.len() >= limit * 4 {
                        break;
                    }
                }
            }
        }
    }
    // 마지막 N개만 유지 (가장 최근 로그가 더 유용)
    if matched.len() > limit {
        let cut = matched.len() - limit;
        matched.drain(0..cut);
    }

    Ok(serde_json::json!({
        "file": file_name,
        "lines": matched,
        "truncated": matched.len() >= limit,
    }))
}

#[tauri::command]
pub async fn retry_failed(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let queue_path = state.topics_dir.parent().unwrap_or(&state.topics_dir).join(".work-queue.json");
    match file_pipeline_core::domain::work_queue::WorkQueue::load(&queue_path) {
        Ok(mut queue) => {
            let count = queue.retry_all_failed();
            queue.save(&queue_path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({"retried": count}))
        }
        Err(e) => Ok(serde_json::json!({"error": e.to_string()})),
    }
}

#[tauri::command]
pub async fn get_errors(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let log_path = state.topics_dir.parent().unwrap_or(&state.topics_dir).join(".error-log.json");
    let log = file_pipeline_core::domain::error_log::ErrorLog::load(&log_path);
    let recent = log.recent(50);
    let by_stage = log.count_by_stage();
    Ok(serde_json::json!({
        "total": log.entries.len(),
        "by_stage": by_stage,
        "recent": recent,
    }))
}

// ── Todos ────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_todos(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = file_pipeline_shared::settings_db::SettingsDb::open(&state.settings_db_path)
        .map_err(|e| e.to_string())?;
    let all = db.list_todos(None, None).map_err(|e| e.to_string())?;
    let pending = all.iter().filter(|t| t["status"] == "open").count();
    let completed = all.iter().filter(|t| t["status"] == "done").count();
    let items: Vec<serde_json::Value> = all.iter().map(|t| {
        serde_json::json!({
            "id": t["id"],
            "text": t["title"],
            "category": t["category"],
            "status": t["status"],
            "source_doc": t["doc_ids"],
            "date": t["created_at"],
            "due_date": t["due_date"],
            "completed": t["status"] == "done",
        })
    }).collect();
    Ok(serde_json::json!({ "items": items, "pending": pending, "completed": completed, "total": all.len() }))
}

#[tauri::command]
pub async fn complete_todo(state: State<'_, AppState>, todo_id: String) -> Result<serde_json::Value, String> {
    let db = file_pipeline_shared::settings_db::SettingsDb::open(&state.settings_db_path)
        .map_err(|e| e.to_string())?;
    let ok = db.complete_todo(&todo_id).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"ok": ok}))
}

#[tauri::command]
pub async fn add_todo(state: State<'_, AppState>, title: String, category: Option<String>, due_date: Option<String>) -> Result<serde_json::Value, String> {
    let db = file_pipeline_shared::settings_db::SettingsDb::open(&state.settings_db_path)
        .map_err(|e| e.to_string())?;
    let cat = category.as_deref().unwrap_or("manual");
    let fp = format!("{:x}", md5_fingerprint(&title));
    let id = db.add_todo(file_pipeline_shared::settings_db::NewTodo {
        title: &title, category: cat, doc_id: None, doc_description: None,
        fingerprint: &fp, source_line: None, source_text: None,
        due_date: due_date.as_deref(),
    }).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"ok": true, "id": id}))
}

fn md5_fingerprint(input: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

// ── 크레덴셜 ────────────────────────────────────────────────

#[tauri::command]
pub async fn list_credentials(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = state.config.read().await;
    let creds: Vec<serde_json::Value> = config.credentials.iter().map(|c| {
        serde_json::json!({
            "id": c.id,
            "name": c.name,
            "provider": c.provider,
            "has_api_key": c.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false),
            "url": c.url,
            "model": c.model,
            "profile_path": c.profile_path,
        })
    }).collect();
    Ok(serde_json::json!({"credentials": creds}))
}

#[tauri::command]
pub async fn save_credential(state: State<'_, AppState>, credential: serde_json::Value) -> Result<serde_json::Value, String> {
    let name = credential["name"].as_str().unwrap_or("").to_string();
    let provider = credential["provider"].as_str().unwrap_or("").to_string();
    if name.is_empty() || provider.is_empty() {
        return Ok(serde_json::json!({"error": "이름과 프로바이더는 필수입니다"}));
    }

    let id = credential["id"].as_str().unwrap_or("").to_string();
    let cred = file_pipeline_shared::config::LlmCredential {
        id: if id.is_empty() { file_pipeline_shared::config::generate_credential_id() } else { id.clone() },
        name: name.clone(),
        provider,
        api_key: credential["api_key"].as_str().map(|s| s.to_string()),
        url: credential["url"].as_str().map(|s| s.to_string()),
        model: credential["model"].as_str().map(|s| s.to_string()),
        profile_path: credential["profile_path"].as_str().map(|s| s.to_string()),
    };

    let mut config = state.config.write().await;
    // id 또는 이름으로 매칭하여 업데이트, 없으면 추가
    if !id.is_empty() {
        if let Some(existing) = config.credentials.iter_mut().find(|c| c.id == id) {
            *existing = cred.clone();
        } else {
            config.credentials.push(cred);
        }
    } else if let Some(existing) = config.credentials.iter_mut().find(|c| c.name == name) {
        let old_id = existing.id.clone();
        *existing = cred;
        existing.id = old_id; // 기존 ID 유지
    } else {
        config.credentials.push(cred);
    }

    // SettingsDb에 저장
    let db = file_pipeline_shared::settings_db::SettingsDb::open(&state.settings_db_path)
        .map_err(|e| e.to_string())?;
    db.migrate_from_config(&config).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({"ok": true}))
}

#[tauri::command]
pub async fn delete_credential(state: State<'_, AppState>, name: String) -> Result<serde_json::Value, String> {
    let mut config = state.config.write().await;
    config.credentials.retain(|c| c.name != name);

    // SettingsDb에 저장
    let db = file_pipeline_shared::settings_db::SettingsDb::open(&state.settings_db_path)
        .map_err(|e| e.to_string())?;
    db.migrate_from_config(&config).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({"ok": true}))
}

// ── 설정 ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = state.config.read().await;
    let mut config_json = serde_json::to_value(&*config).unwrap_or_default();
    mask_secrets(&mut config_json);

    let metadata = file_pipeline_shared::config::config_metadata();
    let meta_json: serde_json::Value = metadata.iter().map(|(section, fields)| {
        let fields_obj: serde_json::Value = fields.iter().map(|(field, meta)| {
            (field.to_string(), serde_json::json!({
                "description": meta.description,
                "field_type": meta.field_type,
                "default_value": meta.default_value,
                "requires_restart": meta.requires_restart,
            }))
        }).collect::<serde_json::Map<String, serde_json::Value>>().into();
        (section.to_string(), fields_obj)
    }).collect::<serde_json::Map<String, serde_json::Value>>().into();

    Ok(serde_json::json!({
        "config": config_json,
        "metadata": meta_json,
    }))
}

/// 마스킹된 시크릿 ("****")을 기존 config의 원본 값으로 복원 (lesson 12).
/// new에서 "****" 또는 빈 문자열인 시크릿 필드는 old의 값으로 되돌림.
/// credentials 배열은 항상 old를 사용 (save_credential / delete_credential로만 관리).
fn restore_masked_secrets(
    new_config: &mut file_pipeline_shared::config::PipelineConfig,
    old_config: &file_pipeline_shared::config::PipelineConfig,
) {
    fn restore(new: &mut Option<String>, old: &Option<String>) {
        if let Some(v) = new.as_ref() {
            if v == "****" || v.is_empty() {
                *new = old.clone();
            }
        }
    }
    // Notification (telegram / slack은 Option<Config>)
    if let (Some(new_tg), Some(old_tg)) = (new_config.notification.telegram.as_mut(), old_config.notification.telegram.as_ref()) {
        restore(&mut new_tg.bot_token, &old_tg.bot_token);
    }
    if let (Some(new_sl), Some(old_sl)) = (new_config.notification.slack.as_mut(), old_config.notification.slack.as_ref()) {
        restore(&mut new_sl.bot_token, &old_sl.bot_token);
    }
    // LLM API keys
    restore(&mut new_config.llm.anthropic_api_key, &old_config.llm.anthropic_api_key);
    restore(&mut new_config.llm.openai_api_key, &old_config.llm.openai_api_key);
    restore(&mut new_config.llm.gemini_api_key, &old_config.llm.gemini_api_key);
    // credentials 배열은 항상 old 사용
    new_config.credentials = old_config.credentials.clone();
}

fn mask_secret_at(config: &mut serde_json::Value, path: &[&str]) {
    let Some((&last, parents)) = path.split_last() else { return };
    let mut current = &mut *config;
    for key in parents {
        match current {
            serde_json::Value::Object(map) => {
                match map.get_mut(*key) {
                    Some(v) => current = v,
                    None => return,
                }
            }
            _ => return,
        }
    }
    if let serde_json::Value::Object(map) = current {
        if let Some(val) = map.get(last) {
            if val.as_str().map(|s| !s.is_empty()).unwrap_or(false) {
                map.insert(last.to_string(), serde_json::Value::String("****".into()));
            }
        }
    }
}

fn mask_secrets(config: &mut serde_json::Value) {
    mask_secret_at(config, &["notification", "telegram", "bot_token"]);
    mask_secret_at(config, &["notification", "slack", "bot_token"]);
    mask_secret_at(config, &["llm", "anthropic_api_key"]);
    mask_secret_at(config, &["llm", "openai_api_key"]);
    mask_secret_at(config, &["llm", "gemini_api_key"]);
}

#[tauri::command]
pub async fn save_config(state: State<'_, AppState>, config_json: String) -> Result<serde_json::Value, String> {
    let mut new_config: file_pipeline_shared::config::PipelineConfig = serde_json::from_str(&config_json)
        .map_err(|e| e.to_string())?;

    // 마스킹된 시크릿("****")은 기존 값으로 복원
    let old_config = state.config.read().await.clone();
    restore_masked_secrets(&mut new_config, &old_config);

    if let Err(errors) = new_config.validate() {
        return Ok(serde_json::json!({"errors": errors}));
    }

    let restart_required = old_config.needs_restart(&new_config);

    // SettingsDb에 저장
    let db = file_pipeline_shared::settings_db::SettingsDb::open(&state.settings_db_path)
        .map_err(|e| e.to_string())?;
    db.migrate_from_config(&new_config).map_err(|e| e.to_string())?;

    *state.config.write().await = new_config;

    Ok(serde_json::json!({
        "ok": true,
        "restart_required": restart_required,
    }))
}

#[tauri::command]
pub async fn export_config_toml(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = state.config.read().await;
    let toml_str = toml::to_string_pretty(&*config).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"toml": toml_str}))
}

#[tauri::command]
pub async fn import_config_toml(state: State<'_, AppState>, toml_content: String) -> Result<serde_json::Value, String> {
    let mut new_config: file_pipeline_shared::config::PipelineConfig = toml::from_str(&toml_content)
        .map_err(|e| format!("TOML 파싱 실패: {}", e))?;

    if let Err(errors) = new_config.validate() {
        return Ok(serde_json::json!({"errors": errors}));
    }

    // 기존 credentials 보존 (import에 credentials가 없으면)
    let old_config = state.config.read().await.clone();
    if new_config.credentials.is_empty() {
        new_config.credentials = old_config.credentials.clone();
    }

    let db = file_pipeline_shared::settings_db::SettingsDb::open(&state.settings_db_path)
        .map_err(|e| e.to_string())?;
    db.migrate_from_config(&new_config).map_err(|e| e.to_string())?;

    *state.config.write().await = new_config;

    Ok(serde_json::json!({"ok": true}))
}

// ── Purge ────────────────────────────────────────────────────
#[tauri::command]
pub async fn rebuild_embeddings(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let service = state.service.clone();
    let result = tokio::task::spawn_blocking(move || {
        let all = service.vector_db.list_all().map_err(|e| e.to_string())?;
        let total = all.len();
        let mut success = 0u32;
        let mut failed = 0u32;

        for doc in &all {
            let path = std::path::PathBuf::from(&doc.path);
            // .zst에서 텍스트 추출
            let text = match service.storage.decompress_temp(&path) {
                Ok(tmp) => {
                    let t = std::fs::read_to_string(&tmp).unwrap_or_default();
                    let _ = std::fs::remove_file(&tmp);
                    t
                }
                Err(_) => { failed += 1; continue; }
            };
            if text.is_empty() { failed += 1; continue; }

            // 임베딩 재생성 (동기 컨텍스트에서 block_on)
            let rt = tokio::runtime::Handle::current();
            let emb = service.embedding.clone();
            let embedding = match rt.block_on(async { emb.embed(&text).await }) {
                Ok(v) => v,
                Err(_) => { failed += 1; continue; }
            };

            // .vec 파일 저장
            let vec_path = path.with_extension("vec");
            let _ = file_pipeline_core::domain::vec_io::save_vec(&vec_path, &embedding);

            // 벡터DB 재색인 (기존 문서 위에 upsert)
            let updated_doc = file_pipeline_core::domain::models::Document {
                origin_path: path.clone(),
                compressed_origin: None,
                processed_path: Some(path),
                metadata: None,
                file_hash: doc.id.clone(),
                embedding,
            };
            match service.vector_db.upsert(&updated_doc) {
                Ok(_) => success += 1,
                Err(_) => failed += 1,
            }
        }

        Ok::<serde_json::Value, String>(serde_json::json!({
            "ok": true,
            "total": total,
            "success": success,
            "failed": failed,
            "message": format!("임베딩 재생성 완료: {}/{} 성공", success, total),
        }))
    })
    .await
    .map_err(|e| e.to_string())?;

    result
}

/// 전체 재가공: originals/*.zst를 inbox로 복사 → 배치 재처리
#[tauri::command]
pub async fn rebuild_all(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = state.config.read().await.clone();
    let paths = config.resolve_paths(None);

    // originals/*.zst → 해제 → inbox에 복사
    let originals_dir = &paths.originals;
    let inbox_dir = &paths.inbox;
    let mut copied = 0u32;

    if let Ok(entries) = std::fs::read_dir(originals_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("zst") { continue; }

            // 해제
            match state.service.storage.decompress_temp(&path) {
                Ok(tmp) => {
                    let filename = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let dest = inbox_dir.join(&filename);
                    if let Ok(_) = std::fs::copy(&tmp, &dest) {
                        copied += 1;
                    }
                    let _ = std::fs::remove_file(&tmp);
                }
                Err(_) => continue,
            }
        }
    }

    Ok(serde_json::json!({
        "ok": true,
        "copied": copied,
        "message": format!("{} 건의 원본을 inbox로 복사했습니다. 배치 처리를 실행하세요.", copied),
    }))
}

/// 벡터DB 재구축: 기존 DB 삭제 → processed/*.zst + *.vec로 재색인
#[tauri::command]
pub async fn rebuild_vectordb(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let service = state.service.clone();
    let result = tokio::task::spawn_blocking(move || {
        // 기존 DB 초기화
        service.vector_db.init().map_err(|e| e.to_string())?;

        let processed_dir = &service.processed_dir;
        let mut indexed = 0u32;
        let mut failed = 0u32;

        if let Ok(entries) = std::fs::read_dir(processed_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("zst") { continue; }

                // .vec 파일에서 임베딩 로드
                let vec_path = path.with_extension("vec");
                let embedding = match file_pipeline_core::domain::vec_io::load_vec(&vec_path) {
                    Ok(v) => v,
                    Err(_) => { failed += 1; continue; }
                };

                // 해시 생성 (파일명 기반)
                let hash = format!("{:x}", md5_hash(path.to_string_lossy().as_bytes()));

                let doc = file_pipeline_core::domain::models::Document {
                    origin_path: path.clone(),
                    compressed_origin: None,
                    processed_path: Some(path),
                    metadata: None,
                    file_hash: hash,
                    embedding,
                };
                match service.vector_db.upsert(&doc) {
                    Ok(_) => indexed += 1,
                    Err(_) => failed += 1,
                }
            }
        }

        Ok::<serde_json::Value, String>(serde_json::json!({
            "ok": true,
            "indexed": indexed,
            "failed": failed,
            "message": format!("벡터DB 재구축 완료: {} 건 색인", indexed),
        }))
    })
    .await
    .map_err(|e| e.to_string())?;

    result
}

fn md5_hash(data: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut h);
    h.finish()
}

// ── 시뮬레이션 dry-run ──────────────────────────────────────

/// 파이프라인 시뮬레이션: 실제 스텝을 실행하되 저장/색인/알림 스킵
/// 텍스트 입력 → 민감 판별 → Fragment → LLM 가공 → 검증 → 임베딩 → 결과 반환 (DB 미저장)
#[tauri::command]
pub async fn simulate_pipeline(
    state: State<'_, AppState>,
    input_text: String,
) -> Result<serde_json::Value, String> {
    let service = state.service.clone();
    let config = state.config.read().await.clone();

    let mut steps: Vec<serde_json::Value> = vec![];
    let start = std::time::Instant::now();

    // 1. 민감 판별 — Phase 91 A1': check_sensitive_and_pii 단일 진입점 사용.
    // 시뮬레이션은 가상 파일명(simulate.txt) + 본문을 함께 검사. 사용자 정의 PII 패턴도 적용.
    let step_start = std::time::Instant::now();
    let user_patterns: Vec<(String, String)> = service.pii_user_patterns.read()
        .expect("pii_user_patterns lock").clone();
    let decision = service.sensitivity_detector.check_sensitive_and_pii(
        std::path::Path::new("simulate.txt"),
        Some(&input_text),
        &user_patterns,
    );
    // config.sensitive.merged_keywords() 분기는 시뮬레이션 전용 — 사용자가 keywords를
    // 즉시 확인하려는 의도. SensitivityDetector는 self.keywords로 이미 기본 키워드 보유.
    let sensitive_by_content = config.sensitive.merged_keywords().iter()
        .any(|kw| input_text.to_lowercase().contains(&kw.to_lowercase()));
    let is_sens = decision.is_sensitive || sensitive_by_content;
    let reason_str = decision.reason.clone()
        .or_else(|| if sensitive_by_content { Some("내용에 민감 키워드 포함".into()) } else { None });
    steps.push(serde_json::json!({
        "name": "민감 판별",
        "status": if is_sens { "fail" } else { "pass" },
        "elapsed_ms": step_start.elapsed().as_millis(),
        "output": if is_sens { format!("민감 감지: {}", reason_str.unwrap_or_else(|| "기준 충족".into())) } else { "통과".into() },
    }));
    if is_sens {
        steps.push(serde_json::json!({"name": "이후 스텝", "status": "skip", "output": "민감 파일 → 중단"}));
        return Ok(serde_json::json!({"steps": steps, "total_ms": start.elapsed().as_millis()}));
    }

    // 2. Fragment 감지
    let step_start = std::time::Instant::now();
    let is_fragment = input_text.trim().len() <= config.schedule.fragment_threshold;
    steps.push(serde_json::json!({
        "name": "Fragment 감지",
        "status": if is_fragment { "skip" } else { "pass" },
        "elapsed_ms": step_start.elapsed().as_millis(),
        "output": if is_fragment { format!("{}자 ≤ {} → Fragment", input_text.trim().len(), config.schedule.fragment_threshold) }
                  else { format!("{}자 > {} → 통과", input_text.trim().len(), config.schedule.fragment_threshold) },
    }));

    // 3. LLM 분류+가공 (실제 호출!)
    let step_start = std::time::Instant::now();
    if !is_fragment {
        let llm = service.llm.clone();
        let registry = service.registry.clone();
        let text_clone = input_text.clone();
        let llm_result = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                llm.classify_and_process_text("simulate.txt", &text_clone, &registry).await
            })
        })
        .await
        .map_err(|e| e.to_string())?;

        match llm_result {
            Ok(result) => {
                let elapsed = step_start.elapsed().as_millis();

                // LLM 결과
                steps.push(serde_json::json!({
                    "name": "LLM 분류+가공",
                    "status": "pass",
                    "elapsed_ms": elapsed,
                    "output": format!("유형: [{}], 키워드: {}, 요약: {}",
                        result.doc_types.join(", "),
                        result.metadata.keywords.len(),
                        result.metadata.summary.chars().take(100).collect::<String>()),
                    "doc_types": result.doc_types,
                    "keywords": result.metadata.keywords,
                    "summary": result.metadata.summary,
                    "content_length": result.content.len(),
                }));

                // 4. 검증
                let step_start = std::time::Instant::now();
                if config.verification.enabled {
                    let required_sections = service.registry.sections_for_types(&result.doc_types);
                    let thresholds = service.registry.thresholds_for_types(&result.doc_types)
                        .or_else(|| service.global_thresholds.clone())
                        .unwrap_or_default();
                    let verification = file_pipeline_core::domain::verification::verify_with_thresholds(
                        &input_text, &result.content, &required_sections,
                        &result.metadata.keywords, result.sections.as_ref(), &thresholds,
                    );
                    let status = match &verification.overall {
                        file_pipeline_core::domain::models::VerificationLevel::Pass => "pass",
                        file_pipeline_core::domain::models::VerificationLevel::Warning(_) => "warning",
                        file_pipeline_core::domain::models::VerificationLevel::Fail(_) => "fail",
                    };
                    steps.push(serde_json::json!({
                        "name": "검증",
                        "status": status,
                        "elapsed_ms": step_start.elapsed().as_millis(),
                        "output": format!("구조:{:.0}% 압축:{:.0}% 키워드:{:.0}% ROUGE:{:.0}%",
                            verification.structure_completeness * 100.0,
                            verification.compression_ratio * 100.0,
                            verification.keyword_coverage * 100.0,
                            verification.rouge_l_recall * 100.0),
                        "details": verification.details,
                    }));
                } else {
                    steps.push(serde_json::json!({"name": "검증", "status": "skip", "elapsed_ms": 0, "output": "비활성화"}));
                }

                // 5. 임베딩
                let step_start = std::time::Instant::now();
                let emb = service.embedding.clone();
                let content = result.content.clone();
                let emb_result = tokio::task::spawn_blocking(move || {
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async { emb.embed(&content).await })
                }).await.map_err(|e| e.to_string())?;

                match emb_result {
                    Ok(vec) => {
                        steps.push(serde_json::json!({
                            "name": "임베딩",
                            "status": "pass",
                            "elapsed_ms": step_start.elapsed().as_millis(),
                            "output": format!("{}차원 벡터 생성", vec.len()),
                        }));
                    }
                    Err(e) => {
                        steps.push(serde_json::json!({
                            "name": "임베딩",
                            "status": "fail",
                            "elapsed_ms": step_start.elapsed().as_millis(),
                            "output": format!("실패: {}", e),
                        }));
                    }
                }

                // 후처리 (dry-run: 저장/색인/알림 스킵)
                steps.push(serde_json::json!({"name": "저장+압축", "status": "skip", "elapsed_ms": 0, "output": "[dry-run] 스킵"}));
                steps.push(serde_json::json!({"name": "벡터DB 색인", "status": "skip", "elapsed_ms": 0, "output": "[dry-run] 스킵"}));
                steps.push(serde_json::json!({"name": "교차참조", "status": "skip", "elapsed_ms": 0, "output": "[dry-run] 스킵"}));
                steps.push(serde_json::json!({"name": "알림", "status": "skip", "elapsed_ms": 0, "output": "[dry-run] 스킵"}));
            }
            Err(e) => {
                steps.push(serde_json::json!({
                    "name": "LLM 분류+가공",
                    "status": "fail",
                    "elapsed_ms": step_start.elapsed().as_millis(),
                    "output": format!("실패: {}", e),
                }));
            }
        }
    } else {
        steps.push(serde_json::json!({"name": "LLM 분류+가공", "status": "skip", "elapsed_ms": 0, "output": "Fragment → 스킵"}));
        steps.push(serde_json::json!({"name": "검증", "status": "skip", "elapsed_ms": 0, "output": "Fragment → 스킵"}));

        // Fragment 임베딩
        let step_start = std::time::Instant::now();
        let emb = service.embedding.clone();
        let text = input_text.clone();
        let emb_result = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async { emb.embed(&text).await })
        }).await.map_err(|e| e.to_string())?;
        match emb_result {
            Ok(vec) => steps.push(serde_json::json!({"name": "임베딩 (Fragment)", "status": "pass", "elapsed_ms": step_start.elapsed().as_millis(), "output": format!("{}차원", vec.len())})),
            Err(e) => steps.push(serde_json::json!({"name": "임베딩 (Fragment)", "status": "fail", "elapsed_ms": step_start.elapsed().as_millis(), "output": format!("{}", e)})),
        }
        steps.push(serde_json::json!({"name": "저장/색인", "status": "skip", "elapsed_ms": 0, "output": "[dry-run] 스킵"}));
    }

    Ok(serde_json::json!({
        "steps": steps,
        "total_ms": start.elapsed().as_millis(),
        "dry_run": true,
    }))
}

// ── Topics ───────────────────────────────────────────────────

#[tauri::command]
pub async fn list_topics(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mut topics = vec![];
    fn walk(dir: &std::path::Path, base: &std::path::Path, out: &mut Vec<serde_json::Value>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, base, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let rel = path.strip_prefix(base).unwrap_or(&path);
                let doc_type = rel.parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                out.push(serde_json::json!({
                    "path": rel.to_string_lossy().replace('\\', "/"),
                    "name": path.file_stem().and_then(|n| n.to_str()).unwrap_or(""),
                    "doc_type": doc_type,
                    "size": size,
                }));
            }
        }
    }
    walk(&state.topics_dir, &state.topics_dir, &mut topics);
    Ok(serde_json::json!({"topics": topics, "total": topics.len()}))
}

#[tauri::command]
pub async fn get_topic(state: State<'_, AppState>, path: String) -> Result<serde_json::Value, String> {
    let full_path = state.topics_dir.join(&path);
    match std::fs::read_to_string(&full_path) {
        Ok(content) => Ok(serde_json::json!({"filename": path, "content": content})),
        Err(e) => Ok(serde_json::json!({"error": e.to_string()})),
    }
}

#[tauri::command]
pub async fn update_topic(state: State<'_, AppState>, path: String, content: String) -> Result<serde_json::Value, String> {
    let full_path = state.topics_dir.join(&path);
    if let Some(parent) = full_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&full_path, &content) {
        Ok(()) => Ok(serde_json::json!({"ok": true})),
        Err(e) => Ok(serde_json::json!({"error": e.to_string()})),
    }
}

// ── 파이프라인 ───────────────────────────────────────────────

// ── 문서 유형 (settings.db) ──────────────────────────────
#[tauri::command]
pub async fn get_token_usage(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let usage = state.service.token_usage.lock().expect("token_usage mutex");
    Ok(serde_json::to_value(&*usage).unwrap_or_default())
}

// ── inbox 감지 제어 ─────────────────────────────────────────

#[tauri::command]
pub async fn get_watcher_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let active = state.watcher_active.load(std::sync::atomic::Ordering::Relaxed);
    Ok(serde_json::json!({"active": active}))
}

#[tauri::command]
pub async fn set_watcher_active(state: State<'_, AppState>, active: bool) -> Result<serde_json::Value, String> {
    state.watcher_active.store(active, std::sync::atomic::Ordering::Relaxed);
    tracing::info!("inbox 감지: {}", if active { "활성화" } else { "일시 정지" });
    file_pipeline_shared::write_log("INFO", &format!("inbox 감지: {}", if active { "ON" } else { "OFF" }));
    Ok(serde_json::json!({"ok": true, "active": active}))
}

// ── 호스트 도구 현황 (Phase 81: settings.db 캐시) ──────────────

fn build_host_tools_response(tools: Vec<(file_pipeline_adapters::driven::preprocessing::preprocessor::HostTool, String)>) -> serde_json::Value {
    let list: Vec<serde_json::Value> = tools.iter().map(|(tool, ver)| {
        serde_json::json!({"tool": format!("{:?}", tool), "version": ver})
    }).collect();
    let exts = ["docx", "xlsx", "pptx", "hwp"];
    let support: Vec<serde_json::Value> = exts.iter().map(|ext| {
        let best = file_pipeline_adapters::driven::preprocessing::preprocessor::HostToolDetector::best_tool_for(ext, &tools);
        serde_json::json!({
            "extension": ext,
            "supported": best.is_some(),
            "tool": best.map(|t| format!("{:?}", t)),
        })
    }).collect();
    serde_json::json!({"tools": list, "extensions": support})
}

#[tauri::command]
pub async fn get_host_tools() -> Result<serde_json::Value, String> {
    // Phase 81: settings.db 캐시 사용. 비었으면 1회 감지 + 저장.
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open(&data_dir.join("settings.db"))
        .map_err(|e| e.to_string())?;
    let tools = file_pipeline_shared::host_tools_cache::ensure_cached(&db)
        .map_err(|e| e.to_string())?;
    Ok(build_host_tools_response(tools))
}

#[tauri::command]
pub async fn test_host_tool(tool: String) -> Result<serde_json::Value, String> {
    let (cmd, args, check): (&str, Vec<&str>, &str) = match tool.as_str() {
        "pandoc" => ("pandoc", vec!["--version"], "pandoc"),
        "python-docx" | "python_docx" => ("python", vec!["-c", "import docx; print('OK:', docx.__version__)"], "python-docx"),
        "openpyxl" => ("python", vec!["-c", "import openpyxl; print('OK:', openpyxl.__version__)"], "openpyxl"),
        "libreoffice" => ("soffice", vec!["--version"], "LibreOffice"),
        "tesseract" => ("tesseract", vec!["--version"], "Tesseract"),
        "marker" => ("marker_single", vec!["--help"], "Marker"),
        "pymupdf4llm" => ("python", vec!["-c", "import pymupdf4llm; print('OK')"], "PyMuPDF4LLM"),
        _ => return Ok(serde_json::json!({"ok": false, "error": format!("알 수 없는 도구: {}", tool)})),
    };

    let mut command = std::process::Command::new(cmd);
    command.args(&args);
    #[cfg(windows)]
    { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }

    match command.output() {
        Ok(output) if output.status.success() => {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let version = ver.lines().next().unwrap_or(check).to_string();
            Ok(serde_json::json!({"ok": true, "tool": check, "version": version}))
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Ok(serde_json::json!({"ok": false, "tool": check, "error": format!("실행 실패: {}", stderr)}))
        }
        Err(e) => {
            Ok(serde_json::json!({"ok": false, "tool": check, "error": format!("도구를 찾을 수 없습니다: {}. 설치가 필요합니다.", e)}))
        }
    }
}

// ── Preprocess 테스트 ───────────────────────────────────────
#[tauri::command]
pub fn get_prompts(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    // DB에서 프롬프트 로드 시도, 없으면 기존 RwLock 내용
    let db = file_pipeline_shared::settings_db::SettingsDb::open(&state.settings_db_path)
        .map_err(|e| e.to_string())?;
    let classify = db.get_prompt("classify").ok().flatten();
    let reprocess = db.get_prompt("reprocess_suffix").ok().flatten();
    let summarize_text = db.get_prompt("summarize_text").ok().flatten();

    if classify.is_some() || reprocess.is_some() || summarize_text.is_some() {
        let content = format!(
            "[classify]\ntemplate = \"\"\"{}\"\"\"\n\n[reprocess]\nsuffix = \"\"\"{}\"\"\"\n\n[summarize_text]\ntemplate = \"\"\"{}\"\"\"",
            classify.as_deref().unwrap_or(""),
            reprocess.as_deref().unwrap_or(""),
            summarize_text.as_deref().unwrap_or(""),
        );
        Ok(serde_json::json!({"content": content}))
    } else {
        let content = file_pipeline_adapters::driven::llm::prompts::get_prompts_content();
        Ok(serde_json::json!({"content": content}))
    }
}

#[tauri::command]
pub fn save_prompts(state: State<'_, AppState>, content: String) -> Result<serde_json::Value, String> {
    // TOML 파싱 검증
    let table: toml::Table = content.parse().map_err(|e: toml::de::Error| format!("TOML 파싱 오류: {e}"))?;

    // DB에 저장
    let db = file_pipeline_shared::settings_db::SettingsDb::open(&state.settings_db_path)
        .map_err(|e| e.to_string())?;

    if let Some(c) = table.get("classify").and_then(|v| v.get("template")).and_then(|v| v.as_str()) {
        db.set_prompt("classify", c).map_err(|e| e.to_string())?;
    }
    if let Some(r) = table.get("reprocess").and_then(|v| v.get("suffix")).and_then(|v| v.as_str()) {
        db.set_prompt("reprocess_suffix", r).map_err(|e| e.to_string())?;
    }
    if let Some(m) = table.get("summarize_text").and_then(|v| v.get("template")).and_then(|v| v.as_str()) {
        db.set_prompt("summarize_text", m).map_err(|e| e.to_string())?;
    }

    // RwLock에도 주입 (핫 리로드)
    file_pipeline_adapters::driven::llm::prompts::inject_prompts(
        table.get("classify").and_then(|v| v.get("template")).and_then(|v| v.as_str()),
        table.get("reprocess").and_then(|v| v.get("suffix")).and_then(|v| v.as_str()),
        table.get("summarize_text").and_then(|v| v.get("template")).and_then(|v| v.as_str()),
    );

    Ok(serde_json::json!({"ok": true}))
}

// Phase 76: 다축 SetupProfile 기반 설정 리뷰
#[tauri::command]
pub async fn setup_review(
    scenario: Option<String>,
    user_role: Option<String>,
    profile: Option<file_pipeline_shared::setup_review::SetupProfile>,
) -> std::result::Result<serde_json::Value, String>
{
    let cfg_path = file_pipeline_shared::config::find_config_path(None);
    let current = file_pipeline_shared::config::PipelineConfig::load(&cfg_path)
        .unwrap_or_else(|_| file_pipeline_shared::config::PipelineConfig::default_config());

    let advice = if let Some(p) = profile {
        file_pipeline_shared::setup_review::build_advice_from_profile(p, &current)
    } else {
        let s = scenario.unwrap_or_default();
        if s.trim().is_empty() { return Err("scenario 또는 profile 중 하나 필수".into()); }
        file_pipeline_shared::setup_review::build_advice(&s, user_role, &current)
    };
    serde_json::to_value(advice).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn setup_apply(
    scenario: Option<String>,
    accepted_paths: Vec<String>,
    profile: Option<file_pipeline_shared::setup_review::SetupProfile>,
    apply_critical: Option<bool>,
) -> std::result::Result<serde_json::Value, String>
{
    let cfg_path = file_pipeline_shared::config::find_config_path(None);
    let current = file_pipeline_shared::config::PipelineConfig::load(&cfg_path)
        .unwrap_or_else(|_| file_pipeline_shared::config::PipelineConfig::default_config());

    let advice = if let Some(p) = profile {
        file_pipeline_shared::setup_review::build_advice_from_profile(p, &current)
    } else {
        let s = scenario.unwrap_or_default();
        if s.trim().is_empty() { return Err("scenario 또는 profile 중 하나 필수".into()); }
        file_pipeline_shared::setup_review::build_advice(&s, None, &current)
    };
    let critical = apply_critical.unwrap_or(false);
    // settings.db로 snapshot 저장 시도 (실패해도 apply 진행)
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir).ok();
    let result = file_pipeline_shared::setup_review::apply_advice_full(
        &cfg_path, &advice, &accepted_paths, critical, db.as_ref(),
    ).map_err(|e| e.to_string())?;
    let needs_restart = advice.changes.iter()
        .filter(|c| result.applied.contains(&c.path))
        .any(|c| c.needs_restart);
    Ok(serde_json::json!({
        "applied": result.applied,
        "snapshot_id": result.snapshot_id,
        "backup": cfg_path.with_extension("toml.bak").to_string_lossy(),
        "needs_restart": needs_restart,
    }))
}

// Phase 77: 스냅샷 관리
#[tauri::command]
pub async fn setup_snapshot_list(limit: Option<u32>)
    -> std::result::Result<serde_json::Value, String>
{
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let snaps = db.list_snapshots(limit.unwrap_or(20) as usize).map_err(|e| e.to_string())?;
    let out: Vec<_> = snaps.into_iter().map(|s| serde_json::json!({
        "id": s.id,
        "created_at": s.created_at,
        "applied_paths": s.applied_paths,
        "rolled_back": s.rolled_back,
        "rollback_reason": s.rollback_reason,
        "has_metrics": s.metrics_json.is_some(),
    })).collect();
    Ok(serde_json::json!({ "snapshots": out, "total": out.len() }))
}

#[tauri::command]
pub async fn setup_snapshot_rollback(snapshot_id: String, reason: String)
    -> std::result::Result<serde_json::Value, String>
{
    if snapshot_id.is_empty() { return Err("snapshot_id 필수".into()); }
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let snap = db.get_snapshot(&snapshot_id).map_err(|e| e.to_string())?
        .ok_or_else(|| format!("스냅샷 없음: {}", snapshot_id))?;
    let cfg_path = file_pipeline_shared::config::find_config_path(None);
    file_pipeline_shared::config_snapshot::rollback_snapshot(&cfg_path, &snap, &reason)
        .map_err(|e| e.to_string())?;
    db.mark_snapshot_rolled_back(&snapshot_id, &reason).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true, "snapshot_id": snapshot_id }))
}

// Phase 82: Decision Log 조회 — accepted 항목에는 연결된 snapshot의 rolled_back 플래그를 합성하여
// 프론트엔드가 Rollback 버튼 노출 여부를 결정할 수 있게 한다.
#[tauri::command]
pub async fn setup_decision_log_list(limit: Option<u32>, snapshot_id: Option<String>)
    -> std::result::Result<serde_json::Value, String>
{
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let entries = if let Some(snap_id) = snapshot_id {
        db.list_decisions_by_snapshot(&snap_id).map_err(|e| e.to_string())?
    } else {
        db.list_decisions(limit.unwrap_or(50) as usize).map_err(|e| e.to_string())?
    };

    // snapshot 상태 cross-reference: snapshot_id → rolled_back 매핑
    let snaps = db.list_snapshots(500).unwrap_or_default();
    let snap_status: std::collections::HashMap<String, bool> = snaps
        .into_iter()
        .map(|s| (s.id, s.rolled_back))
        .collect();

    let enriched: Vec<serde_json::Value> = entries.into_iter().map(|e| {
        let mut v = serde_json::to_value(&e).unwrap_or(serde_json::json!({}));
        let rolled_back = e.snapshot_id.as_deref()
            .and_then(|sid| snap_status.get(sid).copied())
            .unwrap_or(false);
        if let serde_json::Value::Object(ref mut map) = v {
            map.insert("rolled_back".into(), serde_json::Value::Bool(rolled_back));
        }
        v
    }).collect();

    Ok(serde_json::json!({ "count": enriched.len(), "entries": enriched }))
}

// ── Phase 80: 코퍼스 신호 카운터 ────────────────────────────
//
// MCP 도구와 동일한 데이터를 Tauri에서도 노출 (lesson 19 frontend-backend 매핑 정합성).
// settings.db의 영속 카운터를 직접 읽어 GUI Dashboard에서 신호 분포를 표시.

/// 검색 mode 누적 카운터 분포 (default/exact/related/recent/fusion)
#[tauri::command]
pub async fn get_search_mode_stats() -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let rows = db.get_search_mode_counters().map_err(|e| e.to_string())?;
    let total: u64 = rows.iter().map(|(_, c, _)| *c).sum();
    let items: Vec<serde_json::Value> = rows.into_iter().map(|(mode, count, last_at)| {
        let ratio = if total > 0 { count as f32 / total as f32 } else { 0.0 };
        serde_json::json!({ "mode": mode, "count": count, "ratio": ratio, "last_at": last_at })
    }).collect();
    Ok(serde_json::json!({ "modes": items, "total": total }))
}

/// CRAG 신뢰도 누적 카운터 (correct/ambiguous/incorrect)
#[tauri::command]
pub async fn get_crag_stats() -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let rows = db.get_crag_counters().map_err(|e| e.to_string())?;
    let mut counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for (bucket, count, _) in rows { counts.insert(bucket, count); }
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

/// 코퍼스 청크 통계 — 가공본 헤더 샘플링 추정
#[tauri::command]
pub async fn get_chunk_stats(
    state: State<'_, AppState>,
    sample_size: Option<u64>,
) -> std::result::Result<serde_json::Value, String> {
    let sample_size = sample_size.unwrap_or(50) as usize;
    let docs = state.service.vector_db.list_all().unwrap_or_default();
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
        if let Ok(header) = state.service.storage.read_header(&d.path, 50) {
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

/// 처리 메트릭 — verify_pass_rate / quarantine_rate / avg_process_time_ms + 코퍼스 카운터.
/// settings.db 누적 카운터 + service.summary 런타임 스냅샷을 함께 노출 (Phase 80 placeholder 해소).
#[tauri::command]
pub async fn get_processing_metrics(
    state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, String> {
    let stats = state.service.vector_db.stats().map_err(|e| e.to_string())?;
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let summary = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
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

    // 런타임 ProcessingSummary (service 시작 후 누적, 프로세스 재시작 시 리셋).
    // 영속 카운터와 분리 노출하여 "현 세션 vs 전체 누적" 비교 가능.
    let runtime = state.service.summary.lock().ok()
        .map(|s| serde_json::json!({
            "success": s.success,
            "errors": s.errors,
            "duplicates": s.duplicates,
            "sensitive": s.sensitive,
        }))
        .unwrap_or(serde_json::json!({}));

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
        "runtime_summary": runtime,
    }))
}

/// Ruflo C1 1단계: 누적 카운터 분석 → decision_log 자동 추천 INSERT
#[tauri::command]
pub async fn auto_suggest_from_counters(
    _state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let inserted = file_pipeline_shared::auto_suggester::suggest_from_counters(&db)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "inserted": inserted,
        "next": "Decision Log에서 source='auto_suggestion' 항목 검토",
    }))
}

/// Ruflo C1 2단계: suggested → accepted (pipeline.toml 적용)
#[tauri::command]
pub async fn accept_suggested_decision(
    _state: State<'_, AppState>,
    decision_id: i64,
) -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let cfg_path = file_pipeline_shared::config::find_config_path(None);
    let (path, after_value) = file_pipeline_shared::auto_suggester::apply_suggested(
        &db, &cfg_path, decision_id,
    ).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "applied": true,
        "path": path,
        "after_value": after_value,
        "backup": cfg_path.with_extension("toml.bak").to_string_lossy(),
    }))
}

/// Ruflo C1 2단계: suggested → rejected (config 변경 없음)
#[tauri::command]
pub async fn reject_suggested_decision(
    _state: State<'_, AppState>,
    decision_id: i64,
) -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    file_pipeline_shared::auto_suggester::reject_suggested(&db, decision_id)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "rejected": true, "decision_id": decision_id }))
}

/// Ruflo C1: 룰 임계값 목록 (오버라이드 + 디폴트)
#[tauri::command]
pub async fn c1_thresholds_list(
    _state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let rows = db.list_c1_thresholds().map_err(|e| e.to_string())?;
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

/// Ruflo C1: 룰 임계값 upsert
#[tauri::command]
pub async fn c1_threshold_set(
    _state: State<'_, AppState>,
    key: String,
    value: f64,
) -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    db.set_c1_threshold(&key, value).map_err(|e| e.to_string())?;
    // C1 임계값은 auto_suggester가 매 호출마다 settings.db에서 read하므로 별도 reload 불필요 (live reload).
    Ok(serde_json::json!({ "ok": true, "key": key, "value": value, "live_reloaded": true }))
}

/// Ruflo C2: 사용자 정의 PII 패턴 목록
#[tauri::command]
pub async fn pii_patterns_list(
    _state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let rows = db.list_user_pii_patterns().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "user_patterns": rows.into_iter().map(|(n, p, e)| serde_json::json!({
            "name": n, "pattern": p, "enabled": e
        })).collect::<Vec<_>>(),
        "builtin": ["ssn_kr", "credit_card", "email", "phone_kr", "biz_reg_kr"],
    }))
}

/// settings.db에서 활성화된 사용자 PII 패턴을 읽어 service에 live reload.
fn reload_service_pii(state: &State<'_, AppState>) -> Result<usize, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let patterns: Vec<(String, String)> = db.list_user_pii_patterns()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|(_, _, enabled)| *enabled)
        .map(|(n, p, _)| (n, p))
        .collect();
    state.service.reload_pii_patterns(patterns).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pii_pattern_add(
    state: State<'_, AppState>,
    name: String,
    pattern: String,
    enabled: Option<bool>,
) -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    db.add_user_pii_pattern(&name, &pattern, enabled.unwrap_or(true))
        .map_err(|e| e.to_string())?;
    let active_count = reload_service_pii(&state)?;
    Ok(serde_json::json!({ "ok": true, "name": name, "active_count": active_count, "live_reloaded": true }))
}

#[tauri::command]
pub async fn pii_pattern_remove(
    state: State<'_, AppState>,
    name: String,
) -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let removed = db.remove_user_pii_pattern(&name).map_err(|e| e.to_string())?;
    let active_count = reload_service_pii(&state)?;
    Ok(serde_json::json!({ "removed": removed, "name": name, "active_count": active_count, "live_reloaded": true }))
}

/// Ruflo A1: LLM 결과 캐시 전체 비우기
#[tauri::command]
pub async fn clear_llm_cache(
    _state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let deleted = db.clear_llm_cache().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "deleted": deleted }))
}

/// Ruflo A1: LLM 결과 캐시 통계 (entries / total_hits / avg_hits_per_entry) + 마지막 GC 정보
#[tauri::command]
pub async fn get_llm_cache_stats(
    _state: State<'_, AppState>,
) -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db_opt = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir).ok();
    let (entries, total_hits, avg_hits) = db_opt.as_ref()
        .and_then(|db| db.llm_cache_stats().ok())
        .unwrap_or((0, 0, 0.0));
    // 마지막 GC 결과 (성공 시에만 노출. 실패해도 stats는 반환)
    let last_gc = db_opt.as_ref().and_then(|db| db.get_last_llm_cache_gc().ok().flatten())
        .map(|(at, deleted)| serde_json::json!({ "at": at, "deleted": deleted }))
        .unwrap_or(serde_json::Value::Null);
    Ok(serde_json::json!({
        "entries": entries,
        "total_hits": total_hits,
        "avg_hits_per_entry": avg_hits,
        "last_gc": last_gc,
    }))
}

/// Ruflo A1: LRU GC 즉시 트리거. max_entries 초과분을 hits ASC, last_hit_at NULL ASC로 삭제.
/// max_entries=0이면 config의 llm.llm_cache_max_entries 사용.
#[tauri::command]
pub async fn gc_llm_cache_now(
    state: State<'_, AppState>,
    max_entries: Option<u64>,
) -> std::result::Result<serde_json::Value, String> {
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir)
        .map_err(|e| e.to_string())?;
    let cap = match max_entries {
        Some(v) if v > 0 => v,
        _ => {
            let cfg = state.config.read().await;
            cfg.llm.llm_cache_max_entries
        }
    };
    let deleted = db.gc_llm_cache_to(cap).map_err(|e| e.to_string())?;
    // 결과를 settings.db에 기록 (last_gc 카드용)
    let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let _ = db.record_llm_cache_gc(&now, deleted as i64);
    Ok(serde_json::json!({
        "deleted": deleted,
        "max_entries": cap,
        "at": now,
    }))
}

// ── Phase 80: 동작 모듈 ─────────────────────────────────────

/// 사용 가능한 동작 모듈 목록 (가공·검색·운영 그룹별)
/// MCP 도구 목록 + enabled 상태 + (가능하면) 호출 통계.
#[tauri::command]
pub async fn setup_modules_list() -> std::result::Result<serde_json::Value, String> {
    let registry = file_pipeline_shared::setup_modules::ModuleRegistry::default_registry();
    let modules: Vec<serde_json::Value> = registry.all().iter().map(|m| serde_json::json!({
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

/// 선택된 동작 모듈 ID들을 합집합으로 적용 (또는 dryrun)
#[tauri::command]
pub async fn setup_apply_modules(
    module_ids: Vec<String>,
    apply_critical: Option<bool>,
    dryrun: Option<bool>,
) -> std::result::Result<serde_json::Value, String> {
    let apply_critical = apply_critical.unwrap_or(false);
    let dryrun = dryrun.unwrap_or(false);

    let cfg_path = file_pipeline_shared::config::find_config_path(None);
    let current = file_pipeline_shared::config::PipelineConfig::load(&cfg_path)
        .unwrap_or_else(|_| file_pipeline_shared::config::PipelineConfig::default_config());
    let registry = file_pipeline_shared::setup_modules::ModuleRegistry::default_registry();
    let changes = registry.build_changes(&module_ids, &current).map_err(|e| e.to_string())?;

    if dryrun {
        return Ok(serde_json::json!({
            "dryrun": true,
            "module_ids": module_ids,
            "changes": changes,
            "change_count": changes.len(),
        }));
    }

    let profile = file_pipeline_shared::setup_review::SetupProfile {
        description: Some(format!("modules: {}", module_ids.join(", "))),
        ..Default::default()
    };
    let advice = file_pipeline_shared::setup_review::SetupAdvice {
        profile,
        scenario: "modules".into(),
        summary: format!("{}개 모듈 합집합", module_ids.len()),
        changes: changes.clone(),
    };
    let accepted: Vec<String> = changes.iter().map(|c| c.path.clone()).collect();
    let data_dir = file_pipeline_shared::config::find_data_dir(None);
    let db = file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir).ok();
    let context = serde_json::json!({ "module_ids": module_ids });
    let result = file_pipeline_shared::setup_review::apply_advice_full_with_log(
        &cfg_path, &advice, &accepted, apply_critical, db.as_ref(),
        "setup_modules", Some(&context),
    ).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "applied": result.applied,
        "snapshot_id": result.snapshot_id,
        "module_ids": module_ids,
        "backup": cfg_path.with_extension("toml.bak").to_string_lossy(),
    }))
}

// ── Phase 93: GUI 가시화 4 commands (Phase 91 A2 / 92 H1·H3·H5) ─────────────

/// Phase 92 H1: audit_trace 최근 N건 분석 + 이상 신호 반환.
/// JAMES 자체 진화 게이트 흡수 (RBAC 보류, 사용자 검토 권고).
#[tauri::command]
pub async fn get_anomaly_report(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    use file_pipeline_shared::audit_anomaly::{analyze_recent_audit, AnomalyThresholds};
    use file_pipeline_shared::settings_db::SettingsDb;

    let db = SettingsDb::open(&state.settings_db_path).map_err(|e| e.to_string())?;
    let thresholds = AnomalyThresholds::default();
    let report = analyze_recent_audit(&db, &thresholds).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "signals": report.signals.iter().map(|s| serde_json::json!({
            "kind": s.kind,
            "stage": s.stage,
            "summary": s.summary,
            "recommendation": s.recommendation,
        })).collect::<Vec<_>>(),
        "examined_events": report.examined_events,
        "has_anomaly": report.has_anomaly(),
    }))
}

/// Phase 92 H3: MCP 도구 다차원 분류 카탈로그 반환.
/// Mirage Command 3차원 등록 패턴 흡수.
#[tauri::command]
pub async fn get_mcp_tool_catalog_full(_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let catalog = file_pipeline_shared::mcp_server::mcp_tool_catalog_full();
    Ok(serde_json::json!({
        "tools": catalog.iter().map(|m| serde_json::json!({
            "name": m.name,
            "mutates": m.mutates,
            "category": m.category.as_str(),
            "cost": m.cost.as_str(),
        })).collect::<Vec<_>>(),
        "total": catalog.len(),
    }))
}

/// Phase 92 H5: 현재 활성 원격 저장소 어댑터의 capability 반환.
/// Mirage Resource 패턴 흡수. Notion mode 분기 노출.
#[tauri::command]
pub async fn get_remote_storage_capabilities(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let caps = state.service.remote_storage.capabilities();
    Ok(serde_json::json!({
        "backend": caps.backend,
        "can_upload": caps.can_upload,
        "can_download": caps.can_download,
        "can_list": caps.can_list,
        "can_delete": caps.can_delete,
        "mode_options": caps.mode_options,
        "active_mode": caps.active_mode,
        "supports_hard_delete": caps.supports_hard_delete,
        "is_configured": state.service.remote_storage.is_configured(),
    }))
}

/// Phase 91 A2: 출력 PII mask 활성화 여부 조회 (Settings 토글용).
#[tauri::command]
pub async fn get_pii_mask_config(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = state.config.read().await.clone();
    Ok(serde_json::json!({
        "output_pii_mask": config.search.output_pii_mask,
    }))
}
