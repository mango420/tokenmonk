# Architecture

Tokenmonk is a Tauri 2 tray app for macOS and Windows. Rust owns all
data work; the React UI only renders snapshots and sends a few commands.

Related documents: [DESIGN.md](DESIGN.md) (look and feel),
[LOG-FORMAT.md](LOG-FORMAT.md) (input format). Contracts:
`src-tauri/src/contracts/` and `src/lib/contracts.ts`. Database schema v1:
[`src-tauri/src/contracts/schema_v1.sql`](../src-tauri/src/contracts/schema_v1.sql).

## Modules

| Module | Path | Responsibility |
|---|---|---|
| contracts | `src-tauri/src/contracts/` | Wire types, command and event names, limit source trait, schema v1. No logic, no Tauri types. |
| logs | `src-tauri/src/logs/` | Parse JSONL lines into usage records (LOG-FORMAT.md). Pure functions. |
| watcher | `src-tauri/src/watcher/` | Find log files, watch for changes, read new bytes incrementally. |
| db | `src-tauri/src/db/` | SQLite connection, migrations, upserts and queries. |
| aggregate | `src-tauri/src/aggregate/` | Build `DashboardSnapshot` from the database and the latest limit reading. |
| limits | `src-tauri/src/limits/` | `LimitSource` implementations and the polling worker. |
| game | `src-tauri/src/game/` | XP, levels, titles, achievements, retroactive replay. |
| app | `src-tauri/src/app/` | App layer: shared state (`AppState`: cached snapshot, game state, settings, app info, command connection), the ingest publisher (`AppPublisher`: caches each snapshot, emits events, drives the tray ring and the game), startup wiring (`setup`), the game store adapter (`DbGameStore`) and the game facts builder. SQL stays in `db`. |
| update | `src-tauri/src/app/update.rs` | Update check and install through `tauri-plugin-updater`; holds `UpdateStatus` and the last `Update` (see "Updates"). Part of the app layer. |
| commands | `src-tauri/src/commands/` | `#[tauri::command]` handlers; thin, read cached state or delegate. |
| tray, window | `src-tauri/src/{tray,window}/` | Tray icon with live ring (5-hour percent left), popup window placement and show/hide. |
| UI | `src/` | React views: dashboard, settings, about, achievements, toasts, level-up screen. |

The UI never touches files, the database or the network. The only network
request of the app is the update check (see "Updates"), made by Rust.

## Data flow

```
<Claude folder>/projects/**/*.jsonl     (read only; see "Claude folder")
        |  file events + periodic rescan
        v
watcher --new complete lines--> logs (parse, filter) --records--> db (upsert by message id)
                                                                      |
limits worker --LimitReading--> db (limit_snapshots)                  |
                                                                      v
minute tick ----------------------------------------------------> aggregate
                                                                      |
                                              DashboardSnapshot ------+------> game (XP, unlocks)
                                                     |                               |
                                                     v                               v
                                     emit dashboard-updated          emit game-updated,
                                     update tray ring                achievement-unlocked, level-up
                                                     |
                                                     v
                                          UI (React) renders; commands for reads and settings
```

## Threads

- **Main thread**: Tauri event loop, tray, windows. Never blocks on I/O.
- **Ingest worker** (one thread): owns the ingest write connection to
  SQLite. Receives file-change notifications, reads new bytes, parses, upserts,
  then recomputes the snapshot and the game state, stores both in shared
  state and emits events. Also runs the minute tick (day rollover, burn
  rate decay, session idle cut-off).
- **Limits worker** (one thread): calls `LimitSource::fetch` on a fixed
  interval, hands readings to the ingest worker over a channel. A slow or
  failing source never delays log ingestion.
- **Update task** (async, on Tauri's async runtime, not a thread of its
  own): one check at start if `Settings.updateCheck` is on, then only on
  `check_for_updates`; an install only on `install_update`. Never blocks
  the main thread or the workers; at most one check or install runs at a
  time.
- **Command handlers** read the cached snapshot, game state and settings
  from shared state (`app::AppState`, held in an `Arc`, cache behind an
  `RwLock`). Commands that write (settings) use a second connection owned
  by `AppState` and guarded by a mutex; game evaluation and persistence
  are serialised by `AppState::game_lock`. Commands never parse logs.

The database runs in WAL mode with a busy timeout, so reads never wait for
a writer and a write that meets the other connection's lock waits briefly
instead of failing.

## Ingest

1. On start, list all `*.jsonl` files below the root and compare with
   `source_files` (size, offset, mtime).
2. For each new or grown file, read from the stored offset up to the last
   newline, parse each line, update the offset in the same transaction as
   the message upserts.
3. Shrunk or replaced files are re-read from 0; dedup makes it harmless.
4. Afterwards, react to file-system notifications, with a slow periodic
   rescan (for example every 60 s) as a safety net for missed events.
5. The first import may take a while; `DataStatus.state` is `importing`
   with file counts until it finishes, then `ready`.

Upsert: `INSERT ... ON CONFLICT(message_id) DO UPDATE` setting
`ts = MIN(old, new)` and each token column to `MAX(old, new)`. Session,
agent, model and project are identical on every copy of a message, so the
stored values stay. The result does not depend on the order in which files
or lines are read (LOG-FORMAT.md, "Deduplication").

