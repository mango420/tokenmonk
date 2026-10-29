import { mockScenarios, type MockScenarioId } from "../lib/mock";

interface ScenarioSwitchProps {
  value: MockScenarioId;
  onChange: (id: MockScenarioId) => void;
}

const IDS = Object.keys(mockScenarios) as MockScenarioId[];

/** Development-only mock scenario picker; sits beside the popup, off-screen in the app window. */
export function ScenarioSwitch({ value, onChange }: ScenarioSwitchProps) {
  return (
    <aside className="dev-switch" aria-label="Mock scenario">
      <span className="caption">Mock scenario</span>
      <select value={value} onChange={(e) => onChange(e.target.value as MockScenarioId)}>
        {IDS.map((id) => (
          <option key={id} value={id}>
            {mockScenarios[id].label}
          </option>
        ))}
      </select>
    </aside>
  );
}
