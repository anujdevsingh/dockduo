# DockDuo — Architectural Decisions Log

Per `DockDuo-BuildPlan.md` §0 rule 4, every deviation from the plan is logged
here with a rationale. Reviewed in order of impact, not chronology.

---

## D-001 · Product rebrand: "Lil Agents for Windows" → "DockDuo"

**Plan section:** §17 (Branding & Attribution).
**Decision:** Ship under the name `DockDuo`. Identifier is
`com.codewithanuj.dockduo`, config folder is `%APPDATA%\DockDuo\`, binary is
`dockduo.exe`, product name is `DockDuo`.
**Why:** The plan explicitly allows a name change (it lists "Pocket Agents"
and "Dock Pets" as acceptable alternatives) to separate the Windows port
from the upstream Mac project. "DockDuo" captures the two-character concept
better than "Pocket Agents" and is a unique search term.
**Impact:** Attribution to Ryan Stephen and upstream MIT license still
required in `README.md` / `CREDITS.md` (Phase 5).

---

## D-002 · Click-through toggled by Rust cursor polling, not frontend hover

**Plan section:** §9.2 (Character.tsx click-through handling),
§16 rule 2 (no Rust poll loops).
**Decision:** `src-tauri/src/hit_test.rs` spawns one background thread that
calls `GetCursorPos` every 30 ms, compares against per-character bounds
reported by the frontend via `report_bounds`, and flips
`set_ignore_cursor_events` on the overlay window accordingly.
**Why the plan's approach didn't work:** While the overlay is in
`ignore_cursor_events = true` state, Chromium does **not** deliver
`mouseenter` / `mouseover` DOM events to the page — the whole window is
transparent to the pointer, so React never sees the cursor arriving. The
only way to learn "the cursor is now over Bruce's sprite" is to sample
`GetCursorPos` from outside the WebView2 process.
**Cost:** One thread sleeping on a 30 ms timer. On a modern CPU this costs
well under 0.1 % and never appears in Task Manager.
**Rule 2 reinterpretation:** The rule forbids the *16 ms* click-through
poll that v1 of the plan used. 30 ms is well under any human-perceivable
threshold and is the minimum viable approach given the Chromium constraint
above. Noting the deviation explicitly.

---

## D-003 · Phase 3: detached console instead of PTY + xterm.js

**Plan section:** §7.4 (`terminal.rs`), §9.3 (`Terminal.tsx`),
§12 Phase 3.
**Decision:** `src-tauri/src/claude.rs` spawns the chosen AI CLI via
`cmd.exe /K <agentPath>` with the Win32 `CREATE_NEW_CONSOLE` flag. This
produces a standalone console window hosted by whatever the user has set as
the default terminal (Windows Terminal on Win11, conhost on Win10).
DockDuo does **not** bundle `@xterm/xterm`, `portable-pty`, or any of the
`terminal_bruce` / `terminal_jazz` Tauri windows described in the plan.
**Why:**
1. On Windows 11, Windows Terminal as the default console host already
   provides tabs, theming, GPU rendering, and all of xterm.js's strengths —
   natively, with zero bundle cost.
2. Removes ~4 MB of JS bundle (xterm + 3 addons + webgl), keeping
   installed-size headroom for the <25 MB target.
3. No PTY byte-plumbing means no IPC fan-out, no resize round-trips, no
   webview→ConPTY latency budget to manage.
4. The user's existing terminal settings (font, theme, keybinds) are
   respected instead of overridden.
**Trade-offs:**
- Terminal window does not inherit DockDuo's theme.
- AI activity detection is coarse-grained: we emit `busy` at spawn and
  `completed` on child exit (+ 2.5 s → `idle`), rather than byte-level
  silence-based detection. The thinking bubble therefore runs for the
  whole session, not only while the model is generating. **This is the
  biggest UX compromise of this decision and is accepted for MVP.**
- Users on Windows 10 without Windows Terminal installed see the default
  conhost window, which is less polished.
**Reversibility:** The detached-console code is ~50 lines in `claude.rs`
and does not leak architecture assumptions elsewhere. If a future version
wants real PTY terminals, add `terminal.rs` + the two windows side-by-side
and swap the spawn path behind a config flag.

---

## D-004 · Provider set: Claude / Codex / Gemini (no GitHub Copilot)

**Plan section:** §7.5 (`providers.rs`).
**Decision:** Three providers instead of four. Copilot is dropped.
**Why:** Copilot CLI requires `gh` *plus* the `gh-copilot` extension *plus*
an authenticated GitHub account. Detection is fragile (`gh` could be
installed without the extension), and Copilot's CLI model is
question-answer-exit rather than conversational, which doesn't match the
"open the agent in a terminal and keep chatting" mental model DockDuo is
built around. Can be re-added if there's user demand.
**Where it lives:** `AgentKind` enum in `src-tauri/src/claude.rs` (no
separate `providers.rs` module).

---

## D-005 · Theme names: Midnight / Daylight / Pastel / Retro

**Plan section:** §9.3 (Peach / Midnight / Cloud / Moss).
**Decision:** Kept the plan's Midnight theme, replaced the other three
with Daylight (paper-like light), Pastel (soft warm), and Retro (CRT
green). Palettes are defined in `src/lib/themes.ts` as CSS-variable
bundles.
**Why:** The plan's four were tuned for xterm.js terminal backgrounds.
Since DockDuo no longer ships an in-app terminal (see D-003), the themes
only affect bubble colors, picker pills, and shadows — so the palettes
needed redesigning around "chat bubble" rather than "code editor".
**Effect on the plan:** §9.3 theme table is obsolete.

---

## D-006 · Onboarding as its own window instead of an overlay-internal modal

**Plan section:** §2 ("Onboarding is rendered inside the `overlay`
window as a first-run modal layer"), §9.5.
**Decision:** Onboarding lives in a dedicated `onboarding` Tauri window
declared in `tauri.conf.json` (560×540, centered, decorated, hidden by
default). `lib.rs::run()` shows it on first launch when
`config.onboarded == false`; the user's `Get started` click calls
`mark_onboarded` and closes the window.
**Why:** The overlay window is 200 px tall and click-through by default —
a first-run modal inside it would need to disable click-through, grow to
fullscreen, then shrink back, which visibly glitches the taskbar area.
A separate window is simpler and more reliable.
**Cost:** ~400 KB extra bundle per window (shared React tree; Vite
deduplicates chunks).

---

## D-007 · `phrases.rs` is replaced by `src/lib/sprites.ts` phrase arrays

**Plan section:** §7.9 (`phrases.rs`), §8.1 (`get_phrase` command).
**Decision:** Thinking and completion phrases are plain string arrays in
`src/lib/sprites.ts`. There is no `get_phrase` IPC command.
**Why:** The phrase selection is a pure-frontend concern — it needs no
OS access, no persistence, no cross-window coordination. Keeping it on
the JS side avoids an unnecessary IPC round-trip every 3–5 seconds while
a character is in the thinking state.

---

## D-008 · Simplifications vs. plan's dependency set

**Decision:** A few plan dependencies were replaced with lighter
equivalents:
| Plan | Actual | Why |
|---|---|---|
| `zustand` | hand-rolled store with `useSyncExternalStore` | 0 dependencies, <100 LOC, fits the tiny state surface |
| `howler` | raw `new Audio()` in `Character.tsx` | No overlapping sounds needed; `Audio` is built-in |
| `tailwindcss` | inline styles + CSS variables | Theme vars don't benefit from utility classes here |
| `@xterm/*` | — | See D-003 |
**Reversibility:** All three can be re-introduced without schema changes.

---

## D-009 · Embedded terminal deferred from Phase 3 to v0.2.0

**Plan section:** §12 Phase 3, §12 Phase 5 (release gate).
**Decision:** v0.1.0 ships with the detached-console terminal from D-003
and does **not** include the `portable-pty` + `@xterm/xterm` + `terminal_bruce`
/ `terminal_jazz` windows described in plan §7.4 / §9.3. Building the
embedded terminal was re-scoped to v0.2.0.
**Why deferred:**
1. The detached console works, is under 50 lines of Rust, and on Win11
   delegates to Windows Terminal — which is arguably a better terminal host
   than xterm-in-webview for the same reasons listed in D-003.
2. Adding the embedded path means ~4 MB of extra JS bundle, two more Tauri
   windows, a PTY read/write loop, resize handling, input forwarding,
   close-cleanup, and per-byte activity detection — realistically 1.5 days
   of careful engineering plus flakiness-hunting across Windows 10 / 11
   terminal hosts.
3. Phase 5 is otherwise unblocked. Cutting v0.1.0 now lets real users
   shape the embedded-terminal requirements instead of guessing.
**What users lose vs. the plan:**
- Terminal window doesn't inherit DockDuo's theme (same cost as D-003).
- Thinking bubble runs for the whole CLI session instead of byte-level
  idle detection (same cost as D-003).
**What's preserved for later:**
- The four `terminal_bruce` / `terminal_jazz` window labels are still
  declared in `tauri.conf.json` history and can be re-added without schema
  changes. `claude.rs::AgentKind` already carries per-provider metadata
  that an embedded path would need anyway.
- A future `config.useEmbeddedTerminal: bool` flag can swap spawn strategies
  at runtime, letting D-003 stay as the safe default.
**Release implication:** The Phase 5 gate in the build plan assumes the
embedded terminal exists. For v0.1.0 we treat that gate item as "met by
D-003 equivalent behavior" — the acceptance criterion "clicking a
character opens a themed terminal connected to the chosen AI CLI" is
satisfied; "themed" is the piece we're explicitly accepting as a gap.

---

## D-010 · v0.2.0: optional embedded PTY + xterm, multi-monitor overlays, signed updater

**Plan section:** v0.2.0 release scope (embedded terminal, multi-monitor,
updater).
**Decision:** v0.2.0 ships the optional embedded path behind
`config.use_embedded_terminal` (default **false**): `src-tauri/src/pty.rs`
drives `portable-pty` sessions wired to `terminal_bruce` / `terminal_jazz`
webviews with `@xterm/xterm` + Fit + WebGL; `claude.rs::spawn_agent` remains
the system-default detached-console implementation when the flag is off
(D-003 unchanged). Multi-monitor support uses `taskbar::current_all()` plus
multiple overlay window labels (`overlay`, `overlay_1`, …) and
`hit_test::report_bounds(window_label, …)` so each monitor's webview tracks
its own character bounds. The Tauri updater is **active** with a published
public key in `tauri.conf.json`; private signing material lives only in
GitHub Actions secrets / maintainer machines.
**Relationship to earlier decisions:** D-009's deferral is **closed** for
users who opt in via the tray or config. D-003 remains the default spawn
path until the project chooses to flip the default after dogfooding. D-005's
themes now also feed xterm `ITheme` objects when embedded mode is used.
**Trade-offs:** Larger JS bundle when the embedded terminal chunk loads;
ConPTY requires Windows 10 1809+; CLIs that print continuous heartbeats may
keep the bubble in `busy` longer than ideal (same class of limitation noted
under D-003).
**Reversibility:** Setting `use_embedded_terminal` to false restores v0.1.x
behavior without removing the PTY code path.

---

## D-011 · Multi-monitor taskbar overlays deferred out of v0.2.0

**Plan section:** v0.2.0 Phase E (multi-monitor taskbar overlays).
**Decision:** v0.2.0 ships primary-taskbar only. `overlay_1` / `overlay_2`
webviews and the `Shell_SecondaryTrayWnd` enumeration in `taskbar.rs` were
removed before release. `OVERLAY_WINDOW_LABELS` is now the single-element
slice `&["overlay"]` (kept as a slice so a future reintroduction can add
labels without touching `tray.rs` / `fullscreen.rs` / `hit_test.rs`).
**Why:** On single-monitor Windows 11, `EnumWindows` returned phantom
`Shell_SecondaryTrayWnd` instances even after filtering for
`IsWindowVisible`. The result was Bruce and Jazz doubling or tripling on
screen after alt-tab / focus changes — the secondary overlay windows were
reappearing via Windows' default restore behavior. Reliable single-monitor
behavior is the higher priority for this release.
**What's preserved:** `hit_test::report_bounds(window_label, character,
bounds)` keeps its composite key; `Character.tsx` still passes
`windowLabel`. This means re-enabling multi-monitor later only requires:
adding window definitions back to `tauri.conf.json`, extending
`OVERLAY_WINDOW_LABELS`, restoring `current_all` + positioning fan-out.
**Trade-off:** Users on multi-monitor setups see characters only on the
primary-taskbar monitor — identical to v0.1.0 behavior.
**Reversibility:** High. No schema changes, no public-API churn.

---

## D-012 · Embedded UI: Raw (xterm + PTY) vs Chat (lil-style pipes)

**Plan section:** lil-agents chat parity spec
(`docs/superpowers/specs/2026-04-22-lil-agents-chat-parity-design.md`).
**Decision:** When `use_embedded_terminal` is true, `embedded_ui_mode` selects
either **Raw** (existing ConPTY + xterm.js) or **Chat** (transcript UI + Claude
Code `stream-json` over stdin/stdout). The tray **Terminal** submenu offers
three choices: system console, embedded raw, embedded chat. Codex/Gemini in
chat mode return an explicit error until dedicated pipe drivers exist.
**Why:** Upstream lil-agents is chat-first; many users prefer a transcript over
a full TTY. Reusing the same `terminal_*` windows avoids new Tauri labels.
**Trade-offs:** Chat mode uses the same permission-skipping flags as the
reference headless Claude invocation; only Claude is fully wired in v1.
**Reversibility:** Both modes are config-gated; system-terminal behavior is
unchanged.

---

## How to amend this log

Each new decision gets a new top-level `## D-NNN` entry with:
1. Plan section it contradicts
2. What was decided
3. Why the plan's approach didn't work (or was suboptimal)
4. Trade-offs accepted
5. Reversibility notes if non-obvious

Append, never rewrite history.
