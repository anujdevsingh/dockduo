//! Claude Code headless session over stdin/stdout pipes (stream-json).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime};

use crate::binary_resolve::{command_for_agent_binary, detect_agent_binary};

use super::protocol::{ChatEnvelope, ChatEventBody};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Same as `pty::IDLE_AFTER` — sprite bubbles drop to idle after this much
/// silence while the session is marked busy (matches terminal UX).
const CHAT_IDLE_AFTER: Duration = Duration::from_millis(500);

static SESSIONS: once_cell::sync::Lazy<Mutex<HashMap<String, ClaudeSlot>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

struct ClaudeSlot {
    stdin_tx: Sender<String>,
    child: Arc<Mutex<Option<Child>>>,
    /// `(last activity, showing_busy)` — mirrors `pty::SessionHandle::activity`.
    activity: Arc<Mutex<(Instant, bool)>>,
    shutdown: Arc<AtomicBool>,
}

fn emit<R: Runtime>(app: &AppHandle<R>, character: &str, event: ChatEventBody) {
    let env = ChatEnvelope {
        character: character.to_string(),
        event,
    };
    if let Err(e) = app.emit("chat-agent-event", &env) {
        tracing::warn!(error = %e, "chat: emit failed");
    }
}

fn emit_ai_status<R: Runtime>(app: &AppHandle<R>, character: &str, status: &str) {
    let ev = crate::claude::AiStatusEvent {
        character: character.to_string(),
        status: status.to_string(),
    };
    if let Err(e) = app.emit("ai-status-changed", &ev) {
        tracing::warn!(error = %e, "chat: ai-status emit failed");
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

/// Call before writing user JSON — same idea as `pty_write` flipping busy on Enter.
pub fn on_user_input<R: Runtime>(app: &AppHandle<R>, character: &str) -> Result<(), String> {
    let g = SESSIONS.lock().map_err(|e| e.to_string())?;
    let slot = g
        .get(character)
        .ok_or_else(|| format!("no chat session for '{character}'"))?;
    touch_busy(app, character, &slot.activity, &slot.shutdown);
    Ok(())
}

fn spawn_chat_idle_watcher<R: Runtime>(
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

/// Parse one NDJSON line from `claude --output-format stream-json` into UI events.
pub(crate) fn parse_claude_line(line: &str) -> Vec<ChatEventBody> {
    let line = line.trim();
    if line.is_empty() {
        return vec![];
    }
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let mut out = Vec::new();

    let typ = v.get("type").and_then(|t| t.as_str());

    match typ {
        Some("stream_event") => {
            let ev_inner = v.get("event");
            if let Some(sub) = ev_inner {
                if let Some(et) = sub.get("type").and_then(|t| t.as_str()) {
                    // Claude Code stream-json often ends a turn with message_stop
                    // (no separate `result` line), which we must map or Bruce/Jazz
                    // stay idle the whole time.
                    if et == "message_stop" {
                        out.push(ChatEventBody::AssistantDone);
                        out.push(ChatEventBody::TurnComplete);
                    }
                }
            }
            if let Some(text) = extract_delta_text(ev_inner) {
                out.push(ChatEventBody::AssistantDelta { text });
            }
            if let Some((name, summary)) = extract_tool_delta(ev_inner) {
                out.push(ChatEventBody::ToolUse { name, summary });
            }
        }
        Some("assistant") | Some("message") => {
            if let Some(text) = extract_assistant_message_text(&v) {
                out.push(ChatEventBody::AssistantDelta { text });
            }
        }
        Some("result") | Some("final") => {
            out.push(ChatEventBody::AssistantDone);
            out.push(ChatEventBody::TurnComplete);
        }
        Some("error") => {
            let msg = v
                .get("error")
                .and_then(|e| e.as_str())
                .or_else(|| v.get("message").and_then(|m| m.as_str()))
                .unwrap_or("unknown error")
                .to_string();
            out.push(ChatEventBody::Error { message: msg });
            out.push(ChatEventBody::TurnComplete);
        }
        Some("tool_use") | Some("tool_start") => {
            if let Some(name) = v.get("tool_name").or_else(|| v.get("name")).and_then(|x| x.as_str())
            {
                let summary = v
                    .get("input")
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| name.to_string());
                out.push(ChatEventBody::ToolUse {
                    name: name.to_string(),
                    summary,
                });
            }
        }
        Some("tool_result") | Some("tool_end") => {
            let summary = v
                .get("output")
                .or_else(|| v.get("content"))
                .map(|x| x.to_string())
                .unwrap_or_default();
            let is_error = v
                .get("is_error")
                .and_then(|x| x.as_bool())
                .unwrap_or(false);
            out.push(ChatEventBody::ToolResult { summary, is_error });
        }
        _ => {
            if let Some(text) = extract_delta_text(Some(&v)) {
                if !text.is_empty() {
                    out.push(ChatEventBody::AssistantDelta { text });
                }
            }
        }
    }

    out
}

fn extract_delta_text(event: Option<&Value>) -> Option<String> {
    let ev = event?;
    if let Some(t) = ev
        .pointer("/delta/text")
        .and_then(|x| x.as_str())
        .or_else(|| ev.pointer("/delta/partial_json").and_then(|x| x.as_str()))
    {
        return Some(t.to_string());
    }
    if let Some(d) = ev.get("delta") {
        if let Some(t) = d.get("text").and_then(|x| x.as_str()) {
            return Some(t.to_string());
        }
    }
    None
}

fn extract_tool_delta(event: Option<&Value>) -> Option<(String, String)> {
    let ev = event?;
    let name = ev
        .get("tool_name")
        .or_else(|| ev.get("name"))
        .and_then(|x| x.as_str())?;
    let summary = ev
        .get("tool_input")
        .or_else(|| ev.get("input"))
        .map(|x| x.to_string())
        .unwrap_or_else(|| name.to_string());
    Some((name.to_string(), summary))
}

fn extract_assistant_message_text(v: &Value) -> Option<String> {
    let msg = v.get("message")?;
    let content = msg.get("content")?.as_array()?;
    let mut buf = String::new();
    for block in content {
        if let Some(t) = block.get("text").and_then(|x| x.as_str()) {
            buf.push_str(t);
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

pub fn has_claude_chat(character: &str) -> bool {
    SESSIONS
        .lock()
        .map(|g| g.contains_key(character))
        .unwrap_or(false)
}

fn kill_slot(character: &str) {
    if let Ok(mut g) = SESSIONS.lock() {
        if let Some(slot) = g.remove(character) {
            slot.shutdown.store(true, Ordering::SeqCst);
            if let Ok(mut guard) = slot.child.lock() {
                if let Some(mut c) = guard.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
            }
        }
    }
}

/// Tear down a Claude chat session without UI side-effects (e.g. switching agent kind).
pub fn kill_quiet(character: &str) {
    kill_slot(character);
}

/// User closed the chat window or cleared session. Match **natural** PTY exit
/// (`pty` wait thread): `completed` (green bubble + chime) → 2.5s → `idle`.
/// Runs on a background thread so `chat_terminate` returns immediately.
pub fn terminate_user_closed<R: Runtime>(app: &AppHandle<R>, character: &str) {
    let had = SESSIONS
        .lock()
        .map(|g| g.contains_key(character))
        .unwrap_or(false);
    kill_slot(character);
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

/// Start a long-lived Claude pipe session for `character`.
pub fn start<R: Runtime>(app: AppHandle<R>, character: String) -> Result<(), String> {
    {
        let g = SESSIONS.lock().map_err(|e| e.to_string())?;
        if g.contains_key(&character) {
            emit(&app, &character, ChatEventBody::SessionReady);
            return Ok(());
        }
    }

    let bin = detect_agent_binary("claude").ok_or_else(|| {
        "Claude CLI not found. Install: npm install -g @anthropic-ai/claude-code".to_string()
    })?;

    if bin.chars().any(|c| matches!(c, '&' | '|' | '<' | '>' | '^' | '"' | '\n' | '\r')) {
        return Err("resolved claude path contains unsafe characters".to_string());
    }

    // Sandbox Claude's cwd to %APPDATA%\DockDuo\agents\<character>\ so
    // tool-use triggered by `--dangerously-skip-permissions` can only
    // touch the sandbox by default instead of the user's home directory.
    let workdir = super::agent_sandbox_dir(&character)?;
    let mut cmd = command_for_agent_binary(&bin);
    cmd.args([
        "-p",
        "--output-format",
        "stream-json",
        "--input-format",
        "stream-json",
        "--verbose",
        "--dangerously-skip-permissions",
    ])
    .current_dir(&workdir)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd.spawn().map_err(|e| format!("spawn claude: {e}"))?;
    let stdin = child.stdin.take().ok_or_else(|| "claude stdin missing".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "claude stdout missing".to_string())?;
    let stderr = child.stderr.take();

    let child_arc = Arc::new(Mutex::new(Some(child)));

    let activity = Arc::new(Mutex::new((Instant::now(), true)));
    let shutdown = Arc::new(AtomicBool::new(false));

    let (stdin_tx, stdin_rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut stdin = stdin;
        while let Ok(line) = stdin_rx.recv() {
            if stdin.write_all(line.as_bytes()).is_err() {
                break;
            }
            let _ = stdin.flush();
        }
    });

    {
        let mut g = SESSIONS.lock().map_err(|e| e.to_string())?;
        g.insert(
            character.clone(),
            ClaudeSlot {
                stdin_tx,
                child: child_arc.clone(),
                activity: activity.clone(),
                shutdown: shutdown.clone(),
            },
        );
    }

    emit_ai_status(&app, &character, "busy");
    emit(&app, &character, ChatEventBody::SessionReady);

    spawn_chat_idle_watcher(
        app.clone(),
        character.clone(),
        activity.clone(),
        shutdown.clone(),
    );

    let app_reader = app.clone();
    let char_reader = character.clone();
    let activity_reader = activity.clone();
    let shutdown_reader = shutdown.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if shutdown_reader.load(Ordering::SeqCst) {
                break;
            }
            let events = parse_claude_line(&line);
            for ev in events {
                if shutdown_reader.load(Ordering::SeqCst) {
                    break;
                }
                match &ev {
                    ChatEventBody::AssistantDelta { .. }
                    | ChatEventBody::ToolUse { .. }
                    | ChatEventBody::ToolResult { .. }
                    | ChatEventBody::Error { .. } => {
                        touch_busy(
                            &app_reader,
                            &char_reader,
                            &activity_reader,
                            &shutdown_reader,
                        );
                    }
                    ChatEventBody::AssistantDone | ChatEventBody::TurnComplete => {}
                    _ => {}
                }
                emit(&app_reader, &char_reader, ev);
            }
        }
        shutdown_reader.store(true, Ordering::SeqCst);
        emit(
            &app_reader,
            &char_reader,
            ChatEventBody::ProcessExit { code: None },
        );
        let slot = SESSIONS
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&char_reader);
        if slot.is_some() {
            emit_ai_status(&app_reader, &char_reader, "completed");
            std::thread::sleep(Duration::from_millis(2500));
            emit_ai_status(&app_reader, &char_reader, "idle");
        }
        if let Some(slot) = slot {
            if let Ok(mut guard) = slot.child.lock() {
                if let Some(mut c) = guard.take() {
                    let _ = c.wait();
                }
            }
        }
    });

    if let Some(mut err) = stderr {
        let app_err = app.clone();
        let ch = character.clone();
        let activity_err = activity.clone();
        let shutdown_err = shutdown.clone();
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = std::io::Read::read_to_string(&mut err, &mut s);
            if !s.trim().is_empty() && !shutdown_err.load(Ordering::SeqCst) {
                tracing::warn!(character = %ch, stderr = %s, "claude stderr");
                touch_busy(&app_err, &ch, &activity_err, &shutdown_err);
                emit(
                    &app_err,
                    &ch,
                    ChatEventBody::Error {
                        message: s.trim().to_string(),
                    },
                );
            }
        });
    }

    Ok(())
}

