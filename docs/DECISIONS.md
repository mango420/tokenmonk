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

## Release

- `.github/workflows/release.yml` runs on pushed tags `v*` (and `workflow_dispatch`). A `guard` job runs `scripts/guard.mjs --all`, checks that both signing secrets are set and that the tag equals `v<tauri.conf.json version>`; the build jobs only start after it passes.
- Builds: macOS universal (`--target universal-apple-darwin`, both Rust targets installed) and Windows, via `tauri-apps/tauri-action@v1` (also `actions/checkout@v7`, `actions/setup-node@v7`, `Swatinem/rust-cache@v2`, `dtolnay/rust-toolchain@stable`). macOS builds exist only here, never in PR CI.
- Binaries are unsigned: no Apple or Windows code-signing secrets. Only the updater artifacts are signed. The action creates a draft release `Tokenmonk v<version>` and uploads installers, `.sig` files and `latest.json` (`uploadUpdaterJson: true`, `updaterJsonPreferNsis: true`).
- Secrets (repository Actions secrets, never in the repo): `TAURI_SIGNING_PRIVATE_KEY` (key content) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. `GITHUB_TOKEN` is the built-in one (`permissions: contents: write`).

### Cutting a release

1. One time: `npm run tauri signer generate -- -w <path outside the repo>`; store the private key and password as the two secrets; keep the public key for the config change below.
2. Bump the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json` (chore PR), merge to `main`.
3. Tag the merge commit `v<version>` and push the tag. Never trigger the workflow before the config changes below are on `main`.
4. Review the draft release (assets for macOS universal and Windows, `latest.json`), then publish it. The endpoint `releases/latest/download/latest.json` resolves only for a published release.

### `tauri.conf.json` changes applied in the release phase

- `bundle.createUpdaterArtifacts`: `true` (without it no `.sig` files or updater bundles are produced).
- `plugins.updater.pubkey`: the public key content from the generated `.pub` file (content, not a path).
- `bundle.macOS.signingIdentity`: `"-"` (ad-hoc signing; Tauri docs recommend it for macOS builds without an Apple certificate, otherwise Apple Silicon builds can be reported as damaged). Users still approve the app once in Privacy & Security.
- Endpoint stays `https://github.com/mango420/tokenmonk/releases/latest/download/latest.json` (already set).
