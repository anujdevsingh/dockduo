# Phase 3 PTY Terminal Design

## Goal

Complete Phase 3 of the DockDuo build plan by replacing the temporary
`spawn_claude -> cmd.exe /K claude` shortcut with Windows-native, in-app
PTY-backed terminal windows for Bruce and Jazz.

This design stays attached to the existing Phase 3 contract in
[docs/DockDuo-BuildPlan.md](/E:/projects/dockduo/docs/DockDuo-BuildPlan.md).
It does not ship any code from `lil-agents-main`; that folder is reference
only for behavior and UX.

## Current State

The repo already has:

- a working transparent `overlay` Tauri window
- animated Bruce and Jazz sprites
- Rust-side cursor hit testing
- overlay click handling wired from React to Rust
- `ai-status-changed` events consumed by the overlay

The repo does not yet have the actual Phase 3 architecture:

- no `terminal_bruce` or `terminal_jazz` windows in Tauri config
- no `terminal.rs` PTY manager
- no `providers.rs` provider registry
- no in-app terminal UI with xterm.js
- no terminal IPC surface for open, input, resize, and close
- no silence-based AI activity detection

## Scope

This spec covers only the remaining Phase 3 work:

- PTY lifecycle management for per-character sessions
- in-app terminal windows for Bruce and Jazz
- provider registry and Claude availability checks
- typed Tauri commands and events for terminal streaming
- React window routing and xterm.js terminal integration
- AI activity updates driven by PTY output and silence
- terminal cleanup on window close and process exit

This spec does not include Phase 4 work such as tray controls, onboarding,
theme switching UI, sound/theme refactors, or fullscreen detection.

## Design Summary

DockDuo will keep a three-window model:

- `overlay`
- `terminal_bruce`
- `terminal_jazz`

Clicking Bruce or Jazz will no longer launch a separate console host. Instead,
the overlay will request that the matching Tauri terminal window open and attach
to a PTY-backed session owned by Rust.

Rust will own the terminal process lifecycle. React will own terminal rendering.
All terminal bytes move through Tauri events and commands. The overlay keeps its
existing animation and hit-test behavior, but its click action changes from
"spawn Claude in a new console" to "show the correct terminal window and ensure a
session exists for that character."

## Backend Design

### `src-tauri/src/providers.rs`

Add a provider registry that defines the supported CLI providers and their
Windows lookup rules. Phase 3 will keep the structure general for:

- `claude`
- `codex`
- `copilot`
- `gemini`

The initial verified execution path will be `claude`, because that is the Phase
3 acceptance target. The other providers still need correct metadata and PATH
checks so the architecture matches the build plan.

This module will provide:

- provider metadata
- Windows executable resolution
- availability checks
- command construction for spawning the provider inside a PTY

### `src-tauri/src/terminal.rs`

Add a `PtyManager` that owns one live session per character slot. Each session
stores:

- stable `session_id`
- `character_id`
- `provider`
- PTY writer handle for input
- resize handle
- child process handle
- activity state bookkeeping

The manager will expose commands that match the build plan:

- `open_terminal`
- `send_input`
- `resize_terminal`
- `close_terminal`
- `show_terminal`

`open_terminal` will create or replace the session for the requested character,
spawn the provider inside a ConPTY-backed pseudo-terminal, and start a blocking
reader thread that forwards output chunks to the frontend through
`terminal-output` events.

`close_terminal` will kill the child, drop PTY handles, remove the session from
the manager, and emit `terminal-closed`.

`resize_terminal` will resize the PTY using the backend resize handle.

`send_input` will write raw terminal bytes to the session's PTY writer.

`show_terminal` will make `terminal_bruce` or `terminal_jazz` visible and focus
it without starting a second session when one already exists.

### AI Activity Detection

The current process-lifecycle-only `busy -> completed -> idle` model is too
coarse for Phase 3. It will be replaced with output-driven activity:

- when the PTY emits output, the matching character becomes `busy`
- each output chunk refreshes a silence timer
- when no output arrives for `800 ms`, emit `completed`
- after a short celebration pulse, emit `idle`
- if output resumes again, return to `busy`

