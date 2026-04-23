# lil-agents chat parity — Implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add optional **Chat (lil-style)** embedded UI alongside existing **Raw (xterm+PTY)** for `terminal_bruce` / `terminal_jazz`, with **Claude** pipe + NDJSON as Phase 1; Codex/Gemini chat drivers or explicit fallback per [spec](../specs/2026-04-22-lil-agents-chat-parity-design.md).

**Architecture:** Persist `embedded_ui_mode` (`raw` | `chat`) in [`config.rs`](../../../src-tauri/src/config.rs). New Rust module `chat/` owns pipe sessions and emits `chat-agent-event` payloads. React [`main.tsx`](../../../src/main.tsx) routes `terminal_*` to [`AgentChat.tsx`](../../../src/windows/AgentChat.tsx) or existing [`Terminal.tsx`](../../../src/windows/Terminal.tsx). Tray extends terminal submenu to toggle mode.

**Tech stack:** Tauri 2, Rust 2021, React 19, serde_json, existing `AgentKind` from [`claude.rs`](../../../src-tauri/src/claude.rs).

**Authoritative spec:** [`docs/superpowers/specs/2026-04-22-lil-agents-chat-parity-design.md`](../specs/2026-04-22-lil-agents-chat-parity-design.md)

---

## File map (create / modify)

| File | Role |
|------|------|
| Create `src-tauri/src/chat/mod.rs` | Module root, re-exports |
| Create `src-tauri/src/chat/protocol.rs` | `ChatAgentEvent` enum + serde |
| Create `src-tauri/src/chat/claude_pipe.rs` | Claude stdin/stdout session |
| Create `src-tauri/src/chat/session.rs` | Trait + dispatcher by `AgentKind` |
| Create `src-tauri/src/chat/fixtures/*.txt` | Golden stdout for parser tests |
| Modify `src-tauri/src/lib.rs` | `mod chat;`, `generate_handler![...]` |
| Modify `src-tauri/src/config.rs` | `EmbeddedUiMode`, `embedded_ui_mode`, migration v4, setters |
| Modify `src-tauri/src/tray.rs` | Menu for Raw vs Chat when embedded on |
| Modify `src/main.tsx` | Branch `terminal_*` on `embedded_ui_mode` |
| Create `src/windows/AgentChat.tsx` | Transcript + input + slash commands + event listeners |
| Create `src/lib/chatMarkdown.ts` | Markdown subset renderer (pure functions for tests) |
| Create `src/lib/chatMarkdown.test.ts` | Vitest golden tests (add vitest to package.json if absent) |
| Modify `docs/DECISIONS.md` | D-012 |
| Modify `README.md` | Chat vs Raw user docs |

---

### Task 1: Config schema and persistence

**Files:**
- Modify: [`src-tauri/src/config.rs`](../../../src-tauri/src/config.rs)
- Modify: [`src-tauri/src/lib.rs`](../../../src-tauri/src/lib.rs) (only if new command `set_embedded_ui_mode` registered here — can be same PR as Task 1)

- [ ] **Step 1: Add `EmbeddedUiMode` enum**

In `config.rs`, after `Theme`:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddedUiMode {
    #[default]
    Raw,
    Chat,
}
```

- [ ] **Step 2: Extend `AppConfig`**

Add:

```rust
#[serde(default)]
pub embedded_ui_mode: EmbeddedUiMode,
```

Bump `CONFIG_VERSION` to `4`. In `migrate`, if `version < 4`, set `embedded_ui_mode = EmbeddedUiMode::Raw`.

- [ ] **Step 3: Add command `set_embedded_ui_mode`**

Mirror `set_use_embedded_terminal`; update cache + save.

- [ ] **Step 4: Expose in `get_config` JSON**

Frontend already calls `get_config`; TypeScript type in consumers must include `embedded_ui_mode: 'raw' | 'chat'`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "feat(config): add embedded_ui_mode raw|chat for lil-style chat UX"
```

---

### Task 2: Rust protocol types

**Files:**
- Create: `src-tauri/src/chat/protocol.rs`
- Create: `src-tauri/src/chat/mod.rs`

- [ ] **Step 1: Define `ChatAgentEvent`**

Use `#[serde(tag = "type", rename_all = "snake_case")]` with variants: `UserEcho`, `AssistantDelta`, `AssistantDone`, `ToolUse`, `ToolResult`, `Error`, `SessionReady`, `TurnComplete`, `ProcessExit { code: Option<i32> }`. Include `character: String` on each variant or wrap in a struct `ChatEnvelope { character, event }` — **pick one** and use consistently (spec suggests envelope; implement envelope to avoid repeating `character` in every variant).

