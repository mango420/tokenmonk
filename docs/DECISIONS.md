# Decisions

Short log of decisions that are not obvious from the code. Newest entries at the bottom of each section.

## Scaffold

- Created with `create-tauri-app` (template `react-ts`, npm), then stripped of demo content. Tauri 2, React 19, TypeScript, Vite.
- Product name and window title: `Tokenmonk`. Bundle identifier: `app.tokenmonk.desktop`.
- The only window is the tray popup: label `main`, 360 x 680, not resizable, no decorations, hidden at start, skipped in the taskbar. Tray and positioning logic come with later issues.
- Icons are generated with `tauri icon` from the brand mark SVG. Only the desktop sets are kept (no Android/iOS).
- Desktop only. No macOS build in PR CI; macOS is built only in the release workflow on tags.

## v0.1 dependencies

Rust (`src-tauri/Cargo.toml`):

| Crate | Why |
|---|---|
| `tauri` 2 (`tray-icon`, `image-png`) | App shell; tray icon API and `Image` decoding for runtime tray icons |
| `tauri-build` | Build script required by Tauri |
| `serde` (derive), `serde_json` | Contracts and IPC payloads, reading Claude Code JSONL lines |
| `rusqlite` (`bundled`) | Local SQLite store; bundled so no system SQLite is needed on any OS |
| `notify` | Watch the Claude Code project log directory for new lines. No separate debouncer crate for now; one can be added later via a decision here if event bursts need it |
| `chrono` (`serde`) | Timestamps and window math (5 hour and weekly windows) |
| `dirs` | Locate the home directory and the app data directory |
| `reqwest` (`json`, `rustls`, no default features) | Limit lookup against Anthropic; rustls avoids a system OpenSSL dependency |
| `thiserror` | Typed errors in library modules |
| `log` | Logging facade |
| `resvg` | Renders the halo tray icon (SVG to RGBA) at runtime |
| `tempfile` (dev) | Temporary directories in tests |

Tauri plugins (Rust crate and matching npm package):

| Plugin | Why |
|---|---|
| `updater` | In-app updates from GitHub releases |
| `process` | Relaunch after an update |
| `notification` | Limit and milestone notifications |
| `autostart` | Optional start at login |
| `opener` | Open links in the default browser |
| `single-instance` | Only one tray instance at a time (Rust only, no JS API) |
| `positioner` (`tray-icon`) | Place the popup next to the tray icon |

npm: `react`, `react-dom` (UI), `@tauri-apps/api` (IPC), the six plugin packages above (without single-instance), `@fontsource/fraunces`, `@fontsource/manrope`, `@fontsource/jetbrains-mono` (bundled fonts, no network requests at runtime).
Dev: `typescript`, `vite`, `@vitejs/plugin-react`, `@types/react`, `@types/react-dom`, `@tauri-apps/cli`.

`updater`, `autostart` and `single-instance` are desktop-only crates (target-gated in `Cargo.toml`), registered under `#[cfg(desktop)]`.

## Updater

- Endpoint: `https://github.com/mango420/tokenmonk/releases/latest/download/latest.json`.
- `plugins.updater.pubkey` is an empty placeholder in `tauri.conf.json`. The release phase replaces it with the public key; the private key and its password live only in GitHub secrets and outside the repo.
- With the empty placeholder the app starts normally (the plugin only parses the config at startup). Update checks cannot verify a signature, so no update can be installed until the real key is set. `bundle.createUpdaterArtifacts` is also left for the release phase.

## Capabilities

`src-tauri/capabilities/default.json` grants the `main` window only what the registered plugins need: `core:default`, `opener:default`, `notification:default`, `process:allow-restart`, `updater:default`, `autostart:allow-enable|disable|is-enabled`, `positioner:default`.

CSP is strict: `default-src 'self'`, images and fonts from `self` and `data:`, inline styles allowed, IPC via `ipc:` and `http://ipc.localhost` only, no remote hosts. `devCsp` additionally allows the Vite dev server and its HMR websocket on `localhost:1420`. Tauri adds script hashes and style nonces at build time.

## CI

- `.github/workflows/ci.yml` mirrors the Definition of Done on ubuntu and windows, plus a separate `guard` job with full history.
- Actions pinned to released majors: `actions/checkout@v7`, `actions/setup-node@v7`, `Swatinem/rust-cache@v2`, `dtolnay/rust-toolchain@stable`.
- `npm run build` runs before cargo because Tauri's `generate_context!` needs `frontendDist`.