This preserves the overlay behavior you already built while matching the build
plan more closely for long-lived interactive terminal sessions.

### Reuse vs Replacement

We will keep:

- `hit_test.rs`
- taskbar and overlay positioning code
- `ai-status-changed` event consumption in the overlay

We will replace:

- `claude.rs` as the primary session runtime
- direct overlay invocation of `spawn_claude`

`claude.rs` may either be removed entirely or reduced to helper logic reused by
`providers.rs`, depending on which leaves the cleaner backend after
implementation. The shipped architecture should have one clear session path, not
two competing ones.

## Frontend Design

### Window Routing

`src/main.tsx` will stop assuming every window is the overlay. It will route by
window query parameters so the existing single frontend bundle can render:

- `Overlay`
- `Terminal`

`src/windows/Overlay.tsx` will keep sprite rendering and status listening. Its
click handler will change to:

1. ask Rust to show the correct terminal window
2. ensure a terminal session exists for that character and provider

### `src/windows/Terminal.tsx`

Add a dedicated xterm.js wrapper for the per-character terminal window.

Responsibilities:

- create the xterm instance on mount
- load `FitAddon`, `WebglAddon`, and `WebLinksAddon`
- call `open_terminal`
- subscribe to `terminal-output`
- forward keyboard input through `send_input`
- forward resize events through `resize_terminal`
- call `close_terminal` on unmount or window close
- react to `terminal-closed` so the UI does not hang on a dead session

The terminal window remains Phase 3-only in behavior: terminal rendering,
streaming, cleanup, and enough styling to be usable. Theme switching and tray
integration stay out of scope until Phase 4.

### Shared Frontend State

The existing store will remain the source of runtime animation state for the
overlay. It may gain a small amount of terminal-related state if needed, but the
terminal window should keep most session-specific logic local so Phase 3 stays
focused.

## Event and Command Flow

### Open Flow

1. User clicks Bruce or Jazz on the overlay.
2. Overlay invokes `show_terminal` for that character's window label.
3. Terminal window mounts and calls `open_terminal`.
4. Rust resolves the active provider command and spawns the PTY session.
5. Rust emits output chunks to `terminal-output`.
6. Terminal UI writes those bytes into xterm.
7. Rust emits `ai-status-changed` for `busy`, `completed`, and `idle`.
8. Overlay updates the sprite bubble and celebration state through the existing
   store path.

### Close Flow

1. User closes the terminal window or React unmounts.
2. Frontend invokes `close_terminal`.
3. Rust kills the child and drops PTY resources.
4. Rust emits `terminal-closed`.
5. Overlay returns the character to `idle` after any completion pulse.

## Error Handling

Phase 3 should fail clearly when a provider is unavailable.

Rules:

- `open_terminal` returns a typed error if the provider executable cannot be
  resolved
- the terminal window should show a friendly error message instead of blank
  output
- failed session startup must not leave behind a half-open window or leaked PTY
  handles
- opening Bruce and Jazz at the same time must keep their sessions isolated

## Testing and Validation

Phase 3 is complete only when the implementation can satisfy the current build
plan gate:

1. Clicking Bruce opens an in-app terminal window and `claude` is visible in
   Task Manager.
2. Typing in the terminal reaches the CLI and streamed responses appear back in
   the terminal.
3. Resizing the terminal window reflows cleanly.
4. Closing the terminal kills the child quickly and reliably.
5. Bruce and Jazz can each run their own independent session.
6. Repeated open and close cycles do not accumulate orphaned child processes or
   stale PTY sessions.

Local verification for the implementation phase should include:

- `cargo check --manifest-path src-tauri/Cargo.toml`
- `node .\\node_modules\\typescript\\bin\\tsc -p tsconfig.json --noEmit`
- `node .\\node_modules\\vite\\bin\\vite.js build`
- manual open and close repetition while watching Task Manager for child cleanup

## Implementation Notes

- Keep `lil-agents-main` ignored and out of application imports, assets, and
  packaging.
- Follow the build plan structure where it helps, but adapt names and file
  boundaries to the current DockDuo repo when needed.
- Prefer the cleanest single terminal architecture over preserving temporary
  shortcuts.
- Do not mix Phase 4 concerns into this work.
