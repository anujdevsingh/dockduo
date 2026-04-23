//! Per-character chat bubble windows.
//!
//! The two bubbles (`bubble_bruce`, `bubble_jazz`) are declared as hidden
//! windows in `tauri.conf.json` so the webview is guaranteed to load
//! `index.html` at startup. We just reposition/show/hide them on demand —
//! no runtime `WebviewWindowBuilder`, which on Windows + WebView2 can leave
//! us with a blank white window depending on dev-URL timing.

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde::Serialize;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Runtime, WebviewWindow};

use crate::claude::AgentKind;
use crate::taskbar::{self, TaskbarEdge, TaskbarInfo};

/// Bubble footprint in CSS pixels — matches the static windows in
/// `tauri.conf.json`. Keep in sync.
const BUBBLE_WIDTH: f64 = 420.0;
const BUBBLE_HEIGHT: f64 = 520.0;
/// Gap between the bubble's bottom edge and the sprite's top.
const BUBBLE_GAP: f64 = 8.0;

/// Remember which agent kind each bubble is about to chat with so
/// `AgentChat` can call `take_pending_bubble` exactly once on mount.
static PENDING: Lazy<Mutex<HashMap<String, AgentKind>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Tracks whether a bubble is currently "open" (shown) for a given
/// character. We can't rely solely on `WebviewWindow::is_visible` because
/// hidden windows still exist.
static OPEN: Lazy<Mutex<HashMap<String, bool>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize)]
pub struct BubbleWalkEvent {
    pub character: String,
}

fn bubble_label(character: &str) -> String {
    format!("bubble_{character}")
}

fn mark_open(character: &str, open: bool) {
    if let Ok(mut m) = OPEN.lock() {
        m.insert(character.to_string(), open);
    }
}

fn is_open(character: &str) -> bool {
    OPEN.lock()
        .ok()
        .and_then(|m| m.get(character).copied())
        .unwrap_or(false)
}

/// Called by `AgentChat` once it mounts to retrieve the chosen agent kind.
#[tauri::command]
pub fn take_pending_bubble(character: String) -> Option<AgentKind> {
    crate::chat::validate_character(&character).ok()?;
    PENDING.lock().ok()?.remove(&character)
}

/// True if the bubble is currently shown for this character.
#[tauri::command]
pub fn bubble_is_open<R: Runtime>(_app: AppHandle<R>, character: String) -> bool {
    if crate::chat::validate_character(&character).is_err() {
        return false;
    }
    is_open(&character)
}

/// Show the bubble for `character` (or hide it if already visible),
/// positioned above the sprite.
#[tauri::command]
pub fn toggle_bubble<R: Runtime>(
    app: AppHandle<R>,
    character: String,
    kind: AgentKind,
    sprite_center_x: f64,
) -> Result<(), String> {
    crate::chat::validate_character(&character)?;
    let label = bubble_label(&character);
    let window = match app.get_webview_window(&label) {
        Some(w) => w,
        None => {
            eprintln!("[bubble] window '{label}' not found in tauri.conf.json");
            return Err(format!("bubble window '{label}' missing"));
        }
    };

    if is_open(&character) {
        let _ = window.hide();
        mark_open(&character, false);
        let _ = app.emit(
            "sprite-walk-resumed",
            BubbleWalkEvent {
                character: character.clone(),
            },
        );
        eprintln!("[bubble] hide label={label}");
        return Ok(());
    }

    if let Ok(mut p) = PENDING.lock() {
        p.insert(character.clone(), kind);
    }

    position_bubble(&app, &window, sprite_center_x);
    let _ = window.unminimize();
    let _ = window.show();
    // Windows denies `SetForegroundWindow` to alwaysOnTop+skipTaskbar windows
    // when another app "owns" the foreground, so keyboard focus never lands
    // on our input. Toggling always-on-top off+on re-asserts activation in
    // a way Windows permits.
    #[cfg(target_os = "windows")]
    {
        let _ = window.set_always_on_top(false);
        let _ = window.set_focus();
        let _ = window.set_always_on_top(true);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window.set_focus();
    }
    mark_open(&character, true);

    let _ = app.emit(
        "sprite-walk-paused",
        BubbleWalkEvent {
            character: character.clone(),
        },
    );
    let _ = app.emit("bubble-opened", character);
    eprintln!("[bubble] show label={label} sprite_cx={sprite_center_x:.0}");
    Ok(())
}

