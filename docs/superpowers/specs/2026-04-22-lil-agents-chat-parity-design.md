# lil-agents–style chat UX — Design specification

## 1. Purpose

DockDuo today offers **embedded PTY + xterm.js** ([`Terminal.tsx`](../../../src/windows/Terminal.tsx), [`pty.rs`](../../../src-tauri/src/pty.rs)), which is a **real terminal emulator**. The macOS upstream **lil-agents** product presents a **chat transcript UI** (popover): read-only transcript with lightweight markdown, a single-line input, slash commands, and **per-provider Process+Pipe** integrations that speak **structured JSON** to the CLIs—not a PTY.

This spec defines how DockDuo gains **optional UX parity** with lil-agents **without removing** the existing PTY path, so power users keep raw shells and TUIs.

**Reference upstream (MIT):** [ryanstephen/lil-agents](https://github.com/ryanstephen/lil-agents) — especially `TerminalView.swift`, `PopoverTheme.swift`, `AgentSession.swift`, `ClaudeSession.swift`, `CodexSession.swift`.

**Related local spec:** [2026-04-19-phase-3-pty-terminal-design.md](./2026-04-19-phase-3-pty-terminal-design.md) (PTY/xterm; remains in force for Raw mode).

---

## 2. Goals

- **G1 — Dual mode:** User-selectable **Chat (lil-style)** vs **Raw (PTY + xterm)** for embedded agent windows (`terminal_bruce` / `terminal_jazz`).
- **G2 — Chat transcript UX:** Scrollable transcript, user messages prefixed visually (e.g. `>` like upstream), streaming assistant text, tool-use / tool-result lines, errors in a distinct color.
- **G3 — Slash commands (parity):** `/clear`, `/copy`, `/help` in the chat input; behavior aligned with `TerminalView.swift` (clear transcript + optional backend session reset where applicable; copy last assistant response; help text).
- **G4 — Markdown subset:** Headings `#`–`###`, bullet lines `-`/`*`, fenced code blocks ` ``` `, inline `` `code` ``, basic streaming-safe append (see §7).
- **G5 — Phased providers:** **Claude first** (stream-json pipe protocol). Codex and Gemini follow with ported parsers or **temporary fallback** to Raw PTY for that agent until the chat driver ships.
- **G6 — Config persistence:** New setting stored in [`config.rs`](../../../src-tauri/src/config.rs) (schema bump), exposed in tray alongside existing terminal choice.

## 3. Non-goals

- **NG1:** Pixel-perfect AppKit replication (no `NSTextView`; React + DOM or canvas-based transcript only).
- **NG2:** macOS-only features (SF Mono can be suggested via CSS font stack; Windows uses `Cascadia Code`, etc.).
- **NG3:** Copilot or OpenClaw in v1 of chat mode (DockDuo already omits Copilot per [D-004](../../DECISIONS.md); revisit later).
- **NG4:** Replacing detached `cmd.exe` spawn path — unchanged; chat mode applies only when **embedded** window is used.
- **NG5:** Sending transcript content to any DockDuo server — still local-only; same privacy posture as today.

---

## 4. CLI matrix (Chat mode drivers)

Values below mirror **intent** from upstream Swift. Exact flags must be re-verified against installed CLI `--help` on Windows before release; document minimum supported versions in README.

| Provider | Binary | Transport | Upstream pattern | Notes |
|----------|--------|-----------|------------------|-------|
| **Claude** | `claude` | Stdin/stdout **pipes** | `claude` + `-p` + `--output-format stream-json` + `--input-format stream-json` + `--verbose` + `--dangerously-skip-permissions` (see `ClaudeSession.swift`) | Stdin: NDJSON lines with `type: user` and message payload. Stdout: NDJSON stream; Rust parses assistant deltas, tool events, errors. **Phase 1 implementation target.** |
| **Codex** | `codex` | Pipes | Per message: `codex exec --json --full-auto --skip-git-repo-check <prompt>`; multi-turn = stitched conversation string in prompt (`CodexSession.swift`) | **Phase 2:** Port JSONL parser; until then, **fallback:** open Raw PTY or show “use Raw terminal for Codex chat” banner. |
| **Gemini** | `gemini` | TBD | Inspect `GeminiSession.swift` when implementing Phase 3 | **Phase 3:** If no stable JSON API, fallback to PTY for Gemini in Chat mode until spec updated. |

**Detached console mode:** Unchanged — still `claude.rs::spawn_agent` with `CREATE_NEW_CONSOLE`.

---

## 5. Configuration

### 5.1 New fields (`AppConfig`, bump `CONFIG_VERSION` to **4**)

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `embedded_ui_mode` | enum: `raw` \| `chat` | `raw` | When `use_embedded_terminal` is true, which UI hosts the session. `raw` = current xterm+PTY. `chat` = lil-style transcript + pipe drivers. |

**Serde:** Use `#[serde(rename_all = "lowercase")]` on a new `EmbeddedUiMode` enum in [`config.rs`](../../../src-tauri/src/config.rs).

**Migration:** On `CONFIG_VERSION` bump to 4, existing configs without the field deserialize to `raw` via `#[serde(default)]` so current xterm+PTY behavior is preserved.

**Tray:** Extend Terminal submenu: **Raw terminal (xterm)** / **Chat (lil-style)** mutually exclusive when embedded is on; or single submenu with three states: System / Embedded Raw / Embedded Chat (exact copy TBD in implementation plan; must remain clear).

**Naming in UI:** User-facing strings like “Chat (lil-style)” and “Raw terminal” — avoid trademark issues; “inspired by lil-agents” acceptable in docs.

### 5.2 Interaction with `use_embedded_terminal`

| `use_embedded_terminal` | `embedded_ui_mode` | Result |
|-------------------------|--------------------|--------|
| `false` | ignored | Detached `cmd.exe` (today). |
| `true` | `raw` | Current `Terminal.tsx` + `pty.rs`. |
| `true` | `chat` | New `AgentChat.tsx` (name TBD) + new Rust `chat_session` module; no xterm for that window. |

---

## 6. Rust architecture

### 6.1 Module layout (new / touched)

- **New:** `src-tauri/src/chat/` (or split files):
  - `mod.rs` — public API, re-exports
  - `protocol.rs` — shared event types (serde) emitted to frontend
  - `claude_pipe.rs` — long-lived `Command` + stdout reader thread + stdin writer; line-buffered NDJSON parse
  - `codex_pipe.rs` — (Phase 2) one-shot exec per message
  - `gemini_pipe.rs` — (Phase 3) stub / defer
  - `session.rs` — trait `ChatSession`: `start`, `send_user`, `clear`, `terminate`; keyed by `character` (`bruce` \| `jazz`) + `AgentKind`
- **Touch:** [`lib.rs`](../../../src-tauri/src/lib.rs) — register commands + ensure events allowed in capabilities
- **Touch:** [`capabilities/default.json`](../../../src-tauri/capabilities/default.json) if new permissions needed (likely none beyond existing window events)
- **Touch:** [`tray.rs`](../../../src-tauri/src/tray.rs) — persist `embedded_ui_mode`, menu labels

### 6.2 Process management (Windows)

- Use `std::process::Command` with `stdin(Stdio::piped())`, `stdout(Stdio::piped())`, `stderr(Stdio::piped())`.
- Non-blocking or threaded reads: dedicated **blocking read loop** per session on a stdio thread; use channels to a small dispatcher that calls `app.emit` on the main/async boundary Tauri expects (match patterns from [`pty.rs`](../../../src-tauri/src/pty.rs) for thread safety).
- Working directory: user profile or `%USERPROFILE%` (align with `claude.rs` / upstream `FileManager.default.homeDirectoryForCurrentUser`).
- Environment: inherit with optional PATH augmentation (reuse `ShellEnvironment` equivalent: copy from `claude.rs` `detect_binary` paths).

### 6.3 Events to frontend (canonical names)

All payloads include `character: string` (`bruce` | `jazz`). Suggested event name: **`chat-agent-event`** with tagged union JSON:

```json
{ "type": "user_echo", "text": "..." }
{ "type": "assistant_delta", "text": "..." }
{ "type": "assistant_done" }
{ "type": "tool_use", "name": "...", "summary": "..." }
{ "type": "tool_result", "summary": "...", "is_error": false }
{ "type": "error", "message": "..." }
{ "type": "session_ready" }
{ "type": "turn_complete" }
{ "type": "process_exit", "code": 0 }
```

Exact Claude NDJSON → event mapping is implemented by porting logic from `ClaudeSession.processOutput` / line parsing (Rust tests should use golden stdout fixtures).

### 6.4 Commands (invoke)

| Command | Args | Behavior |
|---------|------|----------|
| `chat_start_session` | `character`, `kind: AgentKind` | Spawn pipe session if not running; emit `session_ready` or `error`. |
| `chat_send` | `character`, `text: string` | Write user JSON line to Claude stdin (or run Codex exec in Phase 2). |
| `chat_clear_session` | `character` | Kill process, clear buffers, optionally restart fresh on next send. |
| `chat_terminate` | `character` | Clean shutdown (for window close). |

Reuse `AgentKind` from [`claude.rs`](../../../src-tauri/src/claude.rs) or move to `chat::protocol` if circular deps appear.

### 6.5 AI status / bubbles

- **Chat mode:** Map `assistant_delta` / `tool_*` / silence to `ai-status-changed` similarly to PTY path: `busy` while model streaming or tool running; `idle` after `turn_complete` or timeout; `completed` pulse optional (match [`pty.rs`](../../../src-tauri/src/pty.rs) semantics for overlay consistency).

---

## 7. Frontend architecture

### 7.1 Routing

- [`main.tsx`](../../../src/main.tsx): for `terminal_*` labels, branch on config:
  - If `embedded_ui_mode === 'chat'` → render **`AgentChat`** (new component).
  - Else → existing **`Terminal`** (xterm).

Config can be read once on mount via `get_config`; subscribe to a new event `embedded-ui-mode-changed` from tray if we add one, or poll on window focus (prefer event for consistency with `theme-changed`).

### 7.2 `AgentChat` component (responsibilities)

- Layout: header (provider title, optional “Copy last response”, link to Raw mode help), scrollable transcript `div`, bottom `input` (single line, styled like upstream input bar).
- Listen to `chat-agent-event`; append to transcript model (React state or lightweight store).
- Slash commands handled **client-side** for `/copy` and `/help`; `/clear` calls `chat_clear_session` + clears local transcript.
- Markdown rendering: **subset** implementation — either small custom parser (mirror `TerminalView.renderMarkdown` structure) or dependency (prefer **no new dependency** in v1 if subset stays &lt;200 LOC; else evaluate `marked` / `markdown-it` with sanitization).

### 7.3 Streaming and markdown

- **Risk:** Appending partial markdown during stream can flicker; v1 may append **plain text** during stream and run markdown pass on `assistant_done`, or use upstream approach (incremental append to attributed string equivalent). Spec decision: **v1 = accumulate assistant text in a buffer; render markdown on `assistant_done` or paragraph boundary** to limit layout thrash (document deviation from Mac if Mac renders mid-stream).

### 7.4 Themes

- Reuse [`themes.ts`](../../../src/lib/themes.ts) CSS variables for chrome (popover bg, borders, accent).
- **Optional phase:** Add presets **Peach / Cloud / Moss** as aliases mapping to new CSS variable bundles copied from `PopoverTheme` RGB (§9). Not required for MVP if current four themes suffice.

---

## 8. Theme mapping (PopoverTheme → DockDuo CSS)

MVP: map **Midnight** (`teenageEngineering`) to existing `midnight` theme vars where obvious (dark bg, orange accent). Full table for implementer:

| PopoverTheme field | Suggested DockDuo CSS variable (existing or new) |
|--------------------|---------------------------------------------------|
| `popoverBg` | `--terminal-chrome-bg`, `--terminal-surface-bg` |
| `popoverBorder` | `--terminal-surface-border` |
| `popoverCornerRadius` | inline `borderRadius: 12` in chat shell |
| `titleBarBg` / `titleText` | header bar in `AgentChat` |
| `textPrimary` / `textDim` | `--fg`, muted for transcript meta |
| `accentColor` | `--accent` |
| `errorColor` / `successColor` | `--error`, `--success` (add if missing) |
| `inputBg` | input field background |
| `bubbleBg` / `bubbleBorder` | align with [`Character.tsx`](../../../src/components/Character.tsx) bubble styles when harmonizing |

---

## 9. Error handling

- **CLI not found:** Emit `error` event with install hint (reuse strings from onboarding / `claude.rs`).
- **Parse error:** Emit `error` with truncated line; do not crash reader thread; log via `tracing::warn!`.
- **Process exit:** Emit `process_exit`; UI offers “Restart session” that calls `chat_start_session`.
- **Unsupported provider in chat mode:** If user picks Codex before Phase 2, show modal: “Codex chat mode coming soon. Switch to Raw terminal or Claude.” — or auto-fallback to PTY with toast (pick one in implementation; **spec prefers explicit message** over silent fallback).

---

## 10. Testing strategy

- **Rust unit tests:** NDJSON fixture files under `src-tauri/src/chat/fixtures/`; test parser produces expected event sequence.
- **Integration:** Manual: Windows 10/11, Claude Code installed, toggle Chat mode, send message, verify streaming and `/clear`.
- **Frontend:** Optional Vitest for markdown subset golden strings (if parser extracted to pure TS module).

---

## 11. Documentation & decisions

- Add **D-012** entry to [`docs/DECISIONS.md`](../../DECISIONS.md) after implementation: Chat vs Raw dual mode rationale, Claude pipe flags, defer Codex/Gemini chat parsers.
- Update [`README.md`](../../../README.md) embedded terminal section to describe Chat vs Raw.

---

## 12. Open risks (accepted)

- **CLI flag drift:** Anthropic/OpenAI/Google may change flags; mitigate with version checks and README “minimum CLI version”.
- **Permission flags:** Upstream uses `--dangerously-skip-permissions` for Claude; DockDuo must document security trade-off and consider making it a **config toggle** defaulting to match upstream for parity (product/legal review outside this spec).

---

## 13. Self-review checklist (pre–implementation gate)

| Check | Status |
|-------|--------|
| No contradictory requirement vs PTY spec | Yes — dual mode preserves PTY. |
| Config defaults preserve current behavior | Yes — `embedded_ui_mode: raw` default. |
| Phased rollout explicit | Yes — Claude Phase 1; Codex/Gemini later or fallback. |
| TBD / TODO placeholders | None intentional; Gemini marked TBD by design in §4. |
| Scope fits one implementation plan | Yes — plan file separate. |

**Self-review log:** 2026-04-22 — migration clarified (`serde(default)` → `raw`); streaming markdown strategy explicit (§7.3); no internal contradictions with PTY Phase 3 spec; all relative file links use `../../../` from this spec’s directory to repo root assets.

**User review gate:** Approve this spec (or request edits) before merging implementation PRs.

---

## 14. Approval

- [ ] Author / maintainer sign-off on spec text  
- [ ] User / product owner sign-off  

After approval, proceed to [`docs/superpowers/plans/2026-04-22-lil-agents-chat-parity.md`](../plans/2026-04-22-lil-agents-chat-parity.md) implementation plan.
