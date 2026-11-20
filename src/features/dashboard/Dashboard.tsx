import { useState } from "react";
import { BeadString } from "../../components/BeadString";
import { MonkLarge } from "../../components/Monk";
import { TextLink } from "../../components/TextLink";
import type { AppActions, AppData } from "../../lib/appData";
import { Header } from "./Header";
import { buildDashboard, buildLevel, buildNote, type NoteModel, type WindowModel } from "./model";

interface DashboardProps {
  data: AppData;
  actions: AppActions;
  onOpenSettings: () => void;
  /** Opens Settings scrolled to "Limits". */
  onOpenLimitSetup: () => void;
  onOpenAchievements: () => void;
}

/** The popup's dashboard: header band with the rank, then the limit windows or the no-limits layout. */
export function Dashboard({ data, actions, onOpenSettings, onOpenLimitSetup, onOpenAchievements }: DashboardProps) {
  const { dashboard, game } = data;
  const model = buildDashboard(dashboard, data.limitsConnection);
  const level = buildLevel(game);
  const note = buildNote(dashboard, data.update);
  const [installing, setInstalling] = useState(false);
  const install = () => {
    setInstalling(true);
    void actions.installUpdate().finally(() => setInstalling(false));
  };

  const noteLine =
    note === null ? (
      <p className="note" />
    ) : (
      <NoteLine note={note} installing={installing} onInstall={install} />
    );
  const closing = model.closing.link ? (
    <TextLink small onClick={onOpenLimitSetup}>
      {model.closing.text}
    </TextLink>
  ) : (
    model.closing.text
  );

  return (
    <div className="screen screen--bands">
      <div className="band">
        <Header onOpenSettings={onOpenSettings} />
        <button type="button" className="rank" onClick={onOpenAchievements}>
          <span className="rank__title">{level.title}</span>
          <span className="meta">{level.meta}</span>
        </button>
      </div>
      {model.windows.length > 0 ? (
        <div className="body body--dashboard">
          {model.windows.map((window) => (
            <WindowSection key={window.key} window={window} />
          ))}
          <div className="spacer" />
          <div className="closing">
            <p className="status">{closing}</p>
            {noteLine}
          </div>
        </div>
      ) : (
        <div className="body body--empty">
          <div className="spacer" />
          <MonkLarge />
          <div className="hero-row hero-row--center">
            <span className="hero-run">
              <span className="hero">{model.tokensToday.number}</span>
              {model.tokensToday.unit === "" ? null : <span className="hero__unit">{model.tokensToday.unit}</span>}
            </span>
          </div>
          <span className="hero-label">tokens today</span>
          <div className="rule" />
          <p className="status">{closing}</p>
          <div className="spacer spacer--wide" />
          {noteLine}
        </div>
      )}
    </div>
  );
}

/** One limit window: label and reset time, the hero number, five beads. */
function WindowSection({ window }: { window: WindowModel }) {
  return (
    <section className={window.low ? "window window--low" : "window"} aria-label={window.label}>
      <div className="window__head">
        <h2 className="section-title">{window.label}</h2>
        <span className="meta window__reset">{window.reset}</span>
      </div>
      <div className="hero-row">
        <span className={window.stale ? "hero-run hero--stale" : "hero-run"}>
          <span className="hero">{window.percent}</span>
          <span className="hero__unit">%</span>
        </span>
        <span className="hero-label">left</span>
      </div>
      <BeadString percentLeft={window.percent} stale={window.stale} />
    </section>
  );
}

interface NoteLineProps {
  note: NoteModel;
  installing: boolean;
  onInstall: () => void;
}

function NoteLine({ note, installing, onInstall }: NoteLineProps) {
  return (
    <p className="note">
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
          <TextLink small onClick={installing ? undefined : onInstall} disabled={installing}>
            {note.action}
          </TextLink>
        </span>
      ) : null}
    </p>
  );
}
