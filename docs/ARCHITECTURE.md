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
| commands | `src-tauri/src/commands/` | `#[tauri::command]` handlers; thin, read cached state or delegate. |
| tray, window | `src-tauri/src/{tray,window}/` | Tray icon with live halo, popup window placement and show/hide. |
| UI | `src/` | React views: onboarding, dashboard, settings, toasts, ghost overlay. |

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
                                     update tray halo                achievement-unlocked, level-up
                                                     |
                                                     v
                                          UI (React) renders; commands for reads and settings
```

## Threads

- **Main thread**: Tauri event loop, tray, windows. Never blocks on I/O.
- **Ingest worker** (one thread): owns the only write connection to SQLite.
  Receives file-change notifications, reads new bytes, parses, upserts,
  then recomputes the snapshot and the game state, stores both in shared
  state and emits events. Also runs the minute tick (day rollover, burn
  rate decay, session idle cut-off, season check).
- **Limits worker** (one thread): calls `LimitSource::fetch` on a fixed
  interval, hands readings to the ingest worker over a channel. A slow or
  failing source never delays log ingestion.
- **Command handlers** read the cached snapshot and game state from shared
  state (`Arc<RwLock<..>>` or similar); writes such as settings go through
  the ingest worker or a short-lived write on the same connection guarded by
  a mutex. Commands never parse logs.

The database runs in WAL mode so reads never wait for the writer.

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
- **Limit status**: `ok` if the last good reading is at most 15 minutes old,
  `stale` if older (values still shown), `unavailable` if there is none or
  the source reports `noSource`. `reason` carries the last error.

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

- The trait and its data types are the only contract. No endpoint, URL or
  credential handling is defined here; a concrete online source is added
  only once verified, as a new implementation behind the trait.
- v0.1 ships with a source that always returns `LimitError { reason:
  noSource }` unless a verified source exists. The app then runs on token
  data alone: Max it out shows token figures and the footer explains that
  limit data is unavailable; Do it yourself uses the `tokenHistory` budget.
- Poll interval: every 5 minutes while the app runs, plus once when the
  popup opens if the last reading is older than 1 minute. Each good reading
  is stored in `limit_snapshots`.
- Limit percentages include claude.ai chats; token statistics are Claude
  Code only. The popup footer says so permanently.

## Game

- XP is a ledger (`xp_events`); level, progress and title derive from the
  sum. One XP pool for both modes; each event records the mode it was
  earned in; titles depend on the current mode and the level.
- The game module defines XP rules, the level curve, titles and achievement
  thresholds. Contracts only carry ids and numbers.
- Achievements (v0.1 minimum): First Blood, Night Owl, Rate-Limit Rambo,
  So Close, Token Monk, Boo (secret, easter egg).
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

## Season and easter egg

- The season is computed in Rust from local time (Halloween: 24 Oct 00:00
  to 2 Nov 23:59, both days included) because the tray icon needs it. It is
  exposed as `AppInfo.season`; the minute tick emits `app-info-changed` when
  it flips. The UI maps it to `data-season` on `<html>`.
- Ghost easter egg (all year): the UI counts logo clicks; 3 clicks, each
  within 800 ms of the previous one, show the ghost overlay and call
  `trigger_easter_egg`. Rust unlocks Boo once (`newlyUnlocked = true`, then
  `achievement-unlocked` and `game-updated`); repeats return
  `newlyUnlocked = false` and change nothing.

## Commands and events

Defined in the contracts (names as constants on both sides):

| Command | Returns |
|---|---|
| `get_dashboard` | `DashboardSnapshot` |
| `get_settings` | `Settings` |
| `update_settings { patch }` | `Settings` |
| `complete_onboarding { mode }` | `Settings` |
| `get_game_state` | `GameState` |
| `trigger_easter_egg` | `EasterEggResult` |
| `get_app_info` | `AppInfo` |

| Event | Payload |
|---|---|
| `dashboard-updated` | `DashboardSnapshot` |
| `settings-changed` | `Settings` |
| `game-updated` | `GameState` |
| `achievement-unlocked` | `AchievementUnlocked` |
| `level-up` | `LevelUp` |
| `app-info-changed` | `AppInfo` |
| `navigate` | `NavigateTo` (tray menu opens settings etc.) |

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
| First start, onboarding pending | Ingest runs, game waits | `Settings.mode = noneYet` -> onboarding view |
| First import running | Import in background | `data.state = importing` with file counts |
| No Claude folder or no logs | Keep watching the root for it to appear | `data.state = noLogs`, zero totals, `AppInfo.logsFound = false` |
| Log folder unreadable | Retry on the rescan interval | `data.state = error`, `detail` |
| Malformed or partial line | Skip / retry later, count it | nothing visible |
| No limit source or it fails | Keep last good reading | `limits.status = unavailable` or `stale`, `reason` |
| No session in the last 60 min | | `session = null` |
| No budget basis | | `dailyBudget = null` |
| Database error in a command | | `CommandError { code: storage }` |
| Command before the core is ready | | `CommandError { code: notReady }`; the UI retries on the first event |

The tone of every message follows DESIGN.md: calm, dry, number first.

## Changing contracts

Contracts are fixed for the wave. A change request goes to the architect,
who updates Rust, TypeScript, the schema (as a new migration) and this
document in one PR.
