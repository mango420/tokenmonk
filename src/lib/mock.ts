/**
 * Synthetic data for the UI shell, typed exactly by the contracts.
 * Everything here is made up. Live data replaces it in `appData.ts`.
 */
import type {
  AchievementProgress,
  AppInfo,
  DashboardSnapshot,
  AchievementUnlocked,
  GameState,
  LevelUp,
  Settings,
  TokenBreakdown,
  UpdateStatus,
} from "./contracts";
import { RELEASE_PAGE_BASE } from "./contracts";

const MIN = 60_000;
const HOUR = 60 * MIN;

/** Thursday 29 October 2026, 14:05 local time. */
const NOW = new Date(2026, 9, 29, 14, 5).getTime();
/** Next Thursday, 09:00 local time. */
const WEEKLY_RESET = new Date(2026, 10, 5, 9, 0).getTime();
const NIGHT_OWL_AT = new Date(2026, 9, 29, 2, 47).getTime();

function tokens(total: number): TokenBreakdown {
  const input = Math.round(total * 0.2);
  const output = Math.round(total * 0.1);
  return {
    input,
    output,
    cacheCreation: total - input - output,
    cacheRead: total * 6,
    total,
  };
}

const idle: UpdateStatus = { kind: "idle" };
const releaseUrl = `${RELEASE_PAGE_BASE}v0.4.1`;

const appInfo: AppInfo = {
  version: "0.1.0",
  platform: "windows",
  schemaVersion: 1,
  logsFound: true,
};

const ready = {
  state: "ready",
  filesTotal: 12,
  filesDone: 12,
  lastMessageAt: NOW - 2 * MIN,
  detail: null,
} as const;

const session = (total: number): DashboardSnapshot["session"] => ({
  sessionId: "mock-session-1",
  projectName: "tokenmonk",
  startedAt: NOW - 95 * MIN,
  lastActivityAt: NOW - 2 * MIN,
  tokens: tokens(total),
});

function snapshot(patch: Partial<DashboardSnapshot>): DashboardSnapshot {
  return {
    generatedAt: NOW,
    today: tokens(184_230),
    session: session(38_912),
    burnRate: { tokensPerMinute: 1_240, windowMinutes: 15 },
    limits: {
      status: "ok",
      reason: null,
      detail: null,
      source: "mock",
      fiveHour: { utilization: 72, resetsAt: NOW + 134 * MIN },
      weekly: { utilization: 41, resetsAt: WEEKLY_RESET },
      fetchedAt: NOW,
    },
    data: ready,
    ...patch,
  };
}

const noLimits: DashboardSnapshot["limits"] = {
  status: "unavailable",
  reason: "noSource",
  detail: null,
  source: null,
  fiveHour: null,
  weekly: null,
  fetchedAt: null,
};

const achievements: AchievementProgress[] = [
  {
    id: "firstBlood",
    name: "First Blood",
    description: "Your first token, spent.",
    secret: false,
    xp: 10,
    unlockedAt: NOW - 20 * 24 * HOUR,
    current: null,
    target: null,
    progressLabel: null,
  },
  {
    id: "nightOwl",
    name: "Night Owl",
    description: "Coding after midnight.",
    secret: false,
    xp: 50,
    unlockedAt: NIGHT_OWL_AT,
    current: null,
    target: null,
    progressLabel: null,
  },
  {
    id: "rateLimitRambo",
    name: "Rate-Limit Rambo",
    description: "Hit the limit. On purpose, we assume.",
    secret: false,
    xp: 100,
    unlockedAt: null,
    current: null,
    target: null,
    progressLabel: null,
  },
  {
    id: "soClose",
    name: "So Close",
    description: "Stopped at 99 percent. Restraint or luck.",
    secret: false,
    xp: 60,
    unlockedAt: null,
    current: null,
    target: null,
    progressLabel: null,
  },
  {
    id: "tokenMonk",
    name: "Token Monk",
    description: "A full day under budget.",
    secret: false,
    xp: 150,
    unlockedAt: null,
    current: null,
    target: null,
    progressLabel: null,
  },
  {
    id: "boo",
    name: "Boo",
    description: "Halloween 2026.",
    secret: true,
    xp: 100,
    unlockedAt: null,
    current: null,
    target: null,
    progressLabel: null,
  },
];

