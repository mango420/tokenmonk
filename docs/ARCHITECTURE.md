# Architecture

Tokenmonk v0.1 is a Tauri 2 tray app for macOS and Windows. Rust owns all
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
| commands | `src-tauri/src/commands/` | `#[tauri::command]` handlers; thin, read cached state or delegate. |
| tray, window | `src-tauri/src/{tray,window}/` | Tray icon with live ring (5-hour percent left), popup window placement and show/hide. |
| UI | `src/` | React views: onboarding, dashboard, settings, achievements, toasts, level-up screen. |

The UI never touches files, the database or the network.

## Data flow

```
~/.claude/projects/**/*.jsonl            (read only)
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
- **Limit ETA** (Max it out burn line): needs a 5-hour reading. Percent per
  token in the current window = `fiveHour.utilization` / headline tokens
  since the window start (`resetsAt - 5 h`); ETA = remaining percent /
  (burn rate x percent per token), rounded down to whole minutes. `null`
  without a reading, without tokens in the window or with zero burn; `0` at
  or above 100 %. Because the limit also counts claude.ai chats, the ETA is
  an estimate, which the UI copy reflects ("~").
- **Daily budget**:
  - `weeklyLimit` basis (weekly reading available): `W0` = weekly
    utilisation at the start of the local day (last snapshot before
    midnight, else the first one of the day). `daysLeft` = local calendar
    days from today to the reset day, both included (at least 1).
    `allowance = (100 - W0) / daysLeft`; `usedToday = Wnow - W0`;
    `usedPercent = usedToday / allowance x 100`. If the weekly window resets
    during the day, `W0` restarts at the reset.
  - `tokenHistory` basis (no weekly reading; decided fallback, used by Do
    it yourself when there is no limit data): `allowance` = average
    headline tokens per active day over the previous 7 local days (today
    excluded); `usedToday` = headline tokens today; `daysLeft = 1`,
    `resetsAt = null`. `dailyBudget` is `null` if there is no active day
    in that range.
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
  small file. Tokenmonk never edits Claude Code settings; the user pastes
  one setup snippet (DESIGN.md, Settings, "Limit data"). Sources and
  verified facts: DECISIONS.md, "Limit source".
- Without a usable reading the app runs on token data alone: Max it out
  shows token figures and explains that limit data is missing; Do it
  yourself uses the `tokenHistory` budget.
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
- No `refreshInterval` in the snippet: a timer re-run resends the last
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

### Setup snippet

- `get_limit_setup` returns `LimitSetup { statuslineCommand }`: the current
  executable path (`std::env::current_exe()`) plus ` --statusline`. On
  Windows the path uses forward slashes (Claude Code runs the command
  through Git Bash when installed, which drops unquoted backslashes, else
  through PowerShell). The path is wrapped in double quotes only if it
  contains a space.
- `statuslineCommand` is always the POSIX shell form (macOS shell, Git
  Bash on Windows). Without a space the unquoted form is the same in Git
  Bash and PowerShell. With a space, PowerShell needs the call operator:
  `& "C:/Example Dir/tokenmonk.exe" --statusline`, which is a syntax error
  in Bash, so one string cannot serve both. Rule, decided in the UI with
  no contract change: if `AppInfo.platform = windows` and
  `statuslineCommand` starts with `"`, Settings shows the Git Bash form as
  the default snippet and a second, alternative snippet whose command is
  `"& " + statuslineCommand`, labelled for Claude Code without Git Bash
  (DESIGN.md, Settings, "Limit data"). Rust never builds the PowerShell
  form.
- The UI builds the snippet with `JSON.stringify`, so quotes are escaped:
  `{ "statusLine": { "type": "command", "command": <statuslineCommand> } }`,
  to be merged into the user settings file `~/.claude/settings.json`.
- Chaining an existing status line: Tokenmonk does not run other commands
  in v0.1. A user with their own script adds one line that pipes the same
  input to the bridge and drops its output, for scripts that keep stdin in
  `$input` as the official examples do:
  `printf '%s' "$input" | <statuslineCommand> >/dev/null`. A built-in
  chain flag and PowerShell scripts are out of scope for v0.1.

### To verify during implementation

- Windows release builds use `windows_subsystem = "windows"`. Check that
  the bridge reads piped stdin and that its stdout reaches Claude Code when
  started from Git Bash and from PowerShell. If not, the fallback is a
  separate small console binary, which needs a `Cargo.toml` change
  (orchestrator).
- macOS: check that running the binary inside the ad-hoc signed `.app`
  from a shell works after the first approval.
- Both Windows snippet forms (unquoted path, and `& "path"` with a space)
  under the PowerShell fallback.

### Implementation notes

For backend-platform (and the orchestrator where marked):

1. `commands`: add `get_limit_setup` returning `LimitSetup` (a stub
   returning `statuslineCommand: null` is fine until the path logic
   lands); register it in `lib.rs` next to the other commands
   (orchestrator, shared hotspot).
2. `lib.rs::run` (orchestrator, shared hotspot): if the first argument is
   `--statusline`, run the bridge and return before the Tauri builder is
   created, so no plugin (single-instance included) starts.
3. `limits/bridge.rs`: bridge mode and the shared path helper
   (`dirs::data_dir()` + `app.tokenmonk.desktop` + `statusline.json`).
4. `limits/statusline.rs`: `StatuslineSource` (id `"statusline"`), and
   `default_source()` returns it.
5. `limits/worker.rs`: `POLL_INTERVAL` becomes 30 s; `MIN_REFRESH_GAP`
   stays 1 min.
6. Ingest worker (backend-data): store a reading in `limit_snapshots` only
   if its `fetchedAt` is strictly newer than the newest stored one (one
   statement, `Db::insert_limit_snapshot`).
