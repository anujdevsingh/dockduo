//! Windows Taskbar detection.
//!
//! Detects the taskbar edge, rect, DPI scale, and auto-hide state via
//! `SHAppBarMessage`. Polls every 1000 ms and emits a `taskbar-changed`
//! event whenever the observable state changes.
//!
//! v0.2.0 scope: primary taskbar only. Multi-monitor is deferred — see
//! `docs/DECISIONS.md` D-011.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(windows)]
use windows::Win32::{
    Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    },
    UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
    UI::Shell::{
        SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETSTATE,
        ABM_GETTASKBARPOS, ABS_AUTOHIDE, APPBARDATA,
    },
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskbarEdge {
    Bottom,
    Top,
    Left,
    Right,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskbarInfo {
    pub edge: TaskbarEdge,
    pub rect: [i32; 4],
    pub auto_hide: bool,
    pub dpi_scale: f64,
    pub monitor_rect: [i32; 4],
}

#[cfg(windows)]
pub fn current() -> Result<TaskbarInfo> {
    use std::mem::size_of;

    unsafe {
        let mut abd = APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            ..Default::default()
        };

        let ret = SHAppBarMessage(ABM_GETTASKBARPOS, &mut abd);
        if ret == 0 {
            return Err(anyhow!("SHAppBarMessage(ABM_GETTASKBARPOS) returned 0"));
        }

        let edge = match abd.uEdge {
            x if x == ABE_BOTTOM => TaskbarEdge::Bottom,
            x if x == ABE_TOP => TaskbarEdge::Top,
            x if x == ABE_LEFT => TaskbarEdge::Left,
            x if x == ABE_RIGHT => TaskbarEdge::Right,
            _ => TaskbarEdge::Bottom,
        };

        let rect = [abd.rc.left, abd.rc.top, abd.rc.right, abd.rc.bottom];

        let mut abd_state = APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            ..Default::default()
        };
        let state = SHAppBarMessage(ABM_GETSTATE, &mut abd_state);
        let auto_hide = (state & ABS_AUTOHIDE as usize) != 0;

        let hmon = MonitorFromWindow(abd.hWnd, MONITOR_DEFAULTTONEAREST);
        let mut dpi_x: u32 = 96;
        let mut dpi_y: u32 = 96;
        let _ = GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let dpi_scale = dpi_x as f64 / 96.0;

        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(hmon, &mut mi);
        let monitor_rect = [
            mi.rcMonitor.left,
            mi.rcMonitor.top,
            mi.rcMonitor.right,
            mi.rcMonitor.bottom,
        ];

        Ok(TaskbarInfo {
            edge,
            rect,
            auto_hide,
            dpi_scale,
            monitor_rect,
        })
    }
}

#[cfg(not(windows))]
pub fn current() -> Result<TaskbarInfo> {
    Err(anyhow!("taskbar detection is only implemented on Windows"))
}

/// Spawn an async task that polls the primary taskbar every 1000 ms.
/// On change, emits `taskbar-changed` and repositions the overlay.
pub fn start_polling(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last: Option<TaskbarInfo> = None;
        loop {
            match current() {
                Ok(info) => {
                    let changed = last.as_ref().map(|l| l != &info).unwrap_or(true);
                    if changed {
                        tracing::info!(edge = ?info.edge, "primary taskbar changed");
                        let _ = app.emit("taskbar-changed", info.clone());
                        if let Some(window) = app.get_webview_window("overlay") {
                            if let Err(e) =
                                crate::overlay::position_above_taskbar(&window, &info)
                            {
                                tracing::warn!(error = %e, "reposition overlay");
                            }
                            let _ = window.show();
                        }
                        last = Some(info);
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "taskbar detection failed");
                }
            }
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    });
}
