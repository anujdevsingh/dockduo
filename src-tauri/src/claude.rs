//! Coding-agent process spawning and lifecycle tracking.
//!
//! Detects which CLIs are installed — Claude Code (preferred),
//! OpenAI Codex, Google Gemini — and spawns the chosen one in a fresh
//! console (`cmd.exe /K <agent>` with `CREATE_NEW_CONSOLE`). On Windows 11
//! with Windows Terminal as the default console host, the new console is
//! hosted inside Windows Terminal automatically.
//!
//! A background thread watches each spawned process and emits
//! `ai-status-changed` events so the walking sprites' speech bubbles
//! can reflect busy / completed / idle.

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Mutex;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};

/// Win32 CREATE_NEW_CONSOLE — attaches a brand-new console to the child
/// process so it appears as its own terminal window.
#[cfg(windows)]
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

/// Win32 CREATE_NO_WINDOW — suppresses the console flash when probing
/// for CLI installs via `where.exe`.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// One child process per character slot.
static ACTIVE: Lazy<Mutex<HashMap<String, u32>>> = Lazy::new(Default::default);

/// Which coding-agent CLI to launch. Serialized to/from the frontend
/// so the picker bubble can round-trip the user's choice.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    Claude,
    Codex,
    Gemini,
}

impl AgentKind {
    fn binary(self) -> &'static str {
        match self {
            AgentKind::Claude => "claude",
            AgentKind::Codex => "codex",
            AgentKind::Gemini => "gemini",
        }
    }
    fn display_name(self) -> &'static str {
        match self {
            AgentKind::Claude => "Claude",
            AgentKind::Codex => "Codex",
            AgentKind::Gemini => "Gemini",
        }
    }
    /// Priority order for auto-select when only one agent is present
    /// — lower value wins ties. Currently: Claude < Codex < Gemini.
    const ORDER: [AgentKind; 3] = [AgentKind::Claude, AgentKind::Codex, AgentKind::Gemini];
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    pub kind: AgentKind,
    /// Absolute path (or bare name if on PATH) of the executable.
    pub path: String,
    /// Human-readable label shown in the picker bubble.
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiStatusEvent {
    pub character: String,
    /// "idle" | "busy" | "completed"
    pub status: String,
}

fn emit_status<R: Runtime>(app: &AppHandle<R>, character: &str, status: &str) {
    let ev = AiStatusEvent {
        character: character.to_string(),
        status: status.to_string(),
    };
    if let Err(e) = app.emit("ai-status-changed", ev) {
        tracing::warn!(error = %e, "failed to emit ai-status-changed");
    }
}

/// Try to find `<bin>` on this machine. Checks well-known Windows
/// install locations first, then falls back to `where.exe <bin>` which
/// walks PATH. Returns the absolute path if found.
fn detect_binary(bin: &str) -> Option<String> {
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let candidates = [
        format!("{home}\\.local\\bin\\{bin}.exe"),
        format!("{home}\\.local\\bin\\{bin}.cmd"),
        format!("{home}\\AppData\\Local\\Programs\\{bin}\\{bin}.exe"),
        format!("{home}\\AppData\\Roaming\\npm\\{bin}.cmd"),
        format!("{home}\\AppData\\Roaming\\npm\\{bin}.exe"),
    ];
    for p in &candidates {
        if std::path::Path::new(p).exists() {
            return Some(p.clone());
        }
    }

    // PATH fallback via `where.exe`. Suppress the console flash.
    let mut cmd = Command::new("where");
    cmd.arg(bin)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    if let Ok(output) = cmd.output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first) = stdout.lines().next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

/// List every coding-agent CLI currently installed on this machine.
/// The frontend uses this to decide whether to auto-spawn the single
/// found agent, or show a picker when multiple are available.
#[tauri::command]
pub fn list_agents() -> Vec<AgentInfo> {
    let mut out = Vec::new();
    for kind in AgentKind::ORDER {
        if let Some(path) = detect_binary(kind.binary()) {
            out.push(AgentInfo {
                kind,
                path,
                display_name: kind.display_name().to_string(),
            });
        }
    }
    out
}

/// Spawn the chosen agent in a new console for the given character.
/// De-dupes per character slot — a second click while the terminal is
/// already open is rejected with an "already running" error.
#[tauri::command]
pub fn spawn_agent<R: Runtime>(
    app: AppHandle<R>,
    character: String,
    #[allow(non_snake_case)] agentPath: String,
) -> Result<u32, String> {
    {
        let active = ACTIVE.lock().unwrap();
        if active.contains_key(&character) {
            return Err(format!("already running for '{character}'"));
        }
    }

    if agentPath.trim().is_empty() {
        return Err("empty agent path".into());
    }

    let mut cmd = Command::new("cmd.exe");
    cmd.args(["/K", &agentPath]);

    // CREATE_NEW_CONSOLE attaches a brand-new console window. Do NOT
    // redirect stdio to NUL — that would make `/K` see EOF immediately
    // and the terminal would vanish in milliseconds.
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NEW_CONSOLE);

    let child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn '{agentPath}'"))
        .map_err(|e| e.to_string())?;

    let pid = child.id();
    ACTIVE.lock().unwrap().insert(character.clone(), pid);

    emit_status(&app, &character, "busy");

    // Watch for exit on a background thread.
    let app_handle = app.clone();
    let character_bg = character.clone();
    std::thread::spawn(move || {
        let mut child = child;
        match child.wait() {
            Ok(status) => {
                tracing::info!(?status, character = %character_bg, "agent terminal exited");
            }
            Err(e) => {
                tracing::warn!(error = %e, character = %character_bg, "wait on agent terminal failed");
            }
        }
        emit_status(&app_handle, &character_bg, "completed");
        std::thread::sleep(std::time::Duration::from_millis(2500));
        emit_status(&app_handle, &character_bg, "idle");
        ACTIVE.lock().unwrap().remove(&character_bg);
    });

    Ok(pid)
}
