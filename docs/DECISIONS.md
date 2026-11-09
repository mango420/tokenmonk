# Decisions

Short log of decisions that are not obvious from the code. Newest entries at the bottom of each section.

## Scaffold

- Created with `create-tauri-app` (template `react-ts`, npm), then stripped of demo content. Tauri 2, React 19, TypeScript, Vite.
- Product name and window title: `Tokenmonk`. Bundle identifier: `app.tokenmonk.desktop`.
- The only window is the tray popup: label `main`, 360 x 680, not resizable, no decorations, hidden at start, skipped in the taskbar. Tray and positioning logic come with later issues.
- Icons are generated with `tauri icon` from the brand mark SVG. Only the desktop sets are kept (no Android/iOS). Superseded by "App icon" below: the platform icons now come from `node scripts/icons.mjs`.
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
| `reqwest` (`json`, `rustls`, no default features) | Dropped (2026-11-03): planned for a network limit lookup that v0.1 does not have; see "Design refresh". Removed from `Cargo.toml` (TKM-89) |
| `thiserror` | Typed errors in library modules |
| `log` | Logging facade |
| `resvg` | Renders the tray ring icon (SVG to RGBA) at runtime |
| `tempfile` (dev) | Temporary directories in tests |

Tauri plugins (Rust crate and matching npm package):

| Plugin | Why |
|---|---|
| `updater` | Registered for a later in-app update check (TKM-41); nothing calls it yet |
| `process` | Relaunch after an update |
| `notification` | Limit and milestone notifications |
| `autostart` | Optional start at login |
| `opener` | Open links in the default browser |
| `single-instance` | Only one tray instance at a time (Rust only, no JS API) |
| `positioner` (`tray-icon`) | Place the popup next to the tray icon |

npm: `react`, `react-dom` (UI), `@tauri-apps/api` (IPC), the six plugin packages above (without single-instance), `@fontsource/schibsted-grotesk`, `@fontsource/martian-mono` (bundled fonts, no network requests at runtime; they replace `@fontsource/fraunces`, `@fontsource/manrope` and `@fontsource/jetbrains-mono`, see "Design refresh").
Dev: `typescript`, `vite`, `@vitejs/plugin-react`, `@types/react`, `@types/react-dom`, `@tauri-apps/cli`.

`updater`, `autostart` and `single-instance` are desktop-only crates (target-gated in `Cargo.toml`), registered under `#[cfg(desktop)]`.

## Updater