const maxGame: GameState = {
  mode: "max",
  level: 7,
  title: "Limit Chaser",
  xpTotal: 6_440,
  xpIntoLevel: 1_240,
  xpForLevel: 1_850,
  progress: 1_240 / 1_850,
  recentUnlocks: [
    {
      id: "nightOwl",
      name: "Night Owl",
      detail: "Tokens burned at 02:47",
      xp: 50,
      unlockedAt: NIGHT_OWL_AT,
      retroactive: false,
    },
    {
      id: "firstBlood",
      name: "First Blood",
      detail: null,
      xp: 10,
      unlockedAt: NOW - 20 * 24 * HOUR,
      retroactive: false,
    },
  ],
  achievements,
  retroDone: true,
};

/** A fresh install: nothing unlocked, nothing counted yet. */
const freshGame = (): GameState => ({
  ...maxGame,
  mode: "max",
  level: 1,
  title: "Limit Chaser",
  xpTotal: 0,
  xpIntoLevel: 0,
  xpForLevel: 100,
  progress: 0,
  recentUnlocks: [],
  achievements: achievements.map((a) => ({ ...a, unlockedAt: null })),
  retroDone: true,
});

const importing = (filesDone: number): DashboardSnapshot["data"] => ({
  state: "importing",
  filesTotal: 38,
  filesDone,
  lastMessageAt: null,
  detail: null,
});

const noLogsData: DashboardSnapshot["data"] = {
  state: "noLogs",
  filesTotal: 0,
  filesDone: 0,
  lastMessageAt: null,
  detail: null,
};

const noLogsSnapshot = (): DashboardSnapshot =>
  snapshot({
    today: tokens(0),
    session: null,
    burnRate: { tokensPerMinute: 0, windowMinutes: 15 },
    limits: noLimits,
    data: noLogsData,
  });

export interface MockScenario {
  label: string;
  settings: Settings;
  appInfo: AppInfo;
  dashboard: DashboardSnapshot;
  game: GameState;
  /** What `get_update_status` answers. */
  update: UpdateStatus;
}

/** Fake events for the dev notice buttons. */
export const mockUnlock: AchievementUnlocked = {
  id: "nightOwl",
  name: "Night Owl",
  detail: "Tokens burned at 02:47",
  xp: 50,
  unlockedAt: NOW,
  retroactive: false,
};

export const mockLevelUp: LevelUp = {
  level: 8,
  previousLevel: 7,
  title: "Limit Chaser",
  titleChanged: false,
  retroactive: false,
};

/** `maxGame` with the Halloween 2026 badge (Boo) unlocked a little while ago. */
const booGame: GameState = {
  ...maxGame,
  xpTotal: maxGame.xpTotal + 100,
  xpIntoLevel: maxGame.xpIntoLevel + 100,
  progress: (maxGame.xpIntoLevel + 100) / maxGame.xpForLevel,
  recentUnlocks: [
    { id: "boo", name: "Boo", detail: null, xp: 100, unlockedAt: NOW - 3 * HOUR, retroactive: false },
    ...maxGame.recentUnlocks,
  ],
  achievements: achievements.map((a) => (a.id === "boo" ? { ...a, unlockedAt: NOW - 3 * HOUR } : a)),
};

const maxSettings: Settings = { mode: "max", sound: true, autostart: false, updateCheck: true, keepOpen: false, notifyLimits: true, notifyLevelUps: true };

