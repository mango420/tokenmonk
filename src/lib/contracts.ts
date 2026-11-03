/**
 * Shared contracts between the Rust core and the web UI.
 *
 * 1:1 mirror of `src-tauri/src/contracts/`. Change both sides together, and
 * only through the architect (see docs/ARCHITECTURE.md).
 *
 * Wire rules (set by serde on the Rust side):
 * - Fields are camelCase (`rename_all = "camelCase"`).
 * - Enums are unit enums serialized as camelCase string literals.
 * - `Option<T>` is `T | null` (never omitted), except in patch types where
 *   an absent field means "unchanged" (`field?: T`).
 * - Timestamps are `EpochMs`: Unix epoch milliseconds, UTC.
 * - Command names are snake_case, command arguments single lowercase words,
 *   event names kebab-case.
 */

/** Unix epoch milliseconds, UTC. */
export type EpochMs = number;

// ---------------------------------------------------------------- settings

/** Play mode. `noneYet` means onboarding has not been completed. */
export type Mode = "noneYet" | "max" | "diy";

export interface Settings {
  mode: Mode;
  /** Play a sound on achievement unlocks and level-ups. */
  sound: boolean;
  /** Start Tokenmonk at login. */
  autostart: boolean;
}

/** Partial update for `update_settings`. `mode: "noneYet"` is rejected. */
export interface SettingsPatch {
  mode?: Mode;
  sound?: boolean;
  autostart?: boolean;
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

/** Result of `get_limit_setup`. Tokenmonk never edits Claude Code settings. */
export interface LimitSetup {
  /** Ready-to-paste value for `statusLine.command` in Claude Code's
   * `settings.json` (executable path plus `--statusline`); null if the
   * executable path is unknown. */
  statuslineCommand: string | null;
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
  /** Minutes until the 5-hour limit at this pace; null without limits or burn, 0 when reached. */
  limitEtaMinutes: number | null;
}

export type BudgetBasis = "weeklyLimit" | "tokenHistory";

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
  /** null when neither limits nor enough token history exist. */
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
  /** Only earnable in this mode; null for both. */
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
  /** Do it yourself only: XP if usage stopped now; 0 over budget; null otherwise. */
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
  logsFound: boolean;
}

/** Database schema version the app expects (Rust `schema::SCHEMA_VERSION`). */
export const SCHEMA_VERSION = 1;

// ---------------------------------------------------------------- commands

export const Commands = {
  getDashboard: "get_dashboard",
  getSettings: "get_settings",
  updateSettings: "update_settings",
  completeOnboarding: "complete_onboarding",
  getGameState: "get_game_state",
  getAppInfo: "get_app_info",
  getLimitSetup: "get_limit_setup",
} as const;

export type CommandName = (typeof Commands)[keyof typeof Commands];

export interface UpdateSettingsArgs {
  patch: SettingsPatch;
}

/** `mode` must be "max" or "diy". */
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
}

export type CommandErrorCode =
  | "invalidArgument"
  | "notReady"
  | "storage"
  | "internal";

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
} as const;

export type EventName = (typeof Events)[keyof typeof Events];

export type View = "dashboard" | "settings" | "onboarding" | "achievements";

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
}