## Claude folder

The Claude folder is Claude Code's config folder; the ingest root is its
`projects/` subfolder, and the status-line setup names `settings.json` in
it. Decided in TKM-211, contract in TKM-291.

- Order: `Settings.claudeFolder` (source `setting`), else the environment
  variable `CLAUDE_CONFIG_DIR` when set and non-empty (source `env`), else
  `.claude` in the home directory (source `default`). Apps started by
  autostart or the OS launcher often do not see shell variables, hence the
  setting.
- `update_settings` with `claudeFolder: "<absolute path>"` sets it,
  `claudeFolder: null` resets it to the default, an absent key leaves it
  alone. An empty or relative path is rejected with `invalidArgument`. The
  folder is saved even if it or its `projects/` does not exist.
- A change switches live: stop the running ingest, start a new one on the
  new root. No restart. Data already stored stays; the new folder is read
  in addition, and dedup by message id keeps anything from counting twice.
  History, levels and XP are kept. One root at a time.
- `get_claude_folder` returns `ClaudeFolder { path, source, projectsFound }`
  for the Settings row and the limit-data copy. There is no extra event:
  the UI calls it on start and again after `settings-changed` and
  `app-info-changed`. The core emits `app-info-changed` when `logsFound`
  changes, which includes a switch to a folder with or without `projects/`.
- `AppInfo.logsFound` and `ClaudeFolder.projectsFound` both mean "`projects/`
  exists in the effective folder". Without it the Settings row shows a calm
  hint ("No logs here yet."), never an error.
- The folder picker is the native dialog from `tauri-plugin-dialog`
  (DECISIONS.md, "Claude folder picker"); the UI passes the chosen path to
  `update_settings`.

## Aggregates

All definitions use local time for calendar boundaries and UTC epoch ms for
storage.

- **Headline tokens** = input + output + cache creation (decided). Cache
  reads are stored (`messages.cache_read_tokens`) and carried in
  `TokenBreakdown.cacheRead`, but excluded from `total`, "Tokens today",
  "This session" and the burn rate, because they dwarf everything else and
  would make the numbers meaningless.
- **Tokens today**: messages with a timestamp in the current local day.
- **This session**: the logs have no "session ended" marker, so: take the
  `sessionId` of the most recent assistant message overall (main and
  subagent lines alike). If that message is less than 60 minutes old, that
  session is current and `SessionStats` sums all its deduplicated messages
  (subagents included, any age). Otherwise `session` is `null` (idle). With
  several sessions running in parallel the tile follows the most recently
  active one.
- **Burn rate**: headline tokens in the trailing 15 minutes divided by 15
  (`windowMinutes = 15`).
- No estimates: no daily budget, no limit forecast, no ETA (DESIGN.md,
  "Product line").
- **Limit status**: on every snapshot build (including the minute tick)
  the aggregate first drops each window of the last good reading whose
  `resetsAt` is at or before now (`fiveHour` or `weekly` becomes `null`),
  the same rule the file source applies (Limits, "File source"). Then:
  `ok` if the last good reading is at most 15 minutes old, `stale` if
  older (remaining values still shown), `unavailable` with `reason =
  noSource` if no window is left, and `unavailable` if there is no
  reading or the source reports `noSource` (v0.1: bridge not set up or no
  current window). `reason` carries the last error. Dropping a window
  never deletes rows from `limit_snapshots`.

Window lengths and thresholds above (15 min burn window, 60 min idle,
15 min stale) are aggregate constants, not contract values; changing them
does not change the contracts.

## Limits

```rust
pub trait LimitSource: Send + Sync {
    fn id(&self) -> &'static str;
    fn fetch(&self) -> Result<LimitReading, LimitError>;
}
```

- The trait and its data types are the contract. Sources are swappable
  implementations in the `limits` module.
- v0.1 source: the **status-line bridge**. Local only: no network, no
  credentials, no endpoint. Claude Code pipes a JSON object to its
  configured status-line command; the bridge mode of the Tokenmonk
  executable keeps only the `rate_limits` part and the app reads it from a
  small file. The setup is one click: "Connect limits" writes the
  `statusLine` entry (see "Connect limits" below).
  Sources and verified facts: DECISIONS.md, "Limit source".
- Without a usable reading the app runs on token data alone: tokens today
  is the full view plus one calm line, no warning, no estimate (DESIGN.md,
  "Product line").
- Limit percentages include claude.ai chats; token statistics are Claude
  Code only. The popup footer says so permanently.

### Status-line input (verified, used fields only)

| Field | Type | Meaning |
|---|---|---|
| `rate_limits.five_hour.used_percentage` | number | Used share of the 5-hour window, 0 to 100 |
| `rate_limits.five_hour.resets_at` | number | Window reset, Unix epoch **seconds** |
| `rate_limits.seven_day.used_percentage` | number | Used share of the weekly window, 0 to 100 |
| `rate_limits.seven_day.resets_at` | number | Window reset, Unix epoch seconds |

