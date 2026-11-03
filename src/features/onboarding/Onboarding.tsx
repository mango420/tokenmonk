import { useState } from "react";
import { BrandMark } from "../../components/BrandMark";
import type { AppActions } from "../../lib/appData";
import { Header } from "../dashboard/Header";
import { MODE_DESCRIPTION, MODE_LABEL, type PopupMode } from "../dashboard/model";

interface OnboardingProps {
  onChoose: AppActions["completeOnboarding"];
}

const MODES: readonly PopupMode[] = ["max", "diy"];

const MARK_COLOR: Record<PopupMode, string> = {
  max: "var(--saffron)",
  diy: "var(--jade)",
};

/** First start: "How do you want to vibe?" A click on a mode chooses it at once. */
export function Onboarding({ onChoose }: OnboardingProps) {
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);

  const choose = async (mode: PopupMode) => {
    if (saving) return;
    setSaving(true);
    setFailed(false);
    const ok = await onChoose(mode);
    // On success `settings-changed` switches the view away; only a failure stays here.
    setSaving(false);
    if (!ok) setFailed(true);
  };

  return (
    <div className="screen">
      <Header />
      <h1 className="onboard-title">How do you want to vibe?</h1>
      <p className="onboard-lead">Pick one. You can switch later.</p>
      {MODES.map((mode) => (
        <button
          key={mode}
          type="button"
          className="onboard-choice"
          disabled={saving}
          onClick={() => void choose(mode)}
        >
          <BrandMark size={52} color={MARK_COLOR[mode]} />
          <span>
            <span className="onboard-choice__name">{MODE_LABEL[mode]}</span>
            <span className="onboard-choice__description">{MODE_DESCRIPTION[mode]}</span>
          </span>
        </button>
      ))}
      <p className="note note--onboard" role={failed ? "alert" : undefined}>
        {failed ? "Couldn’t save that. Try again." : "Mindful about your tokens. Or not."}
      </p>
    </div>
  );
}
