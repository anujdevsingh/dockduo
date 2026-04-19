//! DockDuo — library entry. `main.rs` simply calls `run()`.

pub mod claude;
pub mod config;
pub mod fullscreen;
pub mod hit_test;
pub mod hotkey;
pub mod overlay;
pub mod taskbar;
pub mod tray;

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

#[tauri::command]
fn toggle_overlay_visibility<R: Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    tray::toggle_overlay(&app).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            overlay::configure_overlay_window(app)?;
            taskbar::start_polling(app.handle().clone());
            hit_test::start_polling(app.handle().clone());
            fullscreen::start_polling(app.handle().clone());

            if let Err(e) = tray::build(app) {
                tracing::warn!(error = %e, "tray build failed");
            }
            if let Err(e) = hotkey::register(app.handle()) {
                tracing::warn!(error = %e, "global hotkey registration failed");
            }

            // First-run onboarding: reveal the onboarding window only if
            // the user hasn't completed it before. The window ships
            // `visible: false` in tauri.conf.json so it stays hidden on
            // subsequent launches.
            let cfg = config::load();
            if !cfg.onboarded {
                if let Some(w) = app.get_webview_window("onboarding") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_taskbar_info,
            set_ignore_cursor_events,
            toggle_overlay_visibility,
            claude::list_agents,
            claude::spawn_agent,
            hit_test::report_bounds,
            config::get_config,
            config::set_theme,
            config::mark_onboarded,
            config::set_last_agent,
            fullscreen::set_hide_on_fullscreen
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
