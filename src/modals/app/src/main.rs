//! File Pipeline — 단일 바이너리
//!
//! pipeline              → GUI (Dashboard + 트레이 + 백그라운드 서비스)
//! pipeline start        → GUI (동일)
//! pipeline serve        → MCP 서버 (stdio, Claude Code 연동)
//! pipeline stats        → CLI
//! pipeline memo "텍스트" → CLI

// Windows: 콘솔 없이 시작. CLI 모드에서만 AttachConsole로 콘솔 연결.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod commands;
mod service;
mod state;

use clap::Parser;
use tauri::Manager;

fn main() {
    let cli = file_pipeline_shared::cli::Cli::try_parse().unwrap_or_default();

    // auto-init
    file_pipeline_shared::auto_init();

    // CLI/MCP 모드 — 콘솔 attach + 실행
    if !file_pipeline_shared::cli::is_gui_command(&cli) {
        #[cfg(windows)]
        unsafe { winapi_attach_console(); }

        let cfg = file_pipeline_shared::config::find_and_load_config(cli.config.as_deref())
            .unwrap_or_else(|_| file_pipeline_shared::config::PipelineConfig::default_config());
        // CLI: logging.file=true면 파일, false면 콘솔
        let _log_guard = file_pipeline_shared::init_tracing(&cfg.logging.level, cfg.logging.file);

        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        if let Err(e) = rt.block_on(file_pipeline_shared::cli::execute(cli)) {
            file_pipeline_shared::write_log("ERROR", &format!("CLI 오류: {}", e));
            eprintln!("오류: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // GUI 모드 — windows_subsystem = "windows"로 콘솔 없이 시작
    #[cfg(windows)]
    if is_already_running() {
        std::process::exit(0);
    }

    // GUI: 항상 파일 로깅 (콘솔 없으므로)
    let cfg_for_log = file_pipeline_shared::config::find_and_load_config(None)
        .unwrap_or_else(|_| file_pipeline_shared::config::PipelineConfig::default_config());
    let _log_guard = file_pipeline_shared::init_tracing(&cfg_for_log.logging.level, true);

    tracing::info!("File Pipeline GUI 시작");
    file_pipeline_shared::write_log("INFO", "File Pipeline GUI 시작");

    // 서비스 초기화 (Qdrant 체크 포함)
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let app_state = match rt.block_on(async { service::init_app_state() }) {
        Ok(state) => state,
        Err(e) => {
            file_pipeline_shared::write_log("ERROR", &format!("서비스 초기화 실패: {}", e));
            eprintln!("[ERROR] 서비스 초기화 실패: {}", e);
            eprintln!("        pipeline.toml 설정을 확인하세요.");
            wait_and_exit(1);
        }
    };
    std::mem::forget(rt);

    file_pipeline_shared::write_log("INFO", "서비스 준비 완료. Dashboard 시작");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_stats,
            commands::search,
            commands::list_documents,
            commands::get_document,
            commands::get_lint_strong_claims,
            commands::kg_neighbors,
            commands::kg_paths,
            commands::kg_stats,
            commands::get_crossref_stats,
            commands::get_verification_metrics,
            commands::get_progress,
            commands::get_queue,
            commands::get_file_log,
            commands::get_errors,
            commands::get_todos,
            commands::complete_todo,
            commands::add_todo,
            commands::list_credentials,
            commands::save_credential,
            commands::delete_credential,
            commands::get_config,
            commands::save_config,
            commands::export_config_toml,
            commands::import_config_toml,
            commands::list_topics,
            commands::get_topic,
            commands::update_topic,
            commands::retry_failed,
            commands::rebuild_embeddings,
            commands::rebuild_all,
            commands::rebuild_vectordb,
            commands::get_host_tools,
            commands::test_host_tool,
            commands::get_token_usage,
            commands::simulate_pipeline,
            commands::get_watcher_status,
            commands::set_watcher_active,
            commands::get_prompts,
            commands::save_prompts,
            commands::setup_review,
            commands::setup_apply,
            commands::setup_snapshot_list,
            commands::setup_snapshot_rollback,
            commands::setup_decision_log_list,
            // Phase 80 코퍼스 신호 카운터 (lesson 19 frontend-backend 매핑 정합성)
            commands::get_search_mode_stats,
            commands::get_crag_stats,
            commands::get_chunk_stats,
            commands::get_processing_metrics,
            commands::get_llm_cache_stats,
            commands::clear_llm_cache,
            commands::gc_llm_cache_now,
            commands::c1_thresholds_list,
            commands::c1_threshold_set,
            commands::pii_patterns_list,
            commands::pii_pattern_add,
            commands::pii_pattern_remove,
            commands::auto_suggest_from_counters,
            commands::accept_suggested_decision,
            commands::reject_suggested_decision,
            // Phase 80 동작 모듈
            commands::setup_modules_list,
            commands::setup_apply_modules,
            // Phase 93 GUI 가시화 (Phase 91 A2 / 92 H1·H3·H5)
            commands::get_anomaly_report,
            commands::get_mcp_tool_catalog_full,
            commands::get_remote_storage_capabilities,
            commands::get_pii_mask_config,
        ])
        .setup(|app| {
            let state = app.state::<state::AppState>();
            let service_ref = state.inner().clone_for_background();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
                rt.block_on(async {
                    if let Err(e) = service::start_background_tasks_standalone(service_ref).await {
                        tracing::error!("백그라운드 서비스 오류: {}", e);
                    }
                });
            });

            // Ruflo C1: startup 1회 자동 추천 (Phase 80 카운터 분석 → decision_log INSERT)
            // 임계값 미달이면 no-op. 사용자는 Decision Log에서 검토 후 accept_suggested_decision 호출.
            std::thread::spawn(|| {
                let data_dir = file_pipeline_shared::config::find_data_dir(None);
                match file_pipeline_shared::settings_db::SettingsDb::open_or_migrate(&data_dir) {
                    Ok(db) => {
                        match file_pipeline_shared::auto_suggester::suggest_from_counters(&db) {
                            Ok(n) if n > 0 => tracing::info!("[c1-startup] 자동 추천 {}건 INSERT", n),
                            Ok(_) => tracing::debug!("[c1-startup] 임계값 미달, 제안 없음"),
                            Err(e) => tracing::warn!("[c1-startup] 자동 추천 실패: {}", e),
                        }
                    }
                    Err(e) => tracing::warn!("[c1-startup] settings.db 열기 실패: {}", e),
                }
            });

            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            let show_i = MenuItem::with_id(app, "show", "Dashboard 열기", true, None::<&str>)?;
            let stats_i = MenuItem::with_id(app, "stats", "통계 보기", true, None::<&str>)?;
            let watch_i = MenuItem::with_id(app, "toggle_watch", "감지 ON/OFF", true, None::<&str>)?;
            let sep = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &stats_i, &watch_i, &sep, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().expect("icon"))
                .tooltip("File Pipeline")
                .menu(&menu)
                .on_menu_event(move |app_handle, event| {
                    let id: &str = event.id.as_ref();
                    match id {
                        "show" => show_main_window(app_handle),
                        "stats" => show_main_window(app_handle),
                        "toggle_watch" => {
                            if let Some(state) = app_handle.try_state::<state::AppState>() {
                                let current = state.watcher_active.load(std::sync::atomic::Ordering::Relaxed);
                                state.watcher_active.store(!current, std::sync::atomic::Ordering::Relaxed);
                                tracing::info!("트레이: 감지 {}", if !current { "ON" } else { "OFF" });
                            }
                        }
                        "quit" => app_handle.exit(0),
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("File Pipeline 실행 실패");
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// 에러 시 사용자가 읽을 시간을 주고 종료
fn wait_and_exit(code: i32) -> ! {
    eprintln!();
    eprintln!("아무 키나 누르면 종료합니다...");
    let _ = std::io::Read::read(&mut std::io::stdin(), &mut [0u8]);
    std::process::exit(code);
}

#[cfg(windows)]
unsafe fn winapi_attach_console() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(dwProcessId: u32) -> i32;
        fn AllocConsole() -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;
    // 부모 콘솔에 붙기 시도, 실패하면 새 콘솔 할당
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            AllocConsole();
        }
    }
}

/// Named Mutex로 중복 실행 방지 (Windows)
#[cfg(windows)]
fn is_already_running() -> bool {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateMutexW(lpMutexAttributes: *const u8, bInitialOwner: i32, lpName: *const u16) -> *mut u8;
        fn GetLastError() -> u32;
    }
    const ERROR_ALREADY_EXISTS: u32 = 183;

    let name: Vec<u16> = "Global\\FilePipelineSingleInstance\0"
        .encode_utf16()
        .collect();

    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
        if handle.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
            return true;
        }
        // Mutex handle을 유지해서 프로세스 종료까지 잠금 유지
        let _ = handle;
        false
    }
}
