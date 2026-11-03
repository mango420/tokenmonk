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
  LimitSetup,
  Settings,
  TokenBreakdown,
} from "./contracts";

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
    burnRate: { tokensPerMinute: 1_240, windowMinutes: 15, limitEtaMinutes: 41 },
    limits: {
      status: "ok",
      reason: null,
      detail: null,
      source: "mock",
      fiveHour: { utilization: 72, resetsAt: NOW + 134 * MIN },
      weekly: { utilization: 41, resetsAt: WEEKLY_RESET },
      fetchedAt: NOW,
    },
    dailyBudget: patch.limits?.status === "unavailable" ? null : budget(23, 1_100_000),
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

function budget(
  usedPercent: number,
  tokensLeftPerDay: number | null = null,
): NonNullable<DashboardSnapshot["dailyBudget"]> {
  const allowance = 50_000;
  return {
    basis: tokensLeftPerDay === null ? "tokenHistory" : "weeklyLimit",
    allowance,
    usedToday: Math.round((allowance * usedPercent) / 100),
    usedPercent,
    daysLeft: 7,
    resetsAt: WEEKLY_RESET,
    tokensLeftPerDay,
  };
}

const achievements: AchievementProgress[] = [
  {
    id: "firstBlood",
    name: "First Blood",
    description: "Your first token, spent.",
    secret: false,
    mode: null,
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
    mode: "max",
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
    mode: "max",
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
    mode: "max",
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
    mode: "diy",
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
    mode: null,
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
  dailyXpPreview: null,
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

const diyGame: GameState = {
  mode: "diy",
  level: 4,
  title: "Quiet Spender",
  xpTotal: 1_900,
  xpIntoLevel: 560,
  xpForLevel: 1_160,
  progress: 560 / 1_160,
  dailyXpPreview: 77,
  recentUnlocks: [
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
    burnRate: { tokensPerMinute: 0, windowMinutes: 15, limitEtaMinutes: null },
    limits: noLimits,
    data: noLogsData,
  });

export interface MockScenario {
  label: string;
  settings: Settings;
  appInfo: AppInfo;
  dashboard: DashboardSnapshot;
  game: GameState;
  /** What `get_limit_setup` answers; defaults to the unquoted Windows path. */
  limitSetup?: LimitSetup;
}

/** Synthetic `get_limit_setup` answers for the settings preview. */
const setupPlain: LimitSetup = { statuslineCommand: "C:/Tokenmonk/tokenmonk.exe --statusline" };
const setupQuoted: LimitSetup = {
  statuslineCommand: '"C:/Program Files/Tokenmonk/tokenmonk.exe" --statusline',
};
const setupMissing: LimitSetup = { statuslineCommand: null };

export function mockLimitSetup(scenario: MockScenario): LimitSetup {
  return scenario.limitSetup ?? setupPlain;
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

const maxSettings: Settings = { mode: "max", sound: true, autostart: false };
const diySettings: Settings = { mode: "diy", sound: true, autostart: false };

const buildScenarios = () => ({
  max: {
    label: "Max it out, 72 %",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: maxGame,
  },
  booUnlocked: {
    label: "Max it out, Halloween 2026 badge",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({}),
    game: booGame,
  },
  maxDanger: {
    label: "Max it out, 94 %",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({
      burnRate: { tokensPerMinute: 2_900, windowMinutes: 15, limitEtaMinutes: 7 },
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
  },
  maxUnavailable: {
    label: "Max it out, no limit data",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({
      limits: noLimits,
      burnRate: { tokensPerMinute: 12_400, windowMinutes: 15, limitEtaMinutes: null },
    }),
    game: maxGame,
  },
  maxFreshWindow: {
    label: "Max it out, fresh window",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({
      burnRate: { tokensPerMinute: 1_240, windowMinutes: 15, limitEtaMinutes: null },
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
  },
  maxSetupQuoted: {
    label: "Max it out, no limit data, path with space",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({
      limits: noLimits,
      burnRate: { tokensPerMinute: 640, windowMinutes: 15, limitEtaMinutes: null },
    }),
    game: maxGame,
    limitSetup: setupQuoted,
  },
  maxSetupMissing: {
    label: "Max it out, executable path unknown",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({ limits: noLimits }),
    game: maxGame,
    limitSetup: setupMissing,
  },
  diy: {
    label: "Do it yourself, 23 %",
    settings: diySettings,
    appInfo,
    dashboard: snapshot({
      today: tokens(12_480),
      session: session(1_920),
      dailyBudget: budget(23, 480_000),
      limits: {
        status: "ok",
        reason: null,
        detail: null,
        source: "mock",
        fiveHour: { utilization: 18, resetsAt: NOW + 134 * MIN },
        weekly: { utilization: 9, resetsAt: WEEKLY_RESET },
        fetchedAt: NOW,
      },
    }),
    game: diyGame,
  },
  diyOver: {
    label: "Do it yourself, over budget",
    settings: diySettings,
    appInfo,
    dashboard: snapshot({
      today: tokens(61_300),
      session: session(24_800),
      dailyBudget: budget(112),
      limits: noLimits,
    }),
    game: { ...diyGame, dailyXpPreview: 0 },
  },
  diyLearning: {
    label: "Do it yourself, no history",
    settings: diySettings,
    appInfo,
    dashboard: snapshot({
      today: tokens(3_450),
      session: session(3_450),
      dailyBudget: null,
      limits: noLimits,
    }),
    game: { ...diyGame, dailyXpPreview: null },
  },
  onboarding: {
    label: "Onboarding, importing",
    settings: { mode: "noneYet", sound: true, autostart: false },
    appInfo,
    dashboard: snapshot({ data: importing(24), limits: noLimits }),
    game: { ...freshGame(), retroDone: false },
  },
  onboardingReady: {
    label: "Onboarding, logs found",
    settings: { mode: "noneYet", sound: true, autostart: false },
    appInfo,
    dashboard: snapshot({ limits: noLimits }),
    game: { ...freshGame(), retroDone: false },
  },
  onboardingNoLogs: {
    label: "Onboarding, no logs",
    settings: { mode: "noneYet", sound: true, autostart: false },
    appInfo: { ...appInfo, logsFound: false },
    dashboard: noLogsSnapshot(),
    game: freshGame(),
  },
  importingMax: {
    label: "Max it out, importing",
    settings: maxSettings,
    appInfo,
    dashboard: snapshot({ data: importing(24), limits: noLimits }),
    game: { ...freshGame(), retroDone: false },
  },
  noLogs: {
    label: "Max it out, no logs yet",
    settings: maxSettings,
    appInfo: { ...appInfo, logsFound: false },
    dashboard: noLogsSnapshot(),
    game: freshGame(),
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