- [ ] **Step 2: Export module in `chat/mod.rs`**

```rust
pub mod protocol;
pub mod claude_pipe;
pub mod session;
```

- [ ] **Step 3: Wire `mod chat;` in `lib.rs`**

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/chat/mod.rs src-tauri/src/chat/protocol.rs src-tauri/src/lib.rs
git commit -m "feat(chat): add protocol types for chat-agent events"
```

---

### Task 3: Claude pipe session (Phase 1 core)

**Files:**
- Create: `src-tauri/src/chat/claude_pipe.rs`
- Create: `src-tauri/src/chat/session.rs`
- Modify: [`src-tauri/src/claude.rs`](../../../src-tauri/src/claude.rs) — reuse `detect_binary` (pub(crate) already) for `claude` path

- [ ] **Step 1: Implement `ClaudePipeSession::start(app, character)`**

Spawn `Command` with executable from `detect_binary("claude")`, args aligned with spec §4 (verify on Windows against `claude --help`). Pipes for stdin/stdout/stderr. Stderr thread: `tracing::warn!` or emit `Error` event.

- [ ] **Step 2: Stdout reader thread**

Buffer lines; parse NDJSON; map to `ChatAgentEvent` (port semantics from upstream `ClaudeSession.processOutput` — assistant text deltas, tool calls, etc.). Emit via `app.emit("chat-agent-event", envelope)`.

- [ ] **Step 3: `send_user(character, text)`**

Serialize user message per upstream JSON shape (`type: user`, `message: { role, content }`) + newline; write to stdin.

- [ ] **Step 4: `Session` trait in `session.rs`**

`start`, `send`, `clear`, `terminate` keyed by `HashMap<String, Mutex<Box<dyn ChatDriver>>>` or enum per kind; **Phase 1** only Claude needs full implementation.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/chat/
git commit -m "feat(chat): Claude pipe session and NDJSON parser"
```

---

### Task 4: Parser unit tests (TDD)

**Files:**
- Create: `src-tauri/src/chat/claude_pipe.rs` — `#[cfg(test)] mod tests`
- Create: `src-tauri/src/chat/fixtures/claude_stream_sample.txt` — multi-line NDJSON snippet copied from a real `claude -p` run (sanitize secrets)

- [ ] **Step 1: Write failing test `parses_assistant_delta`**

```rust
#[test]
fn parses_assistant_delta() {
    let line = r#"{"type":"stream_event",...}"#; // replace with real shape
    let ev = parse_claude_line(line).expect("parse");
    assert!(matches!(ev, ParsedClaude::AssistantText(_)));
}
```

Run: `cd src-tauri && cargo test chat:: -- --nocapture`  
Expected: FAIL until parser exists.

- [ ] **Step 2: Implement minimal `parse_claude_line` / buffer until tests pass**

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/chat/
git commit -m "test(chat): golden NDJSON fixtures for Claude parser"
```

---

### Task 5: Tauri commands and event registration

**Files:**
- Modify: [`src-tauri/src/lib.rs`](../../../src-tauri/src/lib.rs)
- Modify: `src-tauri/src/chat/session.rs` (command handlers)

- [ ] **Step 1: Add commands**

`chat_start_session(character: String, kind: AgentKind)`, `chat_send(character: String, text: String)`, `chat_clear_session(character: String)`, `chat_terminate(character: String)`.

- [ ] **Step 2: Register in `generate_handler!`**

- [ ] **Step 3: Manual smoke test**

`pnpm tauri dev` — from DevTools in terminal window, `invoke('chat_start_session', { character: 'bruce', kind: 'claude' })` (use TS enum shape matching serde).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/chat/
git commit -m "feat(chat): expose chat_* Tauri commands"
```

---

### Task 6: Tray menu — Embedded Raw vs Chat

**Files:**
- Modify: [`src-tauri/src/tray.rs`](../../../src-tauri/src/tray.rs)

- [ ] **Step 1: When `use_embedded_terminal` is true, show submenu or paired check items**

“Embedded: Raw terminal” vs “Embedded: Chat (lil-style)” — mutually exclusive; calling `set_embedded_ui_mode` + update checks (same pattern as `term_embedded` / `term_system`).

- [ ] **Step 2: On toggle, emit `embedded-ui-mode-changed`**

