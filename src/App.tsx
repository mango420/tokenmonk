import { useCallback, useLayoutEffect, useState } from "react";
import { PrimaryButton } from "./components/PrimaryButton";
import { Popup } from "./components/Popup";
import { ScenarioSwitch } from "./dev/ScenarioSwitch";
import { Dashboard } from "./features/dashboard/Dashboard";
import { Header } from "./features/dashboard/Header";
import { useOsNotifications } from "./features/notifications/useOsNotifications";
import { Onboarding } from "./features/onboarding/Onboarding";
import { SettingsView } from "./features/settings/SettingsView";
import { useAppData, usesMockData } from "./lib/appData";
import { useBackendEvent } from "./lib/backend";
import { Events, type View } from "./lib/contracts";
import { defaultScenario, type MockScenarioId } from "./lib/mock";

export default function App() {
  const [scenario, setScenario] = useState<MockScenarioId>(defaultScenario);
  const app = useAppData(scenario);
  const { data, actions } = app;
  const [requested, setRequested] = useState<View>("dashboard");

  // The tray menu asks for a view (for example Settings).
  useBackendEvent(Events.navigate, ({ view }) => setRequested(view), !usesMockData);
  const backToDashboard = useCallback(() => setRequested("dashboard"), []);

  useOsNotifications(data, app.phase === "ready" && !usesMockData);

  // Mode and season drive the accent in tokens.css. No mode before onboarding: brand saffron.
  const mode = data.settings.mode;
  const season = data.appInfo.season;
  useLayoutEffect(() => {
    const root = document.documentElement;
    if (mode === "noneYet") delete root.dataset.mode;
    else root.dataset.mode = mode;
    if (season === "halloween") root.dataset.season = season;
    else delete root.dataset.season;
  }, [mode, season]);

  // Onboarding is not a choice: it shows while no mode is picked, and only then.
  const view: View =
    mode === "noneYet" ? "onboarding" : requested === "onboarding" ? "dashboard" : requested;

  let screen;
  if (app.phase !== "ready") {
    screen = (
      <>
        <Header mode={null} haloRatio={null} />
        {app.phase === "error" ? (
          <div className="screen screen--status">
            <p className="screen__lead">Couldn’t load that. Try again.</p>
            <PrimaryButton onClick={app.retry}>Try again</PrimaryButton>
          </div>
        ) : null}
      </>
    );
  } else if (view === "onboarding") {
    screen = <Onboarding data={data} onChoose={actions.completeOnboarding} />;
  } else if (view === "settings") {
    screen = <SettingsView data={data} actions={actions} onBack={backToDashboard} />;
  } else {
    screen = <Dashboard data={data} onOpenSettings={() => setRequested("settings")} />;
  }

  return (
    <div className="stage">
      {/* `overlay`: game toasts and level-up (#12), ghost overlay (#16). */}
      <Popup>{screen}</Popup>
      {import.meta.env.DEV && usesMockData ? <ScenarioSwitch value={scenario} onChange={setScenario} /> : null}
    </div>
  );
}
