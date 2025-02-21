// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{thread, time::Duration};
use tauri::{Manager, WindowBuilder};

fn main() {
    // Start the Axum server in a separate thread
    thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            lion_ui::run_server().await;
        });
    });

    // Give the server a moment to start
    thread::sleep(Duration::from_secs(1));

    // Build and run the Tauri application
    tauri::Builder::default()
        .setup(|app| {
            WindowBuilder::new(
                app,
                "main".to_string(),
                tauri::WindowUrl::External("http://127.0.0.1:8080".parse().unwrap())
            )
            .title("Lion UI")
            .inner_size(1200.0, 800.0)
            .build()
            .unwrap();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}