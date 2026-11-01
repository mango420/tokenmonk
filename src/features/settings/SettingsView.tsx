import { useEffect, useRef, useState } from "react";
import { ChoiceTile } from "../../components/ChoiceTile";
import { Icon } from "../../components/Icon";
import { ToggleRow } from "../../components/ToggleRow";
import type { AppActions, AppData } from "../../lib/appData";
import type { SettingsPatch } from "../../lib/contracts";
import { formatInt } from "../../lib/format";
import { MODE_LABEL, modeFill } from "../dashboard/model";
import { LimitData } from "./LimitData";

interface SettingsViewProps {
  data: AppData;
  actions: AppActions;
  onBack: () => void;
  /** Scroll to "Limit data" on open (the dashboard's setup hint asks for it). */
  scrollToLimits?: boolean;
}

/**
 * Settings: every change applies at once through `update_settings`. While the
 * call runs the control stays in its new position; a rejected call flips it back.
 */
export function SettingsView({ data, actions, onBack, scrollToLimits = false }: SettingsViewProps) {
  const limitsRef = useRef<HTMLElement>(null);
  useEffect(() => {
    if (scrollToLimits) limitsRef.current?.scrollIntoView({ block: "start" });
  }, [scrollToLimits]);

  const [pending, setPending] = useState<SettingsPatch>({});
  const [failed, setFailed] = useState(false);
  /** Latest call number per key; only the newest call may clear its pending value. */
  const sequence = useRef<Partial<Record<keyof SettingsPatch, number>>>({});

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onBack();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onBack]);

  const shown = { ...data.settings, ...pending };

  const change = async (patch: SettingsPatch) => {
    setFailed(false);
    const keys = Object.keys(patch) as (keyof SettingsPatch)[];
    const ticket: Partial<Record<keyof SettingsPatch, number>> = {};
    for (const key of keys) {
      ticket[key] = (sequence.current[key] ?? 0) + 1;
      sequence.current[key] = ticket[key];
    }
    setPending((p) => ({ ...p, ...patch }));
    const ok = await actions.updateSettings(patch);
    setPending((p) => {
      const next = { ...p };
      for (const key of keys) if (sequence.current[key] === ticket[key]) delete next[key];
      return next;
    });
    if (!ok) setFailed(true);
  };

  return (
    <>
      <header className="header" data-tauri-drag-region="deep">
        <button type="button" className="icon-button" aria-label="Back to dashboard" onClick={onBack}>
          <Icon name="chevronLeft" />
        </button>
        <h1 className="screen__title">Settings</h1>
      </header>
      <div className="screen screen--settings">
        <div className="number-block">
          <span className="counter live-number">{formatInt(data.game.xpTotal)}</span>
          <span className="caption">XP total. Kept when you switch modes.</span>
        </div>

        <section className="settings-section" aria-labelledby="settings-mode">
          <span id="settings-mode" className="caption">
            Mode
          </span>
          <div className="choice-row" role="radiogroup" aria-label="Mode">
            {(["max", "diy"] as const).map((mode) => (
              <ChoiceTile
                key={mode}
                compact
                icon={mode === "max" ? "flame" : "wrench"}
                name={MODE_LABEL[mode]}
                fill={modeFill(mode, data.appInfo.season)}
                selected={shown.mode === mode}
                onSelect={() => {
                  if (shown.mode !== mode) void change({ mode });
                }}
              />
            ))}
          </div>
          <span className="caption">Titles follow the mode. XP stays.</span>
          {failed ? (
            <span className="caption caption--error" role="alert">
              Couldn’t save that. Try again.
            </span>
          ) : null}
        </section>

        <hr className="hairline" />

        <div className="toggle-list">
          <ToggleRow
            label="Sound"
            caption="On unlocks and level-ups"
            checked={shown.sound}
            onChange={(sound) => void change({ sound })}
          />
          <ToggleRow
            label="Start at login"
            caption="Lives in the tray, not in your way"
            checked={shown.autostart}
            onChange={(autostart) => void change({ autostart })}
          />
        </div>

        <hr className="hairline" />

        <LimitData
          ref={limitsRef}
          limits={data.dashboard.limits}
          platform={data.appInfo.platform}
          loadLimitSetup={actions.loadLimitSetup}
        />

        {data.appInfo.version === "" ? null : (
          <p className="footer-note">Tokenmonk {data.appInfo.version}</p>
        )}
      </div>
    </>
  );
}
