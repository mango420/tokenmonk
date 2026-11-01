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

## Limit source

- v0.1 reads subscription limits through Claude Code's status line (maintainer decision): the Tokenmonk executable has a bridge mode (`--statusline`) that keeps only `rate_limits` and writes it to `<app data dir>/statusline.json`; the app polls that file. Local only: no network, no credentials, no undocumented endpoint. Tokenmonk never edits Claude Code settings; the user pastes a snippet. Details: ARCHITECTURE.md, "Limits".
- Verified (official docs, checked 2026-10-30): `rate_limits.five_hour` and `rate_limits.seven_day`, each with `used_percentage` (0 to 100) and `resets_at` (Unix epoch seconds). Present only for claude.ai Pro and Max subscribers (or behind a Claude apps gateway with spend limits, ignored here) and only after the first API response of a session; each window may be absent on its own and is dropped once `resets_at` passes. Configured as `statusLine: { "type": "command", "command": ... }` in `~/.claude/settings.json` (optional `padding`, `refreshInterval`). Runs at session start, after each assistant message and a few other events, debounced at 300 ms; in-flight runs are cancelled; output shows only after exit code 0. On Windows the command runs through Git Bash when installed, else PowerShell; paths need forward slashes. Runs only in trusted workspaces.
- `rate_limits.seven_day` maps to the contract's `weekly`; `used_percentage` to `utilization`; `resets_at` x 1000 to `resetsAt`.
- Contract changes: new command `get_limit_setup` returning `LimitSetup { statuslineCommand }` (the UI cannot learn the executable path otherwise). No new `LimitReason`: `noSource` now means "not set up" (bridge file missing or no current window), which keeps existing exhaustive UI maps valid.
- Expired windows (`resetsAt <= now`) are dropped both by the file source and by the aggregate on the stored last good reading; no window left means `unavailable` / `noSource`.
- Windows paths with a space: `statuslineCommand` stays the Git Bash form; the UI adds a PowerShell alternative `& <command>` (no contract change).
- `refreshInterval` is not recommended: a timer re-run resends old values and would make them look fresh.
- Chaining an existing status line: documented as one `printf ... | <command> >/dev/null` line for Bash scripts; a built-in chain flag is out of scope for v0.1.
- `limit_snapshots` stores a reading only when its `fetchedAt` is strictly newer than the newest stored row (one `INSERT ... SELECT ... WHERE` statement). An equal or older `fetchedAt` is a repeat poll of the same bridge file and adds no row, also when a window was dropped since (#36).
- The limits view (`LimitState::view`, used for every snapshot) drops each window of the last good reading whose `resetsAt` is at or before now; no window left means `unavailable` / `noSource`. Stored readings and `limit_snapshots` rows stay untouched (#35).
- Tauri `app_data_dir()` resolves to `data_dir/<bundle identifier>`, so the bridge (which runs without Tauri) uses `dirs::data_dir()` plus `app.tokenmonk.desktop`.
- Sources: <https://code.claude.com/docs/en/statusline>, <https://code.claude.com/docs/en/settings>, <https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html>.

## CI

- `.github/workflows/ci.yml` mirrors the Definition of Done on ubuntu and windows, plus a separate `guard` job with full history.
- Actions pinned to released majors: `actions/checkout@v7`, `actions/setup-node@v7`, `Swatinem/rust-cache@v2`, `dtolnay/rust-toolchain@stable`.
- `npm run build` runs before cargo because Tauri's `generate_context!` needs `frontendDist`.

## Release

Superseded for v0.1: see "v0.1 without updater artifacts" below.

- `.github/workflows/release.yml` runs on pushed tags `v*` (and `workflow_dispatch`). A `guard` job runs `scripts/guard.mjs --all`, checks that both signing secrets are set and non-empty (the key is always password-protected), that `plugins.updater.pubkey` is non-empty and `bundle.createUpdaterArtifacts` is `true`, and (on tags) that the tag equals `v<version>` in `tauri.conf.json`, `Cargo.toml` and `package.json`; the build jobs only start after it passes. Until the release-phase config changes below are on `main`, the guard fails on purpose.
- Builds: macOS universal (`--target universal-apple-darwin`, both Rust targets installed) and Windows, via `tauri-apps/tauri-action@v1` (also `actions/checkout@v7`, `actions/setup-node@v7`, `Swatinem/rust-cache@v2`, `dtolnay/rust-toolchain@stable`). macOS builds exist only here, never in PR CI.
- Binaries are unsigned: no Apple or Windows code-signing secrets. Only the updater artifacts are signed. The action creates a draft release `Tokenmonk v<version>` and uploads installers, `.sig` files and `latest.json` (`uploadUpdaterJson: true`, `updaterJsonPreferNsis: true`).
- Secrets (repository Actions secrets, never in the repo): `TAURI_SIGNING_PRIVATE_KEY` (key content) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. `GITHUB_TOKEN` is the built-in one; the workflow is `contents: read` by default and only the build job gets `contents: write`.

### Cutting a release

Superseded for v0.1: see "v0.1 without updater artifacts" below.

1. One time: `npm run tauri signer generate -- -w <path outside the repo>`; store the private key and password as the two secrets; keep the public key for the config change below.
2. Bump the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json` (chore PR), merge to `main`.
3. Tag the merge commit `v<version>` and push the tag. Never trigger the workflow before the config changes below are on `main`.
4. Review the draft release (assets for macOS universal and Windows, `latest.json`), then publish it. The endpoint `releases/latest/download/latest.json` resolves only for a published release.

### `tauri.conf.json` changes applied in the release phase

- `bundle.createUpdaterArtifacts`: `true` (without it no `.sig` files or updater bundles are produced).
- `plugins.updater.pubkey`: the public key content from the generated `.pub` file (content, not a path).
- `bundle.macOS.signingIdentity`: `"-"` (ad-hoc signing; Tauri docs recommend it for macOS builds without an Apple certificate, otherwise Apple Silicon builds can be reported as damaged). Users still approve the app once in Privacy & Security.
- Endpoint stays `https://github.com/mango420/tokenmonk/releases/latest/download/latest.json` (already set).

## v0.1 without updater artifacts (2026-10-31)

- v0.1 ships without auto-update. The repository stays private, binaries are distributed by hand, and the in-app update check (#41) is deferred. No updater signing key exists and none is created for now.
- `release.yml` has one switch, the workflow-level `UPDATER_ARTIFACTS` (currently `'false'`). While off, the guard skips the signing-secret check and requires `bundle.createUpdaterArtifacts` to be unset or false; the build uploads no `latest.json` (`uploadUpdaterJson: false`) and no `.sig` files (`uploadUpdaterSignatures: false`). The tag and version consistency check and `guard.mjs --all` always run. The empty `plugins.updater.pubkey` stays and does no harm.
- `bundle.macOS.signingIdentity` is `"-"` (ad-hoc signing, confirmed by the Tauri 2 macOS signing docs); users still approve the app once in Privacy & Security.
- Switching updater artifacts back on: follow "Cutting a release" step 1 and "`tauri.conf.json` changes applied in the release phase" above (except the signing identity, already set), then set `UPDATER_ARTIFACTS: 'true'` in `release.yml`. The guard then requires both secrets, a non-empty pubkey and `createUpdaterArtifacts: true`.

## Issue keys and feature register (2026-11-01)

- Every issue has the key `TKM-<n>`, where `<n>` is its GitHub issue number. Issues and PRs share one number sequence, so keys have gaps where a PR took the number; that is accepted.
- Issue title: `TKM-<n>: <short>`. Each issue carries exactly one type label: `feature`, `bug` or `chore`.
- Branch: `<prefix>/TKM-<n>-<short-kebab>` with the usual prefixes (feature/ bugfix/ hotfix/ refactor/ chore/ docs/ test/ ci/).
- PR title and squash subject: `<type>(<scope>): <short> (TKM-<n>)`; the PR body still ends in `Closes #<n>`. The PR key check in CI (`scripts/check-pr-key.mjs`) requires the `(TKM-<n>)` title suffix and the branch format, with the same key in both; dependabot and release branches are skipped.
- `docs/FEATURES.md` is the feature register, generated from the issue tracker by `node scripts/features.mjs` (needs an authenticated `gh`). It is never edited by hand; regenerate and commit it when issues change.

## Checks on the register refresh PR (2026-11-01)

- `feature-register.yml` creates the refresh PR with the built-in `GITHUB_TOKEN`. Events raised by `GITHUB_TOKEN` start no workflow runs, so `ci.yml` and `pr-key.yml` would never run on that PR.
- Exception per GitHub docs: `workflow_dispatch` events raised by `GITHUB_TOKEN` do create runs. After pushing the refresh branch, the workflow dispatches both checks with `gh workflow run ci.yml --ref <branch>` and `gh workflow run pr-key.yml --ref <branch>`; this needs `actions: write` on the workflow.
- Chosen because it needs no secret, no GitHub App and no personal access token.
- Fallback if the dispatched runs do not attach to the PR as checks: a fine-grained personal access token, created by the maintainer and stored as a repository secret, used to create the refresh PR.