- `rate_limits` is present only for claude.ai Pro and Max subscribers, and
  only after the first API response of a session. Each window can be
  absent on its own; Claude Code drops a window once its `resets_at` has
  passed.
- `rate_limits.spend_limit` (Claude apps gateway) is ignored in v0.1.
- Every other input field (paths, session, model, cost, workspace and so
  on) is never kept, stored or logged.
- Claude Code runs the command at session start or resume, after each
  assistant message, after `/compact`, on permission or vim mode changes,
  when a window in the last data reaches `resets_at`, and on the optional
  `refreshInterval` timer. Updates are debounced at 300 ms; a run still in
  flight is cancelled by the next one. Output shows only after exit code
  0; a non-zero exit or empty output blanks the status line.

### Bridge mode

- Invocation: `<tokenmonk executable> --statusline`. The flag must be the
  first argument. `lib.rs::run` checks it **before** the Tauri builder is
  created, so no window, tray, plugin (single-instance included), database
  or logger starts. `main.rs` only calls `run`. Bridge code lives in `src-tauri/src/limits/bridge.rs`.
- Steps:
  1. Read stdin to EOF, at most 1 MiB (more counts as bad input).
  2. Parse JSON. Take `rate_limits.five_hour` and `rate_limits.seven_day`.
     A window is valid if `used_percentage` is a finite number >= 0;
     `resets_at` is optional and used only if it is a finite number > 0.
  3. If at least one window is valid, write the bridge file (below);
     otherwise leave an existing file untouched (it keeps the last good
     reading).
  4. Print one short ASCII line to stdout and exit with code 0.
- Output line: `5h 24% | 7d 41%` (percent rounded down, a missing window
  is left out); with no valid window: `Tokenmonk: no limit data yet`.
- Never breaks Claude Code's status line: bad, empty or oversized input,
  write errors and missing directories all end in the fallback line and
  exit code 0. No panics (the release profile uses `panic = "abort"`), no
  stderr output, no retries, no waiting. Target runtime: well under the
  300 ms debounce.
- Atomic write: create the app data directory if missing, write to
  `statusline.json.<pid>.tmp` in the same directory, then rename it over
  `statusline.json` (`std::fs::rename` replaces the target). On error,
  remove the temp file and move on. Parallel sessions may race; the last
  rename wins, which is fine because all sessions report the same account.
- File: `<app data dir>/statusline.json`. The app data dir is
  `dirs::data_dir()` joined with the bundle identifier
  `app.tokenmonk.desktop`; this equals Tauri's `app_data_dir()` (Tauri
  resolves it to `data_dir/<bundle identifier>`, and the app sets no
  `appDirectoriesOverride`). Bridge and file source call the same helper
  so the paths cannot drift.
- Connect adds no `refreshInterval`: a timer re-run resends the last
  values, which would make an old reading look fresh.

Bridge file (camelCase, all timestamps `EpochMs`; invented values):

```json
{
  "version": 1,
  "receivedAt": 1767225600000,
  "fiveHour": { "usedPercentage": 23.5, "resetsAt": 1767240000000 },
  "sevenDay": { "usedPercentage": 41.2, "resetsAt": 1767600000000 }
}
```

- `version` is `1`; `receivedAt` is when the bridge ran; `fiveHour` and
  `sevenDay` are `null` when absent; `resetsAt` is `null` when unknown.
  `resets_at` seconds are multiplied by 1000. Nothing else is written.

### File source (`StatuslineSource`, id `"statusline"`)

| File state | Result |
|---|---|
| Missing | `LimitError { noSource, "status-line bridge not set up" }` |
| Unreadable | `LimitError { other, <io error kind> }` |
| Not JSON, unknown `version`, wrong types | `LimitError { invalidResponse, ... }` |
| Valid | `LimitReading { fiveHour, weekly: sevenDay, fetchedAt: receivedAt }`, `utilization = usedPercentage` |

- Windows whose `resetsAt` is at or before now are dropped (`None`), as
  Claude Code does. If no window is left: `LimitError { noSource, "no
  current window" }`, so the UI shows the setup hint, not dead numbers.
- `default_source()` returns this source in v0.1. `UnavailableSource`
  stays for tests.
- Poll interval: 30 s (a local file read), plus once when the popup opens
  if the last poll is older than the existing minimum refresh gap (1 min).
  The ingest worker stores a reading in `limit_snapshots` only if its
  `fetchedAt` is strictly newer than the newest stored one; an equal or
  older `fetchedAt` is a repeat poll (also when a window was dropped
  since), so idle polling adds no rows.
- Staleness uses the aggregate rule (Aggregates, "Limit status") on
  `fetchedAt = receivedAt`: `ok` up to 15 minutes, then `stale`. The file
  only changes while Claude Code runs, so `stale` is normal after a break.

### Bridge command

- The command that "Connect limits" writes is the current executable path
  (`std::env::current_exe()`) plus ` --statusline`
  (`limits::setup::current_statusline_command`). On Windows the path uses
  forward slashes (Claude Code runs the command through Git Bash when
  installed, which drops unquoted backslashes, else through PowerShell).
  The path is wrapped in double quotes only if it contains a space.
