//! "Start with Windows" toggle, backed by the `autostart` plugin.
//!
//! The plugin registers a per-user launch entry (HKCU Run key on Windows)
//! pointing at the current executable. We expose two thin Tauri commands —
//! `get_autostart` and `set_autostart` — plus a helper the tray menu
//! calls to sync its checkbox label at startup.
//!
//! Note: this module does not touch `config.rs`. The plugin itself is the
//! source of truth for whether autostart is enabled — querying the
//! registry key is cheap and avoids drift between the registry and our
//! own JSON file.

use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;

/// Whether start-with-Windows is currently enabled.
pub fn is_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn get_autostart<R: Runtime>(app: AppHandle<R>) -> bool {
    is_enabled(&app)
}

#[tauri::command]
pub fn set_autostart<R: Runtime>(app: AppHandle<R>, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| e.to_string())?;
    Ok(manager.is_enabled().unwrap_or(enabled))
}
