/**
 * Shared contracts between the Rust core and the web UI.
 *
 * 1:1 mirror of `src-tauri/src/contracts/`. Change both sides together, and
 * only through the architect (see docs/ARCHITECTURE.md).
 *
 * Wire rules (set by serde on the Rust side):
 * - Fields are camelCase (`rename_all = "camelCase"`).
 * - Unit enums are camelCase string literals.
 * - Data-carrying enums (only `UpdateStatus`) are internally tagged
 *   (`tag = "kind"`): a discriminated union on `kind`, variant names and
 *   their fields camelCase.
 * - `Option<T>` is `T | null` (never omitted), except in patch types where
 *   an absent field means "unchanged" (`field?: T`).
 * - Timestamps are `EpochMs`: Unix epoch milliseconds, UTC.
 * - Command names are snake_case, command arguments single lowercase words,
 *   event names kebab-case.
 */

/** Unix epoch milliseconds, UTC. */
export type EpochMs = number;

// ---------------------------------------------------------------- settings

/** Play mode. `noneYet` means onboarding has not been completed. Max is the
 * only mode since TKM-287; `"diy"` is removed and stays only until
 * TKM-301/302/306/307 stop using it (cleanup in TKM-300). Never produce it
 * in new code; stored `diy` is migrated to `max` (TKM-301). */
export type Mode = "noneYet" | "max" | "diy";

export interface Settings {
  mode: Mode;
  /** Play the in-app tone on achievement unlocks and level-ups. Does not
   * apply to system notifications; they use the OS sound. */
  sound: boolean;
  /** Start Tokenmonk at login. */
  autostart: boolean;
  /** "Check for updates automatically": check once at app start. Manual
   * checks ("Check now") work either way. Default true. */
  updateCheck: boolean;
  /** "Keep popup open": no hide on blur, the popup closes only via the
   * tray icon or its Close button. Default false. */
  keepOpen: boolean;
  /** "Limits at 10 % left": one system notification per limit window and
   * reset cycle at 10 % left. Default true. Rust: `bool`, always sent. */
  notifyLimits: boolean;
  /** "Level-ups": a system notification per live level-up. Default true.
   * Rust: `bool`, always sent. */
  notifyLevelUps: boolean;
  /** "Claude folder": the Claude config folder chosen in Settings (the app
   * appends `projects/` itself). Absent or null = not set: the folder then
   * comes from `CLAUDE_CONFIG_DIR` or the default `~/.claude` (see
   * `ClaudeFolder`). Rust: `Option<String>`, omitted when `None`, accepts
   * absent or null. Optional so a `SettingsPatch` can be spread onto it. */
  claudeFolder?: string | null;
}

/** Partial update for `update_settings`. Absent keys stay unchanged.
 * `mode: "noneYet"` is rejected. `mode` is removed by TKM-287 (no mode
 * picker) and stays only until TKM-307 stops using it. `notifyLimits` /
 * `notifyLevelUps` toggle the two notifications. `claudeFolder`: a string sets the folder
 * (non-empty, absolute, else `invalidArgument`; existence is not checked),
 * `null` resets it to the default. Rust: `Option<Option<String>>`. */
export interface SettingsPatch {
  mode?: Mode;
  sound?: boolean;
  autostart?: boolean;
  updateCheck?: boolean;
  keepOpen?: boolean;
  notifyLimits?: boolean;
  notifyLevelUps?: boolean;
  claudeFolder?: string | null;
}

// ------------------------------------------------------------------ limits

export interface LimitWindow {
  /** Used share in percent, 0..100 (may exceed 100). */
  utilization: number;
  resetsAt: EpochMs | null;
}

/** Raw reading of a limit source (Rust `LimitReading`). */
export interface LimitReading {
  fiveHour: LimitWindow | null;
  weekly: LimitWindow | null;
  fetchedAt: EpochMs;
}

export type LimitStatus = "ok" | "stale" | "unavailable";

