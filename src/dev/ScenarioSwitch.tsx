import { getMockScenarios, type MockScenarioId } from "../lib/mock";

interface ScenarioSwitchProps {
  value: MockScenarioId;
  onChange: (id: MockScenarioId) => void;
  /** Fires a fake `achievement-unlocked` event. */
  onUnlock: () => void;
  /** Fires a fake `level-up` event. */
  onLevelUp: () => void;
}

/** Development-only mock scenario picker; sits beside the popup, off-screen in the app window. */
export function ScenarioSwitch({ value, onChange, onUnlock, onLevelUp }: ScenarioSwitchProps) {
  const scenarios = getMockScenarios();
  const ids = Object.keys(scenarios) as MockScenarioId[];
  return (
    <aside className="dev-switch" aria-label="Mock scenario">
      <span className="caption">Mock scenario</span>
      <select value={value} onChange={(e) => onChange(e.target.value as MockScenarioId)}>
        {ids.map((id) => (
          <option key={id} value={id}>
            {scenarios[id].label}
          </option>
        ))}
      </select>
      <button type="button" onClick={onUnlock}>
        Fire unlock
      </button>
      <button type="button" onClick={onLevelUp}>
        Fire level-up
      </button>
    </aside>
  );
}
