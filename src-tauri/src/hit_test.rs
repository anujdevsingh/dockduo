//! Cursor-driven hit testing for the overlay window.
//!
//! The overlay is a full-width click-through window by default (so desktop
//! icons and taskbar buttons remain reachable). Whenever the real cursor
//! enters a character's bounding box, we toggle the window out of
//! click-through mode so React can receive the click. When the cursor
//! leaves, we toggle click-through back on.
//!
//! The frontend pushes character bounds to us via `report_bounds`, then
//! a dedicated polling thread reads `GetCursorPos` every ~30ms and decides.
//!
//! Bounds are in CSS pixels **relative to the overlay window's top-left
//! corner**. The polling thread subtracts the overlay window origin from
//! the global cursor position before testing.
//!
//! Keys are `"{window_label}###{character}"` so multiple overlay webviews
//! (multi-monitor) stay independent.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::Deserialize;
use tauri::{AppHandle, Manager, Runtime};

/// Labels of overlay webviews. v0.2.0 shipped primary-only; multi-monitor
/// is deferred (see `docs/DECISIONS.md` D-011). The code still carries a
/// slice + composite bounds keys so future multi-monitor work only needs
/// to add labels here.
pub const OVERLAY_WINDOW_LABELS: &[&str] = &["overlay"];

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Per-character bounds as last reported by the frontend.
static BOUNDS: OnceLock<Mutex<HashMap<String, Bounds>>> = OnceLock::new();

fn bounds_map() -> &'static Mutex<HashMap<String, Bounds>> {
    BOUNDS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn bounds_key(window_label: &str, character: &str) -> String {
    format!("{window_label}###{character}")
}

/// Frontend reports per-frame bounds for a character in a specific overlay window.
#[tauri::command]
pub fn report_bounds(window_label: String, character: String, bounds: Bounds) {
    bounds_map()
        .lock()
        .unwrap()
        .insert(bounds_key(&window_label, &character), bounds);
}

/// Launch the background thread that polls the cursor position and
/// toggles `set_ignore_cursor_events` based on whether the cursor is
/// over any tracked character.
pub fn start_polling<R: Runtime>(app: AppHandle<R>) {
    std::thread::spawn(move || {
        let mut currently_ignoring: HashMap<String, bool> = HashMap::new();
        loop {
            std::thread::sleep(Duration::from_millis(30));
            let cursor_global = match cursor_position() {
                Some(p) => p,
                None => continue,
            };

            for label in OVERLAY_WINDOW_LABELS {
                let Some(window) = app.get_webview_window(label) else {
                    continue;
                };

                // Use **inner** (client-area) position, not outer. React layout and
                // `report_bounds` are relative to the webview viewport (client origin).
                // `outer_position` includes any frame/chrome offset and breaks hit-testing
                // on borderless overlays at non-1.0 scale factors — clicks never line up.
                let win_origin = match window.inner_position() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let scale = window.scale_factor().unwrap_or(1.0);

                let rel_x = (cursor_global.0 as f64 - win_origin.x as f64) / scale;
                let rel_y = (cursor_global.1 as f64 - win_origin.y as f64) / scale;

                let prefix = format!("{label}###");
                let over_character = {
                    let map = bounds_map().lock().unwrap();
                    map.iter().any(|(k, b)| {
                        k.starts_with(&prefix)
                            && rel_x >= b.x
                            && rel_x <= b.x + b.w
                            && rel_y >= b.y
                            && rel_y <= b.y + b.h
                    })
                };

                let should_ignore = !over_character;
                let prev = currently_ignoring
                    .get(*label)
                    .copied()
                    .unwrap_or(true);
                if should_ignore != prev {
                    let _ = window.set_ignore_cursor_events(should_ignore);
                    currently_ignoring.insert(label.to_string(), should_ignore);
                }
            }
        }
    });
}

/// Global cursor position in physical screen pixels.
#[cfg(windows)]
fn cursor_position() -> Option<(i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut pt = POINT::default();
    unsafe {
        if GetCursorPos(&mut pt).is_ok() {
            Some((pt.x, pt.y))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn cursor_position() -> Option<(i32, i32)> {
    None
}
