# Changelog

All notable changes to DockDuo are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versioning adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] — 2026-04-23

### Added

- **Bubble-only chat:** floating `bubble_bruce` / `bubble_jazz` windows with warm transcript UI (`AgentChat`); Claude, Codex, and Gemini all supported via dedicated pipes.
- **Agent picker** layout fixes and **thinking** dots while the model responds; session persists when the bubble is closed.
- **README** hero image (`readme-hero.png`) and refreshed documentation for the current UX.

### Changed

- **Removed** in-app raw PTY / xterm terminal and tray “Terminal mode” submenu; interaction is sprite → bubble → CLI subprocess.
- **Security:** Windows `.cmd` shims spawned without unsafe `cmd.exe /w` wrappers; `--` argv separators; pinned `where.exe`; per-character sandboxes under `%APPDATA%\DockDuo\agents\`; `character` IPC allow-list (`bruce` / `jazz`); prompt size / NUL sanitisation.

### Removed

- `docs/project-graph/*` from version control (regenerable; see `.gitignore`).

[0.2.1]: https://github.com/anujdevsingh/dockduo/releases/tag/v0.2.1

## [0.2.0] — 2026-04-20

### Added

- **Embedded terminal (optional):** `portable-pty` + `@xterm/xterm` with Fit and WebGL addons in dedicated `terminal_bruce` / `terminal_jazz` windows; theme-aligned ANSI colors via `xtermThemeFor()` in `src/lib/themes.ts`.
- **Config:** `use_embedded_terminal` (default `false`); tray submenu **Terminal → Embedded terminal (xterm)** vs **System terminal (cmd.exe)**.
- **Smarter busy/idle:** PTY path emits `busy` on output activity and returns to `idle` after ~800 ms without output; `completed` when the session or window ends (aligned with the overlay bubble and chime).
- **Multi-monitor (deferred):** shipped primary-taskbar only; secondary overlays were removed due to phantom hits on single-monitor setups — tracked as D-011 and targeted for a later release.
- **Updates:** Tauri updater enabled with signing pubkey in `tauri.conf.json`; **Check for updates…** in the tray (requires `TAURI_SIGNING_PRIVATE_KEY` and password in CI secrets for signed releases).

### Changed

- Detached `cmd.exe /K` spawn path remains the default and is unchanged when `use_embedded_terminal` is off (see `docs/DECISIONS.md` D-003).

### Notes for maintainers

- Generate or supply updater keys locally; never commit the private key. See [Tauri updater](https://v2.tauri.app/plugin/updater/) documentation.

[0.2.0]: https://github.com/anujdevsingh/dockduo/releases/tag/v0.2.0
