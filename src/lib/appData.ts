/**
 * The one place the UI gets its data from.
 *
 * Inside the app: the commands and events from `contracts.ts` (`get_*` once,
 * then `*-updated` / `settings-changed` events carry full state). In a plain
 * browser during development (`vite`): the synthetic scenarios from `mock.ts`,
 * picked by the dev scenario switch. Components only see `AppState`.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { call, describeError, insideTauri, useBackendEvent } from "./backend";
import {
  Commands,
  Events,
  type AppInfo,
  type DashboardSnapshot,
  type GameState,
  type LimitSetup,
  type Mode,
  type Settings,
  type SettingsPatch,
  type UpdateStatus,
} from "./contracts";
import {
  defaultScenario,
  getMockScenarios,
  mockLimitSetup,
  type MockScenarioId,
} from "./mock";

export interface AppData {
  settings: Settings;
  appInfo: AppInfo;
  dashboard: DashboardSnapshot;
  game: GameState;
  update: UpdateStatus;
}

/**
 * `loading`: first reads are in flight. `error`: the core did not deliver
 * settings (for example a command is not implemented yet); `data` then holds
 * calm placeholders and the UI shows a retry screen instead of the dashboard.
 */
export type Phase = "loading" | "ready" | "error";

export interface AppActions {
  /** Applies a settings patch. Resolves `false` when the command was rejected. */
  updateSettings: (patch: SettingsPatch) => Promise<boolean>;
  /** Picks the first mode (`max` or `diy`). Resolves `false` when rejected. */
  completeOnboarding: (mode: Mode) => Promise<boolean>;
  /** Reads the status-line setup values. Rejects when the core can't answer. */
  loadLimitSetup: () => Promise<LimitSetup>;
  /** "Check now". A rejected call reads as `checkFailed`. */
  checkForUpdates: () => Promise<void>;
  /** "Update" / "Try again". Resolves when it failed; a stale status is re-read. */
  installUpdate: () => Promise<void>;
}

export interface AppState {
  phase: Phase;
  data: AppData;
  actions: AppActions;
  retry: () => void;
}

/** Mock data only in a plain browser during development; never in the app. */
export const usesMockData: boolean = import.meta.env.DEV && !insideTauri();

// ------------------------------------------------------------ placeholders

const EMPTY_TOKENS = { input: 0, output: 0, cacheCreation: 0, cacheRead: 0, total: 0 };

function placeholders(): AppData {
  return {
    settings: { mode: "noneYet", sound: true, autostart: false, updateCheck: true, keepOpen: false },
    appInfo: {
      version: "",
      platform: "windows",
      schemaVersion: 0,
      logsFound: false,
    },
    dashboard: {
      generatedAt: Date.now(),
      today: EMPTY_TOKENS,
      session: null,
      burnRate: { tokensPerMinute: 0, windowMinutes: 15, limitEtaMinutes: null },
      limits: {
        status: "unavailable",
        reason: "noSource",
        detail: null,
        source: null,
        fiveHour: null,
        weekly: null,
        fetchedAt: null,
      },
      dailyBudget: null,
      data: { state: "noLogs", filesTotal: 0, filesDone: 0, lastMessageAt: null, detail: null },
    },
    game: {
      mode: "noneYet",
      level: 1,
      title: "",
      xpTotal: 0,
      xpIntoLevel: 0,
      xpForLevel: 1,
      progress: 0,
      dailyXpPreview: null,
      recentUnlocks: [],
      achievements: [],
      retroDone: true,
    },
    update: { kind: "idle" },
  };
}

// -------------------------------------------------------------------- live

