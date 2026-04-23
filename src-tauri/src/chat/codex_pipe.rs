//! Codex — one `codex exec --json` subprocess per user turn; transcript is stitched into the prompt.

use std::collections::HashMap;
use std::fmt::Write;
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime};

use crate::binary_resolve::{command_for_agent_binary, detect_agent_binary};

use super::claude_pipe::parse_claude_line;
use super::protocol::{ChatEnvelope, ChatEventBody};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const CHAT_IDLE_AFTER: Duration = Duration::from_millis(500);

static SESSIONS: once_cell::sync::Lazy<Mutex<HashMap<String, CodexSlot>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

struct CodexSlot {
    history: Arc<Mutex<String>>,
    activity: Arc<Mutex<(Instant, bool)>>,
    shutdown: Arc<AtomicBool>,
    exec_busy: Arc<AtomicBool>,
}

fn emit<R: Runtime>(app: &AppHandle<R>, character: &str, event: ChatEventBody) {
    let env = ChatEnvelope {
        character: character.to_string(),
        event,
    };
    if let Err(e) = app.emit("chat-agent-event", &env) {
        tracing::warn!(error = %e, "codex chat: emit failed");
    }
}

fn emit_ai_status<R: Runtime>(app: &AppHandle<R>, character: &str, status: &str) {
    let ev = crate::claude::AiStatusEvent {
        character: character.to_string(),
        status: status.to_string(),
    };
    if let Err(e) = app.emit("ai-status-changed", &ev) {
        tracing::warn!(error = %e, "codex chat: ai-status emit failed");
    }
}

fn touch_busy<R: Runtime>(
    app: &AppHandle<R>,
    character: &str,
    activity: &Arc<Mutex<(Instant, bool)>>,
    shutdown: &Arc<AtomicBool>,
) {
    if shutdown.load(Ordering::SeqCst) {
        return;
    }
    let mut g = activity.lock().unwrap();
    g.0 = Instant::now();
    if !g.1 {
        g.1 = true;
        emit_ai_status(app, character, "busy");
    }
}

fn spawn_idle_watcher<R: Runtime>(
    app: AppHandle<R>,
    character: String,
    activity: Arc<Mutex<(Instant, bool)>>,
    shutdown: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        while !shutdown.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(150));
            let should_idle = {
                let g = activity.lock().unwrap();
                g.1 && g.0.elapsed() >= CHAT_IDLE_AFTER
            };
            if should_idle {
                let mut g = activity.lock().unwrap();
                if g.1 && g.0.elapsed() >= CHAT_IDLE_AFTER {
                    g.1 = false;
                    if !shutdown.load(Ordering::SeqCst) {
                        emit_ai_status(&app, &character, "idle");
                    }
                }
            }
        }
    });
}

/// Codex `exec --json` NDJSON; falls back to [`parse_claude_line`] for overlapping shapes.
pub(crate) fn parse_codex_line(line: &str) -> Vec<ChatEventBody> {
    let line = line.trim();
    if line.is_empty() {
        return vec![];
    }
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let mut out = Vec::new();

    if let Some(typ) = v.get("type").and_then(|t| t.as_str()) {
        match typ {
            "item.completed" => {
                if let Some(item) = v.get("item") {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        out.push(ChatEventBody::AssistantDelta {
                            text: text.to_string(),
                        });
                        out.push(ChatEventBody::AssistantDone);
                        out.push(ChatEventBody::TurnComplete);
                    }
                }
            }
            "turn.completed" | "response.completed" => {
                out.push(ChatEventBody::AssistantDone);
                out.push(ChatEventBody::TurnComplete);
            }
            "error" => {
                let msg = v
                    .get("error")
                    .and_then(|e| e.as_str())
                    .or_else(|| v.get("message").and_then(|m| m.as_str()))
                    .unwrap_or("unknown error")
                    .to_string();
                out.push(ChatEventBody::Error { message: msg });
                out.push(ChatEventBody::TurnComplete);
            }
            _ => {}
        }
    }

    if out.is_empty() {
        if let Some(text) = v
            .pointer("/delta/text")
            .and_then(|x| x.as_str())
            .or_else(|| v.pointer("/item/text").and_then(|x| x.as_str()))
        {
            out.push(ChatEventBody::AssistantDelta {
                text: text.to_string(),
            });
        }
    }

    out
}

