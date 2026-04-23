//! System tray icon + menu.
//!
//! Tray menu:
//!   • Show / Hide DockDuo  (toggles overlay; also on tray left-click)
//!   • Theme  ▸  Midnight · Daylight · Pastel · Retro   (live switch)
//!   • Start with Windows  (checkbox, backed by autostart plugin)
//!   • Hide on fullscreen  (checkbox, persisted)
//!   • Check for updates
//!   • About
//!   • Quit
//!
//! Theme and hide-on-fullscreen picks are persisted via `config.rs` and
//! broadcast to the overlay window via the `theme-changed` event, so the
//! UI updates without a reload.

use anyhow::Result;
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Emitter, Manager, Runtime,
};

use crate::hit_test::OVERLAY_WINDOW_LABELS;
use crate::{autostart, config::{self, Theme}, updater};

const MENU_ID_TOGGLE: &str = "toggle_overlay";
const MENU_ID_QUIT: &str = "quit";
const MENU_ID_THEME_MIDNIGHT: &str = "theme_midnight";
const MENU_ID_THEME_DAYLIGHT: &str = "theme_daylight";
const MENU_ID_THEME_PASTEL: &str = "theme_pastel";
const MENU_ID_THEME_RETRO: &str = "theme_retro";
const MENU_ID_HIDE_FS: &str = "hide_on_fullscreen";
const MENU_ID_AUTOSTART: &str = "start_with_windows";
const MENU_ID_CHECK_UPDATES: &str = "check_updates";
const MENU_ID_ABOUT: &str = "about";

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build and register the tray icon. Call once from `setup()`.
pub fn build<R: Runtime>(app: &App<R>) -> Result<()> {
    let handle = app.handle();
    let cfg = config::load();

    let toggle = MenuItem::with_id(
        handle,
        MENU_ID_TOGGLE,
        "Show / Hide DockDuo",
        true,
        Some("Ctrl+Shift+L"),
    )?;

    // Theme submenu — four mutually-exclusive "check" items. Tauri v2
    // doesn't ship a true radio primitive, so we simulate it by clearing
    // the other three whenever one is clicked.
    let mk_theme = |id: &str, label: &str, this: Theme| -> Result<CheckMenuItem<R>> {
        let checked = cfg.theme == this;
        Ok(CheckMenuItem::with_id(
            handle,
            id,
            label,
            true,
            checked,
            None::<&str>,
        )?)
    };
    let t_midnight = mk_theme(MENU_ID_THEME_MIDNIGHT, "Midnight", Theme::Midnight)?;
    let t_daylight = mk_theme(MENU_ID_THEME_DAYLIGHT, "Daylight", Theme::Daylight)?;
    let t_pastel = mk_theme(MENU_ID_THEME_PASTEL, "Pastel", Theme::Pastel)?;
    let t_retro = mk_theme(MENU_ID_THEME_RETRO, "Retro", Theme::Retro)?;

    let theme_sub = Submenu::with_items(
        handle,
        "Theme",
        true,
        &[&t_midnight, &t_daylight, &t_pastel, &t_retro],
    )?;

    let autostart_on = autostart::is_enabled(handle);
    let autostart_item = CheckMenuItem::with_id(
        handle,
        MENU_ID_AUTOSTART,
        "Start with Windows",
        true,
        autostart_on,
        None::<&str>,
    )?;

    let hide_fs = CheckMenuItem::with_id(
        handle,
        MENU_ID_HIDE_FS,
        "Hide on fullscreen",
        true,
        cfg.hide_on_fullscreen,
        None::<&str>,
    )?;

    let check_updates = MenuItem::with_id(
        handle,
        MENU_ID_CHECK_UPDATES,
        "Check for updates…",
        true,
        None::<&str>,
    )?;

    let about = MenuItem::with_id(
        handle,
        MENU_ID_ABOUT,
        format!("About DockDuo {}", APP_VERSION),
        true,
        None::<&str>,
    )?;

    let sep1 = PredefinedMenuItem::separator(handle)?;
    let sep2 = PredefinedMenuItem::separator(handle)?;
    let sep3 = PredefinedMenuItem::separator(handle)?;
    let quit = MenuItem::with_id(handle, MENU_ID_QUIT, "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        handle,
        &[
            &toggle,
            &sep1,
            &theme_sub,
            &autostart_item,
            &hide_fs,
            &sep2,
            &check_updates,
            &about,
            &sep3,
            &quit,
        ],
    )?;

    let t_midnight_c = t_midnight.clone();
    let t_daylight_c = t_daylight.clone();
    let t_pastel_c = t_pastel.clone();
    let t_retro_c = t_retro.clone();
    let hide_fs_c = hide_fs.clone();
    let autostart_c = autostart_item.clone();

    let icon = load_tray_icon();

    let mut builder = TrayIconBuilder::with_id("dockduo-tray")
        .tooltip("DockDuo — click to show/hide")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            MENU_ID_TOGGLE => {
                if let Err(e) = toggle_overlay(app) {
                    tracing::warn!(error = %e, "tray toggle failed");
                }
            }
            MENU_ID_QUIT => app.exit(0),
            MENU_ID_THEME_MIDNIGHT => {
                pick_theme(app, Theme::Midnight);
                sync_theme_checks(&t_midnight_c, &t_daylight_c, &t_pastel_c, &t_retro_c, Theme::Midnight);
            }
            MENU_ID_THEME_DAYLIGHT => {
                pick_theme(app, Theme::Daylight);
                sync_theme_checks(&t_midnight_c, &t_daylight_c, &t_pastel_c, &t_retro_c, Theme::Daylight);
            }
            MENU_ID_THEME_PASTEL => {
                pick_theme(app, Theme::Pastel);
                sync_theme_checks(&t_midnight_c, &t_daylight_c, &t_pastel_c, &t_retro_c, Theme::Pastel);
            }
            MENU_ID_THEME_RETRO => {
                pick_theme(app, Theme::Retro);
                sync_theme_checks(&t_midnight_c, &t_daylight_c, &t_pastel_c, &t_retro_c, Theme::Retro);
            }
            MENU_ID_HIDE_FS => {
                let want = hide_fs_c.is_checked().unwrap_or(true);
                if let Err(e) = config::set_hide_on_fullscreen(want) {
                    tracing::warn!(error = %e, "persist hide_on_fullscreen failed");
                }
                if !want {
                    for label in OVERLAY_WINDOW_LABELS {
                        if let Some(win) = app.get_webview_window(label) {
                            let _ = win.show();
                        }
                    }
                }
            }
            MENU_ID_AUTOSTART => {
                let want = autostart_c.is_checked().unwrap_or(false);
                match autostart::set_autostart(app.clone(), want) {
                    Ok(actual) => {
                        if actual != want {
                            let _ = autostart_c.set_checked(actual);
                        }
                        tracing::info!(enabled = actual, "autostart updated");
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "autostart toggle failed");
                        let _ = autostart_c.set_checked(!want);
                    }
                }
            }
            MENU_ID_CHECK_UPDATES => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let result = updater::check_for_updates(app_handle.clone()).await;
                    tracing::info!(
                        status = %result.status,
                        message = %result.message,
                        "update check finished"
                    );
                    let _ = app_handle.emit("update-check-result", &result);
                });
            }
            MENU_ID_ABOUT => {
                let _ = app.emit(
                    "about-opened",
                    serde_json::json!({
                        "version": APP_VERSION,
                        "product": "DockDuo",
                    }),
                );
            }
            other => tracing::debug!(id = %other, "unhandled tray menu id"),
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Err(e) = toggle_overlay(app) {
                    tracing::warn!(error = %e, "tray icon click toggle failed");
                }
            }
        });

    if let Some(img) = icon {
        builder = builder.icon(img);
    }

    builder.build(handle)?;
    Ok(())
}

