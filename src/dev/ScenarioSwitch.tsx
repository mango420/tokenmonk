import { getMockScenarios, type MockScenarioId } from "../lib/mock";

interface ScenarioSwitchProps {
  value: MockScenarioId;
  onChange: (id: MockScenarioId) => void;
}

/** Development-only mock scenario picker; sits beside the popup, off-screen in the app window. */
export function ScenarioSwitch({ value, onChange }: ScenarioSwitchProps) {
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
    </aside>
  );
}
