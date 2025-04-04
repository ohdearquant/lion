// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agents;
mod bridge;
mod commands;
mod events;
mod graph;
mod logging;
mod project;
mod runtime;
mod state;
mod workflows;

use agents::{load_agent, unload_agent, update_agent_state_command};
use bridge::{
    call_plugin_integrated, create_log, list_plugins_integrated, load_plugin_integrated, ping,
    spawn_agent,
};
use commands::{
    clear_logs, close_project, get_recent_logs, get_runtime_status, identify_project, list_agents,
    open_project, update_agent_state,
};
use state::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        // WorkflowManager is now part of AppState, no need to manage separately
        .setup(|app| {
            // Initialize application state
            state::setup_state(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Runtime and system commands
            get_runtime_status,
            // Log commands
            get_recent_logs,
            clear_logs,
            // Project commands
            identify_project,
            open_project,
            close_project,
            // Agent commands
            list_agents,
            update_agent_state,
            update_agent_state_command,
            load_agent,
            unload_agent,
            // Bridge commands
            ping,
            create_log,
            spawn_agent,
            load_plugin_integrated,
            list_plugins_integrated,
            call_plugin_integrated,
        ])
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application");
}
