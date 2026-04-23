import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { renderChatMarkdown } from "../lib/chatMarkdown";
import {
  applyTheme,
  TERMINAL_CHARACTER_ACCENT,
  type ThemeId,
} from "../lib/themes";
import type { AgentKind } from "../store/appStore";

/** If stderr + stdout both surface the same line, Claude may concatenate duplicates. */
function dedupeRepeatedErrorText(s: string): string {
  const t = s.trim();
  if (t.length < 2 || t.length % 2 !== 0) return t;
  const mid = t.length / 2;
  const a = t.slice(0, mid);
  if (a === t.slice(mid)) return a;
  return t;
}

/** Window label `bubble_bruce` / `bubble_jazz` → character id. */
function labelToCharacter(label: string): "bruce" | "jazz" | null {
  if (label === "bubble_bruce") return "bruce";
  if (label === "bubble_jazz") return "jazz";
  return null;
}

type ChatEvent =
  | { type: "user_echo"; text: string }
  | { type: "assistant_delta"; text: string }
  | { type: "assistant_done" }
  | { type: "tool_use"; name: string; summary: string }
  | { type: "tool_result"; summary: string; is_error: boolean }
  | { type: "error"; message: string }
  | { type: "session_ready" }
  | { type: "turn_complete" }
  | { type: "process_exit"; code: number | null };

interface ChatPayload {
  character: string;
  event: ChatEvent;
}

type Row =
  | { id: string; kind: "user"; text: string }
  | { id: string; kind: "assistant"; text: string; streaming: boolean }
  | { id: string; kind: "tool"; name: string; summary: string }
  | { id: string; kind: "tool_result"; summary: string; isError: boolean }
  | { id: string; kind: "error"; message: string }
  | { id: string; kind: "system"; text: string };