- It is always the POSIX shell form (macOS shell, Git Bash on Windows).
  Without a space the unquoted form is the same in Git Bash and
  PowerShell. With a space, PowerShell would need the call operator
  (`& "C:/Example Dir/tokenmonk.exe" --statusline`); Rust never writes
  that form, but "Connect limits" recognises it as Tokenmonk's.
- No command returns this string or the state of the bridge file: Settings
  shows `LimitsConnection`, the popup `DashboardSnapshot.limits`.

### To verify during implementation

- Windows release builds use `windows_subsystem = "windows"`. Check that
  the bridge reads piped stdin and that its stdout reaches Claude Code when
  started from Git Bash and from PowerShell. If not, the fallback is a
  separate small console binary, which needs a `Cargo.toml` change
  (orchestrator).
- macOS: check that running the binary inside the ad-hoc signed `.app`
  from a shell works after the first approval.
- The quoted command form (path with a space) under the PowerShell
  fallback, where only the `& "path"` form runs.

### Implementation notes

For backend-platform (and the orchestrator where marked):

1. `lib.rs::run` (orchestrator, shared hotspot): if the first argument is
   `--statusline`, run the bridge and return before the Tauri builder is
   created, so no plugin (single-instance included) starts.
2. `limits/bridge.rs`: bridge mode and the shared path helper
   (`dirs::data_dir()` + `app.tokenmonk.desktop` + `statusline.json`).
3. `limits/statusline.rs`: `StatuslineSource` (id `"statusline"`), and
   `default_source()` returns it.
4. `limits/worker.rs`: `POLL_INTERVAL` becomes 30 s; `MIN_REFRESH_GAP`
   stays 1 min.
5. Ingest worker (backend-data): store a reading in `limit_snapshots` only
   if its `fetchedAt` is strictly newer than the newest stored one (one
   statement, `Db::insert_limit_snapshot`).
6. Aggregate (backend-data): drop expired windows from the last good
   reading on every snapshot build (Aggregates, "Limit status").

### Connect limits (TKM-287, contract TKM-300)

One click in Settings writes the status line entry; "Disconnect" puts the
previous one back. Implementation: TKM-303 (backend-platform).

- Contract: `get_limits_connection`, `connect_limits`, `disconnect_limits`
  (no arguments, all return `LimitsConnection`), event
  `limits-connection-changed` (`LimitsConnection`), error code
  `claudeSettings`.
- Target: only `settings.json` in the effective Claude folder
  (`ClaudeFolder.path`, "Claude folder"). Never project, local or managed
  settings files. `settingsPath` is that path.
- State, read fresh on every call (no cache, no file watcher; the UI calls
  `get_limits_connection` when Settings opens):

  | `settings.json` | `state` | `problem` |
  |---|---|---|
  | Claude folder unknown or missing | `blocked` | `noClaudeFolder` |
  | Exists, cannot be read | `blocked` | `settingsUnreadable` |
  | Not a JSON object, or `statusLine` not an object | `blocked` | `settingsInvalid` |
  | Missing file, or no `statusLine` | `notConnected` | `null` |
  | `statusLine` is not Tokenmonk's | `notConnected`, `previousStatusLine = true` | `null` |
  | `statusLine` is Tokenmonk's | `connected` | `null` |

  "Tokenmonk's": `statusLine.type` is `"command"` and the command runs a
  Tokenmonk executable with `--statusline` as its first argument. This also
  covers entries pasted by the earlier paste-only setup, so those users
  show as `connected` and can disconnect. For `connected`,
  `previousStatusLine` is `true` when a stored previous entry exists.
- Connect (explicit click only; idempotent):
  1. Classify as above; `blocked` rejects with `claudeSettings`.
  2. If `statusLine` exists and is not Tokenmonk's, store it verbatim (the
     whole JSON value) in `<app data dir>/statusline-previous.json` next to
     the bridge file. An existing record is kept when already connected.
  3. Copy the current file byte for byte to `settings.json.tokenmonk.bak`
     in the same folder (overwritten on each write). A missing file has no
     backup and is created.
  4. Set `statusLine.type` to `"command"` and `statusLine.command` to the
     bridge command (as in "Bridge command"). The documented entry shape is
     `{ type: "command", command, padding?, refreshInterval?,
     hideVimModeIndicator? }`; every other key of a previous `statusLine`
     object, known or unknown, stays unchanged. Every other key of the
     file stays as it was, in its order. A kept `refreshInterval` re-runs
     the bridge on a timer and can make an old reading look fresh
     ("Bridge mode"); accepted, the user's status line comes first.
  5. Write atomically (temp file in the same folder, then rename). Any
     failure rejects with `claudeSettings` and leaves the file as it was.
- Wrapping (pass-through): with a stored previous entry, the bridge writes
  the limits file as before, then runs the previous command with the same
  stdin bytes and prints its stdout unchanged, exiting with its exit code.
  Tokenmonk's own line is printed only when nothing is wrapped. A failing
  previous command never stops the limits file write. The limits file is
  written before the previous command starts, because Claude Code cancels
  an in-flight status line run when a new update starts.
- Shell (verified, code.claude.com/docs/en/statusline): on Windows Claude
  Code runs `statusLine.command` through Git Bash if installed, else
  through PowerShell. The bridge runs the previous command through the
  same shell Claude Code would use, so its output stays the same.