function useLiveAppData(enabled: boolean): AppState {
  const [phase, setPhase] = useState<Phase>("loading");
  const [data, setData] = useState<AppData>(placeholders);
  const [attempt, setAttempt] = useState(0);
  /** Keys an event already filled; a slower initial read must not overwrite them. */
  const fresh = useRef(new Set<keyof AppData>());

  const apply = useCallback(<K extends keyof AppData>(key: K, value: AppData[K]) => {
    fresh.current.add(key);
    setData((previous) => ({ ...previous, [key]: value }));
    if (key === "settings") setPhase("ready");
  }, []);

  useBackendEvent(Events.dashboardUpdated, (v) => apply("dashboard", v), enabled);
  useBackendEvent(Events.settingsChanged, (v) => apply("settings", v), enabled);
  useBackendEvent(Events.gameUpdated, (v) => apply("game", v), enabled);
  useBackendEvent(Events.appInfoChanged, (v) => apply("appInfo", v), enabled);
  useBackendEvent(Events.updateStatusChanged, (v) => apply("update", v), enabled);

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    fresh.current.clear();
    setPhase("loading");

    const reads = [
      ["settings", call(Commands.getSettings)],
      ["appInfo", call(Commands.getAppInfo)],
      ["dashboard", call(Commands.getDashboard)],
      ["game", call(Commands.getGameState)],
      ["update", call(Commands.getUpdateStatus)],
    ] as const;

    void Promise.allSettled(reads.map(([, promise]) => promise)).then((results) => {
      if (cancelled) return;
      results.forEach((result, index) => {
        const key = reads[index]![0];
        if (result.status === "rejected") {
          console.warn(`${key} not available: ${describeError(result.reason)}`);
          return;
        }
        if (!fresh.current.has(key)) {
          // The key is the union member that matches the command's result.
          apply(key, result.value as never);
        }
      });
      setPhase((current) => (current === "ready" ? current : "error"));
    });

    return () => {
      cancelled = true;
    };
  }, [enabled, attempt, apply]);

  const updateSettings = useCallback(
    async (patch: SettingsPatch) => {
      try {
        apply("settings", await call(Commands.updateSettings, { patch }));
        return true;
      } catch (error) {
        console.warn(`update_settings failed: ${describeError(error)}`);
        return false;
      }
    },
    [apply],
  );

  const completeOnboarding = useCallback(
    async (mode: Mode) => {
      try {
        apply("settings", await call(Commands.completeOnboarding, { mode }));
        return true;
      } catch (error) {
        console.warn(`complete_onboarding failed: ${describeError(error)}`);
        return false;
      }
    },
    [apply],
  );

  const loadLimitSetup = useCallback(() => call(Commands.getLimitSetup), []);

  const checkForUpdates = useCallback(async () => {
    try {
      apply("update", await call(Commands.checkForUpdates));
    } catch (error) {
      console.warn(`check_for_updates failed: ${describeError(error)}`);
      apply("update", { kind: "checkFailed", checkedAt: Date.now(), detail: null });
    }
  }, [apply]);

  const installUpdate = useCallback(async () => {
    try {
      apply("update", await call(Commands.installUpdate));
    } catch (error) {
      console.warn(`install_update failed: ${describeError(error)}`);
      try {
        apply("update", await call(Commands.getUpdateStatus));
      } catch (readError) {
        console.warn(`get_update_status failed: ${describeError(readError)}`);
      }
    }
  }, [apply]);

  const retry = useCallback(() => setAttempt((n) => n + 1), []);

  return {
    phase,
    data,
    actions: { updateSettings, completeOnboarding, loadLimitSetup, checkForUpdates, installUpdate },
    retry,
  };
}

// -------------------------------------------------------------------- mock

function useMockAppData(scenario: MockScenarioId): AppState {
  const source = getMockScenarios()[scenario];
  const [settings, setSettings] = useState<Settings>(source.settings);
  useEffect(() => {
    setSettings(getMockScenarios()[scenario].settings);
  }, [scenario]);

  const updateSettings = useCallback(async (patch: SettingsPatch) => {
    setSettings((s) => ({ ...s, ...patch, mode: patch.mode ?? s.mode }));
    return true;
  }, []);
  const completeOnboarding = useCallback(async (mode: Mode) => {
    setSettings((s) => ({ ...s, mode }));
    return true;
  }, []);

  const [update, setUpdate] = useState<UpdateStatus>(source.update);
  useEffect(() => {
    setUpdate(getMockScenarios()[scenario].update);
  }, [scenario]);
  const checkForUpdates = useCallback(async () => {
    setUpdate({ kind: "checking" });
    await new Promise((resolve) => setTimeout(resolve, 600));
    setUpdate({ kind: "upToDate", checkedAt: Date.now() });
  }, []);
  const installUpdate = useCallback(async () => {
    setUpdate((u) => (u.kind === "available" || u.kind === "installFailed" ? { kind: "installing", version: u.version } : u));
  }, []);

  const loadLimitSetup = useCallback(
    async () => mockLimitSetup(getMockScenarios()[scenario]),
    [scenario],
  );

  return {
    phase: "ready",
    data: {
      settings,
      appInfo: source.appInfo,
      dashboard: source.dashboard,
      game: source.game,
      update,
    },
    actions: { updateSettings, completeOnboarding, loadLimitSetup, checkForUpdates, installUpdate },
    retry: () => undefined,
  };
}

// ------------------------------------------------------------------ public

/** `scenario` only selects mock data; live data ignores it. */
export function useAppData(scenario: MockScenarioId = defaultScenario): AppState {
  const live = useLiveAppData(!usesMockData);
  // `DEV` is a build-time constant: production drops the mock hook and its data.
  const mock = import.meta.env.DEV ? useMockAppData(scenario) : null;
  return usesMockData && mock !== null ? mock : live;
}