struct ExecBusyGuard(Arc<AtomicBool>);

impl Drop for ExecBusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn run_one_turn<R: Runtime>(
    app: &AppHandle<R>,
    character: &str,
    prompt: &str,
    history: &Arc<Mutex<String>>,
    activity: &Arc<Mutex<(Instant, bool)>>,
    shutdown: &Arc<AtomicBool>,
) {
    let Some(bin) = detect_agent_binary("codex") else {
        emit(
            app,
            character,
            ChatEventBody::Error {
                message: "Codex CLI not found. Install the OpenAI Codex CLI and ensure `codex` is on PATH."
                    .into(),
            },
        );
        emit(app, character, ChatEventBody::TurnComplete);
        return;
    };

    // Sandbox codex's working directory to a per-character scratch dir
    // rather than %USERPROFILE% so tool-use (file writes, shell execution
    // via --full-auto) has a tight blast radius.
    let workdir = match super::agent_sandbox_dir(character) {
        Ok(p) => p,
        Err(e) => {
            emit(
                app,
                character,
                ChatEventBody::Error {
                    message: format!("codex sandbox dir failed: {e}"),
                },
            );
            emit(app, character, ChatEventBody::TurnComplete);
            return;
        }
    };
    let mut cmd = command_for_agent_binary(&bin);
    // `--` ends option parsing, so a prompt starting with `--` (e.g.
    // `--cwd C:\...`) cannot be interpreted by codex as an additional flag.
    cmd.args([
        "exec",
        "--json",
        "--full-auto",
        "--skip-git-repo-check",
        "--",
        prompt,
    ])
    .current_dir(&workdir)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            emit(
                app,
                character,
                ChatEventBody::Error {
                    message: format!("codex spawn failed: {e}"),
                },
            );
            emit(app, character, ChatEventBody::TurnComplete);
            return;
        }
    };

    if let Some(mut err) = child.stderr.take() {
        let app_e = app.clone();
        let ch = character.to_string();
        let act_e = activity.clone();
        let shut_e = shutdown.clone();
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = std::io::Read::read_to_string(&mut err, &mut s);
            if !s.trim().is_empty() && !shut_e.load(Ordering::SeqCst) {
                tracing::warn!(character = %ch, stderr = %s, "codex stderr");
                touch_busy(&app_e, &ch, &act_e, &shut_e);
                emit(
                    &app_e,
                    &ch,
                    ChatEventBody::Error {
                        message: s.trim().to_string(),
                    },
                );
            }
        });
    }

    let Some(stdout) = child.stdout.take() else {
        emit(
            app,
            character,
            ChatEventBody::Error {
                message: "codex stdout missing".into(),
            },
        );
        emit(app, character, ChatEventBody::TurnComplete);
        return;
    };

    let mut reply = String::new();
    let mut saw_turn_complete = false;
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        if shutdown.load(Ordering::SeqCst) {
            let _ = child.kill();
            break;
        }
        let Ok(line) = line else { break };
        let mut events = parse_codex_line(&line);
        if events.is_empty() {
            events = parse_claude_line(&line);
        }
        for ev in events {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            match &ev {
                ChatEventBody::AssistantDelta { text } => {
                    reply.push_str(text);
                    touch_busy(app, character, activity, shutdown);
                }
                ChatEventBody::ToolUse { .. }
                | ChatEventBody::ToolResult { .. }
                | ChatEventBody::Error { .. } => {
                    touch_busy(app, character, activity, shutdown);
                }
                ChatEventBody::TurnComplete => {
                    saw_turn_complete = true;
                }
                ChatEventBody::AssistantDone => {}
                _ => {}
            }
            emit(app, character, ev);
        }
    }

    let _ = child.wait();

    if shutdown.load(Ordering::SeqCst) {
        return;
    }

    if !saw_turn_complete {
        if !reply.is_empty() {
            emit(app, character, ChatEventBody::AssistantDone);
        }
        emit(app, character, ChatEventBody::TurnComplete);
    }

    if !reply.trim().is_empty() {
        if let Ok(mut h) = history.lock() {
            let _ = writeln!(&mut *h, "Assistant: {}", reply.trim());
        }
    }
}