- Disconnect (explicit click only; idempotent): when `connected`, back up
  as in step 3, then set `statusLine` to the stored previous entry, or
  remove the key if there is none, write atomically, and delete the
  record. When not connected, the file is not touched; a leftover record
  is deleted.
- `limits-connection-changed` follows every successful connect or
  disconnect, and a change of the effective Claude folder.
- Popup line (DESIGN.md, "Product line"): `connected` and no current
  reading: "Limits after your next reply."; `notConnected` or `blocked`:
  "Connect limits in Settings." `DashboardSnapshot.limits` stays
  reading-based; the UI combines both.

To verify in TKM-303 (scout, official docs; not decided here):

- Which shell Claude Code uses for `statusLine.command` on macOS.
- That the previous command's process ends when Claude Code cancels the
  bridge mid-run, and that the Windows GUI-subsystem build can pipe the
  child's stdout.

Key order is settled: `serde_json` runs with its `preserve_order` feature
(TKM-312, DECISIONS.md, "serde_json key order"), so a rewrite keeps the
key order of the user's file.

## Notifications (TKM-287, contract TKM-300)

Implementation: TKM-304. Copy and tone: DESIGN.md, "Product line".

- Limits at 10 % left (`Settings.notifyLimits`, default on): one system
  notification per window (`fiveHour`, `weekly`) and reset cycle, when the
  window's `utilization` reaches 90 or more while `limits.status` is `ok`.
  The cycle is the window's `resetsAt`; a window without `resetsAt` sends
  nothing. Sent cycles survive a restart: they are stored in the
  `app_state` key `limit_notice_cycles` (see Storage).
- Level-ups (`Settings.notifyLevelUps`, default on): one system
  notification per `level-up` with `retroactive = false`.
- System notifications use the OS sound; `Settings.sound` covers only the
  in-app tone and does not apply to them.
- Removed by TKM-287: the 90 % and 100 % warnings and the budget warning.
- Both toggles are patchable via `SettingsPatch.notifyLimits` /
  `.notifyLevelUps` and stored under the `settings` keys `notify_limits` /
  `notify_level_ups`; a missing key reads as `true`.

## Game

- XP is a ledger (`xp_events`); level, progress and title derive from the
  sum. One XP pool; each event records the mode it was earned in: `max`,
  or `diy` on legacy rows of the retired Do it yourself mode. Legacy rows
  stay as history and keep their XP (Storage); new events are always
  `max`.
- XP gain (TKM-255), per local day:
  - 10 XP per 100k headline tokens up to 2M, 2 XP per 100k from 2M to
    5M, nothing above 5M; plus 10 XP per active day (at least one counted
    reply). Ceiling 270 XP per day.
  - Days already awarded in the retired Do it yourself mode keep that
    award (`legacyday` rows after the recount) and are not paid again.
  - Nothing is awarded while `Settings.mode` is `noneYet`; since TKM-324
    every install reads as `max` on open, so this no longer happens.
- Level curve: level `n` spans `200 + 120 * (n - 1)` XP for levels 1 to 25
  (200, 320, 440, ..., 3,080), then a flat 3,080 XP per level, no cap.
  Level 2 needs 200 XP in total, level 10 needs 6,120, level 30 needs
  53,320.
- Titles depend on the level tier:

  | Levels | Title |
  |---|---|
  | 1 to 4 | Token Tourist |
  | 5 to 9 | Prompt Regular |
  | 10 to 14 | Context Hoarder |
  | 15 to 19 | Cache Whisperer |
  | 20 to 29 | Limit Chaser |
  | 30+ | Rate-Limit Royalty |
- The game module defines XP rules, the level curve, titles and achievement
  thresholds. Contracts only carry ids and numbers.
- Achievements (v0.1), name, description, rule and XP:

  | Achievement | Description | Unlocks when | XP |
  |---|---|---|---|
  | First Blood | "Your first token, spent." | first counted message | 10 |
  | Night Owl | "Coding after midnight." | a message between 00:00 and 04:00 local time | 50 |
  | Rate-Limit Rambo | "Hit the limit. On purpose, we assume." | a 5-hour reading at or above 100 % | 100 |
  | So Close | "Stopped at 99 percent. Restraint or luck." | a finished 5-hour window peaking at 99 % or more, below 100 % | 60 |

  Two retired badges can no longer be earned; existing unlocks stay valid
  and keep their XP: Token Monk ("A full day under budget. Retired with
  the budget.", 150 XP, listed only once unlocked) and Boo (shown as
  "Halloween 2026", secret, 100 XP). Already stored
  unlocks and XP events are not rewritten when these rules change, with
  one exception: the XP rework recount below.
- **Recount (TKM-255)**: the game stores the rules version its ledger was
  built with. When the stored version is older than the current one, the
  game drops the `tokens` and `day` rows of `xp_events` once, keeps
  achievement unlocks and their XP and the mode each day was earned in,
  and replays the history under the new rules as a retroactive run (same
  rules as retroactive levelling below: no `achievement-unlocked`, no
  `level-up`, no OS notification, one `game-updated` at the end; the UI
  shows the retro summary once). Every new rule pays at least the old one
  for the same day, so a recount never lowers a level.