/** `noSource`: no limit source is set up (v0.1: the status-line bridge has
 * not written a reading yet, or no window in it is current). */
export type LimitReason =
  | "noSource"
  | "notSignedIn"
  | "network"
  | "invalidResponse"
  | "other";

export interface LimitsView {
  status: LimitStatus;
  /** `null` when `status` is `ok`. */
  reason: LimitReason | null;
  detail: string | null;
  source: string | null;
  fiveHour: LimitWindow | null;
  weekly: LimitWindow | null;
  fetchedAt: EpochMs | null;
}

/** Result of `get_limit_setup`: the bridge command and the bridge file
 * state. The setup itself is one click since TKM-287 (`LimitsConnection`). */
export interface LimitSetup {
  /** Ready-to-paste value for `statusLine.command` in Claude Code's
   * `settings.json` (executable path plus `--statusline`); null if the
   * executable path is unknown. Removed by TKM-287 (paste-only setup);
   * stays until TKM-307 stops showing the snippet. */
  statuslineCommand: string | null;
  /** State of the bridge file `statusline.json` when the command ran. Lets
   * Settings tell "never arrived" from "arrived, no current window". */
  statuslineFile: StatuslineFileInfo;
}

/** What the app knows about the bridge file, without judging it. */
export interface StatuslineFileInfo {
  /** The file exists (false also when the app data directory is unknown). */
  exists: boolean;
  /** `receivedAt` from the file: when the bridge last ran. Null if the file
   * is missing, unreadable or not a valid bridge file. */
  receivedAt: EpochMs | null;
  /** File modification time; useful when the file exists but
   * `receivedAt` is null. Null if missing or not reported by the OS. */
  modifiedAt: EpochMs | null;
}

/** Whether Claude Code's `statusLine` runs the Tokenmonk bridge.
 * `notConnected`: no Tokenmonk entry (or no `settings.json` yet).
 * `connected`: it runs the bridge, alone or wrapping the previous status
 * line. `blocked`: the file cannot be used, see `problem`; nothing is
 * touched. */
export type LimitsConnectionState = "notConnected" | "connected" | "blocked";

/** Why a connection is `blocked`. `noClaudeFolder`: the Claude folder is
 * unknown or missing (never created by Tokenmonk). `settingsUnreadable`:
 * `settings.json` exists but cannot be read. `settingsInvalid`: not a JSON
 * object, or `statusLine` present but not an object. */
export type LimitsConnectionProblem =
  | "noClaudeFolder"
  | "settingsUnreadable"
  | "settingsInvalid";

/** Result of `get_limits_connection`, `connect_limits`, `disconnect_limits`
 * and payload of `limits-connection-changed`. Read fresh on every call. */
export interface LimitsConnection {
  state: LimitsConnectionState;
  /** Absolute path of `settings.json` in the effective Claude folder; null
   * only when the Claude folder is unknown. */
  settingsPath: string | null;
  /** Another status line is set up: wrapped (output unchanged) when
   * `connected`, will be wrapped on connect when `notConnected`. Always
   * false while `blocked`. */
  previousStatusLine: boolean;
  /** Set exactly when `state` is `blocked`. */
  problem: LimitsConnectionProblem | null;
  /** Technical detail for logs, never a headline. */
  detail: string | null;
}

// --------------------------------------------------------------- dashboard

/** Token counts, deduplicated per API message. */
export interface TokenBreakdown {
  input: number;
  output: number;
  cacheCreation: number;
  cacheRead: number;
  /** Headline figure: input + output + cacheCreation. */
  total: number;
}

export interface SessionStats {
  sessionId: string;
  /** Last path component of the session's working directory. */
  projectName: string;
  startedAt: EpochMs;
  lastActivityAt: EpochMs;
  tokens: TokenBreakdown;
}

export interface BurnRate {
  /** Headline tokens per minute over the trailing window. */
  tokensPerMinute: number;
  windowMinutes: number;
  /** Minutes until the 5-hour limit at this pace; null without limits or burn, 0 when reached.
   * Removed by TKM-287 (no ETA); stays until TKM-301/304/306 stop using it. */
  limitEtaMinutes: number | null;
}

