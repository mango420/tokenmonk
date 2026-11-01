import type { AppData } from "../../lib/appData";
import { AchievementRow } from "./AchievementRow";
import { Header } from "./Header";
import { HaloGauge } from "./HaloGauge";
import { LevelCard } from "./LevelCard";
import { StatTiles } from "./StatTiles";
import { WeeklyCard } from "./WeeklyCard";
import {
  buildAchievementRow,
  buildGauge,
  buildLevel,
  buildStatTiles,
  buildWeekly,
  footerNote,
  toPopupMode,
} from "./model";

interface DashboardProps {
  data: AppData;
  onOpenSettings: () => void;
  /** Opens Settings scrolled to "Limit data". */
  onOpenLimitSetup?: () => void;
}

/**
 * The popup's dashboard view. Renders blocks top to bottom into the popup
 * column (see `Popup`); the footer note is pushed to the bottom.
 */
export function Dashboard({ data, onOpenSettings, onOpenLimitSetup }: DashboardProps) {
  const { settings, dashboard, game } = data;
  const mode = toPopupMode(settings.mode);
  const gauge = buildGauge(mode, dashboard, game);
  const achievement = buildAchievementRow(game, dashboard.generatedAt);

  return (
    <>
      <Header
        mode={mode}
        haloRatio={gauge.ratio}
        onOpenSettings={onOpenSettings}
      />
      <HaloGauge model={gauge} onOpenLimitSetup={onOpenLimitSetup} />
      <StatTiles tiles={buildStatTiles(dashboard)} />
      <WeeklyCard model={buildWeekly(mode, dashboard.limits)} onOpenLimitSetup={onOpenLimitSetup} />
      <LevelCard model={buildLevel(game)} />
      {achievement !== null ? <AchievementRow model={achievement} /> : null}
      <p className="footer-note">{footerNote(dashboard.limits)}</p>
    </>
  );
}