pub fn send_user(character: &str, text: &str) -> Result<(), String> {
    let g = SESSIONS.lock().map_err(|e| e.to_string())?;
    let slot = g
        .get(character)
        .ok_or_else(|| format!("no chat session for '{character}'"))?;

    let payload = serde_json::json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": text
        }
    });
    let line = serde_json::to_string(&payload).map_err(|e| e.to_string())? + "\n";
    slot
        .stdin_tx
        .send(line)
        .map_err(|_| "claude stdin channel closed".to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stream_event_text_delta() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hi"}}}"#;
        let evs = parse_claude_line(line);
        assert!(
            evs.iter().any(|e| matches!(e, ChatEventBody::AssistantDelta { text } if text == "Hi"))
        );
    }

    #[test]
    fn parses_nested_delta_text() {
        let line = r#"{"type":"stream_event","event":{"delta":{"text":"x"}}}"#;
        let evs = parse_claude_line(line);
        assert!(
            evs.iter()
                .any(|e| matches!(e, ChatEventBody::AssistantDelta { text } if text == "x"))
        );
    }

    #[test]
    fn parses_error_type() {
        let line = r#"{"type":"error","message":"bad"}"#;
        let evs = parse_claude_line(line);
        assert!(evs.iter().any(
            |e| matches!(e, ChatEventBody::Error { message } if message == "bad")
        ));
    }

    #[test]
    fn parses_message_stop_stream_event() {
        let line = r#"{"type":"stream_event","event":{"type":"message_stop"}}"#;
        let evs = parse_claude_line(line);
        assert!(evs.iter().any(|e| matches!(e, ChatEventBody::TurnComplete)));
    }
}