/// Flip the overlay's visibility. Shared by tray menu, tray click, hotkey.
pub fn toggle_overlay<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let Some(primary) = app.get_webview_window("overlay") else {
        return Ok(());
    };
    let show = !primary.is_visible().unwrap_or(true);
    for label in OVERLAY_WINDOW_LABELS {
        if let Some(w) = app.get_webview_window(label) {
            if show {
                w.show()?;
            } else {
                w.hide()?;
            }
        }
    }
    Ok(())
}

fn pick_theme<R: Runtime>(app: &AppHandle<R>, theme: Theme) {
    if let Err(e) = config::set_theme(theme) {
        tracing::warn!(error = %e, "persist theme failed");
    }
    let payload = match theme {
        Theme::Midnight => "midnight",
        Theme::Daylight => "daylight",
        Theme::Pastel => "pastel",
        Theme::Retro => "retro",
    };
    if let Err(e) = app.emit("theme-changed", payload) {
        tracing::warn!(error = %e, "emit theme-changed failed");
    }
}

fn sync_theme_checks<R: Runtime>(
    midnight: &CheckMenuItem<R>,
    daylight: &CheckMenuItem<R>,
    pastel: &CheckMenuItem<R>,
    retro: &CheckMenuItem<R>,
    selected: Theme,
) {
    let _ = midnight.set_checked(selected == Theme::Midnight);
    let _ = daylight.set_checked(selected == Theme::Daylight);
    let _ = pastel.set_checked(selected == Theme::Pastel);
    let _ = retro.set_checked(selected == Theme::Retro);
}

fn load_tray_icon() -> Option<Image<'static>> {
    const PNG: &[u8] = include_bytes!("../icons/32x32.png");
    Image::from_bytes(PNG).ok()
}
