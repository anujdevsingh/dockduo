//! Global hotkey registration.
//!
//! Currently: `Ctrl+Shift+L` → toggle overlay visibility.
//! Works from anywhere on the desktop, including full-screen apps that
//! haven't exclusively captured the keyboard.

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::tray;

/// The single hotkey combination for toggling overlay visibility.
pub fn toggle_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyL)
}

/// Register the hotkey against the already-initialized global-shortcut
/// plugin. Call this from `setup()` after the plugin is attached.
pub fn register<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<()> {
    let shortcut = toggle_shortcut();
    app.global_shortcut().on_shortcut(shortcut, {
        move |app, _sc, event| {
            // Fire once per press (Pressed), not on key-up — otherwise we'd
            // toggle twice per keystroke.
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let handle = app.app_handle();
            if let Err(e) = tray::toggle_overlay(handle) {
                tracing::warn!(error = %e, "hotkey toggle failed");
            }
        }
    })?;
    tracing::info!("registered Ctrl+Shift+L toggle hotkey");
    Ok(())
}
