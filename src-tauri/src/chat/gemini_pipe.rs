//! Gemini CLI — one subprocess per user turn (`-p` + `--output-format stream-json`).

use std::collections::HashMap;
use std::fmt::Write;
use std::io::Read;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
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

const CHAT_IDLE_AFTER: Duration = Duration::from_millis(500);

static SESSIONS: once_cell::sync::Lazy<Mutex<HashMap<String, GeminiSlot>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

struct GeminiSlot {
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
        tracing::warn!(error = %e, "gemini chat: emit failed");
    }
}

fn emit_ai_status<R: Runtime>(app: &AppHandle<R>, character: &str, status: &str) {
    let ev = crate::claude::AiStatusEvent {
        character: character.to_string(),
        status: status.to_string(),
    };
    if let Err(e) = app.emit("ai-status-changed", &ev) {
        tracing::warn!(error = %e, "gemini chat: ai-status emit failed");
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

/// Parse the final JSON blob from `gemini -p --output-format json`.
/// The documented schema is: `{ "response": "…", "stats": {…}, "error"?: {…} }`.
/// Falls back to raw text if the blob isn't valid JSON.
fn parse_gemini_json(blob: &str) -> Vec<ChatEventBody> {
    let trimmed = blob.trim();
    if trimmed.is_empty() {
        return vec![ChatEventBody::TurnComplete];
    }
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        if let Some(err_obj) = v.get("error") {
            let msg = err_obj
                .get("message")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| err_obj.to_string());
            return vec![
                ChatEventBody::Error { message: msg },
                ChatEventBody::TurnComplete,
            ];
        }
        if let Some(resp) = v.get("response").and_then(|x| x.as_str()) {
            return vec![
                ChatEventBody::AssistantDelta {
                    text: resp.to_string(),
                },
                ChatEventBody::AssistantDone,
                ChatEventBody::TurnComplete,
            ];
        }
    }
    // Not JSON we recognise — treat the whole stdout as the reply.
    vec![
        ChatEventBody::AssistantDelta {
            text: trimmed.to_string(),
        },
        ChatEventBody::AssistantDone,
        ChatEventBody::TurnComplete,
    ]
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
    let Some(bin) = detect_agent_binary("gemini") else {
        emit(
            app,
            character,
            ChatEventBody::Error {
                message: "Gemini CLI not found. Install Google's Gemini CLI and ensure `gemini` is on PATH."
                    .into(),
            },
        );
        emit(app, character, ChatEventBody::TurnComplete);
        return;
    };

    // Sandbox gemini's cwd to a per-character scratch dir — its built-in
    // filesystem tools would otherwise operate relative to %USERPROFILE%.
    let workdir = match super::agent_sandbox_dir(character) {
        Ok(p) => p,
        Err(e) => {
            emit(
                app,
                character,
                ChatEventBody::Error {
                    message: format!("gemini sandbox dir failed: {e}"),
                },
            );
            emit(app, character, ChatEventBody::TurnComplete);
            return;
        }
    };
    let mut cmd = command_for_agent_binary(&bin);
    // `--output-format=json` as a single arg so it can't be mistaken for a
    // value taking the next positional, and `--` ends option parsing so
    // the rest of argv is locked down even if we later append more.
    cmd.args(["-p", prompt, "--output-format=json", "--"])
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
                    message: format!("gemini spawn failed: {e}"),
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
                tracing::warn!(character = %ch, stderr = %s, "gemini stderr");
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

    let Some(mut stdout) = child.stdout.take() else {
        emit(
            app,
            character,
            ChatEventBody::Error {
                message: "gemini stdout missing".into(),
            },
        );
        emit(app, character, ChatEventBody::TurnComplete);
        return;
    };

    touch_busy(app, character, activity, shutdown);

    let mut blob = String::new();
    let _ = stdout.read_to_string(&mut blob);
    let _ = child.wait();

    if shutdown.load(Ordering::SeqCst) {
        return;
    }

    let mut reply = String::new();
    for ev in parse_gemini_json(&blob) {
        if let ChatEventBody::AssistantDelta { text } = &ev {
            reply.push_str(text);
        }
        touch_busy(app, character, activity, shutdown);
        emit(app, character, ev);
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
                "Gemini is still answering the previous message; wait for it to finish.".into(),
            );
        }
        // `gemini -p` is an agent with file-system tools baked in. Feeding
        // it our own labelled transcript makes it think the prior lines are
        // tasks and triggers tool-use ("I will search…"). For this chat UI
        // we send only the current user message and rely on the user to
        // include any context themselves. History is still recorded for
        // display purposes only.
        if let Ok(mut h) = slot.history.lock() {
            let _ = writeln!(&mut *h, "User: {text}");
        }
        let prompt = text.to_string();
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

    let _ = detect_agent_binary("gemini").ok_or_else(|| {
        "Gemini CLI not found. Install Google's Gemini CLI and ensure `gemini` is on PATH."
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
            GeminiSlot {
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
