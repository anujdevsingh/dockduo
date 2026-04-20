# Credits

DockDuo exists because other people did the hard parts first. This page is the
full list of who owns what.

## Upstream project

- **lil-agents** by **Ryan Stephen** — the original macOS dock companion app.
  <https://github.com/ryanstephen/lil-agents>
- DockDuo is a ground-up Windows port. It reuses the upstream's character
  artwork and idea, re-implementing the runtime in Rust + Tauri + React instead
  of Swift. Both the upstream repo and this repo are MIT-licensed.

### Assets inherited from upstream (MIT)

| Asset | Location in this repo |
| --- | --- |
| Bruce sprite sheets (walk / think / celebrate) | [src-tauri/sprites/bruce/](src-tauri/sprites/bruce/) |
| Jazz sprite sheets (walk / think / celebrate) | [src-tauri/sprites/jazz/](src-tauri/sprites/jazz/) |
| Task-complete sound effect | [public/sfx/](public/sfx/) |

No artwork was created or modified; frames are used as-is. If Ryan asks for a
specific attribution line or wants it removed, open an issue and it will be
addressed within a week.

## Where DockDuo diverges from upstream

DockDuo is not a line-by-line port. The decisions log at
[docs/DECISIONS.md](docs/DECISIONS.md) explains every intentional departure
from the macOS original and the Windows build plan. The two most visible to
users:

- **Terminal hosting (D-003).** macOS `lil-agents` runs its AI CLI inside an
  in-app PTY terminal. DockDuo instead spawns the CLI in a detached
  `cmd.exe` / Windows Terminal window. Rationale: Windows Terminal on Win11 is
  already a first-class terminal host, and skipping the in-app PTY keeps the
  bundle under 25 MB.
- **Branding (D-001).** The Windows port ships as "DockDuo" so the two
  projects can evolve independently without confusing users about which
  platform a bug report belongs to.

## Tauri plugins

Pinned in [src-tauri/Cargo.toml](src-tauri/Cargo.toml):

| Plugin | Purpose |
| --- | --- |
| `tauri-plugin-single-instance` | Prevent a second DockDuo process from booting when the user double-clicks the shortcut twice |
| `tauri-plugin-log` | File + stdout logs under `%APPDATA%\DockDuo\logs\` |
| `tauri-plugin-autostart` | "Start with Windows" checkbox in the tray |
| `tauri-plugin-updater` | Plumbed; disabled until v0.1.1 ships a signing keypair |
| `tauri-plugin-global-shortcut` | `Ctrl+Shift+L` overlay toggle |

## Runtime dependencies worth naming

- **Tauri 2** — the desktop-shell framework. <https://tauri.app>
- **WebView2** — Microsoft's Chromium-based webview, shipped with the installer
  via `embedBootstrapper` for Win10 users who don't already have it.
- **React 19 + TypeScript 5.8** — the UI layer.
- **Vite 7** — dev server and production bundler.

## License chain

```
ryanstephen/lil-agents (MIT, 2026) ──┐
                                     ├──> DockDuo (MIT, 2026, Anuj Dev Singh)
Tauri, React, WebView2 (MIT)  ───────┘
```

Every dependency above is MIT or Apache-2.0-compatible. No GPL code is linked
in. If you spot a misattribution, file an issue; it will be fixed, not argued.
