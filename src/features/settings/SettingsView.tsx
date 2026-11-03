import { useEffect, useRef, useState } from "react";
import { Switch } from "../../components/Switch";
import { TextLink } from "../../components/TextLink";
import { TopRow } from "../../components/TopRow";
import type { AppActions, AppData } from "../../lib/appData";
import type { SettingsPatch } from "../../lib/contracts";
import { useEscape } from "../../lib/useEscape";
import { MODE_DESCRIPTION, MODE_LABEL, type PopupMode } from "../dashboard/model";
import { LimitData } from "./LimitData";

interface SettingsViewProps {
  data: AppData;
  actions: AppActions;
  onBack: () => void;
  /** Scroll to "Limit data" on open (the dashboard's setup hint asks for it). */
  scrollToLimits?: boolean;
}

const MODES: readonly PopupMode[] = ["max", "diy"];

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

  useEscape(onBack);

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

  const version = data.appInfo.version;

  return (
    <div className="screen">
      <TopRow>
        <TextLink onClick={onBack}>Back</TextLink>
      </TopRow>
      <div className="scroll">
        <h1 className="title">Settings</h1>

        <span id="settings-mode" className="label">
          Mode
        </span>
        <div role="radiogroup" aria-labelledby="settings-mode">
          {MODES.map((mode) => (
            <button
              key={mode}
              type="button"
              role="radio"
              aria-checked={shown.mode === mode}
              className="mode-row"
              onClick={() => {
                if (shown.mode !== mode) void change({ mode });
              }}
            >
              <span className="mode-row__name">{MODE_LABEL[mode]}</span>
              <span className="mode-row__description">{MODE_DESCRIPTION[mode]}</span>
            </button>
          ))}
        </div>

        <div className="switch-list">
          <div className="switch-row">
            <span>Sound</span>
            <Switch label="Sound" checked={shown.sound} onChange={(sound) => void change({ sound })} />
          </div>
          <div className="switch-row">
            <span>Start at login</span>
            <Switch
              label="Start at login"
              checked={shown.autostart}
              onChange={(autostart) => void change({ autostart })}
            />
          </div>
          <div className="switch-end" />
        </div>
        {failed ? (
          <p className="note note--error" role="alert">
            Couldn’t save that. Try again.
          </p>
        ) : null}

        <LimitData
          ref={limitsRef}
          limits={data.dashboard.limits}
          platform={data.appInfo.platform}
          loadLimitSetup={actions.loadLimitSetup}
        />

        <p className="note note--bottom">
          {version === "" ? "Tokenmonk, MIT." : `Tokenmonk ${version}, MIT.`} Ideas and bugs go to GitHub
          issues.
        </p>
      </div>
    </div>
  );
}
