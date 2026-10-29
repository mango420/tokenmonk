import type { ReactNode } from "react";
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
  /** Seam for the ghost easter egg (#16). */
  onLogoClick?: () => void;
  /** Seam for the Halloween chip (#16). */
  headerBadge?: ReactNode;
}

/**
 * The popup's dashboard view. Renders blocks top to bottom into the popup
 * column (see `Popup`); the footer note is pushed to the bottom.
 */
export function Dashboard({ data, onOpenSettings, onLogoClick, headerBadge }: DashboardProps) {
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
        onLogoClick={onLogoClick}
        badge={headerBadge}
      />
      <HaloGauge model={gauge} />
      <StatTiles tiles={buildStatTiles(dashboard)} />
      <WeeklyCard model={buildWeekly(mode, dashboard.limits)} />
      <LevelCard model={buildLevel(game)} />
      {achievement !== null ? <AchievementRow model={achievement} /> : null}
      <p className="footer-note">{footerNote(dashboard.limits)}</p>
    </>
  );
}