pub fn has_session(character: &str) -> bool {
    SESSIONS
        .lock()
        .map(|g| g.contains_key(character))
        .unwrap_or(false)
}

pub fn kill_quiet(character: &str) {
    if let Ok(mut g) = SESSIONS.lock() {
        if let Some(slot) = g.remove(character) {
            slot.shutdown.store(true, Ordering::SeqCst);
        }
    }
}

pub fn terminate_user_closed<R: Runtime>(app: &AppHandle<R>, character: &str) {
    let had = SESSIONS
        .lock()
        .map(|g| g.contains_key(character))
        .unwrap_or(false);
    kill_quiet(character);
    if !had {
        return;
    }
    let app = app.clone();
    let ch = character.to_string();
    std::thread::spawn(move || {
        emit_ai_status(&app, &ch, "completed");
        std::thread::sleep(Duration::from_millis(2500));
        emit_ai_status(&app, &ch, "idle");
    });
}

pub fn on_user_input<R: Runtime>(app: &AppHandle<R>, character: &str) -> Result<(), String> {
    let g = SESSIONS.lock().map_err(|e| e.to_string())?;
    let slot = g
        .get(character)
        .ok_or_else(|| format!("no chat session for '{character}'"))?;
    touch_busy(app, character, &slot.activity, &slot.shutdown);
    Ok(())
}

pub fn send_user<R: Runtime>(
    app: &AppHandle<R>,
    character: &str,
    text: &str,
) -> Result<(), String> {
    on_user_input(app, character)?;

    let (history, activity, shutdown, exec_busy, prompt) = {
        let g = SESSIONS.lock().map_err(|e| e.to_string())?;
        let slot = g
            .get(character)
            .ok_or_else(|| format!("no chat session for '{character}'"))?;
        if slot
            .exec_busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(
                "Codex is still answering the previous message; wait for it to finish.".into(),
            );
        }
        let mut h = slot.history.lock().map_err(|e| {
            slot.exec_busy.store(false, Ordering::SeqCst);
            e.to_string()
        })?;
        if let Err(e) = writeln!(&mut *h, "User: {text}") {
            slot.exec_busy.store(false, Ordering::SeqCst);
            return Err(e.to_string());
        }
        let prompt = h.clone();
        (
            slot.history.clone(),
            slot.activity.clone(),
            slot.shutdown.clone(),
            slot.exec_busy.clone(),
            prompt,
        )
    };

    let app_c = app.clone();
    let ch = character.to_string();
    std::thread::spawn(move || {
        let _guard = ExecBusyGuard(exec_busy);
        run_one_turn(&app_c, &ch, &prompt, &history, &activity, &shutdown);
    });

    Ok(())
}

pub fn start<R: Runtime>(app: AppHandle<R>, character: String) -> Result<(), String> {
    {
        let g = SESSIONS.lock().map_err(|e| e.to_string())?;
        if g.contains_key(&character) {
            emit(&app, &character, ChatEventBody::SessionReady);
            return Ok(());
        }
    }

    let _ = detect_agent_binary("codex").ok_or_else(|| {
        "Codex CLI not found. Install the OpenAI Codex CLI and ensure `codex` is on PATH."
            .to_string()
    })?;

    let activity = Arc::new(Mutex::new((Instant::now(), false)));
    let shutdown = Arc::new(AtomicBool::new(false));
    let history = Arc::new(Mutex::new(String::new()));
    let exec_busy = Arc::new(AtomicBool::new(false));

    SESSIONS
        .lock()
        .map_err(|e| e.to_string())?
        .insert(
            character.clone(),
            CodexSlot {
                history: history.clone(),
                activity: activity.clone(),
                shutdown: shutdown.clone(),
                exec_busy,
            },
        );

    spawn_idle_watcher(
        app.clone(),
        character.clone(),
        activity,
        shutdown,
    );
    emit(&app, &character, ChatEventBody::SessionReady);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_item_completed_agent_message() {
        let line = r#"{"type":"item.completed","item":{"type":"agent_message","text":"Hello"}}"#;
        let evs = parse_codex_line(line);
        assert!(
            evs.iter()
                .any(|e| matches!(e, ChatEventBody::AssistantDelta { text } if text == "Hello"))
        );
        assert!(evs.iter().any(|e| matches!(e, ChatEventBody::TurnComplete)));
    }
}
