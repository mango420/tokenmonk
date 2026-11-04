import { useState } from "react";
import { TextLink } from "../../components/TextLink";
import type { AppActions, AppData } from "../../lib/appData";
import { Header } from "./Header";
import { HaloGauge } from "./HaloGauge";
import { LevelBlock } from "./LevelBlock";
import { Segment } from "./Segment";
import { StatTiles } from "./StatTiles";
import {
  buildLevel,
  buildNote,
  buildRing,
  buildStats,
  hasNoLimitData,
  toPopupMode,
  type WindowChoice,
} from "./model";

interface DashboardProps {
  data: AppData;
  actions: AppActions;
  onOpenSettings: () => void;
  /** Opens Settings scrolled to "Limit data". */
  onOpenLimitSetup: () => void;
  onOpenAchievements: () => void;
}

/** The popup's dashboard: one big number, two stats, the level block at the bottom. */
export function Dashboard({ data, actions, onOpenSettings, onOpenLimitSetup, onOpenAchievements }: DashboardProps) {
  const { settings, dashboard, game } = data;
  // Local UI state: starts at "5 hours" and is not saved.
  const [choice, setChoice] = useState<WindowChoice>("fiveHour");
  const ring = buildRing(toPopupMode(settings.mode), dashboard, choice);
  const note = buildNote(dashboard, data.update);
  const [installing, setInstalling] = useState(false);
  const install = () => {
    setInstalling(true);
    void actions.installUpdate().finally(() => setInstalling(false));
  };

  return (
    <div className="screen">
      <Header onOpenSettings={onOpenSettings} />
      <Segment value={choice} onChange={setChoice} hidden={hasNoLimitData(dashboard.limits)} />
      <HaloGauge model={ring} />
      <p className="status">{ring.status}</p>
      <p className="reset live-number">{ring.reset}</p>
      <StatTiles stats={buildStats(dashboard)} />
      <LevelBlock model={buildLevel(game)} onOpen={onOpenAchievements} />
      <p className="note">
        {note.kind === "limitSetup" ? (
          <TextLink small onClick={onOpenLimitSetup}>
            {note.text}
          </TextLink>
        ) : (
          <>
            {note.segments.map((segment, index) =>
              segment.mono ? (
                <span key={index} className="live-number">
                  {segment.text}
                </span>
              ) : (
                segment.text
              ),
            )}
            {note.kind === "update" && note.action !== null ? (
              <span className="note__action">
                <TextLink small onClick={installing ? undefined : install} disabled={installing}>
                  {note.action}
                </TextLink>
              </span>
            ) : null}
          </>
        )}
      </p>
    </div>
  );
}
