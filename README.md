# DockDuo

[![Release](https://img.shields.io/github/v/release/anujdevsingh/dockduo?color=7c3aed&label=release)](https://github.com/anujdevsingh/dockduo/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-4a5568)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11%20x64-0078d4)](https://github.com/anujdevsingh/dockduo/releases/latest)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24C8DB)](https://tauri.app)

**Two animated pixel-art characters — Bruce and Jazz — that live on your Windows taskbar and open your AI coding CLI in a single click.**

<!-- DEMO_GIF: drop demo.gif here (recorded with ScreenToGif, ~5 s loop) -->
<!-- ![DockDuo demo](demo.gif) -->

DockDuo is the Windows port of Ryan Stephen's macOS
[`lil-agents`](https://github.com/ryanstephen/lil-agents). MIT-licensed, no
telemetry, no cloud, no account, no phoning home. About 25 MB on disk,
~50 MB RAM idle.

---

## Quick install

Download **[`DockDuo_0.1.0_x64-setup.exe`](https://github.com/anujdevsingh/dockduo/releases/latest)** → double-click → accept the SmartScreen prompt once → done.

No admin required. Installs per-user to `%LOCALAPPDATA%\Programs\DockDuo\`.

---

## What it does

- Two sprite characters walk back and forth just above your taskbar.
- **Click Bruce** → opens **Claude Code** in a new terminal window.
- **Click Jazz** → opens **Codex** or **Gemini** (your pick during onboarding
  or in the tray menu).
- While the CLI is running, the character shows a thinking bubble cycling
  through **44 rotating phrases** ("hmm...", "cooking...", "triangulating",
  "consulting the oracle", …).
- When the CLI exits, the character **celebrates** with one of 14 completion
  phrases ("done!", "ship it", "ta-da!") and a soft chime plays.
- **`Ctrl+Shift+L`** hides or shows the whole overlay instantly.

---

## Supported AI CLIs

DockDuo does **not** bundle any CLI. You install whichever you want; DockDuo
auto-detects them at launch and re-checks when you open the agent picker.

| Provider    | Binary   | Install command                             |
| ----------- | -------- | ------------------------------------------- |
| Claude Code | `claude` | `npm install -g @anthropic-ai/claude-code`  |
| Codex       | `codex`  | `npm install -g @openai/codex`              |
| Gemini CLI  | `gemini` | `npm install -g @google/generative-ai-cli`  |

If a CLI isn't installed yet, the onboarding screen shows you the exact
install command and re-checks after you've run it.

> DockDuo never reads your CLI's prompts, its output, or its auth tokens. It
> spawns the binary in a new, fully detached `cmd.exe` window and watches
> only the OS process handle to know when you're done.

---

## Install in detail

1. Go to the
   [Releases page](https://github.com/anujdevsingh/dockduo/releases/latest)
   and download `DockDuo_0.1.0_x64-setup.exe`.
2. Double-click it. No admin prompt — it installs per-user to
   `%LOCALAPPDATA%\Programs\DockDuo\`.
3. Launch DockDuo from the Start menu. The onboarding window walks you
   through picking your AI provider.

### Windows SmartScreen warning

DockDuo is not code-signed for v0.1.0 (EV certificates cost ~$300/yr and the
project is currently free). On first launch you will see:

> **Windows protected your PC**
> Microsoft Defender SmartScreen prevented an unrecognized app from starting.

This is expected. To run anyway:

1. Click **More info** (small text under the message).
2. Click **Run anyway** (button that appears after "More info").

You only need to do this once. Signing is on the roadmap once the project
crosses ~500 downloads or picks up a sponsor.

---

## First run

1. Onboarding window opens (one time only).
2. Pick a provider for Bruce, pick a provider for Jazz. You can choose the
   same CLI for both if you want.
3. Hit **Get started**. The window closes and the characters start walking.
4. A tray icon appears in your system tray with all the same controls.

---

## Keyboard shortcuts

| Shortcut        | Action                            |
| --------------- | --------------------------------- |
| `Ctrl+Shift+L`  | Toggle the whole overlay on / off |

---

## Tray menu

Right-click the DockDuo tray icon for:

- **Show / Hide DockDuo** — same as `Ctrl+Shift+L`
- **Theme** — Midnight · Daylight · Pastel · Retro (live switch, no restart)
- **Start with Windows** — per-user registry entry, no admin needed
- **Hide on fullscreen** — auto-hides when a fullscreen app takes focus
  (games, videos)
- **Check for updates…** — disabled in v0.1.0; re-enabled in v0.1.1 once the
  updater signing keypair ships
- **About DockDuo 0.1.0**
- **Quit**

---

## System requirements

- **OS:** Windows 10 (21H2+) or Windows 11, 64-bit
- **WebView2:** auto-installed by the bundled bootstrapper if missing
- **Disk:** ~25 MB installed
- **RAM:** ~50 MB idle
- **Display:** any; multi-monitor is supported but primary-taskbar only in
  v0.1.0

---

## Privacy & data

- **No telemetry.** DockDuo does not send analytics, crash reports, or usage
  stats anywhere — there is no backend.
- **No cloud, no account, no sign-in.** Everything is local.
- **Only outgoing connection:** the updater check against the GitHub
  Releases page. This is currently **disabled** and ships off in v0.1.0.
- **Your CLI's data stays yours.** DockDuo launches the CLI as a detached
  child process in a new terminal window — it cannot read the stdio of that
  process.
- Config and logs stay in `%APPDATA%\DockDuo\`; nothing leaves your machine.

---

## Configuration & logs

DockDuo writes config and logs under your user profile only:

- **Config:** `%APPDATA%\DockDuo\config.json`
- **Logs:** `%APPDATA%\DockDuo\logs\DockDuo.log`

Uninstalling removes the install dir. If you want a full wipe, also delete
`%APPDATA%\DockDuo\`.

---

## Uninstall

1. **Settings → Apps → Installed apps → DockDuo → Uninstall**, or
2. run `%LOCALAPPDATA%\Programs\DockDuo\uninstall.exe`.
3. (Optional) Delete `%APPDATA%\DockDuo\` to wipe config + logs.

---

## Build from source

```powershell
# Prereqs: Node 20+, pnpm 9+, Rust stable, WebView2 SDK
git clone https://github.com/anujdevsingh/dockduo.git
cd dockduo
pnpm install
pnpm tauri dev          # runs in dev mode with hot reload
pnpm tauri build        # produces the NSIS installer
```

The installer lands in `src-tauri/target/release/bundle/nsis/`.

### Project layout

```
src/                  React 19 / TypeScript frontend (overlay + onboarding)
src-tauri/src/        Rust backend (windows, tray, CLI spawning, config)
src-tauri/icons/      App icons (all sizes)
public/sprites/       Sprite sheets for Bruce and Jazz (150 frames each)
public/sounds/        Completion chimes
scripts/              Sprite conversion helpers (FFmpeg + sharp)
docs/DockDuo-BuildPlan.md    Canonical build plan (phases 1–6)
docs/DECISIONS.md            Architectural divergences from upstream (D-001 … D-009)
.github/workflows/           CI build + tag-driven release pipelines
```

---

## FAQ

**Does it work offline?**
Yes. Overlay, animations, and CLI spawning are 100% local. The CLI you
launch (Claude / Codex / Gemini) needs its own internet; that's on it, not
on DockDuo.

**Can I use a CLI that isn't in the list?**
Not in v0.1.0 — the three providers are hardcoded in `claude.rs`. Custom
providers are on the roadmap (v0.3.0).

**Why two characters instead of one?**
It matches the upstream macOS app — one slot per "primary" agent so you can
split workflows (e.g. Bruce for long-running Claude Code, Jazz for quick
Codex runs). You can always point both at the same CLI if you prefer.

**Why a new `cmd.exe` window instead of an embedded terminal?**
See [`docs/DECISIONS.md`](docs/DECISIONS.md) D-003. Short version: Windows
Terminal is already excellent for the job, bundling a PTY + xterm.js added
~4 MB and a fragile IPC layer, and users liked having a real terminal they
already knew. An embedded PTY remains an optional target for v0.2.0
(D-009).

**Will DockDuo read or log my prompts / CLI output?**
No. DockDuo spawns the CLI as a detached child process in its own console —
it has no handle on the child's stdio. It only knows when the process
exits.

**Why is "Check for updates…" disabled?**
v0.1.0 ships without an updater signing keypair. Once the release signing
flow is set up (v0.1.1), the tray button goes live and the app will check
GitHub Releases on demand.

**Why SmartScreen yells at me?**
Because the installer isn't code-signed yet. See
[Windows SmartScreen warning](#windows-smartscreen-warning) above.

---

## Roadmap

- **v0.1.1** — Updater keypair + live "Check for updates" button
- **v0.2.0** — Optional in-app PTY terminal via xterm.js (see D-003 / D-009)
- **v0.2.0** — Multi-monitor taskbar detection fixes
- **v0.3.0** — Custom-provider plugin (run any CLI, not just the three built-in)
- **Later** — More character pairs, themeable sound packs, Linux port

---

## Credits & License

DockDuo is MIT-licensed. Character artwork and sounds are inherited under
MIT from the upstream macOS project. Full attribution lives in
[`CREDITS.md`](CREDITS.md); license text is in [`LICENSE`](LICENSE).

Huge thanks to **Ryan Stephen** for the original
[`lil-agents`](https://github.com/ryanstephen/lil-agents) on macOS — Bruce
and Jazz were born there, and they walk on your Windows taskbar today
because Ryan's MIT license made it possible.

Built on [Tauri 2](https://tauri.app) with React 19 + Rust stable.

---

## Contributing / feedback

- **Bug?** Open an
  [issue](https://github.com/anujdevsingh/dockduo/issues).
- **Feature request?** Open an issue or a PR. Small focused PRs welcome.
- **Just want to say "this is cute"?** Star the repo.
