//! Auto-hide the overlay when a fullscreen app (game, video player) is
//! focused.
//!
//! Windows exposes `SHQueryUserNotificationState` which reports
//! `QUNS_BUSY` / `QUNS_RUNNING_D3D_FULL_SCREEN` / `QUNS_PRESENTATION_MODE`
//! when a fullscreen app is in the foreground. We poll once per second
//! (cheap, no window hooks) and show/hide the overlay accordingly.
//!
//! Gated by `config.hide_on_fullscreen`; a user who toggles the setting
//! off sees the overlay stay put.

use std::time::Duration;

use tauri::{AppHandle, Manager, Runtime};

use crate::config;
use crate::hit_test::OVERLAY_WINDOW_LABELS;

#[cfg(windows)]
const POLL_INTERVAL_MS: u64 = 1000;

/// Kick off the background thread that watches for fullscreen apps.
pub fn start_polling<R: Runtime>(app: AppHandle<R>) {
    #[cfg(windows)]
    std::thread::spawn(move || poll_loop(app));
    #[cfg(not(windows))]
    let _ = app;
}

#[cfg(windows)]
fn poll_loop<R: Runtime>(app: AppHandle<R>) {
    use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUERY_USER_NOTIFICATION_STATE};

    // Track the hide decision we made so we don't thrash show/hide when
    // the user manually toggles via tray/hotkey.
    let mut we_hid_it = false;

    loop {
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));

        let cfg = config::load();
        if !cfg.hide_on_fullscreen {
            // Setting disabled — if we previously hid, restore; otherwise
            // leave the overlay alone.
            if we_hid_it {
                for label in OVERLAY_WINDOW_LABELS {
                    if let Some(win) = app.get_webview_window(label) {
                        let _ = win.show();
                    }
                }
                crate::bubble::show_all(&app);
                we_hid_it = false;
            }
            continue;
        }

        let state: QUERY_USER_NOTIFICATION_STATE = unsafe {
            match SHQueryUserNotificationState() {
                Ok(s) => s,
                Err(e) => {
                    tracing::debug!(error = %e, "SHQueryUserNotificationState failed");
                    continue;
                }
            }
        };
        use windows::Win32::UI::Shell::{
            QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
        };
        let is_fullscreen = state == QUNS_BUSY
            || state == QUNS_RUNNING_D3D_FULL_SCREEN
            || state == QUNS_PRESENTATION_MODE;

        let Some(primary) = app.get_webview_window("overlay") else {
            continue;
        };
        let currently_visible = primary.is_visible().unwrap_or(true);

        if is_fullscreen && currently_visible {
            for label in OVERLAY_WINDOW_LABELS {
                if let Some(w) = app.get_webview_window(label) {
                    let _ = w.hide();
                }
            }
            crate::bubble::hide_all(&app);
            we_hid_it = true;
        } else if !is_fullscreen && we_hid_it {
            for label in OVERLAY_WINDOW_LABELS {
                if let Some(w) = app.get_webview_window(label) {
                    let _ = w.show();
                }
            }
            crate::bubble::show_all(&app);
            we_hid_it = false;
        }
    }
}

/// Tauri command the frontend uses to flip the config flag for hide-on-fullscreen.
/// Thin wrapper so the frontend needs a single `invoke` surface.
#[tauri::command]
pub fn set_hide_on_fullscreen<R: Runtime>(
    app: AppHandle<R>,
    enabled: bool,
) -> Result<(), String> {
    config::set_hide_on_fullscreen(enabled).map_err(|e| e.to_string())?;
    // If the user just turned it off and we were hiding the overlay,
    // show it immediately so they don't have to wait for the next poll.
    if !enabled {
        for label in OVERLAY_WINDOW_LABELS {
            if let Some(win) = app.get_webview_window(label) {
                let _ = win.show();
            }
        }
    }
    Ok(())
}
