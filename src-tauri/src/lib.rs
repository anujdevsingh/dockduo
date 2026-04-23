//! DockDuo — library entry. `main.rs` simply calls `run()`.

pub mod autostart;
pub mod binary_resolve;
pub mod bubble;
pub mod chat;
pub mod claude;
pub mod config;
pub mod fullscreen;
pub mod hit_test;
pub mod hotkey;
pub mod overlay;
pub mod taskbar;
pub mod tray;
pub mod updater;

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
    // Resolve the per-user log directory once so the log plugin can
    // pin output to %APPDATA%\DockDuo\logs\DockDuo.log. Falls back to the
    // OS temp dir if we somehow can't resolve %APPDATA%.
    let log_dir = dirs::config_dir()
        .map(|d| d.join("DockDuo").join("logs"))
        .unwrap_or_else(std::env::temp_dir);
    let _ = std::fs::create_dir_all(&log_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            for label in hit_test::OVERLAY_WINDOW_LABELS {
                if let Some(win) = app.get_webview_window(label) {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            if let Some(win) = app.get_webview_window("onboarding") {
                let _ = win.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::default()
                .clear_targets()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Folder {
                        path: log_dir,
                        file_name: Some("DockDuo".into()),
                    },
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .level_for("tao", tauri_plugin_log::log::LevelFilter::Warn)
                .level_for(
                    "tao::platform_impl",
                    tauri_plugin_log::log::LevelFilter::Warn,
                )
                .level_for(
                    "tao::platform_impl::platform::event_loop::runner",
                    tauri_plugin_log::log::LevelFilter::Warn,
                )
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            claude::check_cli_available,
            bubble::toggle_bubble,
            bubble::close_bubble,
            bubble::take_pending_bubble,
            bubble::bubble_is_open,
            hit_test::report_bounds,
            config::get_config,
            config::set_theme,
            config::mark_onboarded,
            config::set_last_agent,
            chat::session::chat_start_session,
            chat::session::chat_send,
            chat::session::chat_clear_session,
            chat::session::chat_terminate,
            fullscreen::set_hide_on_fullscreen,
            autostart::get_autostart,
            autostart::set_autostart,
            updater::check_for_updates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
