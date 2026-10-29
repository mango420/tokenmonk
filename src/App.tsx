import { useEffect, useState } from "react";
import { Popup } from "./components/Popup";
import { ScenarioSwitch } from "./dev/ScenarioSwitch";
import { Dashboard } from "./features/dashboard/Dashboard";
import { useAppData } from "./lib/appData";
import type { View } from "./lib/contracts";
import { defaultScenario, type MockScenarioId } from "./lib/mock";

export default function App() {
  const [scenario, setScenario] = useState<MockScenarioId>(defaultScenario);
  // Live data (#13): drop the argument, `useAppData` then reads the backend.
  const data = useAppData(scenario);
  // Views other than the dashboard arrive with their own issues; `navigate` events (#13) set this.
  const [view, setView] = useState<View>("dashboard");

  // Mode drives the accent in tokens.css. Absent before onboarding: brand saffron.
  const mode = data.settings.mode;
  useEffect(() => {
    const root = document.documentElement;
    if (mode === "noneYet") delete root.dataset.mode;
    else root.dataset.mode = mode;
  }, [mode]);

  return (
    <div className="stage">
      <Popup>
        {view === "dashboard" ? (
          <Dashboard data={data} onOpenSettings={() => setView("settings")} />
        ) : (
          <button type="button" className="placeholder-back" onClick={() => setView("dashboard")}>
            Back to dashboard
          </button>
        )}
      </Popup>
      {import.meta.env.DEV ? <ScenarioSwitch value={scenario} onChange={setScenario} /> : null}
    </div>
  );
}