- **Retroactive levelling**: on the first evaluation with a mode set
  (`Settings.mode` is not `noneYet`), the game replays the stored history in time order, writing
  `xp_events` and unlocks with `retroactive = 1`. During the replay no
  `achievement-unlocked` or `level-up` events are emitted; one
  `game-updated` follows at the end with `retroDone = true`, and the UI may
  show one summary instead of a toast storm. `app_state.retro_done` makes
  it run once.
- Live evaluation runs after every ingest batch and on the minute tick;
  each unlock emits `achievement-unlocked`, each level change `level-up`,
  then `game-updated`.
- `(kind, ref)` uniqueness in `xp_events` makes every award idempotent.

## Updates

Spec for TKM-41; implementation in #161 (core) and #162 (UI). Verified
against the plugin docs and source (sources in DECISIONS.md, "Updater").

**Who.** Rust only, through `tauri-plugin-updater` (already registered
under `#[cfg(desktop)]`) and `AppHandle::restart` (core Tauri). The webview
never calls the updater or process plugin; it reads `UpdateStatus` and
calls three commands. No new dependency.

**When.**

- At start: if `Settings.updateCheck` is `true`, `app::setup` spawns one
  check on `tauri::async_runtime` once the settings are loaded. No delay,
  no retry, no periodic timer. With the setting off the status stays
  `idle`.
- By hand: `check_for_updates` ("Check now" in Settings), whatever the
  setting says. Turning the setting on does not start a check by itself.
- From the tray: the menu item "Check for updates" (Tray menu) runs the
  same check as `check_for_updates`, whatever the setting says.
- Install: only on `install_update` (the user clicked "Update"). Nothing
  installs silently.

**State.** The status lives in a separately managed `UpdateState`
(`src-tauri/src/app/updates.rs`), not in `AppState`. It holds the current
`UpdateStatus` (start: `idle`) and, after an `available` result, the plugin's
`Update` value (it is `Clone + Send + Sync`) for the install. Every transition is stored, then
emitted as `update-status-changed` with the full status.

**Check** (`check_for_updates` and the start check):

1. Busy (`UpdateStatus::is_busy`: `checking`, `downloading` or
   `installing`): return the current status, start nothing.
2. Read `plugins.updater.pubkey` from `app.config().plugins` (a map of
   plugin name to JSON). Missing or blank: status `unavailable`, no
   request. This is the dev case and every build without updater
   artifacts; it must never panic. If building the updater
   (`app.updater_builder()...build()`) fails: `unavailable` as well, error
   logged.
3. Status `checking` (emitted), then build the updater with
   `.timeout(Duration::from_secs(20))` and `check().await`:
   - `Ok(Some(update))`: `available { version, url, checkedAt }`.
     `version` is `update.version` without a leading "v"; `url` is
     `update::release_page_url(version)`, the GitHub release page (tags
     are `v<version>`). `update.body` (release notes) is not used.
   - `Ok(None)`: `upToDate { checkedAt }` (the plugin returns `None` when
     the remote version is not newer than the running one).
   - `Err(e)`: `checkFailed { checkedAt, detail: Some(e.to_string()) }`.
     Offline and timeouts surface as `Error::Reqwest`; a 404 on
     `latest.json` (the latest published release has no updater
     artifacts) as
     `Error::ReleaseNotFound`; invalid JSON as `Error::Serialization`; a
     release without this platform as `Error::TargetNotFound`. All map to
     `checkFailed`, logged, no popup, no notification.
4. Resolve the command with the final status.

**Install** (`install_update`):

1. Busy: return the current status. No installable version
   (`UpdateStatus::installable_version` is `None`): reject with
   `CommandError { code: invalidArgument }`.
2. Status `downloading { version, percent: null }` (emitted), then
   `update.download_and_install(on_chunk, on_download_finish).await`
   (TKM-212). Signature in tauri-plugin-updater 2.x (docs.rs,
   `Update::download_and_install`):
   `on_chunk: FnMut(usize, Option<u64>)` gets the length of each received
   chunk and the response's `Content-Length` (`None` when missing or
   unparsable); `on_download_finish: FnOnce()` runs once after the last
   chunk, before the signature check and the install.
   - `on_chunk`: add the chunk length to a running `u64` total, compute
     `update::download_percent(total, content_length)` and, only if it
     differs from the stored percent, store and emit
     `downloading { version, percent }`. So at most 101 progress events
     per download; with an unknown size the percent stays `null` and
     nothing more is emitted. The callback is synchronous: store and emit
     without awaiting, never block on the download.
   - `on_download_finish`: status `installing { version }` (emitted). It
     covers the signature check and the install.
3. On error: `installFailed { version, url, detail }`, resolve with it.
   The stored `Update` stays, so "Try again" repeats step 2 (from
   `downloading`, percent `null`). A failed signature check arrives here
   after `installing` was emitted, which is fine: `installFailed` follows.