/** Removed by TKM-287 (no budget); stays until TKM-301/306 stop using it. */
export type BudgetBasis = "weeklyLimit" | "tokenHistory";

/** Removed by TKM-287 (no budget); stays until TKM-301/302/306 stop using it. */
export interface DailyBudget {
  basis: BudgetBasis;
  /** Today's allowance in the unit of `basis` (weekly % points or tokens). */
  allowance: number;
  usedToday: number;
  /** usedToday / allowance * 100; may exceed 100. */
  usedPercent: number;
  /** Days until the weekly reset, today included (at least 1). */
  daysLeft: number;
  resetsAt: EpochMs | null;
  /**
   * Headline tokens that may still be used per day, today included, until the
   * weekly reset; an estimate (the weekly limit also counts claude.ai chats).
   * weeklyLimit basis only: tokens since the weekly window start per weekly
   * percent point, times (100 - utilization) / daysLeft, at least 0.
   * null with tokenHistory, at 0 % weekly utilization or without tokens.
   */
  tokensLeftPerDay: number | null;
}

export type DataState = "importing" | "ready" | "noLogs" | "error";

export interface DataStatus {
  state: DataState;
  filesTotal: number;
  filesDone: number;
  lastMessageAt: EpochMs | null;
  detail: string | null;
}

export interface DashboardSnapshot {
  generatedAt: EpochMs;
  today: TokenBreakdown;
  session: SessionStats | null;
  burnRate: BurnRate;
  limits: LimitsView;
  /** null when neither limits nor enough token history exist. Removed by
   * TKM-287; stays until TKM-301/306 stop using it. */
  dailyBudget: DailyBudget | null;
  data: DataStatus;
}

// -------------------------------------------------------------------- game

export type AchievementId =
  | "firstBlood"
  | "nightOwl"
  | "rateLimitRambo"
  | "soClose"
  | "tokenMonk"
  /** Legacy "Halloween 2026" badge: retired, no longer earnable. */
  | "boo";

export const ACHIEVEMENT_IDS: readonly AchievementId[] = [
  "firstBlood",
  "nightOwl",
  "rateLimitRambo",
  "soClose",
  "tokenMonk",
  "boo",
] as const;

export interface AchievementProgress {
  id: AchievementId;
  name: string;
  description: string;
  /** Hidden until unlocked (only Boo, which can no longer be earned). */
  secret: boolean;
  /** Only earnable in this mode; null for both. Removed by TKM-287 (one
   * mode); stays until TKM-302 stops using it. */
  mode: Mode | null;
  xp: number;
  unlockedAt: EpochMs | null;
  current: number | null;
  target: number | null;
  /** For example "4 of 7". */
  progressLabel: string | null;
}

export interface AchievementUnlocked {
  id: AchievementId;
  name: string;
  /** For example "Tokens burned at 00:47". */
  detail: string | null;
  xp: number;
  unlockedAt: EpochMs;
  /** From the first-start history replay; not toasted one by one. */
  retroactive: boolean;
}

export interface LevelUp {
  level: number;
  previousLevel: number;
  title: string;
  titleChanged: boolean;
  retroactive: boolean;
}

export interface GameState {
  mode: Mode;
  level: number;
  title: string;
  xpTotal: number;
  xpIntoLevel: number;
  xpForLevel: number;
  /** xpIntoLevel / xpForLevel, 0..1. */
  progress: number;
  /** Do it yourself only: XP if usage stopped now; 0 over budget; null otherwise.
   * Removed by TKM-287; stays until TKM-302/306 stop using it. */
  dailyXpPreview: number | null;
  /** Newest first. */
  recentUnlocks: AchievementUnlocked[];
  /** Display order. */
  achievements: AchievementProgress[];
  retroDone: boolean;
}

// --------------------------------------------------------------------- app

export type Platform = "macos" | "windows" | "linux";

