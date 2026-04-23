//! Shared Windows agent binary path resolution (no dependency on `claude` / `chat`).

use std::process::{Command, Stdio};

/// Build a [`Command`] that runs `resolved_path` on Windows whether it's an `.exe`,
/// `.cmd`, or `.bat` file.
///
/// # Why we do *not* shell out to `cmd.exe`
///
/// The previous implementation wrapped `.cmd`/`.bat` shims with
/// `cmd.exe /d /c <path> <args...>`. That is *less* safe than calling them
/// directly: Rust's built-in argv quoting targets
/// `CommandLineToArgvW` (used by normal EXEs), which does **not** match
/// `cmd.exe`'s looser parser. A user prompt containing `"&calc&"` would
/// inject `calc` into the shell command line.
///
/// Since Rust 1.77.2 (the fix for CVE-2024-24576, aka BatBadBut), calling
/// `Command::new("path.cmd").arg(user_input)` applies a batch-file-aware
/// escape automatically and rejects arguments containing characters that
/// cannot be safely quoted. We rely on that, and do *not* go through
/// `cmd.exe` for any agent binary.
pub fn command_for_agent_binary(resolved_path: &str) -> Command {
    Command::new(resolved_path)
}

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Find `claude` / `codex` / `gemini` on PATH or well-known install dirs.
pub fn detect_agent_binary(bin: &str) -> Option<String> {
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

    // Fall back to `where.exe`, but pin its absolute path. A bare
    // `Command::new("where")` is resolved against %PATH% — a planted
    // `where.exe` higher on %PATH% would then be trusted to tell us where
    // the agent lives. Always use the copy shipped with Windows.
    let where_exe = where_exe_path();
    let mut cmd = Command::new(&where_exe);
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

/// Absolute path to `where.exe`, anchored at the real Windows install dir
/// via `%SystemRoot%` (falls back to `C:\Windows` if unset).
fn where_exe_path() -> String {
    #[cfg(windows)]
    {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        return format!("{root}\\System32\\where.exe");
    }
    #[cfg(not(windows))]
    {
        "where".to_string()
    }
}
