use std::{env, path::PathBuf, process::Command};
use tauri::{AppHandle, Manager, Window, WindowBuilder, WindowUrl};

/// Check if the Lion UI backend server is running
pub fn is_server_running() -> bool {
    // In a real implementation, this would actually check if the server process is running
    // For now, we just return true
    true
}

/// Get the path to the Lion UI executable
pub fn get_lion_ui_path() -> PathBuf {
    // In a real implementation, this would determine the path to the Lion UI server executable
    // based on the current executable location

    // For now, just return a placeholder path
    if cfg!(windows) {
        PathBuf::from(r"C:\Program Files\Lion\lion_ui.exe")
    } else if cfg!(target_os = "macos") {
        PathBuf::from("/Applications/Lion.app/Contents/MacOS/lion_ui")
    } else {
        // Linux
        PathBuf::from("/usr/local/bin/lion_ui")
    }
}

/// Start the Lion UI backend server if it's not already running
pub fn ensure_server_running() -> Result<(), String> {
    if is_server_running() {
        return Ok(());
    }

    // Get the path to the Lion UI executable
    let lion_ui_path = get_lion_ui_path();

    // Start the server process
    match Command::new(lion_ui_path).arg("--daemon").spawn() {
        Ok(_) => {
            println!("Started Lion UI server");
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("Failed to start Lion UI server: {}", e);
            println!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// Create or focus the logs window
pub fn open_logs_window(app: &AppHandle) -> Result<Window, tauri::Error> {
    if let Some(window) = app.get_window("logs") {
        window.show()?;
        window.set_focus()?;
        Ok(window)
    } else {
        WindowBuilder::new(app, "logs", WindowUrl::App("logs.html".into()))
            .title("Lion UI - Log Viewer")
            .inner_size(900.0, 600.0)
            .build()
    }
}

/// Create or focus the agents window
pub fn open_agents_window(app: &AppHandle) -> Result<Window, tauri::Error> {
    if let Some(window) = app.get_window("agents") {
        window.show()?;
        window.set_focus()?;
        Ok(window)
    } else {
        WindowBuilder::new(app, "agents", WindowUrl::App("agents.html".into()))
            .title("Lion UI - Agent Manager")
            .inner_size(800.0, 600.0)
            .build()
    }
}

/// Create or focus the plugins window
pub fn open_plugins_window(app: &AppHandle) -> Result<Window, tauri::Error> {
    if let Some(window) = app.get_window("plugins") {
        window.show()?;
        window.set_focus()?;
        Ok(window)
    } else {
        WindowBuilder::new(app, "plugins", WindowUrl::App("plugins.html".into()))
            .title("Lion UI - Plugin Manager")
            .inner_size(800.0, 600.0)
            .build()
    }
}

/// Get the URL for the Lion UI API
pub fn get_api_url() -> String {
    // In a real implementation, this would determine the correct URL based on configuration
    // For now, return a default URL
    "http://localhost:8080".to_string()
}
