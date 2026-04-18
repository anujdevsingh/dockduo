# Lil Agents for Windows — Agent-Ready Build Plan

**Version:** 2.0 (April 2026)
**Status:** READY FOR AGENT EXECUTION
**Target agent:** Cursor / Claude Code / any Rust + TypeScript-capable coding agent
**Target human:** Anuj Dev Singh (solo developer, 6-week MVP timeline)
**Upstream project:** https://github.com/ryanstephen/lil-agents (MIT, macOS original by Ryan Stephen)

---

## 0. How to Read This Document (Agent Instructions)

You are an autonomous coding agent tasked with building a full Windows application based on this specification. Read the entire document before writing any code. The document is ordered so that each section depends only on sections above it.

**Execution rules:**
1. Follow the exact versions pinned in Section 3. Do not substitute "latest" — pinned versions are verified compatible as of April 2026.
2. Complete one **Phase** (Section 12) at a time. Do not start Phase N+1 until all Phase N acceptance criteria in Section 13 are demonstrably met.
3. After each module is implemented, run `cargo check`, `pnpm tsc --noEmit`, and `cargo tauri dev` to verify it builds and launches before moving on.
4. If you encounter an ambiguity in this spec, prefer the simpler solution and document the decision in `DECISIONS.md` at the repo root.
5. Do **not** add scope beyond what this document describes. Features marked "OUT OF SCOPE" must stay out.
6. When your choice contradicts a Tauri/Rust warning or deprecation notice that appeared after April 2026, follow the newer guidance and log it in `DECISIONS.md`.

**What this plan changes from v1.0:**
- Drops the Rust 16 ms click-through poll loop. Uses Tauri v2's native `setIgnoreCursorEvents` instead.
- Collapses the separate thought-bubble window into the main overlay window.
- Drops EV code signing from MVP (unsigned release on GitHub Releases).
- Adds global hotkey, full-screen app detection, first-run CLI detection, crash logging.
- Tightens timeline from 10 weeks to 6 weeks.
- Upgrades all dependencies to April-2026-current versions.

---

## 1. Product Summary (One Page)

**Lil Agents for Windows** is a lightweight system-tray application that places two animated pixel-art characters — Bruce and Jazz — just above the Windows Taskbar. The characters walk back and forth continuously. Clicking a character opens a themed PTY-backed terminal connected to the user's chosen AI CLI (Claude Code, OpenAI Codex, GitHub Copilot CLI, or Google Gemini CLI). While the CLI is producing output, the character plays a "thinking" animation and a thought bubble floats above it with rotating playful phrases. When the CLI completes, the character plays a celebration animation and a sound effect fires.

**Non-goals / out of scope:**
- Linux port
- Bundling any AI CLI tools (user must install these themselves)
- macOS-Windows feature divergence (match the Mac feature set exactly)
- New character artwork (reuse Mac sprites; extraction covered in Section 10)
- Accounts, cloud sync, analytics, telemetry
- Model API calls from inside the app (the CLI process owns all AI interaction)

**Core product constraints:**
- Installed size under 25 MB
- Idle RAM under 50 MB
- Idle CPU under 1%
- 30 FPS animation lock
- Zero telemetry, fully local
- No admin privileges required to install or run

---

## 2. Architecture Overview

Strict two-layer architecture with Tauri's IPC bridge between them.

| Layer | Technology | Owns |
|---|---|---|
| OS bindings | `windows` crate (windows-rs) | Taskbar rect, DPI, monitor enumeration, foreground-window detection |
| Backend | Rust + Tauri 2.10 runtime | Window lifecycle, PTY processes, tray, events, updates, config |
| IPC bridge | Tauri commands + events | Type-safe async message passing |
| Frontend | React 19 + TypeScript 5.6 | Canvas animation, xterm.js, themes, UI state |
| Build | Cargo + Vite 6 + Tauri CLI 2.9 | Compile, bundle, package |

### Process model at runtime

1. **`lil-agents.exe`** — single Tauri host process, single-instance-enforced, owns Rust async runtime
2. **WebView2** — one instance per visible window, embedded via COM, renders React frontend
3. **AI CLI child process** — spawned on demand when user opens a terminal, one per character session, runs inside a ConPTY-backed pseudo-terminal

Communication paths:
- Main process ↔ WebView2 via Tauri IPC (typed commands + events)
- Main process ↔ CLI child via PTY master/slave pair (raw bytes)
- No direct WebView2 ↔ CLI communication exists

### Window model (3 windows total — down from 5 in v1.0)

| Window ID | Style | Visible | Purpose |
|---|---|---|---|
| `overlay` | Transparent, frameless, topmost, no taskbar, click-through except over character sprites | Always | Bruce + Jazz sprite animation + inline thought bubbles (rendered as DOM siblings) |
| `terminal_bruce` | Themed, borderless popover | On click | Full PTY terminal for Bruce's AI CLI session |
| `terminal_jazz` | Themed, borderless popover | On click | Full PTY terminal for Jazz's AI CLI session |

Onboarding is rendered inside the `overlay` window as a first-run modal layer, not as a separate window.

---

## 3. Technology Stack (Pinned Versions — April 2026)

### Rust crates (Cargo.toml)

| Crate | Version | Purpose |
|---|---|---|
| `tauri` | `2.10` | Desktop framework |
| `tauri-build` | `2.5` | Build-time codegen |
| `tauri-plugin-updater` | `2` | Auto-update |
| `tauri-plugin-single-instance` | `2` | Single-instance enforcement |
| `tauri-plugin-global-shortcut` | `2` | Global hotkey (Ctrl+Shift+L to toggle overlay) |
| `tauri-plugin-log` | `2` | Crash/event logging to `%APPDATA%\LilAgents\logs` |
| `tauri-plugin-autostart` | `2` | Optional start-on-boot |
| `windows` | `0.60` | Win32 API bindings (SHAppBarMessage, DPI, GetForegroundWindow) |
| `portable-pty` | `0.9` | ConPTY-backed pseudo-terminal |
| `tokio` | `1` (features = `["full"]`) | Async runtime |
| `serde` | `1` (features = `["derive"]`) | Serialisation |
| `serde_json` | `1` | Config + IPC payloads |
| `anyhow` | `1` | Error handling in non-library code |
| `thiserror` | `2` | Error types in library code |
| `tracing` | `0.1` | Structured logs |
| `once_cell` | `1` | Lazy statics |

