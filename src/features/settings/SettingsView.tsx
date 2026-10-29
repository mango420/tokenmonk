import { useEffect, useState } from "react";
import { ChoiceTile } from "../../components/ChoiceTile";
import { Icon } from "../../components/Icon";
import { ToggleRow } from "../../components/ToggleRow";
import type { AppActions, AppData } from "../../lib/appData";
import type { SettingsPatch } from "../../lib/contracts";
import { formatInt } from "../../lib/format";
import { MODE_LABEL } from "../dashboard/model";

interface SettingsViewProps {
  data: AppData;
  actions: AppActions;
  onBack: () => void;
}

/**
 * Settings: every change applies at once through `update_settings`. While the
 * call runs the control stays in its new position; a rejected call flips it back.
 */
export function SettingsView({ data, actions, onBack }: SettingsViewProps) {
  const [pending, setPending] = useState<SettingsPatch>({});
  const [failed, setFailed] = useState(false);

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
    setPending((p) => ({ ...p, ...patch }));
    const ok = await actions.updateSettings(patch);
    setPending((p) => {
      const next = { ...p };
      for (const key of Object.keys(patch) as (keyof SettingsPatch)[]) delete next[key];
      return next;
    });
    if (!ok) setFailed(true);
  };

  return (
    <>
      <header className="header">
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
                fill={mode === "max" ? "saffron" : "jade"}
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

        {data.appInfo.version === "" ? null : (
          <p className="footer-note">Tokenmonk {data.appInfo.version}</p>
        )}
      </div>
    </>
  );
}
