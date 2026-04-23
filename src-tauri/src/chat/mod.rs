pub mod claude_pipe;
pub mod codex_pipe;
pub mod gemini_pipe;
pub mod protocol;
pub mod session;

pub use session::{chat_clear_session, chat_send, chat_start_session, chat_terminate};

/// True if this character has an active embedded chat session (any agent).
pub fn has_any_chat_session(character: &str) -> bool {
    claude_pipe::has_claude_chat(character)
        || codex_pipe::has_session(character)
        || gemini_pipe::has_session(character)
}

/// Closed set of valid character ids. The IPC boundary accepts a free-form
/// `String` from the webview, which we use as a key in several HashMaps and
/// to construct filesystem paths / window labels — so we reject anything
/// that isn't a known character at the edge.
pub fn validate_character(character: &str) -> Result<(), String> {
    match character {
        "bruce" | "jazz" => Ok(()),
        _ => Err(format!("invalid character: {character:?}")),
    }
}

/// Per-character sandbox directory at `%APPDATA%\DockDuo\agents\<character>\`.
/// Used as `current_dir` for spawned agent CLIs so any tool-use (file writes,
/// shell execution by Claude's `--dangerously-skip-permissions` mode, etc.)
/// has a tight blast radius instead of landing in `%USERPROFILE%`.
///
/// The directory is created on demand; returns a filesystem path suitable
/// for `Command::current_dir`.
pub fn agent_sandbox_dir(character: &str) -> Result<std::path::PathBuf, String> {
    validate_character(character)?;
    let base = dirs::data_dir()
        .map(|d| d.join("DockDuo"))
        .or_else(|| {
            // Fallback to %APPDATA%\DockDuo if dirs can't figure it out.
            std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("DockDuo"))
        })
        .ok_or_else(|| "could not locate app data dir".to_string())?;
    let dir = base.join("agents").join(character);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("create sandbox {}: {e}", dir.display()))?;
    Ok(dir)
}