function newId(): string {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export default function AgentChat() {
  const [rows, setRows] = useState<Row[]>([]);
  const [input, setInput] = useState("");
  const [banner, setBanner] = useState<string | null>(null);
  const [thinking, setThinking] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const startedRef = useRef(false);
  const lastKindRef = useRef<AgentKind | null>(null);
  const disposedInnerRef = useRef(false);

  const label = useMemo(() => {
    try {
      return getCurrentWebviewWindow().label;
    } catch {
      return "";
    }
  }, []);

  const character = labelToCharacter(label) ?? "bruce";
  const accentStrip = TERMINAL_CHARACTER_ACCENT[character];
  const accentTitle = character === "bruce" ? "Bruce" : "Jazz";

  const closeBubble = useCallback(() => {
    void invoke("close_bubble", { character }).catch(() => {});
  }, [character]);

  const endSession = useCallback(async () => {
    try {
      await invoke("chat_terminate", { character });
    } catch {
      /* non-fatal */
    }
    startedRef.current = false;
    setRows([]);
    closeBubble();
  }, [character, closeBubble]);

  const scrollToBottom = useCallback(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [rows, scrollToBottom]);

  const focusInputSoon = useCallback(() => {
    // Windows needs a couple of animation frames after `show()` before the
    // input is actually focusable. Retry until focus sticks.
    let tries = 0;
    const tick = () => {
      tries += 1;
      const el = inputRef.current;
      if (el && document.activeElement !== el) {
        el.focus();
      }
      if (tries < 8 && document.activeElement !== inputRef.current) {
        setTimeout(tick, 50);
      }
    };
    tick();
  }, []);

  useEffect(() => {
    document.title = `DockDuo · ${accentTitle}`;
    focusInputSoon();
    const onWinFocus = () => focusInputSoon();
    window.addEventListener("focus", onWinFocus);
    return () => window.removeEventListener("focus", onWinFocus);
  }, [accentTitle, focusInputSoon]);

  useEffect(() => {
    invoke<{ theme: ThemeId }>("get_config")
      .then((cfg) => applyTheme(cfg.theme))
      .catch(() => {});
    const unThemePromise = listen<ThemeId>("theme-changed", (e) => {
      applyTheme(e.payload);
    });
    return () => {
      void unThemePromise.then((u) => u());
    };
  }, []);

  useEffect(() => {
    const win = getCurrentWebviewWindow();
    let unlistenClose: (() => void) | undefined;
    let unChat: (() => void) | undefined;
    // IMPORTANT: use a local `cancelled` per effect invocation. React
    // StrictMode double-invokes effects in dev; a shared ref can race with
    // the second mount and fail to unregister the first listener, which
    // doubles every event (visible as repeated assistant text).
    let cancelled = false;
    disposedInnerRef.current = false;

    const pushAssistantDelta = (delta: string) => {
      setThinking(false);
      setRows((prev) => {
        const last = prev[prev.length - 1];
        if (last?.kind === "assistant" && last.streaming) {
          return [
            ...prev.slice(0, -1),
            { ...last, text: last.text + delta },
          ];
        }
        return [
          ...prev,
          { id: newId(), kind: "assistant", text: delta, streaming: true },
        ];
      });
    };

    const finalizeAssistant = () => {
      setThinking(false);
      setRows((prev) => {
        const last = prev[prev.length - 1];
        if (last?.kind === "assistant" && last.streaming) {
          return [...prev.slice(0, -1), { ...last, streaming: false }];
        }
        return prev;
      });
    };

    const greetingFor = (kind: AgentKind): string => {
      switch (kind) {
        case "claude":
          return "Claude is ready. How can I help?";
        case "codex":
          return "Codex is ready. What would you like to build?";
        case "gemini":
          return "Gemini is ready. Ask me anything.";
        default:
          return "Agent is ready.";
      }
    };

    const start = async () => {
      const kindRaw = await invoke<AgentKind | null>("take_pending_bubble", {
        character,
      });
      if (!kindRaw) return;
      const switching =
        startedRef.current && lastKindRef.current !== kindRaw;
      if (
        startedRef.current &&
        lastKindRef.current === kindRaw
      ) {
        return;
      }
      lastKindRef.current = kindRaw;
      try {
        await invoke("chat_start_session", { character, kind: kindRaw });
        startedRef.current = true;
        setBanner(null);
        if (switching) setRows([]);
        setRows((p) => [
          ...p,
          { id: newId(), kind: "system", text: greetingFor(kindRaw) },
        ]);
      } catch (e) {
        setBanner(String(e));
        return;
      }
    };

    const bootstrap = async () => {
      const unChatReg = await listen<ChatPayload>("chat-agent-event", (ev) => {
        if (ev.payload.character !== character) return;
        const { event: body } = ev.payload;
        switch (body.type) {
          case "assistant_delta":
            pushAssistantDelta(body.text);
            break;
          case "assistant_done":
          case "turn_complete":
            finalizeAssistant();
            break;
          case "tool_use":
            setRows((p) => [
              ...p,
              {
                id: newId(),
                kind: "tool",
                name: body.name,
                summary: body.summary,
              },
            ]);
            break;
          case "tool_result":
            setRows((p) => [
              ...p,
              {
                id: newId(),
                kind: "tool_result",
                summary: body.summary,
                isError: body.is_error,
              },
            ]);
            break;
          case "error": {
            setThinking(false);
            const message = dedupeRepeatedErrorText(body.message);
            setRows((p) => {
              const last = p[p.length - 1];
              if (last?.kind === "error" && last.message === message) return p;
              return [...p, { id: newId(), kind: "error", message }];
            });
            break;
          }
          case "process_exit":
            setThinking(false);
            startedRef.current = false;
            setRows((p) => [
              ...p,
              {
                id: newId(),
                kind: "system",
                text: "[session ended]",
              },
            ]);
            break;
          case "session_ready":
          case "user_echo":
            break;
          default:
            break;
        }
      });

      if (cancelled) {
        unChatReg();
        return;
      }
      unChat = unChatReg;
      await start();
    };

    // Because the bubble window is re-used between opens (not recreated),
    // we listen for `bubble-opened` so a fresh agent kind picked from the
    // sprite picker actually takes effect.
    let unBubbleOpened: (() => void) | undefined;
    listen<string>("bubble-opened", (ev) => {
      if (ev.payload !== character) return;
      void start();
      focusInputSoon();
    }).then((un) => {
      if (cancelled) {
        un();
      } else {
        unBubbleOpened = un;
      }
    });

    void bootstrap().catch((e) => {
      setBanner(String(e));
    });

    // The bubble window is "closed" from the Rust side when the user
    // clicks the sprite again or clicks outside. Closing the window must
    // NOT end the chat session — only the explicit End session button
    // does that. So the close handler here is a no-op beyond default.
    void win
      .onCloseRequested(() => {
        // Allow close to proceed; session keeps running.
      })
      .then((un) => {
        unlistenClose = un;
      });

    return () => {
      cancelled = true;
      disposedInnerRef.current = true;
      unlistenClose?.();
      unChat?.();
      unBubbleOpened?.();
    };
  }, [character]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        closeBubble();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [closeBubble]);

  const onSubmit = async () => {
    const raw = input.trim();
    if (!raw) return;
    setInput("");

    if (raw === "/clear") {
      try {
        await invoke("chat_clear_session", { character });
        setRows([]);
        const kind = lastKindRef.current;
        if (kind) {
          await invoke("chat_start_session", { character, kind });
          startedRef.current = true;
          setBanner(null);
        }
      } catch (e) {
        setBanner(String(e));
      }
      return;
    }

    if (raw === "/copy") {
      const lastAssistant = [...rows].reverse().find((r) => r.kind === "assistant");
      if (lastAssistant && lastAssistant.kind === "assistant") {
        try {
          await navigator.clipboard.writeText(lastAssistant.text);
          setRows((p) => [
            ...p,
            { id: newId(), kind: "system", text: "[copied last reply]" },
          ]);
        } catch {
          setRows((p) => [
            ...p,
            { id: newId(), kind: "system", text: "[copy failed]" },
          ]);
        }
      } else {
        setRows((p) => [
          ...p,
          { id: newId(), kind: "system", text: "[nothing to copy]" },
        ]);
      }
      return;
    }

    if (raw === "/help") {
      setRows((p) => [
        ...p,
        {
          id: newId(),
          kind: "system",
          text: "Commands: /clear (new session), /copy (last assistant reply), /help",
        },
      ]);
      return;
    }

    if (!startedRef.current) {
      setRows((p) => [
        ...p,
        {
          id: newId(),
          kind: "system",
          text: "[session not ready — click the sprite to reopen]",
        },
      ]);
      return;
    }

    setRows((p) => [...p, { id: newId(), kind: "user", text: raw }]);
    setThinking(true);
    try {
      await invoke("chat_send", { character, text: raw });
    } catch (e) {
      setThinking(false);
      setRows((p) => [
        ...p,
        { id: newId(), kind: "error", message: String(e) },
      ]);
    }
  };

  const chromeFont =
    'system-ui, -apple-system, "Segoe UI", Roboto, sans-serif';

  // Claude-inspired warm cream palette (light mode).
  const C = {
    surface: "#FAF9F5",
    surfaceDeep: "#F4F1EA",
    border: "#E8E4DA",
    borderSoft: "#EFEBE2",
    text: "#1F1E1D",
    textMuted: "#6F6E69",
    accent: "#D97757",
    accentInk: "#FFFFFF",
    assistantBg: "#FFFFFF",
    errorBg: "#FCE8E4",
    errorBorder: "#E8A99C",
    errorText: "#8A2B1E",
  };

  return (
    <div
      onMouseDown={() => focusInputSoon()}
      style={{
        width: "100vw",
        height: "100vh",
        margin: 0,
        boxSizing: "border-box",
        overflow: "hidden",
        display: "flex",
        flexDirection: "column",
        background: C.surface,
        border: `1px solid ${C.border}`,
        borderRadius: 14,
        color: C.text,
      }}
    >
      <style>{`
        @keyframes dd-dot-bounce {
          0%, 80%, 100% { transform: translateY(0); opacity: 0.35; }
          40% { transform: translateY(-4px); opacity: 1; }
        }
        .dd-dot {
          width: 6px;
          height: 6px;
          border-radius: 999px;
          display: inline-block;
          animation: dd-dot-bounce 1.1s infinite ease-in-out both;
        }
      `}</style>
      <div
        style={{
          flex: 1,
          minHeight: 0,
          position: "relative",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        <header
          style={{
            flexShrink: 0,
            display: "flex",
            alignItems: "center",
            gap: 10,
            padding: "10px 14px",
            fontFamily: chromeFont,
            userSelect: "none",
            background: C.surface,
            borderBottom: `1px solid ${C.borderSoft}`,
          }}
        >
          <span
            style={{
              width: 3,
              height: 20,
              borderRadius: 2,
              background: accentStrip,
              flexShrink: 0,
            }}
            aria-hidden
          />
          <span
            style={{
              fontSize: 13,
              fontWeight: 600,
              letterSpacing: "-0.01em",
              color: C.text,
            }}
          >
            {accentTitle}
          </span>
          <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
            <button
              type="button"
              onClick={() => void endSession()}
              title="End this chat session"
              style={{
                padding: "4px 10px",
                borderRadius: 999,
                border: `1px solid ${C.errorBorder}`,
                background: C.errorBg,
                color: C.errorText,
                fontSize: 11,
                fontWeight: 600,
                fontFamily: chromeFont,
                cursor: "pointer",
              }}
            >
              End session
            </button>
            <button
              type="button"
              onClick={closeBubble}
              title="Close (Esc) — session keeps running"
              aria-label="Close bubble"
              style={{
                width: 24,
                height: 24,
                borderRadius: 999,
                border: `1px solid ${C.border}`,
                background: "transparent",
                color: C.textMuted,
                fontSize: 14,
                lineHeight: 1,
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              ×
            </button>
          </div>
        </header>

        {banner ? (
          <div
            style={{
              margin: "8px 12px 0",
              padding: "8px 10px",
              borderRadius: 8,
              fontFamily: chromeFont,
              fontSize: 12,
              color: C.errorText,
              background: C.errorBg,
              border: `1px solid ${C.errorBorder}`,
            }}
          >
            {banner}
          </div>
        ) : null}

        <div
          ref={scrollRef}
          style={{
            flex: 1,
            minHeight: 0,
            overflowY: "auto",
            padding: "12px 14px",
            fontFamily: chromeFont,
            fontSize: 13.5,
            lineHeight: 1.5,
            color: C.text,
            background: C.surfaceDeep,
          }}
        >
          {rows.map((r) => {
            if (r.kind === "user") {
              return (
                <div
                  key={r.id}
                  style={{
                    marginBottom: 10,
                    display: "flex",
                    justifyContent: "flex-end",
                  }}
                >
                  <div
                    style={{
                      maxWidth: "88%",
                      padding: "9px 12px",
                      borderRadius: 14,
                      borderBottomRightRadius: 4,
                      background: C.accent,
                      color: C.accentInk,
                      whiteSpace: "pre-wrap",
                      wordBreak: "break-word",
                      boxShadow: "0 1px 2px rgba(0,0,0,0.06)",
                    }}
                  >
                    {r.text}
                  </div>
                </div>
              );
            }
            if (r.kind === "assistant") {
              return (
                <div
                  key={r.id}
                  style={{
                    marginBottom: 10,
                    display: "flex",
                    justifyContent: "flex-start",
                  }}
                >
                  <div
                    style={{
                      maxWidth: "92%",
                      padding: "9px 12px",
                      borderRadius: 14,
                      borderBottomLeftRadius: 4,
                      background: C.assistantBg,
                      border: `1px solid ${C.border}`,
                      color: C.text,
                      boxShadow: "0 1px 2px rgba(0,0,0,0.04)",
                    }}
                  >
                    <div style={{ opacity: r.streaming ? 0.85 : 1 }}>
                      {renderChatMarkdown(r.text)}
                    </div>
                  </div>
                </div>
              );
            }
            if (r.kind === "tool") {
              return (
                <div
                  key={r.id}
                  style={{
                    marginBottom: 8,
                    fontSize: 12,
                    color: C.textMuted,
                  }}
                >
                  <strong style={{ color: C.accent }}>{r.name}</strong> —{" "}
                  <span style={{ opacity: 0.9 }}>{r.summary}</span>
                </div>
              );
            }
            if (r.kind === "tool_result") {
              return (
                <div
                  key={r.id}
                  style={{
                    marginBottom: 8,
                    fontSize: 12,
                    color: r.isError ? C.errorText : C.textMuted,
                  }}
                >
                  {r.isError ? "Error: " : "Result: "}
                  {r.summary}
                </div>
              );
            }
            if (r.kind === "error") {
              return (
                <div
                  key={r.id}
                  style={{
                    marginBottom: 10,
                    color: C.errorText,
                    fontSize: 12,
                  }}
                >
                  {r.message}
                </div>
              );
            }
            return (
              <div
                key={r.id}
                style={{
                  marginBottom: 8,
                  fontSize: 12,
                  color: C.textMuted,
                  fontStyle: "italic",
                }}
              >
                {r.text}
              </div>
            );
          })}

          {thinking ? (
            <div
              style={{
                marginBottom: 10,
                display: "flex",
                justifyContent: "flex-start",
              }}
            >
              <div
                style={{
                  padding: "10px 14px",
                  borderRadius: 14,
                  borderBottomLeftRadius: 4,
                  background: C.assistantBg,
                  border: `1px solid ${C.border}`,
                  boxShadow: "0 1px 2px rgba(0,0,0,0.04)",
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 4,
                }}
                aria-label={`${accentTitle} is thinking`}
              >
                <span className="dd-dot" style={{ background: C.textMuted }} />
                <span
                  className="dd-dot"
                  style={{
                    background: C.textMuted,
                    animationDelay: "0.15s",
                  }}
                />
                <span
                  className="dd-dot"
                  style={{
                    background: C.textMuted,
                    animationDelay: "0.3s",
                  }}
                />
              </div>
            </div>
          ) : null}
        </div>

        <div
          style={{
            flexShrink: 0,
            padding: "10px 12px",
            borderTop: `1px solid ${C.borderSoft}`,
            background: C.surface,
            display: "flex",
            gap: 8,
            alignItems: "center",
          }}
        >
          <input
            ref={inputRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                void onSubmit();
              }
            }}
            placeholder="Message… (/clear /copy /help)"
            style={{
              flex: 1,
              minWidth: 0,
              padding: "10px 12px",
              borderRadius: 10,
              border: `1px solid ${C.border}`,
              background: "#FFFFFF",
              color: C.text,
              fontFamily: chromeFont,
              fontSize: 13.5,
              outline: "none",
            }}
          />
          <button
            type="button"
            onClick={() => void onSubmit()}
            style={{
              padding: "10px 16px",
              borderRadius: 10,
              border: "none",
              cursor: "pointer",
              fontWeight: 600,
              fontSize: 13,
              fontFamily: chromeFont,
              background: C.accent,
              color: C.accentInk,
              boxShadow: "0 1px 2px rgba(0,0,0,0.08)",
            }}
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
}