4. On success:
   - **Windows**: the plugin launches the installer and the process ends
     inside the install call (`std::process::exit(0)`); the command never
     resolves. The plugin's `on_before_exit` hook runs just before; no
     cleanup is needed there (SQLite runs in WAL mode, ingest commits per
     batch). The installer runs in the plugin's default `passive` mode
     (progress window, no questions) and starts the app again afterwards
     (`restart_after_install`, default `true`).
   - **macOS**: the plugin replaces the `.app` bundle (atomic swap, or an
     administrator prompt if the bundle is not writable), then Rust calls
     `app.restart()` from the async task (not the main thread, so the exit
     events are delivered). The new version starts; the command never
     resolves.

**Signature.** The plugin verifies every download against the configured
public key; this cannot be turned off. With an empty key the app never
gets that far (check step 2).

**Request.** A GET to the endpoint in `tauri.conf.json`
(`https://github.com/mango420/tokenmonk/releases/latest/download/latest.json`),
no body, no query, headers `Accept: application/json` and
`User-Agent: tauri-plugin-updater/<plugin version>`. GitHub answers with a
redirect to the file of the latest release. The download is a GET of the
platform `url` from `latest.json` (GitHub redirects it to its file host).
README "Privacy" states this for users.

**UI.** `get_update_status` on start, then `update-status-changed`.
"Update" and "Try again" call `install_update`, "Check now" calls
`check_for_updates`; the release link opens `url` with the opener plugin
(`openUrl`; https URLs are allowed by `opener:default`). Copy and
placement: DESIGN.md ("Update line", "Settings").

## Tray menu

Built in `tray::init` with `tauri::menu` items, handled in the tray's
`on_menu_event`. Ids are internal to `tray` (not contracts):

| Id | Label | Action |
|---|---|---|
| `open` | "Open Tokenmonk" | `window::show` |
| `settings` | "Settings" | `window::show`, then emit `navigate` with `NavigateTo { view: settings }` |
| `check-updates` | "Check for updates" | as `settings`, then spawn the update check on `tauri::async_runtime` |
| (separator) | | |
| `quit` | "Quit" | `app.exit(0)` |

- `check-updates` calls the same function as the `check_for_updates`
  command (`app::check_update`, Updates, "Check"), so the busy rule, the
  missing-pubkey case and the status mapping are identical. The handler
  does not wait for the result and shows nothing itself; every
  transition goes out as `update-status-changed` and the Settings update
  row renders it (DESIGN.md, "Tray menu").
- Order: `navigate` is emitted before the check starts, so Settings is
  the visible view when `checking` arrives. If the UI is not listening
  yet it still gets the status from `get_update_status` on start.
- No new command, event or payload; `View::Settings` and
  `update-status-changed` already exist. No new dependency.
- No "About" item (TKM-125): About is reached from the Settings footer
  only, so `tray` needs no change. `View::About` exists for the UI's own
  navigation; Rust never emits `navigate` with it.

### Hide on blur vs. keep open

- Default (`Settings.keep_open = false`): `window` hides the popup once a
  blur has settled (`BLUR_SETTLE`); a tray click right after such a hide
  (`BLUR_GRACE`) only dismisses, it does not reopen.
- `keep_open = true`: blur is ignored, the popup stays visible (it is
  `alwaysOnTop`) and hides only via the tray icon toggle or the title
  bar "Close" (`CloseRequested`). Read from the
  cached `Settings` on each blur, so a change via `update_settings`
  applies at once without a restart.
- Stored like the other switches; a stored settings set without the
  field loads as `false` (`#[serde(default)]`). No new command or event.

## Commands and events

Defined in the contracts (names as constants on both sides):

| Command | Returns |
|---|---|
| `get_dashboard` | `DashboardSnapshot` |
| `get_settings` | `Settings` |
| `update_settings { patch }` | `Settings` |
| `get_game_state` | `GameState` |
| `get_app_info` | `AppInfo` |
| `get_update_status` | `UpdateStatus` |
| `check_for_updates` | `UpdateStatus` after the check (see Updates) |
| `install_update` | `UpdateStatus`, only on failure; success exits or restarts the app |
| `get_claude_folder` | `ClaudeFolder` (effective folder, its source, whether `projects/` exists; see Claude folder) |
| `get_limits_connection` | `LimitsConnection` (see Limits, "Connect limits") |
| `connect_limits` | `LimitsConnection`; rejects with `claudeSettings` |
| `disconnect_limits` | `LimitsConnection`; rejects with `claudeSettings` |

| Event | Payload |
|---|---|
| `dashboard-updated` | `DashboardSnapshot` |
| `settings-changed` | `Settings` |
| `game-updated` | `GameState` |
| `achievement-unlocked` | `AchievementUnlocked` |
| `level-up` | `LevelUp` |
| `app-info-changed` | `AppInfo` |
| `navigate` | `NavigateTo` (tray menu opens settings etc.; views: dashboard, settings, achievements, about) |
| `update-status-changed` | `UpdateStatus` |
| `limits-connection-changed` | `LimitsConnection` |

Payloads are full state, never diffs. On start the UI calls the `get_*`
commands once, then follows the events. Every command returns
`Result<T, CommandError>`.