const buildScenarios = () => ({
  max: {
    label: "Max it out, 72 %",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: maxGame,
    update: idle,
  },
  booUnlocked: {
    label: "Max it out, Halloween 2026 badge",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: booGame,
    update: idle,
  },
  maxDanger: {
    label: "Max it out, 94 %",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({
      burnRate: { tokensPerMinute: 2_900, windowMinutes: 15 },
      limits: {
        status: "ok",
        reason: null,
        detail: null,
        source: "mock",
        fiveHour: { utilization: 94, resetsAt: NOW + 52 * MIN },
        weekly: { utilization: 63, resetsAt: WEEKLY_RESET },
        fetchedAt: NOW,
      },
    }),
    game: maxGame,
    update: idle,
  },
  maxWeekLow: {
    label: "Max it out, week runs out first",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({
      limits: {
        status: "ok",
        reason: null,
        detail: null,
        source: "mock",
        fiveHour: { utilization: 30, resetsAt: NOW + 134 * MIN },
        weekly: { utilization: 88, resetsAt: WEEKLY_RESET },
        fetchedAt: NOW,
      },
    }),
    game: maxGame,
    update: idle,
  },
  maxStale: {
    label: "Max it out, stale",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({
      limits: {
        status: "stale",
        reason: null,
        detail: null,
        source: "mock",
        fiveHour: { utilization: 72, resetsAt: NOW + 134 * MIN },
        weekly: { utilization: 41, resetsAt: WEEKLY_RESET },
        fetchedAt: NOW - 55 * MIN,
      },
    }),
    game: maxGame,
    update: idle,
  },
  maxUnavailable: {
    label: "Max it out, no limit data",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({
      limits: noLimits,
      burnRate: { tokensPerMinute: 12_400, windowMinutes: 15 },
    }),
    game: maxGame,
    update: idle,
  },
  maxFreshWindow: {
    label: "Max it out, fresh window",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({
      burnRate: { tokensPerMinute: 1_240, windowMinutes: 15 },
      limits: {
        status: "ok",
        reason: null,
        detail: null,
        source: "mock",
        fiveHour: null,
        weekly: { utilization: 41, resetsAt: WEEKLY_RESET },
        fetchedAt: NOW,
      },
    }),
    game: maxGame,
    update: idle,
  },
  maxSetupQuoted: {
    label: "Max it out, no limit data, path with space",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({
      limits: noLimits,
      burnRate: { tokensPerMinute: 640, windowMinutes: 15 },
    }),
    game: maxGame,
    update: idle,
  },
  maxSetupArrived: {
    label: "Max it out, status line ran, no current window",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({ limits: noLimits }),
    game: maxGame,
    update: idle,
  },
  maxSetupUnreadable: {
    label: "Max it out, status line file unreadable",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({ limits: noLimits }),
    game: maxGame,
    update: idle,
  },
  maxSetupMissing: {
    label: "Max it out, executable path unknown",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({ limits: noLimits }),
    game: maxGame,
    update: idle,
  },
  importingMax: {
    label: "Max it out, importing",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({ data: importing(24), limits: noLimits }),
    game: { ...freshGame(), retroDone: false },
    update: idle,
  },
  noLogs: {
    label: "Max it out, no logs yet",
    settings: maxSettings,
    appInfo: { ...appInfo, logsFound: false },
    dashboard: noLogsSnapshot(),
    game: freshGame(),
    update: idle,
  },
  logError: {
    label: "Max it out, logs unreadable",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({
      limits: noLimits,
      data: { state: "error", filesTotal: 12, filesDone: 4, lastMessageAt: null, detail: null },
    }),
    game: maxGame,
    update: idle,
  },
  updateAvailable: {
    label: "Max it out, Update available",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: maxGame,
    update: { kind: "available", version: "0.4.1", url: releaseUrl, checkedAt: NOW - 5 * 60_000 },
  },
  updateChecking: {
    label: "Max it out, Update: checking",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: maxGame,
    update: { kind: "checking" },
  },
  updateUpToDate: {
    label: "Max it out, Update: up to date",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: maxGame,
    update: { kind: "upToDate", checkedAt: NOW - 5 * 60_000 },
  },
  updateCheckFailed: {
    label: "Max it out, Update: check failed",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: maxGame,
    update: { kind: "checkFailed", checkedAt: NOW, detail: null },
  },
  updateDownloading: {
    label: "Max it out, Update: downloading",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: maxGame,
    update: { kind: "downloading", version: "0.4.1", percent: 42 },
  },
  updateInstalling: {
    label: "Max it out, Update: installing",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: maxGame,
    update: { kind: "installing", version: "0.4.1" },
  },
  updateInstallFailed: {
    label: "Max it out, Update: install failed",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: maxGame,
    update: { kind: "installFailed", version: "0.4.1", url: releaseUrl, detail: null },
  },
  updateUnavailable: {
    label: "Max it out, Update: not set up in this build",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: maxGame,
    update: { kind: "unavailable" },
  },
}) satisfies Record<string, MockScenario>;

type Scenarios = ReturnType<typeof buildScenarios>;
let scenarios: Scenarios | null = null;

/** Built on first use so production bundles can drop this module entirely. */
export function getMockScenarios(): Scenarios {
  scenarios ??= buildScenarios();
  return scenarios;
}

export type MockScenarioId = keyof Scenarios;

export const defaultScenario: MockScenarioId = "max";