Rust toolchain: **stable 1.85+** (2024 edition).

### NPM / pnpm packages (package.json)

| Package | Version | Purpose |
|---|---|---|
| `react` | `^19.2.5` | UI framework |
| `react-dom` | `^19.2.5` | |
| `typescript` | `^5.6` | Type checker |
| `@tauri-apps/api` | `^2` | IPC bindings |
| `@tauri-apps/plugin-updater` | `^2` | |
| `@tauri-apps/plugin-global-shortcut` | `^2` | |
| `@tauri-apps/plugin-log` | `^2` | |
| `@xterm/xterm` | `^6.0.0` | Terminal emulator (note: new scoped name, old `xterm` package is deprecated) |
| `@xterm/addon-fit` | `^0.11.0` | Auto-resize terminal to container |
| `@xterm/addon-webgl` | `^0.19.0` | GPU-accelerated renderer (huge perf win) |
| `@xterm/addon-web-links` | `^0.12.0` | Clickable URLs in terminal output |
| `howler` | `^2.2.4` | Sound playback |
| `zustand` | `^5` | Global state |
| `tailwindcss` | `^4` | Styling (note: CSS-first config, no `tailwind.config.ts` needed) |
| `@vitejs/plugin-react` | `^4` | Vite React plugin |

### Build tools

| Tool | Version | Install |
|---|---|---|
| Tauri CLI | `^2.9` | `cargo install tauri-cli --version "^2"` |
| Node.js | `20 LTS or 22 LTS` | `nodejs.org` |
| pnpm | `^9` | `npm i -g pnpm` |
| Vite | `^6` | (transitive) |
| Visual Studio Build Tools 2022 | Latest | "Desktop development with C++" workload |
| FFmpeg | `^7` | Asset conversion (one-time) |

**Do not use** `bun` as package manager — while fast, Tauri's asset-bundling path is best-tested with pnpm.

---

## 4. Project Directory Structure

```
lil-agents-windows/
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # App entry, Tauri builder, plugin wiring
│   │   ├── commands.rs           # All #[tauri::command] handlers
│   │   ├── overlay.rs            # Transparent window setup, click-through toggle
│   │   ├── taskbar.rs            # SHAppBarMessage polling, TaskbarInfo struct
│   │   ├── terminal.rs           # PTY lifecycle, CLI spawning, I/O streaming
│   │   ├── tray.rs               # System tray icon + menu
│   │   ├── config.rs             # %APPDATA%\LilAgents\config.json read/write
│   │   ├── providers.rs          # CLI detection + command table
│   │   ├── phrases.rs            # Thought bubble phrase pool
│   │   ├── fullscreen.rs         # Detect full-screen foreground apps
│   │   └── lib.rs                # Module re-exports
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json          # Permission capability file
│   ├── icons/                    # .ico + all PNG sizes
│   └── build.rs
├── src/                          # React frontend
│   ├── main.tsx                  # Entry; routes to Overlay or Terminal based on window label
│   ├── App.tsx                   # Top-level window router
│   ├── windows/
│   │   ├── Overlay.tsx           # Main overlay: characters + thought bubbles + onboarding modal
│   │   └── Terminal.tsx          # xterm.js terminal window
│   ├── components/
│   │   ├── Character.tsx         # Canvas sprite animation engine
│   │   ├── ThoughtBubble.tsx     # Inline CSS bubble, rendered inside Overlay
│   │   ├── Onboarding.tsx        # First-run modal
│   │   └── TrayMenuSync.tsx      # Listens for tray events, updates state
│   ├── hooks/
│   │   ├── useTaskbar.ts         # Subscribe to taskbar-changed events
│   │   ├── useCharacterAnimation.ts
│   │   └── usePtySession.ts      # xterm.js + Tauri PTY wiring
│   ├── store/
│   │   └── appStore.ts           # Zustand: provider, theme, positions, visibility
│   ├── lib/
│   │   ├── sprites.ts            # Sprite sheet loader + frame computation
│   │   ├── sounds.ts             # Howler instances
│   │   └── ipc.ts                # Typed wrappers around Tauri invoke/listen
│   ├── styles/
│   │   └── global.css            # Tailwind v4 import + theme CSS variables
│   └── types/
│       └── ipc.ts                # Shared Rust<->TS type definitions (mirror of commands.rs)
├── assets/                       # Bundled into binary
│   ├── sprites/
│   │   ├── bruce.png             # Sprite sheet, all 4 states
│   │   ├── bruce.json            # { frameWidth, frameHeight, fps, states }
│   │   ├── jazz.png
│   │   └── jazz.json
│   └── sounds/
│       ├── task-complete.ogg
│       ├── error.ogg
│       └── startup.ogg
├── scripts/
│   ├── extract-mac-assets.js     # Pull .mov files from mac .app bundle
│   ├── convert-sprites.mjs       # FFmpeg + sharp pipeline
│   └── gen-manifest.mjs
├── .github/workflows/
│   ├── build.yml                 # On every PR
│   └── release.yml               # On version tag
├── docs/
│   ├── ARCHITECTURE.md           # Agent writes this after Phase 2
│   ├── DECISIONS.md              # Agent logs deviations here
│   └── TROUBLESHOOTING.md
├── package.json
├── pnpm-lock.yaml
├── vite.config.ts
├── tsconfig.json
├── README.md
└── LICENSE                       # MIT
```

---

## 5. Bootstrap Commands (Copy-Paste Ready)

Execute these in order. Assume a clean Windows 11 dev machine.

```powershell
# Step 1 — prerequisites (one-time, skip if already installed)
# Install Visual Studio Build Tools 2022 with "Desktop development with C++" workload
# Install Node.js 20 LTS from nodejs.org
# Install Rust from rustup.rs (accept defaults)

# Step 2 — pnpm + Tauri CLI
npm install -g pnpm
cargo install tauri-cli --version "^2"

# Step 3 — scaffold project (from parent directory)
pnpm create tauri-app@latest lil-agents-windows `
  --template react-ts `
  --manager pnpm `
  --identifier "com.codewithanuj.lilagents"

