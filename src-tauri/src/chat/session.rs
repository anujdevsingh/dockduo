//! Tauri commands for the embedded chat (Claude long-lived pipe, Codex/Gemini turn-based exec).

use tauri::{AppHandle, Emitter, Runtime};

use crate::claude::AgentKind;

use super::claude_pipe;
use super::codex_pipe;
use super::gemini_pipe;
use super::protocol::{ChatEnvelope, ChatEventBody};

fn active_chat_kind(character: &str) -> Option<AgentKind> {
    if claude_pipe::has_claude_chat(character) {
        Some(AgentKind::Claude)
    } else if codex_pipe::has_session(character) {
        Some(AgentKind::Codex)
    } else if gemini_pipe::has_session(character) {
        Some(AgentKind::Gemini)
    } else {
        None
    }
}

/// Cap on a single user message. Agent CLIs can technically accept more,
/// but an unbounded argv string is a DoS vector from a compromised webview
/// and ~32 KB is already well past any realistic chat input.
const MAX_PROMPT_BYTES: usize = 32 * 1024;

fn sanitise_prompt(text: &str) -> Result<String, String> {
    // Strip lone CR bytes (normalise to \n) and reject NUL, which some CLIs
    // interpret as end-of-input.
    let cleaned = text.replace('\r', "");
    if cleaned.contains('\0') {
        return Err("message contains NUL byte".into());
    }
    if cleaned.len() > MAX_PROMPT_BYTES {
        return Err(format!(
            "message too long ({} bytes, max {MAX_PROMPT_BYTES})",
            cleaned.len()
        ));
    }
    Ok(cleaned)
}

#[tauri::command]
pub fn chat_start_session<R: Runtime>(
    app: AppHandle<R>,
    character: String,
    kind: AgentKind,
) -> Result<(), String> {
    super::validate_character(&character)?;
    if let Some(active) = active_chat_kind(&character) {
        if active == kind {
            // Same agent already running — no-op so reopening the bubble keeps
            // the existing transcript alive.
            return Ok(());
        }
        claude_pipe::kill_quiet(&character);
        codex_pipe::kill_quiet(&character);
        gemini_pipe::kill_quiet(&character);
    }

    match kind {
        AgentKind::Claude => claude_pipe::start(app, character),
        AgentKind::Codex => codex_pipe::start(app, character),
        AgentKind::Gemini => gemini_pipe::start(app, character),
    }
}

#[tauri::command]
pub fn chat_send<R: Runtime>(
    app: AppHandle<R>,
    character: String,
    text: String,
) -> Result<(), String> {
    super::validate_character(&character)?;
    let text = sanitise_prompt(&text)?;
    if claude_pipe::has_claude_chat(&character) {
        claude_pipe::on_user_input(&app, &character)?;
        claude_pipe::send_user(&character, &text)?;
        return Ok(());
    }
    if codex_pipe::has_session(&character) {
        codex_pipe::send_user(&app, &character, &text)?;
        return Ok(());
    }
    if gemini_pipe::has_session(&character) {
        gemini_pipe::send_user(&app, &character, &text)?;
        return Ok(());
    }
    Err(format!(
        "No chat session for '{character}'. Click the sprite to open a bubble."
    ))
}

#[tauri::command]
pub fn chat_clear_session<R: Runtime>(app: AppHandle<R>, character: String) -> Result<(), String> {
    super::validate_character(&character)?;
    claude_pipe::kill_quiet(&character);
    codex_pipe::kill_quiet(&character);
    gemini_pipe::kill_quiet(&character);
    let env = ChatEnvelope {
        character: character.clone(),
        event: ChatEventBody::SessionReady,
    };
    let _ = app.emit("chat-agent-event", &env);
    Ok(())
}

#[tauri::command]
pub fn chat_terminate<R: Runtime>(app: AppHandle<R>, character: String) -> Result<(), String> {
    super::validate_character(&character)?;
    if claude_pipe::has_claude_chat(&character) {
        claude_pipe::terminate_user_closed(&app, &character);
    } else if codex_pipe::has_session(&character) {
        codex_pipe::terminate_user_closed(&app, &character);
    } else if gemini_pipe::has_session(&character) {
        gemini_pipe::terminate_user_closed(&app, &character);
    }
    Ok(())
}