7. Aggregate (backend-data): drop expired windows from the last good
   reading on every snapshot build (Aggregates, "Limit status").

## Game

- XP is a ledger (`xp_events`); level, progress and title derive from the
  sum. One XP pool for both modes; each event records the mode it was
  earned in; the title depends on the current mode only: "Limit Chaser"
  (Max it out, also before onboarding) and "Quiet Spender" (Do it
  yourself), at every level.
- The game module defines XP rules, the level curve, titles and achievement
  thresholds. Contracts only carry ids and numbers.
- Achievements (v0.1), name, description, rule and XP:

  | Achievement | Description | Unlocks when | XP |
  |---|---|---|---|
  | First Blood | "Your first token, spent." | first counted message | 10 |
  | Night Owl | "Coding after midnight." | a message between 00:00 and 04:00 local time | 50 |
  | Rate-Limit Rambo | "Hit the limit. On purpose, we assume." | a 5-hour reading at or above 100 % | 100 |
  | So Close | "Stopped at 99 percent. Restraint or luck." | a finished 5-hour window peaking at 99 % or more, below 100 % | 60 |
  | Token Monk | "A full day under budget." | one finished local day with a daily budget and `usedPercent` below 100 | 150 |

  Mode restrictions and the level curve are unchanged. Plus Boo (shown as
  "Halloween 2026"): a retired, secret legacy badge that can no longer be
  earned; existing unlocks stay valid and keep their XP. Already stored
  unlocks and XP events are not rewritten when these rules change.
- **Retroactive levelling**: after onboarding is completed for the first
  time, the game replays the stored history in time order, writing
  `xp_events` and unlocks with `retroactive = 1`. During the replay no
  `achievement-unlocked` or `level-up` events are emitted; one
  `game-updated` follows at the end with `retroDone = true`, and the UI may
  show one summary instead of a toast storm. `app_state.retro_done` makes
  it run once.
- Live evaluation runs after every ingest batch and on the minute tick;
  each unlock emits `achievement-unlocked`, each level change `level-up`,
  then `game-updated`.
- `(kind, ref)` uniqueness in `xp_events` makes every award idempotent.

## Commands and events

Defined in the contracts (names as constants on both sides):

| Command | Returns |
|---|---|
| `get_dashboard` | `DashboardSnapshot` |
| `get_settings` | `Settings` |
| `update_settings { patch }` | `Settings` |
| `complete_onboarding { mode }` | `Settings` |
| `get_game_state` | `GameState` |
| `get_app_info` | `AppInfo` |
| `get_limit_setup` | `LimitSetup` (status-line setup command, see Limits) |

| Event | Payload |
|---|---|
| `dashboard-updated` | `DashboardSnapshot` |
| `settings-changed` | `Settings` |
| `game-updated` | `GameState` |
| `achievement-unlocked` | `AchievementUnlocked` |
| `level-up` | `LevelUp` |
| `app-info-changed` | `AppInfo` |
| `navigate` | `NavigateTo` (tray menu opens settings etc.; views: dashboard, settings, onboarding, achievements) |

Payloads are full state, never diffs. On start the UI calls the `get_*`
commands once, then follows the events. Every command returns
`Result<T, CommandError>`.

Serialization: struct fields camelCase; enums are unit enums as camelCase
strings (a future data-carrying enum would be internally tagged with
`"kind"`); `Option` is `null`; timestamps are epoch milliseconds.

## Storage

- One SQLite file in the platform app data directory. Never inside the
  repository, never in the Claude folder.
- Schema v1: [`schema_v1.sql`](../src-tauri/src/contracts/schema_v1.sql).
  The db module applies it as migration 1 and tracks the version with
  `PRAGMA user_version`. Later changes are new numbered migrations, agreed
  with the architect first; `SCHEMA_VERSION` in the contracts follows.
- Tables: `source_files` (read offsets), `messages` (deduplicated usage),
  `limit_snapshots`, `settings`, `xp_events`, `achievements`, `app_state`.
- If the database is corrupt or from a newer version, it is moved aside
  (renamed with a timestamp) and rebuilt from the logs; game progress is
  rebuilt by the retroactive replay.

## Error and empty states

| Situation | Core behaviour | What the UI gets |
|---|---|---|
| First start, onboarding pending | Ingest runs, game waits; the popup opens by itself on every start while onboarding is pending | `Settings.mode = noneYet` -> onboarding view |
| First import running | Import in background | `data.state = importing` with file counts |
| No Claude folder or no logs | Keep watching the root for it to appear | `data.state = noLogs`, zero totals, `AppInfo.logsFound = false` |
| Log folder unreadable | Retry on the rescan interval | `data.state = error`, `detail` |
| Malformed or partial line | Skip / retry later, count it | nothing visible |
| Bridge not set up, or no current window | | `limits.status = unavailable`, `reason = noSource` |
| Limit file unreadable or invalid | Keep last good reading | `limits.status` by its age (`unavailable` without one), `reason` |
| A window of the last good reading has reset | Drop that window (`resetsAt <= now`) | that window `null`; no window left: `unavailable`, `reason = noSource` |
| No session in the last 60 min | | `session = null` |
| No budget basis | | `dailyBudget = null` |
| Database error in a command | | `CommandError { code: storage }` |
| Command before the core is ready | | `CommandError { code: notReady }`; the UI retries on the first event |

The tone of every message follows DESIGN.md: calm, dry, number first.

## Changing contracts

Contracts are fixed for the wave. A change request goes to the architect,
who updates Rust, TypeScript, the schema (as a new migration) and this
document in one PR.
