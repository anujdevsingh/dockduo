//! DockDuo — library entry. `main.rs` simply calls `run()`.

pub mod overlay;
pub mod taskbar;

use tauri::{Manager, Runtime};

#[tauri::command]
fn get_taskbar_info() -> Result<taskbar::TaskbarInfo, String> {
    taskbar::current().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_ignore_cursor_events<R: Runtime>(
    app: tauri::AppHandle<R>,
    window_label: String,
    ignore: bool,
) -> Result<(), String> {
    let window = app
        .get_webview_window(&window_label)
        .ok_or_else(|| format!("window '{}' not found", window_label))?;
    window.set_ignore_cursor_events(ignore).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            overlay::configure_overlay_window(app)?;
            taskbar::start_polling(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_taskbar_info,
            set_ignore_cursor_events
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
