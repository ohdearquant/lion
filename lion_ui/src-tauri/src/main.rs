#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
    WindowBuilder, WindowEvent, WindowUrl,
};

mod bridge;
mod utils;

fn main() {
    // Configure the system tray
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide");
    let show = CustomMenuItem::new("show".to_string(), "Show");
    let logs = CustomMenuItem::new("logs".to_string(), "Open Logs");
    
    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_item(hide)
        .add_item(logs)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);
    
    let system_tray = SystemTray::new().with_menu(tray_menu);
    
    // Spawn the backend server as a separate process
    std::thread::spawn(|| {
        // In a real implementation, we would start the Lion UI server here
        // For now, we just print a message
        println!("Lion UI server would start here in a separate process");
    });

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    std::process::exit(0);
                }
                "hide" => {
                    if let Some(window) = app.get_window("main") {
                        window.hide().unwrap();
                    }
                }
                "show" => {
                    if let Some(window) = app.get_window("main") {
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                }
                "logs" => {
                    if let Some(window) = app.get_window("logs") {
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    } else {
                        let logs_window = WindowBuilder::new(
                            app,
                            "logs".to_string(),
                            WindowUrl::App("logs.html".into()),
                        )
                        .title("Lion UI - Log Viewer")
                        .inner_size(900.0, 600.0)
                        .build()
                        .unwrap();
                    }
                }
                _ => {}
            },
            _ => {}
        })
        .on_window_event(|event| match event.event() {
            WindowEvent::CloseRequested { api, .. } => {
                if event.window().label() == "main" {
                    // Hide the window instead of closing it
                    event.window().hide().unwrap();
                    api.prevent_close();
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            bridge::ping,
            bridge::create_log,
            bridge::spawn_agent,
            bridge::load_plugin,
            bridge::load_plugin_integrated,
            bridge::list_plugins_integrated,
            bridge::call_plugin_integrated
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
