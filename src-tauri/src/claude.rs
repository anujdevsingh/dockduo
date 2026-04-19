//! Claude Code process spawning and lifecycle tracking.
//!
//! `spawn_claude` launches `cmd.exe /K claude` with a brand-new console
//! attached (`CREATE_NEW_CONSOLE`). On Windows 11 with Windows Terminal
//! set as the default console host, the new console is hosted inside
//! Windows Terminal automatically. A background thread watches each
//! spawned process and emits `ai-status-changed` events so the walking
//! sprites' speech bubbles can reflect busy / completed / idle.

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Mutex;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

/// Win32 CREATE_NEW_CONSOLE — attaches a brand-new console to the child
/// process so it appears as its own terminal window. On Windows 11 with
/// Windows Terminal set as the default console host, the new console is
/// hosted inside Windows Terminal automatically.
#[cfg(windows)]
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

/// One child process per character slot. Tracked so we don't spawn
/// duplicates and can query "is this character busy?" cheaply.
static ACTIVE: Lazy<Mutex<HashMap<String, u32>>> = Lazy::new(Default::default);

#[derive(Debug, Clone, Serialize)]
pub struct AiStatusEvent {
    pub character: String,
    /// "idle" | "busy" | "completed"
    pub status: String,
}

/// Emit a status change to the overlay window.
fn emit_status<R: Runtime>(app: &AppHandle<R>, character: &str, status: &str) {
    let ev = AiStatusEvent {
        character: character.to_string(),
        status: status.to_string(),
    };
    if let Err(e) = app.emit("ai-status-changed", ev) {
        tracing::warn!(error = %e, "failed to emit ai-status-changed");
    }
}

/// Resolve the `claude` executable. Prefers an explicit path if found in
/// the standard Windows install location, otherwise trusts PATH.
fn resolve_claude() -> Result<String> {
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let candidates = [
        format!("{home}\\.local\\bin\\claude.exe"),
        format!("{home}\\AppData\\Local\\Programs\\claude\\claude.exe"),
    ];
    for candidate in &candidates {
        if std::path::Path::new(candidate).exists() {
            return Ok(candidate.clone());
        }
    }
    if Command::new("claude.exe")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
    {
        return Ok("claude.exe".to_string());
    }
    Err(anyhow!(
        "claude executable not found — install Claude Code and ensure it's on PATH"
    ))
}

/// Spawn a Claude terminal for the given character. Returns the child
/// PID so the frontend can correlate events if needed.
///
/// De-dupes on the character slot: if a terminal is already open for
/// Bruce, clicking Bruce again returns an error instead of spawning a
/// second window.
#[tauri::command]
pub fn spawn_claude<R: Runtime>(app: AppHandle<R>, character: String) -> Result<u32, String> {
    {
        let active = ACTIVE.lock().unwrap();
        if active.contains_key(&character) {
            return Err(format!("claude already running for '{character}'"));
        }
    }

    let claude_path = resolve_claude().map_err(|e| e.to_string())?;

    let mut cmd = Command::new("cmd.exe");
    cmd.args(["/K", &claude_path]);

    // CREATE_NEW_CONSOLE attaches a brand-new console window to cmd.exe.
    // Do NOT redirect stdin/stdout/stderr to Stdio::null() — if we do,
    // cmd.exe's handles point to NUL instead of the new console, so
    // `/K` gets EOF immediately and claude.exe reads EOF on stdin,
    // both exit within milliseconds and the terminal window vanishes.
    // Leaving them unset lets the child inherit fresh handles bound to
    // the new console, which is what we want.
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NEW_CONSOLE);

    let child = cmd
        .spawn()
        .with_context(|| "failed to spawn terminal")
        .map_err(|e| e.to_string())?;

    let pid = child.id();
    ACTIVE.lock().unwrap().insert(character.clone(), pid);

    // Character enters "busy" state the moment we spawn.
    emit_status(&app, &character, "busy");

    // Watch the process exit in a background thread. When the terminal
    // closes, flash "completed" then return to "idle".
    let app_handle = app.clone();
    let character_bg = character.clone();
    std::thread::spawn(move || {
        let mut child = child;
        match child.wait() {
            Ok(status) => {
                tracing::info!(?status, character = %character_bg, "claude terminal exited");
            }
            Err(e) => {
                tracing::warn!(error = %e, character = %character_bg, "wait on claude terminal failed");
            }
        }
        emit_status(&app_handle, &character_bg, "completed");
        std::thread::sleep(std::time::Duration::from_millis(2500));
        emit_status(&app_handle, &character_bg, "idle");
        ACTIVE.lock().unwrap().remove(&character_bg);
    });

    Ok(pid)
}
