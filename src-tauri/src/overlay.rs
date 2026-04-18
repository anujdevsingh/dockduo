//! Overlay window setup and positioning.
//!
//! Configures the transparent click-through overlay and repositions it
//! over the taskbar whenever the taskbar moves.

use anyhow::Result;
use tauri::{App, LogicalPosition, LogicalSize, Manager, WebviewWindow};

use crate::taskbar::{TaskbarEdge, TaskbarInfo};

/// Overlay height in CSS pixels — matches taskbar height on standard 1080p.
const OVERLAY_HEIGHT_CSS: f64 = 200.0;

pub fn configure_overlay_window(app: &App) -> Result<()> {
    let window = app
        .get_webview_window("overlay")
        .ok_or_else(|| anyhow::anyhow!("overlay window not found"))?;

    // Click-through by default; the frontend toggles this off when the
    // cursor enters a character sprite.
    window.set_ignore_cursor_events(true)?;

    #[cfg(windows)]
    apply_tool_window_style(&window)?;

    if let Ok(info) = crate::taskbar::current() {
        if let Err(e) = position_above_taskbar(&window, &info) {
            tracing::warn!(error = %e, "initial overlay positioning failed");
        }
    }

    Ok(())
}

#[cfg(windows)]
fn apply_tool_window_style(window: &WebviewWindow) -> Result<()> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    let raw = window.hwnd()?.0 as isize;
    let hwnd = HWND(raw as *mut core::ffi::c_void);
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new = current | WS_EX_TOOLWINDOW.0 as isize | WS_EX_NOACTIVATE.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new);
    }
    Ok(())
}

/// Position the overlay directly above the taskbar edge.
///
/// Reads physical-pixel rects from `TaskbarInfo` and converts to CSS
/// pixels by dividing by `dpi_scale` (Tauri expects logical units).
pub fn position_above_taskbar(window: &WebviewWindow, tb: &TaskbarInfo) -> Result<()> {
    let scale = if tb.dpi_scale > 0.0 { tb.dpi_scale } else { 1.0 };
    let mon_l = tb.monitor_rect[0] as f64 / scale;
    let mon_t = tb.monitor_rect[1] as f64 / scale;
    let mon_r = tb.monitor_rect[2] as f64 / scale;
    let mon_b = tb.monitor_rect[3] as f64 / scale;
    let tb_l = tb.rect[0] as f64 / scale;
    let tb_t = tb.rect[1] as f64 / scale;
    let tb_r = tb.rect[2] as f64 / scale;
    let tb_b = tb.rect[3] as f64 / scale;

    let (x, y, w, h) = match tb.edge {
        TaskbarEdge::Bottom => (
            mon_l,
            tb_t - OVERLAY_HEIGHT_CSS,
            mon_r - mon_l,
            OVERLAY_HEIGHT_CSS,
        ),
        TaskbarEdge::Top => (mon_l, tb_b, mon_r - mon_l, OVERLAY_HEIGHT_CSS),
        TaskbarEdge::Left => (tb_r, mon_t, OVERLAY_HEIGHT_CSS, mon_b - mon_t),
        TaskbarEdge::Right => (
            tb_l - OVERLAY_HEIGHT_CSS,
            mon_t,
            OVERLAY_HEIGHT_CSS,
            mon_b - mon_t,
        ),
    };

    window.set_position(LogicalPosition::new(x, y))?;
    window.set_size(LogicalSize::new(w, h))?;
    Ok(())
}