/// Close the bubble window from the frontend (Esc, close button, click
/// outside). Keeps the chat session alive.
#[tauri::command]
pub fn close_bubble<R: Runtime>(app: AppHandle<R>, character: String) -> Result<(), String> {
    crate::chat::validate_character(&character)?;
    let label = bubble_label(&character);
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.hide();
    }
    mark_open(&character, false);
    let _ = app.emit(
        "sprite-walk-resumed",
        BubbleWalkEvent {
            character: character.clone(),
        },
    );
    Ok(())
}

/// Hide all bubbles (e.g. when a fullscreen app takes the foreground).
pub fn hide_all<R: Runtime>(app: &AppHandle<R>) {
    for c in ["bruce", "jazz"] {
        if let Some(w) = app.get_webview_window(&bubble_label(c)) {
            let _ = w.hide();
        }
    }
}

/// Show previously-hidden bubble windows if they were open.
pub fn show_all<R: Runtime>(app: &AppHandle<R>) {
    for c in ["bruce", "jazz"] {
        if is_open(c) {
            if let Some(w) = app.get_webview_window(&bubble_label(c)) {
                let _ = w.show();
            }
        }
    }
}

/// Position the bubble so its bottom-center sits above the sprite's head,
/// clamped inside the monitor rect.
fn position_bubble<R: Runtime>(
    app: &AppHandle<R>,
    bubble: &WebviewWindow<R>,
    sprite_center_x_css: f64,
) {
    let Some(overlay) = app.get_webview_window("overlay") else {
        eprintln!("[bubble] overlay window missing; cannot position bubble");
        return;
    };

    let scale = overlay.scale_factor().unwrap_or(1.0);
    let overlay_pos = match overlay.outer_position() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[bubble] overlay.outer_position failed: {e:?}");
            return;
        }
    };
    let overlay_x_logical = overlay_pos.x as f64 / scale;
    let overlay_y_logical = overlay_pos.y as f64 / scale;

    let mut bubble_x = overlay_x_logical + sprite_center_x_css - BUBBLE_WIDTH / 2.0;
    let mut bubble_y = overlay_y_logical - BUBBLE_HEIGHT - BUBBLE_GAP;

    if let Ok(tb) = taskbar::current() {
        let (mon_left, mon_top, mon_right, mon_bottom) = monitor_logical_rect(&tb);
        match tb.edge {
            TaskbarEdge::Top => {
                let overlay_bottom_logical = overlay_y_logical + overlay_height_logical(&overlay);
                bubble_y = overlay_bottom_logical + BUBBLE_GAP;
            }
            TaskbarEdge::Left => {
                bubble_x = overlay_x_logical + overlay_width_logical(&overlay) + BUBBLE_GAP;
                bubble_y = overlay_y_logical + sprite_center_x_css - BUBBLE_HEIGHT / 2.0;
            }
            TaskbarEdge::Right => {
                bubble_x = overlay_x_logical - BUBBLE_WIDTH - BUBBLE_GAP;
                bubble_y = overlay_y_logical + sprite_center_x_css - BUBBLE_HEIGHT / 2.0;
            }
            TaskbarEdge::Bottom => {}
        }
        bubble_x = bubble_x.max(mon_left).min(mon_right - BUBBLE_WIDTH);
        bubble_y = bubble_y.max(mon_top).min(mon_bottom - BUBBLE_HEIGHT);
    }

    eprintln!(
        "[bubble] position overlay=({overlay_x_logical:.0},{overlay_y_logical:.0}) \
         sprite_cx={sprite_center_x_css:.0} → bubble=({bubble_x:.0},{bubble_y:.0})"
    );
    let _ = bubble.set_position(LogicalPosition::new(bubble_x, bubble_y));
    let _ = bubble.set_size(LogicalSize::new(BUBBLE_WIDTH, BUBBLE_HEIGHT));
}

fn overlay_width_logical<R: Runtime>(overlay: &WebviewWindow<R>) -> f64 {
    let scale = overlay.scale_factor().unwrap_or(1.0);
    overlay
        .outer_size()
        .map(|s| s.width as f64 / scale)
        .unwrap_or(0.0)
}

fn overlay_height_logical<R: Runtime>(overlay: &WebviewWindow<R>) -> f64 {
    let scale = overlay.scale_factor().unwrap_or(1.0);
    overlay
        .outer_size()
        .map(|s| s.height as f64 / scale)
        .unwrap_or(0.0)
}

fn monitor_logical_rect(tb: &TaskbarInfo) -> (f64, f64, f64, f64) {
    let s = if tb.dpi_scale > 0.0 { tb.dpi_scale } else { 1.0 };
    (
        tb.monitor_rect[0] as f64 / s,
        tb.monitor_rect[1] as f64 / s,
        tb.monitor_rect[2] as f64 / s,
        tb.monitor_rect[3] as f64 / s,
    )
}
