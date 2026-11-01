import { useCallback, useLayoutEffect, useState } from "react";
import { PrimaryButton } from "./components/PrimaryButton";
import { Popup } from "./components/Popup";
import { ScenarioSwitch } from "./dev/ScenarioSwitch";
import { Dashboard } from "./features/dashboard/Dashboard";
import { useGameNotices } from "./features/game";
import { Header } from "./features/dashboard/Header";
import { useOsNotifications } from "./features/notifications/useOsNotifications";
import { Onboarding } from "./features/onboarding/Onboarding";
import { SettingsView } from "./features/settings/SettingsView";
import { useAppData, usesMockData } from "./lib/appData";
import { useBackendEvent } from "./lib/backend";
import { Events, type View } from "./lib/contracts";
import { defaultScenario, mockLevelUp, mockUnlock, type MockScenarioId } from "./lib/mock";
import { usePopupVisible } from "./lib/popupVisible";

export default function App() {
  const [scenario, setScenario] = useState<MockScenarioId>(defaultScenario);
  const app = useAppData(scenario);
  const { data, actions } = app;
  const [requested, setRequested] = useState<View>("dashboard");
  /** Settings opens scrolled to "Limit data" (the dashboard's setup hint). */
  const [limitsTarget, setLimitsTarget] = useState(false);

  const open = useCallback((view: View, toLimits = false) => {
    setLimitsTarget(toLimits);
    setRequested(view);
  }, []);

  // The tray menu asks for a view (for example Settings).
  useBackendEvent(Events.navigate, ({ view }) => open(view), !usesMockData);
  const backToDashboard = useCallback(() => open("dashboard"), [open]);

  // Achievement toasts and the level-up screen live in the popup overlay slot.
  const popupVisible = usePopupVisible();
  const notices = useGameNotices({
    game: data.game,
    sound: data.settings.sound,
    popupVisible,
  });
  useBackendEvent(Events.achievementUnlocked, notices.pushUnlock, !usesMockData);
  useBackendEvent(Events.levelUp, notices.pushLevelUp, !usesMockData);

  useOsNotifications(data, app.phase === "ready" && !usesMockData);

  // The mode drives the accent in tokens.css. No mode before onboarding: brand saffron.
  const mode = data.settings.mode;
  useLayoutEffect(() => {
    const root = document.documentElement;
    if (mode === "noneYet") delete root.dataset.mode;
    else root.dataset.mode = mode;
  }, [mode]);

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
    screen = <SettingsView
        data={data}
        actions={actions}
        onBack={backToDashboard}
        scrollToLimits={limitsTarget}
      />;
  } else {
    screen = (
      <Dashboard
        data={data}
        onOpenSettings={() => open("settings")}
        onOpenLimitSetup={() => open("settings", true)}
      />
    );
  }

  return (
    <div className="stage">
      <Popup
        overlay={notices.node}
      >{screen}</Popup>
      {import.meta.env.DEV && usesMockData ? (
        <ScenarioSwitch
          value={scenario}
          onChange={setScenario}
          onUnlock={() => notices.pushUnlock(mockUnlock)}
          onLevelUp={() => notices.pushLevelUp(mockLevelUp)}
        />
      ) : null}
    </div>
  );
}