export interface AppInfo {
  version: string;
  platform: Platform;
  schemaVersion: number;
  /** Whether `projects/` exists in the effective Claude folder. */
  logsFound: boolean;
}

/** Environment variable for another Claude config folder (Rust
 * `app::CLAUDE_CONFIG_DIR_ENV`). */
export const CLAUDE_CONFIG_DIR_ENV = "CLAUDE_CONFIG_DIR";

/** Where the effective Claude folder comes from. Order: `setting`
 * (`Settings.claudeFolder`), else `env` (`CLAUDE_CONFIG_DIR`, set and
 * non-empty), else `default` (`.claude` in the home directory). */
export type ClaudeFolderSource = "setting" | "env" | "default";

/** Result of `get_claude_folder`: the Claude config folder in use. Logs are
 * read below `<path>/projects/`; the status-line setup lives in
 * `<path>/settings.json`. */
export interface ClaudeFolder {
  /** Absolute path, for display. Null only with source `default` when the
   * home directory is unknown. */
  path: string | null;
  source: ClaudeFolderSource;
  /** Whether `<path>/projects/` exists (same value as `AppInfo.logsFound`). */
  projectsFound: boolean;
}

/** Public GitHub repository (Rust `app::REPO_URL`), the About screen's
 * "Source" link. No trailing slash. */
export const REPO_URL = "https://github.com/mango420/tokenmonk";

/** GitHub issues (Rust `app::ISSUES_URL`), the About screen's "Ideas and
 * bugs" link and the only contact channel. */
export const ISSUES_URL = "https://github.com/mango420/tokenmonk/issues";

/** Database schema version the app expects (Rust `schema::SCHEMA_VERSION`). */
export const SCHEMA_VERSION = 1;

// ------------------------------------------------------------------ update

/** Base of a version's GitHub release page (Rust `update::RELEASE_PAGE_BASE`);
 * the page of 0.4.1 is this plus "v0.4.1". */
export const RELEASE_PAGE_BASE = "https://github.com/mango420/tokenmonk/releases/tag/";

/**
 * State of the update check (Rust `UpdateStatus`, tagged by `kind`).
 * `idle`: no check since start. `unavailable`: this build cannot update
 * itself (empty updater key, as in development); no request is made.
 * `checkFailed`: offline, network error, timeout, 404 or invalid
 * `latest.json`. `downloading` (TKM-212): `install_update` is fetching the
 * package; `percent` is the whole percent 0 to 100 (Rust
 * `update::download_percent`), `null` while the size is unknown or when the
 * server sends no length. Emitted with `percent: null` first, then once per
 * whole-percent change (at most 101 events); with an unknown size nothing
 * more until `installing`. `installing`: download done, signature check and
 * install running; on success the app exits or restarts, no further status
 * follows. `detail` is technical text for logs only.
 */
export type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "upToDate"; checkedAt: EpochMs }
  | {
      kind: "available";
      /** New version without a leading "v", for example "0.4.1". */
      version: string;
      /** GitHub release page of that version. */
      url: string;
      checkedAt: EpochMs;
    }
  | { kind: "unavailable" }
  | { kind: "checkFailed"; checkedAt: EpochMs; detail: string | null }
  | { kind: "downloading"; version: string; percent: number | null }
  | { kind: "installing"; version: string }
  | { kind: "installFailed"; version: string; url: string; detail: string | null };

export type UpdateStatusKind = UpdateStatus["kind"];

// ---------------------------------------------------------------- commands