cd lil-agents-windows

# Step 4 — add Rust dependencies
cd src-tauri
cargo add tauri-plugin-updater@2
cargo add tauri-plugin-single-instance@2
cargo add tauri-plugin-global-shortcut@2
cargo add tauri-plugin-log@2
cargo add tauri-plugin-autostart@2
cargo add windows@0.60 --features "Win32_UI_Shell,Win32_UI_WindowsAndMessaging,Win32_Graphics_Gdi,Win32_Foundation,Win32_UI_HiDpi"
cargo add portable-pty@0.9
cargo add tokio@1 --features full
cargo add serde@1 --features derive
cargo add serde_json
cargo add anyhow
cargo add thiserror@2
cargo add tracing
cargo add once_cell
cd ..

# Step 5 — add JS dependencies
pnpm add @tauri-apps/plugin-updater @tauri-apps/plugin-global-shortcut @tauri-apps/plugin-log
pnpm add @xterm/xterm@^6 @xterm/addon-fit@^0.11 @xterm/addon-webgl @xterm/addon-web-links
pnpm add howler zustand
pnpm add -D @types/howler tailwindcss@4 @tailwindcss/vite

# Step 6 — verify the skeleton runs
pnpm tauri dev
# If a default Tauri window opens, the toolchain is working. Close it and proceed.
```

---

## 6. tauri.conf.json — Full Configuration

Replace the generated `src-tauri/tauri.conf.json` with this. Values are the exact shape the agent should produce; adjust paths only if the scaffold differs.

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Lil Agents",
  "version": "0.1.0",
  "identifier": "com.codewithanuj.lilagents",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "withGlobalTauri": false,
    "windows": [
      {
        "label": "overlay",
        "url": "index.html?window=overlay",
        "title": "Lil Agents",
        "width": 1920,
        "height": 200,
        "x": 0,
        "y": 880,
        "transparent": true,
        "decorations": false,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "resizable": false,
        "focus": false,
        "shadow": false,
        "acceptFirstMouse": true,
        "visible": true
      },
      {
        "label": "terminal_bruce",
        "url": "index.html?window=terminal&character=bruce",
        "title": "Bruce",
        "width": 720,
        "height": 520,
        "decorations": false,
        "transparent": true,
        "resizable": true,
        "visible": false,
        "center": true
      },
      {
        "label": "terminal_jazz",
        "url": "index.html?window=terminal&character=jazz",
        "title": "Jazz",
        "width": 720,
        "height": 520,
        "decorations": false,
        "transparent": true,
        "resizable": true,
        "visible": false,
        "center": true
      }
    ],
    "security": { "csp": null },
    "trayIcon": {
      "id": "main",
      "iconPath": "icons/tray.ico",
      "iconAsTemplate": false,
      "menuOnLeftClick": false
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.ico"
    ],
    "windows": {
      "nsis": {
        "installMode": "perUser",
        "displayLanguageSelector": false,
        "languages": ["English"]
      },
      "webviewInstallMode": { "type": "embedBootstrapper" }
    }
  },
  "plugins": {
    "updater": {
      "active": false,
      "endpoints": [
        "https://github.com/anujdevsingh/lil-agents-windows/releases/latest/download/latest.json"
      ],
      "pubkey": "TO_BE_GENERATED_IN_PHASE_5"
    }
  }
}
```

**Key choices explained:**
- `focus: false` on overlay — prevents stealing focus from the user's active app
- `acceptFirstMouse: true` — lets character clicks land even when the overlay isn't focused
- `shadow: false` — no drop shadow on transparent window (removes the grey halo bug)
- `nsis` + `perUser` — no admin rights required; installs to `%LOCALAPPDATA%\Programs\Lil Agents`
- `webviewInstallMode: embedBootstrapper` — ships a tiny WebView2 installer for Windows 10 users who don't have it
- Updater `active: false` in MVP; flip to `true` in Phase 5 once signing keys exist

---

## 7. Rust Module Specifications

Each subsection specifies the **responsibilities, public API, and key implementation notes** for one Rust module. The agent writes idiomatic Rust matching these contracts.

### 7.1 `main.rs`

