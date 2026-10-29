/**
 * The one place the UI gets its data from.
 *
 * Today: synthetic scenarios from `mock.ts`. Issue #13 replaces the body of
 * `useAppData` with the commands and events from `contracts.ts`
 * (`get_dashboard`, `dashboard-updated`, ...). The returned shape and every
 * component below stay as they are.
 */
import type { AppInfo, DashboardSnapshot, GameState, Settings } from "./contracts";
import { defaultScenario, mockScenarios, type MockScenarioId } from "./mock";

export interface AppData {
  settings: Settings;
  appInfo: AppInfo;
  dashboard: DashboardSnapshot;
  game: GameState;
}

/** `scenario` only selects mock data; live data ignores it. */
export function useAppData(scenario: MockScenarioId = defaultScenario): AppData {
  return mockScenarios[scenario];
}
