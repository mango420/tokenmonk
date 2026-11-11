import { useState } from "react";
import { BeadGauge } from "../../components/BeadGauge";
import { TextLink } from "../../components/TextLink";
import type { AppActions, AppData } from "../../lib/appData";
import { Header } from "./Header";
import { LevelBlock } from "./LevelBlock";
import { buildDashboard, buildLevel, buildNote, toPopupMode } from "./model";

interface DashboardProps {
  data: AppData;
  actions: AppActions;
  onOpenSettings: () => void;
  /** Opens Settings scrolled to "Limit data". */
  onOpenLimitSetup: () => void;
  onOpenAchievements: () => void;
}

/** The popup's dashboard: bead gauge, one big number, meta row, week bar, level card. */
export function Dashboard({ data, actions, onOpenSettings, onOpenLimitSetup, onOpenAchievements }: DashboardProps) {
  const { settings, dashboard, game } = data;
  const mode = toPopupMode(settings.mode);
  const { gauge, week } = buildDashboard(mode, dashboard);
  const note = buildNote(dashboard, data.update);
  const [installing, setInstalling] = useState(false);
  const install = () => {
    setInstalling(true);
    void actions.installUpdate().finally(() => setInstalling(false));
  };

  return (
    <div className="screen screen--dashboard">
      <Header mode={mode} onOpenSettings={onOpenSettings} />
      <p className="caption dash-caption">{gauge.caption}</p>
      <BeadGauge percentLeft={gauge.percentLeft} stale={gauge.stale} />
      <div className="hero-block">
        <div className="hero-row">
          <span className={`hero live-number${gauge.small ? " hero--sm" : ""}${gauge.stale ? " hero--stale" : ""}`}>
            {gauge.hero}
          </span>
          <span className="hero-label">{gauge.heroLabel}</span>
        </div>
        <p className="status">{gauge.status}</p>
      </div>
      <div className="meta-row meta live-number">
        <span>{gauge.metaLeft}</span>
        <span>{gauge.metaRight}</span>
      </div>
      {week === null ? null : (
        <section className={week.low ? "week card" : "week"}>
          <div className="week__row">
            <span className="caption">This week</span>
            <span className="meta live-number">{week.figure}</span>
          </div>
          <div className="bar bar--week" aria-hidden="true">
            <div
              className={week.stale ? "bar__fill bar__fill--stale" : "bar__fill"}
              style={{ width: `${week.percentLeft}%` }}
            />
          </div>
          {week.low ? <p className="small week__low">{week.lowLine}</p> : null}
        </section>
      )}
      <div className="spacer" />
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