export const Commands = {
  getDashboard: "get_dashboard",
  getSettings: "get_settings",
  updateSettings: "update_settings",
  completeOnboarding: "complete_onboarding",
  getGameState: "get_game_state",
  getAppInfo: "get_app_info",
  getLimitSetup: "get_limit_setup",
  getUpdateStatus: "get_update_status",
  /** Resolves when the check is done (at once while one is running). */
  checkForUpdates: "check_for_updates",
  /** Resolves only on failure (`installFailed`); on success the app exits
   * or restarts. Rejects with `invalidArgument` if nothing is installable. */
  installUpdate: "install_update",
  /** No event of its own: call again after `settings-changed` and
   * `app-info-changed`. */
  getClaudeFolder: "get_claude_folder",
  /** Reads `settings.json` fresh; call when Settings opens. */
  getLimitsConnection: "get_limits_connection",
  /** Explicit click only. Idempotent; rejects with `claudeSettings` when
   * blocked or the write fails (file unchanged). Emits
   * `limits-connection-changed` on success. */
  connectLimits: "connect_limits",
  /** Explicit click only. Restores the previous status line (or removes
   * `statusLine`). Idempotent; rejects like `connectLimits`. */
  disconnectLimits: "disconnect_limits",
} as const;

export type CommandName = (typeof Commands)[keyof typeof Commands];

export interface UpdateSettingsArgs {
  patch: SettingsPatch;
}

/** `mode` must be "max" ("diy" is removed by TKM-287 and accepted only
 * until TKM-301/306 drop it). */
export interface CompleteOnboardingArgs {
  mode: Mode;
}

/** Arguments per command (`undefined`: call without arguments). */
export interface CommandArgs {
  get_dashboard: undefined;
  get_settings: undefined;
  update_settings: UpdateSettingsArgs;
  complete_onboarding: CompleteOnboardingArgs;
  get_game_state: undefined;
  get_app_info: undefined;
  get_limit_setup: undefined;
  get_update_status: undefined;
  check_for_updates: undefined;
  install_update: undefined;
  get_claude_folder: undefined;
  get_limits_connection: undefined;
  connect_limits: undefined;
  disconnect_limits: undefined;
}

/** Resolved value per command. */
export interface CommandResults {
  get_dashboard: DashboardSnapshot;
  get_settings: Settings;
  update_settings: Settings;
  complete_onboarding: Settings;
  get_game_state: GameState;
  get_app_info: AppInfo;
  get_limit_setup: LimitSetup;
  get_update_status: UpdateStatus;
  check_for_updates: UpdateStatus;
  install_update: UpdateStatus;
  get_claude_folder: ClaudeFolder;
  get_limits_connection: LimitsConnection;
  connect_limits: LimitsConnection;
  disconnect_limits: LimitsConnection;
}

/** `claudeSettings`: `connect_limits` or `disconnect_limits` could not use
 * `settings.json` in the Claude folder; the file is unchanged. */
export type CommandErrorCode =
  | "invalidArgument"
  | "notReady"
  | "storage"
  | "internal"
  | "claudeSettings";

/** Rejection value of every failed command. */
export interface CommandError {
  code: CommandErrorCode;
  message: string;
}

// ------------------------------------------------------------------ events

export const Events = {
  dashboardUpdated: "dashboard-updated",
  settingsChanged: "settings-changed",
  gameUpdated: "game-updated",
  achievementUnlocked: "achievement-unlocked",
  levelUp: "level-up",
  appInfoChanged: "app-info-changed",
  navigate: "navigate",
  updateStatusChanged: "update-status-changed",
  /** After connect or disconnect succeeded, and after the effective Claude
   * folder changed. */
  limitsConnectionChanged: "limits-connection-changed",
} as const;

export type EventName = (typeof Events)[keyof typeof Events];

/** Popup views (Rust `View`, camelCase). `about`: version, licence and the
 * GitHub links, opened from the Settings footer; Back returns to Settings. */
export type View = "dashboard" | "settings" | "onboarding" | "achievements" | "about";

export interface NavigateTo {
  view: View;
}

/** Payload per event. */
export interface EventPayloads {
  "dashboard-updated": DashboardSnapshot;
  "settings-changed": Settings;
  "game-updated": GameState;
  "achievement-unlocked": AchievementUnlocked;
  "level-up": LevelUp;
  "app-info-changed": AppInfo;
  navigate: NavigateTo;
  "update-status-changed": UpdateStatus;
  "limits-connection-changed": LimitsConnection;
}
