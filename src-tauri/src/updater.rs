//! Update checking — plumbed, but disabled in MVP per build plan §12 Phase 5.
//!
//! The updater plugin is initialised in `lib.rs` so the command surface
//! exists and is stable. Until signing keys are generated and
//! `tauri.conf.json > plugins.updater.active` flips to true, calling
//! `check_for_updates` from the tray returns a user-facing message
//! saying updates are not configured yet, rather than blowing up.

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Serialize)]
pub struct UpdateCheckResult {
    /// "available" | "up-to-date" | "disabled" | "error"
    pub status: &'static str,
    pub message: String,
    pub version: Option<String>,
}

/// Asynchronously ask the updater endpoint whether a newer release exists.
///
/// Returns a structured result instead of a bare bool so the frontend /
/// tray can show a meaningful toast. The `disabled` variant is returned
/// when the plugin isn't configured with an endpoint yet.
#[tauri::command]
pub async fn check_for_updates<R: Runtime>(app: AppHandle<R>) -> UpdateCheckResult {
    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            return UpdateCheckResult {
                status: "disabled",
                message: format!(
                    "Update check unavailable — signing keys not configured ({e})."
                ),
                version: None,
            };
        }
    };

    match updater.check().await {
        Ok(Some(update)) => UpdateCheckResult {
            status: "available",
            message: format!("DockDuo {} is available.", update.version),
            version: Some(update.version.clone()),
        },
        Ok(None) => UpdateCheckResult {
            status: "up-to-date",
            message: "You're on the latest version of DockDuo.".into(),
            version: None,
        },
        Err(e) => UpdateCheckResult {
            status: "error",
            message: format!("Update check failed: {e}"),
            version: None,
        },
    }
}
