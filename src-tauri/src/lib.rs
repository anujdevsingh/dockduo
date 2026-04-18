//! DockDuo — library entry. `main.rs` simply calls `run()`.

pub mod overlay;
pub mod taskbar;

#[tauri::command]
fn get_taskbar_info() -> Result<taskbar::TaskbarInfo, String> {
    taskbar::current().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            overlay::configure_overlay_window(app)?;
            taskbar::start_polling(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_taskbar_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