Serialization: struct fields camelCase; unit enums as camelCase strings;
the one data-carrying enum (`UpdateStatus`) is internally tagged with
`"kind"`, variant and field names camelCase (`{ "kind": "upToDate",
"checkedAt": 0 }`), a discriminated union in TypeScript; `Option` is
`null`; timestamps are epoch milliseconds. Two marked exceptions:
`SettingsPatch` fields are `skip_serializing_if = "Option::is_none"` (TS
optional keys), and `SettingsPatch.claudeFolder` is a double option
(absent = unchanged, `null` = reset); `Settings.claudeFolder` is omitted
when `None` and reads absent or `null` as `None` (TS `claudeFolder?:
string | null`, both meaning "not set"). `Settings.notifyLimits` and
`.notifyLevelUps` read absent as `true` (`serde(default = ...)`).

One mode: `Mode` keeps the value `diy` only to read legacy ledger rows
(`xp_events.mode`, see Storage and Game). Since TKM-324 every install is
`max` on open (Storage), so `Settings.mode` and `GameState.mode` are
`max`; `noneYet` stays a wire value but no longer occurs, and is not a
first-start signal. `SettingsPatch` has no `mode`, and no command sets the
mode.

## Storage

- One SQLite file in the platform app data directory. Never inside the
  repository, never in the Claude folder.
- Schema v1: [`schema_v1.sql`](../src-tauri/src/contracts/schema_v1.sql).
  The db module applies it as migration 1 and tracks the version with
  `PRAGMA user_version`. Later changes are new numbered migrations, agreed
  with the architect first; `SCHEMA_VERSION` in the contracts follows.
- Tables: `source_files` (read offsets), `messages` (deduplicated usage),
  `limit_snapshots`, `settings`, `xp_events`, `achievements`, `app_state`.
- `settings` keys: `mode`, `sound`, `autostart`, `update_check` (JSON
  boolean, since TKM-41). A missing key means the default
  (`update_check`: `true`); new keys need no migration. `claude_folder`
  (JSON string, TKM-291): the absolute path; missing key = not set, a
  reset deletes the key. `notify_limits`, `notify_level_ups` (JSON
  boolean, TKM-300): missing key = `true`. A stored `mode` of `"diy"` is
  migrated to `"max"` (TKM-301); `xp_events.mode` keeps its history.
  Since TKM-324 a missing or `"noneYet"` `mode` becomes `"max"` on open
  too, so every install starts in `max`.
- First start (decision 2026-11-17, TKM-329): the popup opens by itself
  once, on the very first start, tracked by a first-start flag in
  `app_state`. Later starts show only the tray icon. The mode plays no
  part in it.
- `app_state` key `limit_notice_cycles` (TKM-300, owned by backend-data,
  read and written through `Db` by the notification code of TKM-304):
  JSON `{ "fiveHour": <resetsAt> | null, "weekly": <resetsAt> | null }`,
  the `resetsAt` (epoch ms) of the last cycle whose 10 % notification was
  sent per window. A notification is due when the window's current
  `resetsAt` differs from the stored one; missing key or `null` = none
  sent. No migration needed.
- `UpdateStatus` is not stored; every start begins at `idle`.
- If the database is corrupt or from a newer version, it is moved aside
  (renamed with a timestamp) and rebuilt from the logs; game progress is
  rebuilt by the retroactive replay.

## Error and empty states

| Situation | Core behaviour | What the UI gets |
|---|---|---|
| Very first start | Mode reads as `max` (TKM-324), ingest and game run; the popup opens by itself once, set by a first-start flag in `app_state` (TKM-329) | `Settings.mode = max`, `data.state = importing` |
| First import running | Import in background | `data.state = importing` with file counts |
| No Claude folder or no logs | Keep watching the root for it to appear | `data.state = noLogs`, zero totals, `AppInfo.logsFound = false` |
| Log folder unreadable | Retry on the rescan interval | `data.state = error`, `detail` |
| Malformed or partial line | Skip / retry later, count it | nothing visible |
| Bridge not set up, or no current window | | `limits.status = unavailable`, `reason = noSource` |
| Limit file unreadable or invalid | Keep last good reading | `limits.status` by its age (`unavailable` without one), `reason` |
| A window of the last good reading has reset | Drop that window (`resetsAt <= now`) | that window `null`; no window left: `unavailable`, `reason = noSource` |
| `settings.json` unusable for connect | Touch nothing | `LimitsConnection.state = blocked`, `problem`; connect or disconnect rejects with `claudeSettings` |
| Connect or disconnect write fails | File unchanged | `CommandError { code: claudeSettings }` |
| No session in the last 60 min | | `session = null` |
| Database error in a command | | `CommandError { code: storage }` |
| Command before the core is ready | | `CommandError { code: notReady }`; the UI retries on the first event |
| Empty updater public key (dev, builds without updater artifacts) | No request | `UpdateStatus = unavailable` |
| Offline, timeout, 404 or invalid `latest.json` | Log, no retry | `UpdateStatus = checkFailed` |
| Download, signature or install error | Log, keep the update | `UpdateStatus = installFailed` |

The tone of every message follows DESIGN.md: calm, dry, number first.

## Changing contracts

Contracts are fixed for the wave. A change request goes to the architect,
who updates Rust, TypeScript, the schema (as a new migration) and this
document in one PR.
