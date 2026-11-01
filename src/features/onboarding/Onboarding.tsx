import { useState } from "react";
import { ChoiceTile } from "../../components/ChoiceTile";
import { PrimaryButton } from "../../components/PrimaryButton";
import type { AppActions, AppData } from "../../lib/appData";
import type { Mode } from "../../lib/contracts";
import { formatInt } from "../../lib/format";
import { Header } from "../dashboard/Header";
import { importRatio, MODE_LABEL, modeFill, type PopupMode } from "../dashboard/model";

interface OnboardingProps {
  data: AppData;
  onChoose: AppActions["completeOnboarding"];
}

const DESCRIPTION: Record<PopupMode, string> = {
  max: "Use every token you pay for. XP for burning the limit.",
  diy: "Give the AI as little as possible. XP for staying under budget.",
};

function importLine(data: AppData["dashboard"]): string {
  switch (data.data.state) {
    case "importing":
      return `Reading logs · ${formatInt(data.data.filesDone)} / ${formatInt(data.data.filesTotal)} files`;
    case "ready":
      return `${formatInt(data.today.total)} tokens found today`;
    case "noLogs":
      return "No Claude Code logs yet";
    case "error":
      return "Can’t read the logs.";
  }
}

/** First start: "How do you want to vibe?" Two tiles, one button. */
export function Onboarding({ data, onChoose }: OnboardingProps) {
  const [selected, setSelected] = useState<PopupMode | null>(null);
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);

  const choose = async () => {
    if (selected === null || saving) return;
    setSaving(true);
    setFailed(false);
    const mode: Mode = selected;
    const ok = await onChoose(mode);
    // On success `settings-changed` switches the view away; only a failure stays here.
    setSaving(false);
    if (!ok) setFailed(true);
  };

  return (
    <>
      <Header mode={null} haloRatio={importRatio(data.dashboard.data)} />
      <div className="screen screen--onboarding">
        <h1 className="screen__title">How do you want to vibe?</h1>
        <div className="choice-group" role="radiogroup" aria-label="Mode">
          {(["max", "diy"] as const).map((mode) => (
            <ChoiceTile
              key={mode}
              icon={mode === "max" ? "flame" : "wrench"}
              name={MODE_LABEL[mode]}
              description={DESCRIPTION[mode]}
              fill={modeFill(mode)}
              selected={selected === mode}
              disabled={saving}
              onSelect={() => setSelected(mode)}
            />
          ))}
        </div>
        <p className="caption">Both paths level up. Switch any time in settings.</p>
        <div className="screen__bottom">
          {failed ? (
            <p className="caption caption--error" role="alert">
              Couldn’t save that. Try again.
            </p>
          ) : null}
          <PrimaryButton
            fullWidth
            disabled={selected === null || saving}
            fill={selected === null ? undefined : modeFill(selected)}
            onClick={() => void choose()}
          >
            {selected === null ? "Pick one" : `Start with ${MODE_LABEL[selected]}`}
          </PrimaryButton>
          <p className="import-line live-number">{importLine(data.dashboard)}</p>
        </div>
      </div>
    </>
  );
}
