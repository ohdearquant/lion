#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::menu::{MenuId, MenuItemId};
use tauri::{
    CustomMenuItem, Manager, SystemTrayMenu, SystemTrayMenuItem, TrayIcon, Window, WindowEvent,
    WindowUrl,
};

mod bridge;
mod utils;

fn main() {
    // Configure the system tray
    let quit = CustomMenuItem::new(MenuItemId::new("quit"), "Quit");
    let hide = CustomMenuItem::new(MenuItemId::new("hide"), "Hide");
    let show = CustomMenuItem::new(MenuItemId::new("show"), "Show");
    let logs = CustomMenuItem::new(MenuItemId::new("logs"), "Open Logs");

    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_item(hide)
        .add_item(logs)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let tray_icon = TrayIcon::new()
        .with_menu(tray_menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quit" => {
                std::process::exit(0);
            }
            "hide" => {
                if let Some(window) = app.get_window("main") {
                    let _ = window.hide();
                }
            }
            "show" => {
                if let Some(window) = app.get_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "logs" => {
                if let Some(window) = app.get_window("logs") {
                    let _ = window.show();
                    let _ = window.set_focus();
                } else {
                    let _ = Window::new(app.handle(), "logs", WindowUrl::App("logs.html".into()))
                        .title("Lion UI - Log Viewer")
                        .inner_size(900.0, 600.0)
                        .build();
                }
            }
            _ => {}
        });

    // Spawn the backend server as a separate process
    std::thread::spawn(|| {
        // In a real implementation, we would start the Lion UI server here
        println!("Lion UI server would start here in a separate process");
    });

    tauri::Builder::default()
        .plugin(tray_icon)
        .on_window_event(|event| match event.event() {
            WindowEvent::CloseRequested { api, .. } => {
                if event.window().label() == "main" {
                    // Hide the window instead of closing it
                    let _ = event.window().hide();
                    api.prevent_close();
                }
            }
            _ => {}
        })
        .plugin(
            tauri::plugin::TauriPlugin::new().register_uri_scheme_protocol(
                "lion",
                |_app, _request| {
                    // You can handle custom URI schemes here
                    // For now, return an empty success response
                    Ok(tauri::http::Response::new(200))
                },
            ),
        )
        .invoke_handler(tauri::generate_handler![
            bridge::ping,
            bridge::create_log,
            bridge::spawn_agent,
            bridge::load_plugin_integrated,
            bridge::list_plugins_integrated,
            bridge::call_plugin_integrated,
            bridge::get_recent_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