- Endpoint: `https://github.com/mango420/tokenmonk/releases/latest/download/latest.json`.
- `plugins.updater.pubkey` is an empty placeholder in `tauri.conf.json`. The release phase replaces it with the public key; the private key and its password live only in GitHub secrets and outside the repo.
- With the empty placeholder the app starts normally (the plugin only parses the config at startup). Update checks cannot verify a signature, so no update can be installed until the real key is set. `bundle.createUpdaterArtifacts` is also left for the release phase.
- In-app update check (TKM-41, 2026-11-04; spec in ARCHITECTURE.md "Updates", UI in DESIGN.md "Update line" and "Settings"):
  - Rust runs check and install (`UpdaterExt::updater_builder`, `Updater::check`, `Update::download_and_install`, then `AppHandle::restart` on macOS); the webview only calls `get_update_status`, `check_for_updates`, `install_update` and listens to `update-status-changed`. One place for the pubkey guard and the error mapping, no updater or process calls from the UI. The capability grants no updater or process permission.
  - When: once at start while `Settings.updateCheck` is on (default on), and on "Check now". No periodic timer (maintainer decision in #41). Install only on the user's click; no silent auto-install.
  - Empty or missing `plugins.updater.pubkey` (dev, builds without updater artifacts): status `unavailable`, no request. Reason: `check()` itself does not use the key (only the download verification does), so without the guard a dev build would report updates it can never install.
  - Offline, timeout, 404 on `latest.json`, invalid JSON, missing platform: status `checkFailed`, calm copy, no popup. Verified in the plugin source: a non-2xx response only logs and tries the next endpoint, so with one endpoint a 404 ends in `Error::ReleaseNotFound`; a transport error is kept and returned (`Error::Reqwest`). Check timeout 20 s via `UpdaterBuilder::timeout`.
  - Release link: `https://github.com/mango420/tokenmonk/releases/tag/v<version>` (`contracts::update::RELEASE_PAGE_BASE`); no release notes in the app (`Update.body` unused).
  - Platform behaviour (plugin docs and docs.rs `Update::install`): Windows exits the app when the installer starts (`std::process::exit(0)` after `on_before_exit`), installer mode `passive` by default, `restart_after_install` defaults to `true`; macOS and Linux replace the app in place and need a relaunch, so Rust calls `AppHandle::restart()` after `download_and_install` returns.
  - Request (plugin source, `updater.rs`): GET to the configured endpoint, no body and no query beyond the URL; `Accept: application/json` and `User-Agent: tauri-plugin-updater/<version>`; endpoint placeholders (`{{current_version}}`, `{{target}}`, `{{arch}}`, `{{bundle_type}}`) are not used in our URL. The download is a GET with `Accept: application/octet-stream`. Checked 2026-11-04: the endpoint currently answers 302 to the latest release's `latest.json`, which is 404 (no updater artifacts yet); release assets redirect to `release-assets.githubusercontent.com`.
  - Sources: https://v2.tauri.app/plugin/updater/, https://v2.tauri.app/plugin/process/, https://v2.tauri.app/plugin/opener/, https://docs.rs/tauri-plugin-updater/2.14.1/tauri_plugin_updater/struct.Update.html, https://docs.rs/tauri-plugin-updater/2.14.1/tauri_plugin_updater/struct.UpdaterBuilder.html, https://docs.rs/tauri-plugin-updater/2.14.1/tauri_plugin_updater/enum.Error.html, https://docs.rs/tauri-plugin-updater/2.14.1/tauri_plugin_updater/trait.UpdaterExt.html, https://docs.rs/tauri/2.12.0/tauri/struct.AppHandle.html (`restart`, `config`), https://docs.rs/tauri-utils/latest/tauri_utils/config/struct.PluginConfig.html, https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/updater.rs.

## Capabilities

`src-tauri/capabilities/default.json` grants the `main` window only what the registered plugins need: `core:default`, `opener:default`, `notification:default`, `autostart:allow-enable|disable|is-enabled`, `positioner:default`.

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

Superseded: updater artifacts by "v0.1 without updater artifacts", versioning, tagging and the release flow by "Automatic versioning with release-please (2026-11-01)" below. The build jobs, guard checks and secrets described here still apply.

- `.github/workflows/release.yml` runs on pushed tags `v*` (and `workflow_dispatch`). A `guard` job runs `scripts/guard.mjs --all`, checks that both signing secrets are set and non-empty (the key is always password-protected), that `plugins.updater.pubkey` is non-empty and `bundle.createUpdaterArtifacts` is `true`, and (on tags) that the tag equals `v<version>` in `tauri.conf.json`, `Cargo.toml` and `package.json`; the build jobs only start after it passes. Until the release-phase config changes below are on `main`, the guard fails on purpose.
- Builds: macOS universal (`--target universal-apple-darwin`, both Rust targets installed) and Windows, via `tauri-apps/tauri-action@v1` (also `actions/checkout@v7`, `actions/setup-node@v7`, `Swatinem/rust-cache@v2`, `dtolnay/rust-toolchain@stable`). macOS builds exist only here, never in PR CI.
- Binaries are unsigned: no Apple or Windows code-signing secrets. Only the updater artifacts are signed. The action creates a draft release `Tokenmonk v<version>` and uploads installers, `.sig` files and `latest.json` (`uploadUpdaterJson: true`, `updaterJsonPreferNsis: true`).
- Secrets (repository Actions secrets, never in the repo): `TAURI_SIGNING_PRIVATE_KEY` (key content) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. `GITHUB_TOKEN` is the built-in one; the workflow is `contents: read` by default and only the build job gets `contents: write`.

### Cutting a release

Superseded: releases are cut through the release PR, see "Automatic versioning with release-please (2026-11-01)" below. Manual version bumps and hand-pushed tags are gone.

1. Only when updater artifacts are switched back on, one time: `npm run tauri signer generate -- -w <path outside the repo>`; store the private key and password as the two secrets; keep the public key for the config change below.
2. Cut the release as described in "Automatic versioning with release-please (2026-11-01)".
3. When updater artifacts are on: the endpoint `releases/latest/download/latest.json` resolves only for a published release, so check `latest.json` on the draft before publishing it.

### `tauri.conf.json` changes applied in the release phase

- `bundle.createUpdaterArtifacts`: `true` (without it no `.sig` files or updater bundles are produced).
- `plugins.updater.pubkey`: the public key content from the generated `.pub` file (content, not a path).
- `bundle.macOS.signingIdentity`: `"-"` (ad-hoc signing; Tauri docs recommend it for macOS builds without an Apple certificate, otherwise Apple Silicon builds can be reported as damaged). Users still approve the app once in Privacy & Security.
- Endpoint stays `https://github.com/mango420/tokenmonk/releases/latest/download/latest.json` (already set).

## v0.1 without updater artifacts (2026-10-31)

- v0.1 ships without auto-update. The repository stays private, binaries are distributed by hand, and the in-app update check (#41) is deferred. No updater signing key exists and none is created for now.
- Update (2026-11-03): the repository is public and the installers are published on GitHub Releases (v0.1.0 and v0.2.0). Still no auto-update, no signing key and no updater artifacts; the in-app update check stays deferred (TKM-41).
- `release.yml` has one switch, the workflow-level `UPDATER_ARTIFACTS` (currently `'false'`). While off, the guard skips the signing-secret check and requires `bundle.createUpdaterArtifacts` to be unset or false; the build uploads no `latest.json` (`uploadUpdaterJson: false`) and no `.sig` files (`uploadUpdaterSignatures: false`). The tag and version consistency check and `guard.mjs --all` always run. The empty `plugins.updater.pubkey` stays and does no harm.
- `bundle.macOS.signingIdentity` is `"-"` (ad-hoc signing, confirmed by the Tauri 2 macOS signing docs); users still approve the app once in Privacy & Security.
- Switching updater artifacts back on: follow "Cutting a release" step 1 and "`tauri.conf.json` changes applied in the release phase" above (except the signing identity, already set), then set `UPDATER_ARTIFACTS: 'true'` in `release.yml`. The guard then requires both secrets, a non-empty pubkey and `createUpdaterArtifacts: true`.
- Update (2026-11-04): the in-app update check is specified and its contracts exist (TKM-41, see "Updater"); implementation follows in #161 (core) and #162 (UI). Release-side state is unchanged: no signing key, `UPDATER_ARTIFACTS: 'false'`, empty pubkey. Until the release-side switch (#163, once the maintainer key exists) builds report `unavailable` and make no update request. The first build with a key can only update to later releases that publish `latest.json`; earlier installs (without a key) never update in-app and need one manual install.
- Update (2026-11-04, TKM-163): updater artifacts are on. `plugins.updater.pubkey` holds the public key, `bundle.createUpdaterArtifacts` is `true`, `UPDATER_ARTIFACTS` is `'true'`; the private key and password are GitHub secrets. Releases now carry `.sig` files and `latest.json` (see the expected-assets note under "Release ownership"). Test builds on `dev` have no secrets and pass `--config '{"bundle":{"createUpdaterArtifacts":false}}'`.

## Issue keys and feature register (2026-11-01)

- Every issue has the key `TKM-<n>`, where `<n>` is its GitHub issue number. Issues and PRs share one number sequence, so keys have gaps where a PR took the number; that is accepted.
- Issue title: `TKM-<n>: <short>`. Each issue carries exactly one type label: `feature`, `bug` or `chore`.
- Branch: `<prefix>/TKM-<n>-<short-kebab>` with the usual prefixes (feature/ bugfix/ hotfix/ refactor/ chore/ docs/ test/ ci/). Superseded by "Branch name is the issue key" below.
- PR title and squash subject: `<type>(<scope>): <short> (TKM-<n>)`; the PR body still ends in `Closes #<n>`. The PR key check in CI (`scripts/check-pr-key.mjs`) requires the `(TKM-<n>)` title suffix and the branch format, with the same key in both; dependabot and release branches are skipped.
- `docs/FEATURES.md` is the feature register, generated from the issue tracker by `node scripts/features.mjs` (needs an authenticated `gh`). It is never edited by hand; regenerate and commit it when issues change.

## Checks on the register refresh PR (2026-11-01)

- `feature-register.yml` creates the refresh PR with the built-in `GITHUB_TOKEN`. Events raised by `GITHUB_TOKEN` start no workflow runs, so `ci.yml` and `pr-key.yml` would never run on that PR.
- Exception per GitHub docs: `workflow_dispatch` events raised by `GITHUB_TOKEN` do create runs. After pushing the refresh branch, the workflow dispatches both checks with `gh workflow run ci.yml --ref <branch>` and `gh workflow run pr-key.yml --ref <branch>`; this needs `actions: write` on the workflow.
- Chosen because it needs no secret, no GitHub App and no personal access token.
- Fallback if the dispatched runs do not attach to the PR as checks: a fine-grained personal access token, created by the maintainer and stored as a repository secret, used to create the refresh PR.

## Automatic versioning with release-please (2026-11-01)

- `googleapis/release-please-action@v4` in `.github/workflows/release-please.yml`, run on every push to `main`. Config: `release-please-config.json` and `.release-please-manifest.json` (starts at `0.1.0`, matching the existing tag `v0.1.0`). Release type `node` (bumps `package.json` and `package-lock.json`) plus extra files `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` and `src-tauri/tauri.conf.json`, so all versions move together and the tag check in `release.yml` holds.
- Version rule: Conventional Commits on `main` since the last tag decide the bump. Before 1.0 (`bump-minor-pre-major: true`, `bump-patch-for-minor-pre-major: false`): any `feat` gives a minor bump, `fix` only gives a patch bump, a breaking change gives a minor bump. `chore`, `ci`, `docs`, `refactor` and `test` alone give no release. The number of features does not matter; one `feat` or ten give the same bump. Tags are `v<version>` (no component in the tag).
- Release PR: release-please keeps one PR open on a `release-please--*` branch, titled `chore(main): release x.y.z`, with the bumped versions and the new `CHANGELOG.md` section. Its header and footer are custom and carry no attribution. It has no TKM key; `scripts/check-pr-key.mjs` skips release-please branches.
- One release per version: merging the release PR makes release-please create the tag and a draft GitHub release with the changelog as notes (`draft: true`, `force-tag-creation: true`). `release.yml` (`tauri-action`, `releaseDraft: true`) runs on the tag, finds that draft by tag and only uploads the installers. Publishing the draft before both builds have finished makes `tauri-action` create a second release, so publish only afterwards.
- Token: repository secret `RELEASE_PLEASE_TOKEN` (fine-grained PAT or GitHub App token, never in the repo). With it, the release PR runs CI and the tag push triggers `release.yml`. Without it the built-in `GITHUB_TOKEN` is used: events it creates trigger no workflows, so the release PR gets no CI checks, and a `dispatch-release` job (`actions: write`) runs `gh workflow run release.yml --ref <tag>` after a release is created. Without `RELEASE_PLEASE_TOKEN` the repository setting "Allow GitHub Actions to create and approve pull requests" must be on, otherwise release-please cannot open the release PR.
- `CHANGELOG.md` is generated from now on: release-please prepends a `## [x.y.z]` section per release. It is not edited by hand and has no `[Unreleased]` section (release-please inserts above the first `## [` heading, so it would land above it). The hand-written `0.1.0` section stays as history. The generated notes contain mainly `feat`, `fix`, `perf`, `revert` and `deps` commits, plus breaking changes, so write user-visible changes as those types.
- Carry-over: the Halloween removal was merged after `v0.1.0` as `refactor` commits and does not show up in the generated notes. Add it to the notes of the next draft release by hand before publishing: "Removed: Halloween season look and the ghost easter egg. Changed: Boo is now a legacy Halloween 2026 badge; existing unlocks stay, it can no longer be earned." Not done for v0.2.0 (published 2026-11-02 with the generated notes only); still open for the v0.3.0 draft.

### How to cut a release now

Superseded in part by "Dev and main branches (2026-11-04)" below: feature and fix PRs now target `dev`, and changes reach `main` through a promotion PR. Steps 2 to 4 still apply to the release PR on `main`.

1. Merge feature and fix PRs as usual; release-please updates the release PR after every push to `main`.
2. When ready, merge the release PR through the merge queue (green checks, squash with the PR title as subject). Exceptions: its subject `chore(main): release x.y.z` has no TKM key, and there is no manual rebase or force-push, because release-please rebuilds its branch from `main` on every push (manual changes would be overwritten). Without `RELEASE_PLEASE_TOKEN` the PR has no CI checks; the gate is then a reviewer check of the version and changelog diff (superseded by "Windows-only release test (2026-11-08)" below: dispatching the checks by hand is required before merging).
3. Wait until both `release.yml` builds (macOS universal and Windows) have finished and uploaded to the draft release `v<version>`.
4. Review the draft (assets for both platforms, notes), install its assets and run [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md) on macOS and Windows, then publish it. Gate, fix-forward policy and hotfix path: [RELEASE-FLOW.md](RELEASE-FLOW.md) (TKM-111). Superseded by "Windows-only release test (2026-11-08)" below: no install test of the draft; after the asset check the orchestrator publishes it.

## Design refresh (2026-11-03)

The popup follows the refreshed prototype; DESIGN.md and `tokens.css` describe it (TKM-79).

- Fonts: Schibsted Grotesk (UI text, weights 400, 500, 700) and Martian Mono (all data numbers, weights 300, 400, 500), bundled through `@fontsource/schibsted-grotesk` and `@fontsource/martian-mono` (owner decision). They replace Fraunces, Manrope and JetBrains Mono. The package swap and the imports land in TKM-80; no CDN, no runtime network.
- Theme: light and dark both ship; the theme follows the OS, `data-theme` forces one. New palettes and accents (saffron #E39A0B, jade #1F8A64). No danger colour: the ring never turns red, the status copy carries the urgency. `tokens.css` keeps a block of legacy aliases so the current components render until the frontend refresh removes them (removed in TKM-98; the icon and badge sizes became regular tokens).
- Ring direction: the popup ring and the tray icon show percent left and drain as a window is used (before: percent used, filling up). The popup ring shows the window picked in the "5 hours" / "Week" segment in both modes; the Do it yourself budget gauge is gone and the budget moves to the "Daily budget" stat. Tray: ring plus centre dot, both in the mode accent, 5-hour percent left in both modes, dotted track without limit data (owner decision; the prototype's light grey dot is replaced by the accent).
- The header brand mark is static (arc and dot), no longer a live halo. The monk files in `assets/brand/` stay the app icon source for now (superseded by "App icon").
- The header brand mark line and the tray "ring plus centre dot" part are superseded by "Icon system (2026-11-04, final)" below.
- Motion: only the ring draw (1.1 s, first dashboard render after start), the toast slide (280 ms) and the switch knob (150 ms); all off under `prefers-reduced-motion: reduce`.
- Not in the prototype but kept, so nothing regresses: Settings keeps the "Limit data" section, and the dashboard note links to it while `limits.reason = noSource`. The level-up screen and the empty, import, stale and error states are derived from the prototype's parts; DESIGN.md marks their copy as derived.
- Game (owner decision, prototype copy and rules): one title per mode at every level, "Limit Chaser" (Max it out, also before onboarding) and "Quiet Spender" (Do it yourself); the level ladders go. New descriptions for all five achievements. XP: First Blood 10 (was 25), So Close 60 (was 100), Rate-Limit Rambo 100, Night Owl 50 and Token Monk 150 unchanged. Night Owl: a message between 00:00 and 04:00 local time (was 02:00 to 04:00; only the start moved). Token Monk: one finished day under the daily budget, `usedPercent` below 100 (was 7 days in a row below 20 %), so it becomes a one-shot without `target`. Level curve and mode restrictions unchanged. Stored unlocks and XP events are not rewritten. Boo stays retired (see the carry-over under "Automatic versioning with release-please").
- Contracts: `View` gains `"achievements"` (Rust `View::Achievements`, serde camelCase), opened from the dashboard level block. Additive: a UI that does not handle it falls back to the dashboard.
- Contract change (TKM-91): `DailyBudget.tokensLeftPerDay` (TS `number | null`, Rust `tokens_left_per_day: Option<f64>`, serde camelCase, `null` never omitted). Meaning: headline tokens that may still be used per day, today included, until the weekly reset. `weeklyLimit` basis: tokens per weekly percent point = headline tokens since the weekly window start (`resetsAt` minus 7 days) divided by the current weekly utilization; value = `(100 - utilization) / daysLeft` times that ratio, at least 0. `null` with the `tokenHistory` basis, at 0 % weekly utilization or without tokens in the window. Like the limit ETA it is an estimate, because the weekly limit also counts claude.ai chats. Adding a field to `DailyBudget` breaks every struct literal of it outside the contracts (aggregate, app tests, the frontend mock), so it landed in its own PR together with those call sites (TKM-91). The aggregate computes it (TKM-81).
- No network in v0.1, resolved: the limit source is the local status-line bridge ("Limit source"). A source that reads the OAuth usage endpoint was evaluated and dropped. `reqwest` is therefore unused; it is marked dropped in the dependency table and was removed from `Cargo.toml` (TKM-89). `LimitReason::Network` stays in the contracts for compatibility; no v0.1 source reports it.

## Release ownership (2026-11-03)

- release-please drives the version, the tag and the notes: it bumps `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json` and `.release-please-manifest.json` together, writes `CHANGELOG.md`, creates the tag `v<version>` and the draft release. Nothing else bumps versions or pushes tags.
- `release.yml` (`tauri-action`) only builds and uploads: it finds the draft by tag and attaches the installers. It never creates a tag, bumps a version or publishes. Publishing the draft stays a manual maintainer step. Superseded by "Windows-only release test (2026-11-08)" below: the orchestrator publishes the draft once builds and assets are complete.
- Expected assets per release (`UPDATER_ARTIFACTS` is `'true'` since TKM-163): four installers, `Tokenmonk_<version>_universal.dmg`, `Tokenmonk_<version>_universal.app.tar.gz`, `Tokenmonk_<version>_x64-setup.exe` and `Tokenmonk_<version>_x64_en-US.msi`; a `.sig` file for the `.app.tar.gz`, the `-setup.exe` and the `.msi`; and `latest.json` (`updaterJsonPreferNsis: true`, so Windows updates use the `-setup.exe`). GitHub adds the two source archives. Releases built while the switch was `'false'` (up to v0.3.x) carry only the four installers.
- The guard allows exactly one co-author trailer in commit messages, `github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>`. GitHub appends it when a maintainer squash-merges a PR whose commits the Actions bot authored (the release PR, the register refresh PR). It names the GitHub Actions bot, not an assistant. Every other co-author trailer stays blocked. Cause: the v0.3.0 tag commit carried it, the release guard failed and no installers were built (TKM-104).
- Rebuilding an existing tag: start `release.yml` from `main` with the input `tag` (for example `v0.3.0`). The run checks out and builds the tag, checks that it is a tag matching the app version, and runs the guard script from `main` over the tag's tree and history. A plain re-run would use the tag's own, older guard. The tag is never moved, deleted or recreated.

## Popup header mark (2026-11-03)

- The popup top row shows the monk mark (`assets/brand/tokenmonk-mark.svg`) at 26 px instead of the ring brand mark (TKM-121). It stays static. The file is used as is, with its own fixed colours, in both themes and both modes.
- The ring brand mark (`tokenmonk-ring-mark.svg`, arc and dot in the mode accent) stays for the onboarding and level-up screens at 52 px. The header line of "Design refresh" above is superseded for the top row only.
- Both points are superseded by "App icon" below: the top row shows `tokenmonk-icon.svg`, the level-up screen the mono monk mark, and `tokenmonk-ring-mark.svg` is gone.
- Superseded by "Icon system (2026-11-04, final)" below.

## Dev and main branches (2026-11-04)

Reverses the "`main` only" rule of TKM-111 ([RELEASE-FLOW.md](RELEASE-FLOW.md), formerly "`main` is the only long-lived branch") and step 1 of "How to cut a release now" (TKM-140). The full flow is in [RELEASE-FLOW.md](RELEASE-FLOW.md).

- Why: `main` only ever contains code the maintainer has tested as an installed build. Until now a change was first tested as an installed app after it had landed on `main` and spent a version; the draft gate caught problems, but only after the fact.
- Two long-lived branches. `dev` takes every feature, fix, docs, chore, ci and test PR (squash, PR title as subject, same keys and checks as before). `main` takes only promotion PRs from `dev`, hotfix PRs, the release PR and the back-merge source.
- Test build: every push to `dev` (and a manual dispatch) builds unsigned test installers for macOS universal and Windows and keeps them as workflow artifacts. No release, no version bump, no tag, no prerelease version. Prerelease versions were not adopted: whether the Windows MSI accepts them is not verified here, and with the updater off nobody would receive them.
- Promotion: when `dev` has passed the maintainer's test of the installed test build, a PR with head `dev` and base `main` (`chore(release): promote dev to main`) is merged with a merge commit, so every `feat` and `fix` keeps its own changelog line. Only the maintainer starts a promotion.
- release-please runs on `main` only and works as before: release PR, tag `v<version>`, draft release. The maintainer tests the draft with [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md) and publishes it by hand (superseded by "Windows-only release test (2026-11-08)" below: no draft test, the orchestrator publishes). After each stable release `main` is merged back into `dev` (merge commit).
- Unchanged: fix-forward, tags never move, the release PR is squash-merged, "Release ownership" and "v0.1 without updater artifacts" still apply.
- New rules that follow from dry-runs on scratch branches: the freeze between merging a promotion and merging its release PR, and no edits to version files, `CHANGELOG.md` or `.release-please-manifest.json` on `dev` (RELEASE-FLOW.md, "Promotion PR" and "Back-merge").
- The labelled PR build of idea #115 is covered in large part by the `dev` test build.
- Repository settings, rulesets and workflow changes land through the cutover plan in [RELEASE-FLOW.md](RELEASE-FLOW.md); until step 6 of that plan is done, the previous flow stays in force.

## App icon (2026-11-04)

- New app icon "N1": the monk without a ring on an ink rounded-square body replaces the monk-in-ring brand mark (TKM-153). Owner decision. Description, colours and grid: [DESIGN.md](DESIGN.md), "App icon".
- Files in `assets/brand/`: `tokenmonk-icon.svg` (macOS grid, 824 px body at 100,100, rx 185 on a 1024 canvas; also the popup top row mark at 26 px), `tokenmonk-icon-windows.svg` (same artwork, viewBox `84 84 856 856`, source of `icon.ico` only) and `tokenmonk-mark-mono.svg` (one colour, transparent, recoloured by `fill`; the level-up screen at 52 px in the mode accent).
- The ring stays only in the tray gauge (drawn in Rust, unchanged) and in the onboarding brand mark, drawn inline in `src/components/BrandMark.tsx` (the onboarding part is superseded by "New app icon everywhere" below). `tokenmonk-ring-mark.svg` is removed; `tokenmonk-mark.svg` is removed too (TKM-153), as nothing imports it any more.
- Platform icons are generated by `node scripts/icons.mjs` (TKM-154), desktop sets only (no Android/iOS). This replaces the `tauri icon` step of "Scaffold".
- The popup uses of the files above (top row, level-up) are superseded by "Icon system (2026-11-04, final)" below; files and generation stay.

## New app icon everywhere (2026-11-04)

- Owner decision (TKM-185): the app icon (`assets/brand/tokenmonk-icon.svg`) replaces the old ring brand mark everywhere. The onboarding mode choice shows the app icon at 52 px, same file and colours in both rows; the inline ring mark in `src/components/BrandMark.tsx` is removed.
- The ring stays only in the tray gauge, which now carries the new monk inside the ring (TKM-188): the mono mark paths in the mode accent (black on the macOS template) replace the centre dot; ring r 10, arc width 2.4. Details: [DESIGN.md](DESIGN.md), "Tray icon".
- Superseded by "Icon system (2026-11-04, final)" below.

## Icon system (2026-11-04, final)

Owner decision (TKM-202), supersedes TKM-185 for the popup and the monk colour of TKM-188. One monk, three jobs; details and sizes: [DESIGN.md](DESIGN.md), "Icon system".

- A. App icon: the tile (`assets/brand/tokenmonk-icon.svg`, `tokenmonk-icon-windows.svg`, fixed colours) only where the OS shows the app: taskbar, dock, Start menu, installer, notifications, the generated icon set. Never inside the popup.
- B. Monk mark: `assets/brand/tokenmonk-mark-mono.svg` with its paths unchanged, two-tone: head (with ears and eye cut-outs) and sleeve in `--ink`, robe in `--accent`; follows theme and mode, no tile, no ring. The only brand mark in the popup: top row 26 px (`--brand-mark-size`), onboarding rows 52 px with each row in its own mode accent (Max it out saffron, Do it yourself jade), level-up 52 px in the current mode accent. `--onboard-mark-size` is renamed `--mark-lg-size` (52 px); the old name is removed (TKM-224).
- C. Tray gauge (Rust): arc in `--accent` (its length means percent left). Monk head and sleeve in `--tray-track` #8A8D8B at full opacity, robe in `--accent` of the current mode, matching the popup monk mark. Without limit data: dotted track and the whole monk at opacity 0.45 (new token `--tray-monk-nodata-opacity`, mirrored like the other tray tokens). The macOS template stays black ink; the no-data fade applies there too.

## Popup with log-only data (2026-11-07)

Architect decision (TKM-228, part of TKM-180). Details: [DESIGN.md](DESIGN.md), "Dashboard", "Note line", "Dashboard states", "Limit data". No contract change; every field already exists.

- Without limits but with a `tokenHistory` budget (Do it yourself only), the ring fills with tokens today against the average active day instead of showing the track only. It fills rather than drains so the arc and the hero (tokens today) move the same way. It caps at a closed circle; above 100 % the status line says "Over budget. The AI wins today."; no colour change.
- With `dailyBudget = null` the second stat tile shows "This session" (`session.tokens.total`, unit the project name) instead of "n/a"; idle shows "n/a" / "no active session".
- The limit ETA shows in the reset line on the "5 hours" tab while limits are `ok`, only if it comes before the window reset: "Limit in ~25 min, resets in 2h 14m". Same rounding as the OS notification.
- `tokensLeftPerDay` is shown with a `~` prefix: it is an estimate, the weekly limit also counts claude.ai chats.
- Status line placement: the official docs (https://code.claude.com/docs/en/statusline, checked 2026-11-07) call it "a customizable bar at the bottom of Claude Code" that "appears at the bottom of the interface". They do not say "terminal" and say nothing about the desktop app. "Terminal only" is an inference, not a documented fact. Observed on one machine, not verified: the status line did not run while Claude Code was used from the desktop app. Copy therefore names neither the terminal nor the desktop app as certain and says "may not run".

## About screen in v0.1 (2026-11-08)

Maintainer moved the About screen into v0.1 scope; architect design (TKM-125). Details: [DESIGN.md](DESIGN.md), "About"; [ARCHITECTURE.md](ARCHITECTURE.md), "Tray menu".

- Contracts: `View` gains `about` (Rust `View::About`, camelCase like the other variants). New constants `REPO_URL` and `ISSUES_URL` in `src/lib/contracts.ts` and `contracts::app`; a test ties them and `RELEASE_PAGE_BASE` to one repository. No new command, event or payload field.
- Entry point: an "About" text link in the Settings footer note only. No tray item, so `tray` and `commands` stay unchanged; the tray menu stays short and About is a rare visit.
- Content: version (`AppInfo.version`), MIT licence, links to the GitHub repository and its issues. No personal data, no other links. The Settings note drops "Ideas and bugs go to GitHub issues."; About carries it.
- Back and Esc on About return to Settings, where it was opened from.
- No new design token.

## Update download progress (2026-11-08)

Maintainer moved download progress into v0.1 (TKM-212); contract and design by the architect. Details: [ARCHITECTURE.md](ARCHITECTURE.md), "Updates"; [DESIGN.md](DESIGN.md), "Update line" and Settings item 6.

- New `UpdateStatus` variant `downloading { version, percent }` between `available` and `installing`; `percent` is a whole number 0 to 100 or `null` (unknown size). Same `kind` tag and camelCase rules as the other variants; no new event, it rides on `update-status-changed`.
- Source: tauri-plugin-updater 2.x on docs.rs (`Update::download_and_install`, `on_chunk: FnMut(usize, Option<u64>)`, `on_download_finish: FnOnce()`) and https://v2.tauri.app/plugin/updater/, checked 2026-11-08. The content length is `None` when the server sends no `Content-Length`.
- Throttle: Rust emits only when the whole percent changes, so at most 101 progress events per download. The UI adds no throttle of its own.
- Copy: "Downloading 0.4.1, 42%." with the percent as a live number; without a percent "Downloading 0.4.1.". No progress bar, no spinner.

## Windows-only release test (2026-11-08)

Maintainer decision after the v0.5.0 release (TKM-247). Supersedes the draft test and the manual publish in "Release ownership", "How to cut a release now" and "Dev and main branches". Flow: [RELEASE-FLOW.md](RELEASE-FLOW.md); test: [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md).

- The maintainer tests test builds and releases on Windows only. macOS ships untested by the maintainer; the community tests it after publishing, and macOS reports become `bug` issues that are fixed forward. Builds are unchanged: macOS universal and Windows.
- One gate: the maintainer's Windows test of the `dev` test build for the `dev` head that is promoted, before the promotion. No separate draft test. Problems found later, on either platform, become `bug` issues and are fixed forward. A hotfix is gated the same way: a test build dispatched on the hotfix branch (`test-build.yml` has `workflow_dispatch`), tested on Windows before the hotfix PR is merged.
- Publishing: once both `release.yml` build jobs are green and the draft lists all expected assets ("Release ownership": four installers, three `.sig` files, `latest.json`), the orchestrator publishes it. The maintainer no longer publishes by hand. Publishing before both build jobs finish still makes `tauri-action` create a second release.
- Release notes: the generated changelog notes are used as they are. A review by the maintainer is optional, not a gate.
- Release PR: without `RELEASE_PLEASE_TOKEN`, dispatching `ci.yml` and `pr-key.yml` on the release branch is required before merging. Without those checks the `main` ruleset refuses the merge ("base branch policy prohibits the merge"), as on the v0.5.0 release PR (#245).
- Why: the maintainer can only test on Windows, and the draft test repeated the test-build test of the same code.

## Other-mode achievement hint (2026-11-09)

Architect design for TKM-126. Details: [DESIGN.md](DESIGN.md), "Achievements", item 7.

- Locked achievements that only count in the other mode get one extra line under the description: "Max it out only. Not on this mode's menu." (or "Do it yourself only. ..."). Small style, `ink-2`, full opacity; the state stays "Locked". No accent, no icon, no new token.
- No hint on unlocked rows, on achievements that count in both modes, or before onboarding (`noneYet`).
- Contracts unchanged: `AchievementProgress.mode` (Rust `Option<Mode>`, TS `Mode | null`) and `GameState.mode` already carry the scope, and the game module already fills it.

## XP and level rework (2026-11-09)

Maintainer decision (TKM-255, docs TKM-259). Supersedes the titles and "Level curve ... unchanged" of "Design refresh", and its "not rewritten" rule for this one recount. Details: [ARCHITECTURE.md](ARCHITECTURE.md), "Game".

- Max it out: soft ceiling instead of the hard daily cap. 10 XP per 100k headline tokens up to 2M, 2 XP per 100k from 2M to 5M, nothing above; plus 10 XP per active day. At most 270 XP per day.
- Do it yourself: per finished active day 150 XP up to 50 % of the daily budget, linear down to 30 XP at 100 %, 0 over budget, 50 XP without a budget. The active-day bonus is Max it out only; the per-day row already covers DIY.
- Level curve: span `200 + 120 * (n - 1)` for levels 1 to 25, then a flat 3,080 XP per level, no cap. Target: first level-up on day 1 or 2, level 10 after about two months, a level roughly every three weeks later on.
- Titles in six tiers per mode (1 to 4, 5 to 9, 10 to 14, 15 to 19, 20 to 29, 30+), final copy as in the TKM-255 table. No contract change: `GameState.title` and `LevelUp.title` already carry the title.
- One-time recount: ledgers built with an older rules version drop their `tokens` and `day` XP rows and replay history under the new rules, as a silent retroactive run with one summary screen. Achievement unlocks and their XP stay. Every new rule pays at least the old one for the same day, so no level goes down.
- Why: the hard cap stalled the bar on busy days, small days earned nothing, DIY levelled about 2.5 times slower and rewarded idling, and late levels took more than a month each.

## Branch name is the issue key (2026-11-09, TKM-267)

- A branch is named exactly `TKM-<n>`: no type prefix, no kebab suffix. The type lives in the PR title and the label, the topic in the issue.
- One open PR per issue at a time; a follow-up PR for the same issue reuses the name after the first one is merged and its branch deleted.
- Hotfixes into `main` have no prefix to tell them apart any more: `scripts/check-pr-key.mjs` accepts a `TKM-<n>` head into `main` only when the title type is `fix` or `revert`. `dev` and `release-please--*` heads stay allowed as before.
