mod agent;
mod claude;
mod commands;
mod database;
mod llm_client;
mod mcp;
mod skills;
mod tools;

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

    // Initialize MCP manager
    let mcp_manager = Arc::new(MCPManager::new());
    let db_arc = Arc::new(db);

    // Auto-connect enabled MCP servers will be done in the tauri app setup

    let app_state = Arc::new(AppState {
        db: db_arc,
        claude_client: Mutex::new(None),
        mcp_manager,
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
            let _app_handle = app.handle().clone();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
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
                });
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