Payload: `"raw" | "chat"` so open terminal windows can react without restart.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/tray.rs
git commit -m "feat(tray): toggle embedded UI raw vs chat mode"
```

---

### Task 7: Frontend routing and `AgentChat`

**Files:**
- Modify: [`src/main.tsx`](../../../src/main.tsx)
- Create: [`src/windows/AgentChat.tsx`](../../../src/windows/AgentChat.tsx)
- Modify: [`package.json`](../../../package.json) — add `vitest` only if adding `chatMarkdown.test.ts`

- [ ] **Step 1: In `main.tsx`, for `label.startsWith("terminal_")`**

Call `get_config()`, if `use_embedded_terminal && embedded_ui_mode === 'chat'` render `<AgentChat />`, else `<Terminal />`.

Subscribe to `embedded-ui-mode-changed` to `window.location.reload()` or swap state via small wrapper component with `useState` (reload is acceptable for v1).

- [ ] **Step 2: Implement `AgentChat.tsx`**

- `useEffect`: `listen('chat-agent-event', ...)` append to transcript state.
- Header with character name; optional Copy last response button.
- Input: on Enter, `invoke('chat_send', ...)`; implement `/clear`, `/copy`, `/help` locally per spec §3.
- On mount: `invoke('chat_start_session', { character, kind })` after resolving `kind` from `take_pending_embedded` + `set_pending_embedded` flow used by [`Overlay.tsx`](../../../src/windows/Overlay.tsx) (reuse same pending pattern as PTY).

- [ ] **Step 3: Wire `ai-status-changed`**

Map streaming/tool events to `appStore.setAiStatus` like [`Terminal.tsx`](../../../src/windows/Terminal.tsx).

- [ ] **Step 4: Commit**

```bash
git add src/main.tsx src/windows/AgentChat.tsx
git commit -m "feat(ui): AgentChat window for lil-style embedded mode"
```

---

### Task 8: Markdown subset (`chatMarkdown.ts`)

**Files:**
- Create: [`src/lib/chatMarkdown.ts`](../../../src/lib/chatMarkdown.ts)

- [ ] **Step 1: Export `renderChatMarkdown(plain: string): ReactNode` or HTML string**

Port structure from `TerminalView.renderMarkdown` (headings, lists, fences, inline backticks). Keep dependency-free if possible.

- [ ] **Step 2: If using Vitest, add test**

```ts
import { describe, it, expect } from "vitest";
import { renderChatMarkdownToHtml } from "./chatMarkdown";

describe("chatMarkdown", () => {
  it("renders heading", () => {
    expect(renderChatMarkdownToHtml("# Hi")).toContain("Hi");
  });
});
```

Run: `pnpm exec vitest run src/lib/chatMarkdown.test.ts`

- [ ] **Step 3: Commit**

```bash
git add src/lib/chatMarkdown.ts src/lib/chatMarkdown.test.ts package.json
git commit -m "feat(ui): markdown subset for AgentChat transcript"
```

---

### Task 9: Codex / Gemini fallback

**Files:**
- Modify: `src/windows/AgentChat.tsx`
- Modify: `src-tauri/src/chat/session.rs`

- [ ] **Step 1: If `kind != Claude` and chat driver not implemented**

`invoke('chat_start_session')` returns error string; UI shows banner: “Chat mode supports Claude only. Switch to Raw terminal in the tray or pick Claude.”

- [ ] **Step 2: Commit**

```bash
git add src/windows/AgentChat.tsx src-tauri/src/chat/
git commit -m "fix(chat): explicit non-Claude fallback in chat mode"
```

---

### Task 10: Docs — D-012 and README

**Files:**
- Modify: [`docs/DECISIONS.md`](../../DECISIONS.md)
- Modify: [`README.md`](../../../README.md)

- [ ] **Step 1: Add `## D-012 · Embedded UI: Raw (xterm+PTY) vs Chat (lil-style pipes)`**

Document dual mode, Claude-first scope, reference spec path.

- [ ] **Step 2: README “Embedded terminal” section**

Document Chat vs Raw; link to lil-agents for UX inspiration; security note on Claude flags if matching upstream.

- [ ] **Step 3: Commit**

```bash
git add docs/DECISIONS.md README.md
git commit -m "docs: D-012 and README for chat vs raw embedded UI"
```

---

## Verification checklist (end of implementation)

- [ ] `cargo test` passes in `src-tauri`
- [ ] `pnpm build` passes
- [ ] Manual: tray → Embedded Chat → click Bruce → transcript + Claude stream works
- [ ] Manual: tray → Embedded Raw → xterm still works
- [ ] Manual: system terminal (detached) unchanged

---

## Note on user approval

The [spec §14](../specs/2026-04-22-lil-agents-chat-parity-design.md) requires human sign-off before **merging** risky PRs. This plan assumes engineering can work on a feature branch until approval.
