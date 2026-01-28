mod agent;
mod capture;
mod claude;
mod commands;
mod database;
mod docs;
mod excel;
mod llm_client;
mod mcp;
mod skills;
mod tools;
mod trace;
mod workstream;

use capture::{CaptureBuffer, ActiveSourceTracker, ClipboardMonitor, CaptureConfig};
use commands::AppState;
use mcp::MCPManager;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize database
    let db = database::Database::new().expect("Failed to initialize database");

    // Initialize MCP tables
    db.create_mcp_tables().expect("Failed to create MCP tables");

    // Initialize Trace tables
    db.create_trace_tables().expect("Failed to create Trace tables");

    // Initialize Document tables
    db.create_docs_table().expect("Failed to create Docs tables");

    // Initialize WorkStream tables (lean activity tracking)
    db.create_workstream_tables().expect("Failed to create WorkStream tables");

    // Initialize Capture tables (rich capture & source linking)
    db.create_capture_tables().expect("Failed to create Capture tables");

    // Initialize MCP manager
    let mcp_manager = Arc::new(MCPManager::new());
    let db_arc = Arc::new(db);

    // Initialize Capture buffer and source tracker
    let capture_buffer = Arc::new(CaptureBuffer::new());
    let source_tracker = Arc::new(ActiveSourceTracker::new());

    // Initialize clipboard monitor
    let capture_config = db_arc.get_capture_config().unwrap_or_default();
    let clipboard_monitor = Arc::new(ClipboardMonitor::new(
        capture_buffer.clone(),
        Arc::new(Mutex::new(capture_config)),
    ));

    // Auto-connect enabled MCP servers will be done in the tauri app setup

    let app_state = Arc::new(AppState {
        db: db_arc,
        claude_client: Mutex::new(None),
        mcp_manager,
        excel_watcher: Mutex::new(None),
        capture_buffer,
        source_tracker,
        clipboard_monitor,
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_platform,
            commands::get_settings,
            commands::save_settings,
            commands::test_connection,
            commands::list_conversations,
            commands::create_conversation,
            commands::update_conversation_title,
            commands::delete_conversation,
            commands::get_messages,
            commands::add_message,
            commands::send_chat_message,
            commands::send_chat_with_tools,
            commands::run_agent,
            commands::list_tasks,
            commands::get_task,
            commands::create_task,
            commands::delete_task,
            commands::run_task_agent,
            commands::get_task_messages,
            commands::get_skills_list,
            commands::list_mcp_servers,
            commands::save_mcp_server,
            commands::delete_mcp_server,
            commands::connect_mcp_server,
            commands::disconnect_mcp_server,
            commands::get_mcp_server_statuses,
            commands::execute_mcp_tool,
            // Excel commands
            commands::excel_read,
            commands::excel_validate,
            commands::excel_apply,
            commands::excel_watch,
            commands::excel_get_sheets,
            commands::excel_checksum,
            commands::excel_backup,
            // Data panel commands
            commands::get_data_panel,
            commands::save_data_panel,
            commands::delete_data_panel,
            commands::list_data_panels,
            // MCP Apps commands
            commands::fetch_mcp_app_resource,
            commands::get_mcp_app_tools,
            commands::create_mcp_app_instance,
            // Trace commands
            commands::log_trace,
            commands::list_traces,
            commands::delete_trace,
            commands::clear_traces,
            commands::get_trace_settings,
            commands::save_trace_settings,
            // Suggestion commands
            commands::list_suggestions,
            commands::update_suggestion_status,
            commands::delete_suggestion,
            commands::generate_suggestions,
            commands::apply_suggestion,
            // Browser commands
            commands::open_browser_window,
            commands::create_embedded_browser,
            commands::update_embedded_browser_bounds,
            commands::navigate_embedded_browser,
            commands::close_embedded_browser,
            // Document commands
            commands::create_document,
            commands::get_document,
            commands::update_document,
            commands::list_documents,
            commands::delete_document,
            // WorkStream commands (lean activity tracking)
            commands::ws_create_block,
            commands::ws_create_manual_block,
            commands::ws_update_block,
            commands::ws_get_block,
            commands::ws_list_blocks,
            commands::ws_delete_block,
            commands::ws_get_timeline,
            commands::ws_enhance_summary,
            commands::ws_create_milestone,
            commands::ws_list_milestones,
            commands::ws_cleanup,
            // Capture commands (rich capture & source linking)
            commands::report_page_context,
            commands::update_page_context,
            commands::capture_search,
            commands::update_search_click,
            commands::capture_ai_exchange,
            commands::flush_capture_buffer,
            commands::activate_source,
            commands::deactivate_source,
            commands::create_source_link,
            commands::get_document_provenance,
            commands::get_capture_config,
            commands::update_capture_config,
            commands::get_recent_browse,
            commands::get_recent_search,
            commands::get_recent_ai_exchange,
            commands::start_clipboard_monitor,
            commands::stop_clipboard_monitor,
            commands::set_clipboard_source,
            commands::get_recent_clipboard,
            commands::capture_doc_edit,
            commands::get_recent_doc_edit,
            commands::export_and_clear_captures,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            // Auto-connect enabled MCP servers
            let app_state = app.state::<Arc<AppState>>();
            let db = app_state.db.clone();
            let mcp_manager = app_state.mcp_manager.clone();
            let clipboard_monitor = app_state.clipboard_monitor.clone();
            let _app_handle = app.handle().clone();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    // Auto-connect MCP servers
                    if let Ok(servers) = db.get_mcp_servers() {
                        for server in servers {
                            if server.enabled {
                                if let Err(e) = mcp_manager.connect_server(&server).await {
                                    eprintln!("Failed to auto-connect MCP server '{}': {}", server.name, e);
                                } else {
                                    println!("Auto-connected MCP server: {}", server.name);
                                }
                            }
                        }
                    }

                    // Start clipboard monitor
                    clipboard_monitor.start().await;
                    println!("Clipboard monitor started");
                });
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