Entry point. Registers plugins, builds windows, installs tray, runs.

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus existing overlay on duplicate launch
        }))
        .plugin(tauri_plugin_log::Builder::default()
            .targets([
                tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Folder {
                    path: dirs::data_dir().unwrap().join("LilAgents/logs"),
                    file_name: Some("app".into()),
                }),
                tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
            ])
            .build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .setup(|app| {
            overlay::configure_overlay_window(app)?;
            taskbar::start_polling(app.handle().clone());
            tray::build_tray(app)?;
            fullscreen::start_monitoring(app.handle().clone());
            commands::register_global_shortcut(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_taskbar_info,
            commands::open_terminal,
            commands::close_terminal,
            commands::send_input,
            commands::resize_terminal,
            commands::set_ignore_cursor_events,
            commands::set_provider,
            commands::set_theme,
            commands::get_config,
            commands::get_phrase,
            commands::check_cli_available,
            commands::toggle_overlay,
            commands::check_for_updates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 7.2 `overlay.rs`

**Responsibility:** Configure the transparent overlay window. Does NOT implement manual click-through polling — that is handled by the frontend via `setIgnoreCursorEvents`.

**Public functions:**
- `configure_overlay_window(app: &App) -> Result<()>` — sets initial ignore-cursor-events state to `true` so background is click-through by default, then positions the window over the taskbar.
- `position_above_taskbar(window: &WebviewWindow, tb: &TaskbarInfo) -> Result<()>` — calculates x/y/width/height from TaskbarInfo and calls `window.set_position()` + `window.set_size()`.

**Key implementation notes:**
- Call `window.set_ignore_cursor_events(true)` at startup. The **frontend** toggles this to `false` when the cursor enters a character's bounding box.
- Apply `WS_EX_TOOLWINDOW` via `windows-rs` after window creation so the overlay never appears in Alt+Tab:
  ```rust
  use windows::Win32::UI::WindowsAndMessaging::{
      GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW, WS_EX_NOACTIVATE,
  };
  let hwnd = HWND(window.hwnd()?.0 as _);
  let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
  SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_TOOLWINDOW.0 as isize | WS_EX_NOACTIVATE.0 as isize);
  ```
- Do NOT manually set `WS_EX_LAYERED` or `WS_EX_TRANSPARENT` — Tauri v2's `transparent: true` + `set_ignore_cursor_events` already produce these.

### 7.3 `taskbar.rs`

**Responsibility:** Detect Windows Taskbar edge, rect, DPI, auto-hide state. Emit `taskbar-changed` event when any changes.

**Public types:**
```rust
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskbarEdge { Bottom, Top, Left, Right }

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskbarInfo {
    pub edge: TaskbarEdge,
    pub rect: [i32; 4],        // [left, top, right, bottom]
    pub auto_hide: bool,
    pub dpi_scale: f64,        // 1.0 = 96 DPI, 1.25 = 120 DPI, etc.
    pub monitor_rect: [i32; 4],
}
```

**Public functions:**
- `fn current() -> Result<TaskbarInfo>` — one-shot detection via `SHAppBarMessage(ABM_GETTASKBARPOS)` + `GetDpiForMonitor`.
- `fn start_polling(app: AppHandle)` — spawns a Tokio task that calls `current()` every 1000 ms. On change (PartialEq check), emits `taskbar-changed` event and calls `overlay::position_above_taskbar`.

**Key implementation notes:**
- Poll interval: **1000 ms**, not 500 ms. Taskbar rarely changes; 500 ms was premature optimization.
- Use `SHAppBarMessage(ABM_GETSTATE)` to read `ABS_AUTOHIDE`.
- For DPI, call `GetDpiForMonitor` on the monitor containing the taskbar HWND.
- Rect values are in **physical pixels**. The frontend must divide by `dpi_scale` to position in CSS pixels.

### 7.4 `terminal.rs`

**Responsibility:** Full lifecycle of PTY sessions — spawn, stream I/O, resize, teardown.

**Public types:**
```rust
pub struct PtySession {
    pub id: String,              // uuid
    pub character_id: String,    // "bruce" | "jazz"
    pub provider: String,        // "claude" | "codex" | "copilot" | "gemini"
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
}

pub struct PtyManager {
    sessions: Arc<RwLock<HashMap<String, Arc<Mutex<PtySession>>>>>,
}
```

**Public functions:**
- `fn new() -> Self`
- `async fn open(&self, app: AppHandle, character_id: String, provider: String) -> Result<String>` — returns session_id
- `async fn close(&self, session_id: &str) -> Result<()>`
- `async fn send_input(&self, session_id: &str, data: &str) -> Result<()>`
- `async fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<()>`

**Key implementation notes:**
- Use `portable_pty::native_pty_system()` which resolves to ConPTY on Windows.
- Spawn a `tokio::spawn_blocking` (not `tokio::spawn`) for the PTY read loop — PTY reads are blocking on Windows.
- Emit `terminal-output` events in chunks of up to 4 KiB to balance latency vs event overhead.
- Emit `ai-activity-started` on the first byte after ≥800 ms of silence; emit `ai-activity-ended` after the stdout stream is silent for 800 ms. Track per-session state.
- On `close`: kill the child via `child.kill()` (portable-pty handles `TerminateProcess` internally), drop the master, remove from sessions map.
- Wrap PTY creation in a timeout (5 s) so a CLI that fails to start doesn't hang the UI.

### 7.5 `providers.rs`

**Responsibility:** Know which AI CLIs are supported and whether they're currently installed.

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct Provider {
    pub id: &'static str,           // "claude" | "codex" | "copilot" | "gemini"
    pub display_name: &'static str, // "Claude Code"
    pub command: &'static str,      // "claude" | "codex" | "gh" | "gemini"
    pub args: &'static [&'static str],
    pub install_url: &'static str,
    pub install_hint: &'static str, // e.g. "npm install -g @anthropic-ai/claude-code"
}

pub const PROVIDERS: &[Provider] = &[
    Provider { id: "claude", display_name: "Claude Code", command: "claude", args: &[], install_url: "https://docs.claude.com/en/docs/claude-code", install_hint: "npm install -g @anthropic-ai/claude-code" },
    Provider { id: "codex", display_name: "OpenAI Codex", command: "codex", args: &[], install_url: "https://github.com/openai/codex", install_hint: "npm install -g @openai/codex" },
    Provider { id: "copilot", display_name: "GitHub Copilot", command: "gh", args: &["copilot"], install_url: "https://cli.github.com", install_hint: "gh extension install github/gh-copilot" },
    Provider { id: "gemini", display_name: "Google Gemini", command: "gemini", args: &[], install_url: "https://github.com/google-gemini/gemini-cli", install_hint: "npm install -g @google/gemini-cli" },
];

pub fn is_available(id: &str) -> bool { /* `where.exe {command}` */ }
```

### 7.6 `tray.rs`

**Responsibility:** Build the system tray icon with context menu.

Menu structure (top to bottom):
1. **Provider** submenu — radio buttons for each provider in `PROVIDERS`
2. **Theme** submenu — radio buttons: Peach / Midnight / Cloud / Moss
3. ─── separator ───
4. **Toggle overlay** (`Ctrl+Shift+L`)
5. **Start with Windows** (checkbox)
6. ─── separator ───
7. **Check for updates**
8. **About**
9. **Quit**

Build using `tauri::tray::TrayIconBuilder` and `tauri::menu::MenuBuilder`. Radio-button state comes from `config::get_config()`; changes write back via `set_provider` / `set_theme`.

### 7.7 `config.rs`

**Responsibility:** Read/write `%APPDATA%\LilAgents\config.json`. Use `dirs::data_dir()`.

```rust
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub active_provider: String,     // default "claude"
    pub active_theme: String,        // default "peach"
    pub first_run: bool,             // default true
    pub overlay_visible: bool,       // default true
    pub start_with_windows: bool,    // default false
    pub sound_enabled: bool,         // default true
}

pub fn load() -> Config { /* read file, fallback to Default */ }
pub fn save(cfg: &Config) -> Result<()> { /* atomic write: .tmp + rename */ }
```

Atomic write is important — partial writes on crash must not corrupt the file.

### 7.8 `fullscreen.rs`

**Responsibility:** Detect when a full-screen app (game, video, presentation) is in the foreground. Hide the overlay in those cases.

```rust
pub fn start_monitoring(app: AppHandle) {
    tokio::spawn(async move {
        let mut last_state = false;
        loop {
            let is_fs = is_foreground_fullscreen().unwrap_or(false);
            if is_fs != last_state {
                app.emit("fullscreen-state", is_fs).ok();
                last_state = is_fs;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}

fn is_foreground_fullscreen() -> Result<bool> {
    // GetForegroundWindow -> GetWindowRect -> compare to MonitorFromWindow rect
    // Return true if window rect == monitor rect AND window is not the desktop/shell
}
```

The frontend listens for `fullscreen-state` and hides characters when `true`.

### 7.9 `phrases.rs`

40+ thought bubble phrases, bundled. Random selection without repetition until exhausted, then reshuffled.

```rust
pub const PHRASES: &[&str] = &[
    "thinking very hard...",
    "consulting the oracle...",
    "almost there...",
    "running the numbers...",
    "checking my notes...",
    "hmm, interesting...",
    "let me think...",
    // ... 33+ more
];
```

### 7.10 `commands.rs`

All `#[tauri::command]` handlers. See Section 8 for the full IPC contract.

---

## 8. Tauri IPC Contract (Definitive)

### 8.1 Commands (frontend → backend)

| Command | Input | Returns / Effect |
|---|---|---|
| `get_taskbar_info` | – | `TaskbarInfo` JSON |
| `open_terminal` | `{ character_id: string, provider: string }` | `{ session_id: string }` |
| `close_terminal` | `{ session_id: string }` | `()` |
| `send_input` | `{ session_id: string, data: string }` | `()` |
| `resize_terminal` | `{ session_id: string, cols: number, rows: number }` | `()` |
| `set_ignore_cursor_events` | `{ window_label: string, ignore: boolean }` | `()` — calls `window.set_ignore_cursor_events(ignore)` |
| `set_provider` | `{ provider: string }` | Persists to config |
| `set_theme` | `{ theme: string }` | Persists to config |
| `get_config` | – | `Config` JSON |
| `get_phrase` | `{ exclude: string[] }` | `{ phrase: string }` |
| `check_cli_available` | `{ provider: string }` | `{ available: boolean, install_hint: string }` |
| `toggle_overlay` | – | `()` — shows/hides overlay window |
| `check_for_updates` | – | Triggers updater |

### 8.2 Events (backend → frontend)

| Event | Payload | Meaning |
|---|---|---|
| `taskbar-changed` | `TaskbarInfo` | Taskbar moved or DPI changed |
| `terminal-output` | `{ session_id: string, data: number[] }` | Raw PTY stdout bytes (as `Vec<u8>` serialized as number array) |
| `terminal-closed` | `{ session_id: string, exit_code: number }` | CLI process exited |
| `ai-activity-started` | `{ character_id: string }` | Trigger think state + thought bubble |
| `ai-activity-ended` | `{ character_id: string, success: boolean }` | Trigger celebrate or walk |
| `fullscreen-state` | `boolean` | Full-screen foreground app detected (hide/show overlay) |
| `tray-provider-changed` | `{ provider: string }` | User picked provider from tray |
| `tray-theme-changed` | `{ theme: string }` | User picked theme from tray |
| `update-available` | `{ version: string, notes: string }` | Updater found new version |

---

## 9. Frontend Specifications

### 9.1 Window router (`main.tsx`)

Parses `window` query param to mount the right tree:

```tsx
const params = new URLSearchParams(window.location.search);
const windowType = params.get("window"); // "overlay" | "terminal"
const character = params.get("character"); // "bruce" | "jazz" (terminal only)

const root = createRoot(document.getElementById("root")!);
root.render(
  windowType === "terminal"
    ? <TerminalWindow character={character!} />
    : <OverlayWindow />
);
```

### 9.2 `Character.tsx` — Canvas sprite animation engine

**Props:** `{ character: "bruce" | "jazz", taskbarInfo: TaskbarInfo, state: AnimationState }`

**Behavior:**
- Loads sprite sheet + manifest once on mount.
- Runs `requestAnimationFrame` loop. Each frame:
  1. Compute delta-time, clamped to max 50 ms (prevents jumps after tab-out).
  2. Advance frame index based on `fps` from manifest.
  3. Update x-position (walk state only); reverse direction at overlay bounds.
  4. `ctx.clearRect` + `ctx.drawImage` with source rect.
- On state change, emit bounds to Zustand store so `onMouseEnter`/`onMouseLeave` handlers on an invisible overlay div can toggle `set_ignore_cursor_events`.

**Animation states:** `walk` (0–23) | `think` (24–47) | `celebrate` (48–63) | `idle` (64–79). Frame ranges are in the manifest, not hard-coded.

**Click-through handling (key improvement over v1.0):**

Instead of a Rust poll loop, use a transparent `<div>` positioned over each character's current bounds. The `div` has:
- `pointer-events: auto`
- `onMouseEnter` → `invoke("set_ignore_cursor_events", { window_label: "overlay", ignore: false })`
- `onMouseLeave` → `invoke("set_ignore_cursor_events", { window_label: "overlay", ignore: true })`
- `onClick` → open that character's terminal window

Update the div's `left`/`top`/`width`/`height` every frame via the rAF loop. Zero Rust polling. Zero CPU waste.

### 9.3 `Terminal.tsx` — xterm.js wrapper

**Mount sequence:**
```tsx
const term = new Terminal({
  fontFamily: "'JetBrains Mono', 'Cascadia Code', monospace",
  fontSize: 13,
  cursorBlink: true,
  theme: themeColors[activeTheme],
  allowProposedApi: true,
});
term.loadAddon(new FitAddon());
term.loadAddon(new WebglAddon());  // GPU acceleration
term.loadAddon(new WebLinksAddon());
term.open(containerRef.current!);
fitAddon.fit();

const { session_id } = await invoke("open_terminal", { character_id, provider });

const unlisten = await listen<{ session_id: string; data: number[] }>(
  "terminal-output",
  ({ payload }) => {
    if (payload.session_id === session_id) {
      term.write(new Uint8Array(payload.data));
    }
  }
);

term.onData((d) => invoke("send_input", { session_id, data: d }));
term.onResize(({ cols, rows }) => invoke("resize_terminal", { session_id, cols, rows }));

// On unmount: invoke("close_terminal", { session_id }); unlisten(); term.dispose();
```

**Themes (CSS variables):**

| Theme | Background | Foreground | Cursor | Accent |
|---|---|---|---|---|
| Peach | `#FFF0E8` | `#2D1B0E` | `#E8845A` | `#E8845A` |
| Midnight | `#0D1117` | `#E6EDF3` | `#58A6FF` | `#58A6FF` |
| Cloud | `#F4F6F8` | `#1C2526` | `#9DABB8` | `#9DABB8` |
| Moss | `#0F1A0F` | `#C8E6C9` | `#4CAF50` | `#4CAF50` |

### 9.4 `ThoughtBubble.tsx` — inline CSS component

Rendered inside `Overlay.tsx` as a DOM sibling to the character. No separate Tauri window. Position: `absolute`, computed from the character's current sprite bounds (one row above + offset).

**Animation sequence:**
1. Fade-in 200 ms
2. Display 3500 ms
3. Fade-out 300 ms
4. Request new phrase via `invoke("get_phrase", { exclude: recentPhrases })`
5. Repeat while `ai-activity-started` state is active

### 9.5 `Onboarding.tsx` — first-run modal

Renders as an overlay on top of `Overlay.tsx` when `config.first_run === true`.

Flow:
1. Welcome screen ("Meet Bruce and Jazz")
2. CLI detection: for each of the 4 providers, call `check_cli_available`. Show ✅ or ❌ next to each, with install-hint button that opens `install_url` in default browser via `@tauri-apps/plugin-shell`.
3. Provider selection: radio buttons for only the ✅ providers (disabled for ❌).
4. Theme selection: 4 preview tiles.
5. "You're all set" — writes `first_run: false` to config, closes modal.

### 9.6 Zustand store (`appStore.ts`)

```ts
interface AppStore {
  activeProvider: string;
  activeTheme: string;
  taskbarInfo: TaskbarInfo | null;
  overlayVisible: boolean;
  fullscreenActive: boolean;
  characterBounds: Record<"bruce" | "jazz", { x: number; y: number; w: number; h: number }>;
  aiActivity: Record<"bruce" | "jazz", "idle" | "thinking" | "celebrating">;

  setProvider: (p: string) => void;
  setTheme: (t: string) => void;
  setTaskbarInfo: (tb: TaskbarInfo) => void;
  setCharacterBounds: (c: "bruce" | "jazz", b: {...}) => void;
  setAiActivity: (c: "bruce" | "jazz", s: "idle" | "thinking" | "celebrating") => void;
  setFullscreen: (b: boolean) => void;
}
```

No persistence inside Zustand — the Rust `config.rs` is source of truth. Zustand is just runtime cache.

---

## 10. Asset Pipeline

### 10.1 Extracting Mac assets (one-time)

The upstream Lil Agents macOS app ships HEVC-alpha `.mov` files inside its `.app` bundle under `Contents/Resources`. On a Mac, `bruce.mov` and `jazz.mov` can be copied directly. If you don't have a Mac:

```bash
# Download the latest .dmg from lilagents.xyz
# Mount it, or extract with 7-Zip on Windows
7z x LilAgents.dmg
# Navigate to LilAgents.app/Contents/Resources/
# Copy bruce.mov and jazz.mov into scripts/raw-assets/
```

### 10.2 Converting to sprite sheets (`scripts/convert-sprites.mjs`)

```js
// Pseudocode outline — agent writes the full script
import { execSync } from "node:child_process";
import sharp from "sharp";
import { writeFileSync } from "node:fs";

const FPS = 30;
const CHARS = ["bruce", "jazz"];
const STATES = ["walk", "think", "celebrate", "idle"];
// Assumes mac assets are named bruce_walk.mov, bruce_think.mov, etc.

for (const char of CHARS) {
  const allFrames = [];
  const stateRanges = {};
  let cursor = 0;

  for (const state of STATES) {
    const input = `scripts/raw-assets/${char}_${state}.mov`;
    execSync(`ffmpeg -y -i ${input} -vf fps=${FPS} -pix_fmt rgba scripts/tmp/${char}_${state}_%04d.png`);
    const frames = /* list PNGs */;
    stateRanges[state] = [cursor, cursor + frames.length - 1];
    cursor += frames.length;
    allFrames.push(...frames);
  }

  // Pack horizontally with sharp
  const frameWidth = 128;   // measure from first frame
  const frameHeight = 128;
  const sheet = sharp({ create: {
    width: frameWidth * allFrames.length,
    height: frameHeight,
    channels: 4,
    background: { r: 0, g: 0, b: 0, alpha: 0 }
  }});
  const composites = allFrames.map((p, i) => ({ input: p, left: i * frameWidth, top: 0 }));
  await sheet.composite(composites).png({ compressionLevel: 9 }).toFile(`assets/sprites/${char}.png`);

  writeFileSync(`assets/sprites/${char}.json`, JSON.stringify({
    frameWidth, frameHeight, fps: FPS, frameCount: allFrames.length, states: stateRanges
  }, null, 2));
}
```

**License note for agent:** Before shipping, verify the Mac repo's LICENSE covers the sprite assets. If `LICENSE` is MIT but the artwork is not explicitly covered, open an issue on `ryanstephen/lil-agents` asking for confirmation, or generate replacement pixel art. Do not ship without resolution. Log the decision in `DECISIONS.md`.

### 10.3 Sound effects

Three bundled `.ogg` files: `task-complete.ogg`, `error.ogg`, `startup.ogg`. Keep each under 50 KB. Use royalty-free sources (freesound.org CC0) if not extracting from Mac app.

---

## 11. CI/CD

### 11.1 `.github/workflows/build.yml` (every PR)

```yaml
name: Build
on: [pull_request]
jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: pnpm }
      - uses: pnpm/action-setup@v4
        with: { version: 9 }
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with: { workspaces: "./src-tauri -> target" }
      - run: pnpm install --frozen-lockfile
      - run: pnpm tsc --noEmit
      - run: pnpm --filter . run lint
      - run: cargo test --manifest-path src-tauri/Cargo.toml
      - run: pnpm tauri build --no-bundle
```

### 11.2 `.github/workflows/release.yml` (on tag `v*`)

```yaml
name: Release
on:
  push:
    tags: ['v*']
jobs:
  release:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - # ... same setup as build.yml ...
      - run: pnpm tauri build
      - uses: softprops/action-gh-release@v2
        with:
          files: |
            src-tauri/target/release/bundle/nsis/*.exe
            src-tauri/target/release/bundle/nsis/latest.json
          generate_release_notes: true
```

**No EV code signing in MVP.** Users will see a SmartScreen warning on first install. Accept this. Put a note in the README with steps to click past the warning. Revisit signing once you hit ~500 downloads or have a sponsor.

---

## 12. Phase Plan (6 Weeks)

### Phase 1 — Foundation (Week 1)
Scaffold project, basic overlay, taskbar detection.

- [ ] Run bootstrap commands from Section 5.
- [ ] Replace `tauri.conf.json` with Section 6 content.
- [ ] Implement `taskbar.rs`: `current()` and `start_polling()`.
- [ ] Implement `overlay.rs`: `configure_overlay_window()` + `position_above_taskbar()`.
- [ ] Implement `main.rs` setup hook wiring the above.
- [ ] Frontend: basic `OverlayWindow` that renders a 200 px colored bar above the taskbar.
- [ ] Gate: running `pnpm tauri dev` shows a transparent window with a colored stripe exactly above the taskbar on 100%, 125%, and 150% DPI screens.

### Phase 2 — Character Animation (Week 2)
Sprite engine + state machine.

- [ ] Write `scripts/convert-sprites.mjs`; run on Mac assets; produce `assets/sprites/{bruce,jazz}.{png,json}`.
- [ ] Implement `Character.tsx` with rAF loop, 4-state machine, delta-time + clamp.
- [ ] Wire to `useTaskbar` hook; characters reposition on `taskbar-changed`.
- [ ] Implement click-through toggle via `set_ignore_cursor_events` (Section 9.2).
- [ ] Gate: characters walk smoothly at 30 FPS, reverse at overlay edges, click-through works (clicks pass through empty space but land on characters).

### Phase 3 — PTY + Terminal (Week 3)
Real AI CLI integration.

- [ ] Implement `terminal.rs` PtyManager with open/close/send_input/resize.
- [ ] Implement `providers.rs` with PROVIDERS + is_available.
- [ ] Implement `Terminal.tsx` with xterm.js 6 + Fit + Webgl + WebLinks addons.
- [ ] Wire all 5 terminal-related IPC commands.
- [ ] Implement AI activity detection (800 ms silence threshold).
- [ ] Gate: clicking Bruce opens a terminal, Claude Code runs inside it, keystrokes work, resize works, close cleans up process (verified via Task Manager).

### Phase 4 — Polish (Week 4)
Tray, themes, thought bubbles, sounds, onboarding, config.

- [ ] Implement `tray.rs` with full menu (Section 7.6).
- [ ] Implement `config.rs` with atomic writes.
- [ ] Implement all 4 themes as CSS variables.
- [ ] Implement `ThoughtBubble.tsx` + `phrases.rs`.
- [ ] Implement sound effects via Howler.
- [ ] Implement `Onboarding.tsx` with CLI detection UI.
- [ ] Implement `fullscreen.rs` + hide overlay when active.
- [ ] Implement global hotkey (Ctrl+Shift+L).
- [ ] Gate: every feature in the tray menu works. All 4 themes switch live. Thought bubbles cycle. Task-complete sound plays. Onboarding runs exactly once.

### Phase 5 — Release (Week 5)
Build, package, ship.

- [ ] Finalize NSIS installer config (`perUser`, no admin needed).
- [ ] Set up GitHub Actions: build + release workflows from Section 11.
- [ ] Write README with: demo GIF (record with ScreenToGif), install instructions, SmartScreen bypass note, supported CLIs.
- [ ] Write LICENSE (MIT) and CREDITS.md (acknowledging Ryan Stephen's original).
- [ ] Cut `v0.1.0` tag → first public release.
- [ ] Post to r/rust, r/Windows10, Hacker News, your own Twitter/LinkedIn.

### Phase 6 — Iteration (Week 6)
Based on feedback from first ~50 users.

- [ ] Triage issues on GitHub.
- [ ] Ship v0.1.1 with the top 3 bug fixes.
- [ ] Enable auto-updater (generate Tauri signing keypair, flip `plugins.updater.active` to `true`).

**What's deliberately cut from the v1.0 plan:**
- EV code signing certificate
- Separate thought bubble window
- Rust 16 ms click-through poll
- WebView2 runtime bundling complexity (the `embedBootstrapper` mode already handles it)
- Windows 10 regression matrix (just document "works on Win11, best-effort on Win10 21H2+")

---

## 13. Acceptance Tests Per Phase

Every phase ends with a manual smoke test run. Agent documents results in `docs/TEST-LOG.md`.

### Phase 1 gate
1. `cargo tauri dev` launches without errors.
2. Transparent overlay window appears above the taskbar with no visible chrome.
3. Console log prints correct `TaskbarEdge` when user moves taskbar to top/left/right and back.
4. On a 125% DPI display, `dpi_scale` reads `1.25`.

### Phase 2 gate
1. Bruce and Jazz sprites render and walk smoothly at 30 FPS (verify in Chrome DevTools Performance tab — `frames` line should be steady 33 ms).
2. Both characters reverse direction at overlay edges without snapping.
3. Click lands on the character sprite (opens a `console.log`); click on empty space passes through to the desktop (e.g., selects an icon).
4. Moving the taskbar to the top edge repositions both characters within 1 second.

### Phase 3 gate
1. Clicking Bruce opens terminal window; `claude` process visible in Task Manager.
2. Typing in the terminal shows in Claude Code; Claude's responses stream back.
3. Resizing the terminal window reflows the terminal without artifacts.
4. Closing the terminal window kills the `claude` child within 500 ms (verify in Task Manager).
5. Opening Bruce's terminal and Jazz's terminal simultaneously spawns two independent processes.
6. 50 consecutive open/close cycles leak zero handles (verify with `handle.exe` or Task Manager handle count).

### Phase 4 gate
1. Tray menu shows all items; every radio group enforces single selection; checkbox for autostart works.
2. Switching theme applies immediately to any open terminal without reload.
3. Typing a long-running command in Claude → character enters `think` state within 200 ms, thought bubble appears, phrase rotates every ~4 s, sound plays on completion.
4. First launch shows onboarding; second launch does not.
5. Launching a full-screen game hides the overlay; exiting the game shows it again.
6. `Ctrl+Shift+L` toggles overlay visibility from anywhere.

### Phase 5 gate
1. NSIS installer runs without admin prompt on a fresh Win11 VM.
2. App launches, creates `%APPDATA%\LilAgents\config.json` on first run.
3. Uninstaller removes all installed files; `%APPDATA%\LilAgents` is preserved (user data policy).
4. Installer total size on disk under 25 MB.
5. Idle RAM under 50 MB after 5 min run (verify in Task Manager — check both Tauri process and WebView2 child).

---

## 14. Performance Targets (Verified Means of Measurement)

| Metric | Target | Measurement |
|---|---|---|
| Installed size | < 25 MB | File size of `Lil Agents_0.1.0_x64-setup.exe` |
| Idle RAM | < 50 MB | Sum of `lil-agents.exe` + all `msedgewebview2.exe` children in Task Manager Details tab |
| Idle CPU | < 1% | Task Manager averaged over 30 s with overlay visible |
| Startup time | < 800 ms | Stopwatch from double-click to first frame of characters walking |
| Terminal input latency | < 20 ms | Type `a`, measure delay until glyph renders (use 120 Hz monitor + phone slow-mo if needed) |
| Animation framerate | 30 FPS locked | Chrome DevTools Performance tab on a Canvas sample |
| Taskbar reaction | < 1500 ms | Stopwatch from moving taskbar to characters repositioning |

---

## 15. Risk Register (Updated)

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Tauri v2 transparency bugs on specific Intel/AMD/NVIDIA driver combos | Medium | High | Test on CI with windows-latest runner; if flicker reproduces, add `decorations: false` + solid-color fallback with rounded-corner SVG mask |
| `set_ignore_cursor_events` latency too high for responsive hover | Low | Medium | Measure in Phase 2. If >50 ms, fall back to Win32 `SetWindowLongPtrW` toggle (still avoid the poll loop) |
| WebView2 missing on old Windows 10 | Low | Medium | `webviewInstallMode: embedBootstrapper` ships a small installer inside the NSIS package |
| CLI binary not in PATH | **High** | Low (with mitigation) | `providers.rs::is_available()` + onboarding shows install hints; terminal window shows friendly error if CLI missing |
| SmartScreen blocks unsigned installer | **High** | Medium | Accept for MVP; README has bypass instructions; sign later once reputation/budget allows |
| Taskbar auto-hide invisible characters | Medium | Medium | `taskbar.rs::auto_hide` flag → frontend positions at `monitor_rect.bottom - character_height - 4` in auto-hide mode |
| PTY resource leak on abnormal CLI exit | Low | Medium | `PtyManager` wraps sessions in `Arc<Mutex<>>`; Drop impl ensures `child.kill()` + master drop; 30 s watchdog task |
| Asset licensing unclear | **Medium** | **High** | Before Phase 5: verify Mac repo's LICENSE covers sprite art. If not, replace with original pixel art (budget: 2 days, or $50 on Fiverr) |
| Sprite animation stutter on low-end hardware | Low | Low | Delta-time with max-clamp already handles this; 30 FPS is trivial |

---

## 16. What the Agent Must NOT Do

Explicit anti-scope to prevent agent drift:

1. **Do not** implement a second transparent Tauri window for thought bubbles. Render inline inside `Overlay.tsx`.
2. **Do not** implement a manual Rust click-through poll loop. Use `set_ignore_cursor_events`.
3. **Do not** add analytics, telemetry, or any outbound HTTP except the Tauri updater endpoint.
4. **Do not** bundle AI CLIs. Detect them, don't install them.
5. **Do not** add account/login flows.
6. **Do not** pursue EV code signing in MVP.
7. **Do not** support Linux or macOS. This is Windows-only.
8. **Do not** invent features not in this document. If tempted, add to `docs/FUTURE.md` instead of implementing.
9. **Do not** use the deprecated `xterm` npm package — use `@xterm/xterm@6`.
10. **Do not** use Tauri v1 tutorials, StackOverflow answers, or LLM training data hints that predate October 2024. The v1→v2 API migration was significant.

---

## 17. Branding & Attribution

**Name:** `Lil Agents for Windows` (acceptable under MIT license). Alternative: `Pocket Agents` or `Dock Pets` if Anuj wants separation from upstream.

**README credits section (required):**

```markdown
## Credits

This is an unofficial Windows port of [Lil Agents](https://github.com/ryanstephen/lil-agents)
by Ryan Stephen. The original macOS application is released under the MIT license.
All character design and artwork originates from Ryan's project. This port
reimplements the feature set using Tauri v2, Rust, and React for the Windows
platform. Huge thanks to Ryan for the original vision.
```

---

## 18. Final Agent Execution Checklist

Before starting, agent confirms:

- [ ] I have read Sections 0 through 17 in full.
- [ ] I understand Section 16 (what not to do).
- [ ] I will use the exact pinned versions from Section 3.
- [ ] I will not proceed from Phase N to Phase N+1 until Section 13 gate is met.
- [ ] I will log every deviation in `docs/DECISIONS.md` with rationale.
- [ ] I will write `docs/TEST-LOG.md` entries after each phase gate.
- [ ] I will NOT add scope beyond what is specified.

When ready, begin Phase 1. Good luck. Make Bruce and Jazz feel at home on Windows.

---

*End of build plan. Length: ~11,000 words. Target delivery: working v0.1.0 on GitHub Releases within 6 weeks of Phase 1 start.*
