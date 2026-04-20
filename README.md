# DockDuo

Two animated pixel-art characters — Bruce and Jazz — that live on your Windows
taskbar and open your AI coding CLI in a click.

<!-- DEMO_GIF: drop demo.gif here (recorded with ScreenToGif, ~5 s loop) -->
<!-- ![DockDuo demo](demo.gif) -->

DockDuo is the Windows port of Ryan Stephen's macOS
[`lil-agents`](https://github.com/ryanstephen/lil-agents). MIT-licensed, no
telemetry, no cloud, no account.

---

## What it does

- Two characters walk back and forth just above your taskbar.
- Click **Bruce** to open **Claude Code** in a new terminal window. Click
  **Jazz** to open **Codex** or **Gemini** (pick in onboarding / tray).
- While the CLI is running, the character shows a thinking bubble with one of
  40+ rotating phrases. When the CLI exits, the character celebrates and a
  soft chime plays.
- `Ctrl+Shift+L` hides or shows the whole overlay.

---

## Install

1. Go to the [Releases page](https://github.com/anujdevsingh/dockduo/releases/latest)
   and download `DockDuo_0.1.0_x64-setup.exe`.
2. Double-click it. No admin prompt — it installs per-user to
   `%LOCALAPPDATA%\Programs\DockDuo\`.
3. Launch DockDuo from the Start menu. The onboarding window walks you through
   picking your AI provider.

### Windows SmartScreen warning

DockDuo is not code-signed for v0.1.0 (EV certificates cost ~$300/yr and the
project is currently free). On first launch you will see:

> **Windows protected your PC**
> Microsoft Defender SmartScreen prevented an unrecognized app from starting.

This is expected. To run anyway:

1. Click **More info** (small text under the message).
2. Click **Run anyway** (button that appears after "More info").

You only need to do this once. Signing is on the roadmap once the project
crosses ~500 downloads or gets a sponsor.

---

## Supported AI CLIs

DockDuo does **not** bundle any CLI. You install whichever you want, DockDuo
detects them at launch.

| Provider | Install command |
| --- | --- |
| Claude Code | `npm install -g @anthropic-ai/claude-code` |
| Codex | `npm install -g @openai/codex` |
| Gemini CLI | `npm install -g @google/generative-ai-cli` |

If a CLI isn't installed yet, the onboarding screen shows you the exact
install command and re-checks after you run it.

---

## First run

1. Onboarding window opens (one time only).
2. Pick a provider for Bruce, pick a provider for Jazz. You can choose the
   same CLI for both if you want.
3. Hit **Get started**. The window closes and the characters start walking.
4. A tray icon appears in your system tray with all the same controls.

---

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+Shift+L` | Toggle the whole overlay on / off |

---

## Tray menu

Right-click the DockDuo tray icon for:

- **Show / Hide DockDuo** — same as `Ctrl+Shift+L`
- **Theme** — Midnight · Daylight · Pastel · Retro (live switch)
- **Start with Windows** — per-user registry entry, no admin needed
- **Hide on fullscreen** — auto-hides when a fullscreen app takes focus
- **Check for updates…** — disabled in v0.1.0; re-enabled in v0.1.1
- **About DockDuo 0.1.0**
- **Quit**

---

## System requirements

- **OS:** Windows 10 (21H2+) or Windows 11, 64-bit
- **WebView2:** auto-installed by the bundled bootstrapper if missing
- **Disk:** ~25 MB installed
- **RAM:** ~50 MB idle

---

## Configuration & logs

DockDuo writes config and logs under your user profile only:

- Config: `%APPDATA%\DockDuo\config.json`
- Logs: `%APPDATA%\DockDuo\logs\DockDuo.log`

Uninstalling removes the install dir. If you want a full wipe, also delete
`%APPDATA%\DockDuo\`.

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

---

## Roadmap (post v0.1.0)

- v0.1.1 — Signing keypair + live updater toggle
- v0.2.0 — Optional in-app PTY terminal with xterm.js (currently deferred;
  see [docs/DECISIONS.md](docs/DECISIONS.md) D-003 / D-009)
- v0.2.0 — Multi-monitor taskbar detection fixes
- Later — More character pairs, sound pack theming

---

## Credits & License

DockDuo is MIT-licensed. Character artwork and sounds are inherited under
MIT from the upstream macOS project. Full attribution lives in
[CREDITS.md](CREDITS.md). License text is in [LICENSE](LICENSE).

Built on [Tauri 2](https://tauri.app) with React 19.

If you find a bug, open an issue. If you want a feature, open an issue. If you
just want to say "this is cute", star the repo.
