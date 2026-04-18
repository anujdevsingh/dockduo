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

    #[cfg(windows)]
    disable_dwm_backdrop(&window);

    #[cfg(windows)]
    force_transparent_webview(&window);

    if let Ok(info) = crate::taskbar::current() {
        if let Err(e) = position_above_taskbar(&window, &info) {
            tracing::warn!(error = %e, "initial overlay positioning failed");
        }
    }

    Ok(())
}

/// Disable Windows 11 DWM backdrop effects (Mica, Acrylic, Tabbed) on the
/// overlay. When `transparent: true` is set in Tauri, DWM sometimes applies
/// an automatic acrylic-blur backdrop, which paints a translucent blurred
/// version of what's behind the window. We want pure see-through, so we
/// explicitly set the backdrop type to NONE.
#[cfg(windows)]
fn disable_dwm_backdrop(window: &WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMSBT_NONE, DWMWA_SYSTEMBACKDROP_TYPE,
        DWM_SYSTEMBACKDROP_TYPE,
    };

    let Ok(hwnd_raw) = window.hwnd() else {
        eprintln!("[dockduo] hwnd() failed in disable_dwm_backdrop");
        return;
    };
    let hwnd = HWND(hwnd_raw.0 as *mut core::ffi::c_void);

    // 1. Modern path: DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE = NONE)
    // Disables Mica / Acrylic / Tabbed backdrops on Windows 11.
    let backdrop: DWM_SYSTEMBACKDROP_TYPE = DWMSBT_NONE;
    unsafe {
        match DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
        ) {
            Ok(()) => eprintln!("[dockduo] DWMWA_SYSTEMBACKDROP_TYPE=NONE set"),
            Err(e) => eprintln!("[dockduo] DwmSetWindowAttribute(SYSTEMBACKDROP_TYPE) FAILED: {e:?}"),
        }
    }

    // 2. Legacy path (Windows 10 / old Win11): SetWindowCompositionAttribute
    // with ACCENT_DISABLED. This is an undocumented API that disables
    // acrylic / blur-behind / any window accent applied by the shell.
    disable_window_accent(hwnd);
}

/// Call the undocumented `SetWindowCompositionAttribute` to force
/// ACCENT_DISABLED on the window, disabling any blur-behind / acrylic
/// effect that the Windows shell may have applied.
#[cfg(windows)]
fn disable_window_accent(hwnd: windows::Win32::Foundation::HWND) {
    #[repr(C)]
    struct ACCENT_POLICY {
        accent_state: u32,
        accent_flags: u32,
        gradient_color: u32,
        animation_id: u32,
    }
    #[repr(C)]
    struct WINDOWCOMPOSITIONATTRIBDATA {
        attrib: u32,
        pv_data: *mut core::ffi::c_void,
        cb_data: usize,
    }
    const WCA_ACCENT_POLICY: u32 = 19;
    const ACCENT_DISABLED: u32 = 0;

    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows::core::PCSTR;

    type SetWindowCompositionAttributeFn = unsafe extern "system" fn(
        hwnd: windows::Win32::Foundation::HWND,
        data: *mut WINDOWCOMPOSITIONATTRIBDATA,
    ) -> i32;

    unsafe {
        let user32: HMODULE = match GetModuleHandleW(windows::core::w!("user32.dll")) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("[dockduo] GetModuleHandleW(user32) FAILED: {e:?}");
                return;
            }
        };
        let proc = GetProcAddress(user32, PCSTR(b"SetWindowCompositionAttribute\0".as_ptr()));
        let Some(proc) = proc else {
            eprintln!("[dockduo] SetWindowCompositionAttribute not found in user32.dll");
            return;
        };
        let set_attr: SetWindowCompositionAttributeFn = std::mem::transmute(proc);

        let mut policy = ACCENT_POLICY {
            accent_state: ACCENT_DISABLED,
            accent_flags: 0,
            gradient_color: 0,
            animation_id: 0,
        };
        let mut data = WINDOWCOMPOSITIONATTRIBDATA {
            attrib: WCA_ACCENT_POLICY,
            pv_data: &mut policy as *mut _ as *mut core::ffi::c_void,
            cb_data: std::mem::size_of::<ACCENT_POLICY>(),
        };
        let rc = set_attr(hwnd, &mut data);
        eprintln!("[dockduo] SetWindowCompositionAttribute(ACCENT_DISABLED) returned {rc}");
    }
}

/// Force WebView2 to paint a fully transparent default background.
///
/// Tauri's `transparent: true` makes the native window transparent, but
/// WebView2 still paints its own opaque default backdrop behind the page,
/// which leaves a visible translucent band on the overlay. The only
/// authoritative fix is calling `ICoreWebView2Controller2::put_DefaultBackgroundColor`.
#[cfg(windows)]
fn force_transparent_webview(window: &WebviewWindow) {
    let res = window.with_webview(|webview| {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2Controller2, COREWEBVIEW2_COLOR,
        };
        use windows::core::Interface;
        unsafe {
            let controller = webview.controller();
            match controller.cast::<ICoreWebView2Controller2>() {
                Ok(controller2) => {
                    let transparent = COREWEBVIEW2_COLOR {
                        A: 0,
                        R: 0,
                        G: 0,
                        B: 0,
                    };
                    match controller2.SetDefaultBackgroundColor(transparent) {
                        Ok(()) => {
                            eprintln!("[dockduo] WebView2 DefaultBackgroundColor set to transparent");
                        }
                        Err(e) => {
                            eprintln!("[dockduo] SetDefaultBackgroundColor FAILED: {e:?}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[dockduo] cast to ICoreWebView2Controller2 FAILED: {e:?}");
                }
            }
        }
    });
    if let Err(e) = res {
        eprintln!("[dockduo] with_webview FAILED: {e:?}");
    }
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
