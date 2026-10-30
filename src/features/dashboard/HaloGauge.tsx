import { useState } from "react";
import { Card } from "../../components/Card";
import { TextButton } from "../../components/TextButton";
import type { GaugeModel } from "./model";

/** Reads a pixel token (for example `--gauge-size`) from the document root once. */
function useTokenPx(name: string): number {
  const [value] = useState(() => {
    const raw = getComputedStyle(document.documentElement).getPropertyValue(name);
    const parsed = Number.parseFloat(raw);
    return Number.isFinite(parsed) ? parsed : 0;
  });
  return value;
}

interface HaloGaugeProps {
  model: GaugeModel;
  /** Opens Settings at "Limit data". */
  onOpenLimitSetup?: () => void;
}

/** Halo gauge card: ring with counter on the left, burn or mood text on the right. */
export function HaloGauge({ model, onOpenLimitSetup }: HaloGaugeProps) {
  const size = useTokenPx("--gauge-size");
  const radius = useTokenPx("--gauge-radius");
  const circumference = 2 * Math.PI * radius;
  const centre = size / 2;
  const filled = model.ratio !== null && model.ratio > 0;

  return (
    <Card className="gauge-card">
      <div className="gauge">
        <svg className="gauge__ring" viewBox={`0 0 ${size} ${size}`} aria-hidden="true">
          <circle className="gauge__track" cx={centre} cy={centre} r={radius} />
          {filled && model.ratio !== null ? (
            <circle
              className={`gauge__fill${model.danger ? " gauge__fill--danger" : ""}`}
              cx={centre}
              cy={centre}
              r={radius}
              strokeDasharray={`${(circumference * model.ratio).toFixed(1)} ${circumference.toFixed(1)}`}
              transform={`rotate(-90 ${centre} ${centre})`}
            />
          ) : null}
        </svg>
        <div className="gauge__centre">
          <span className={`gauge__value live-number${model.stale ? " gauge__value--stale" : ""}`}>
            {model.value}
          </span>
          <span className="gauge__caption">{model.caption}</span>
        </div>
      </div>
      <div className="gauge-card__text">
        <span className={`gauge-card__heading${model.headingAlert ? " gauge-card__heading--alert" : ""}`}>
          {model.heading}
        </span>
        {model.detail !== null && model.detail.action === "openLimitSetup" ? (
          <TextButton onClick={onOpenLimitSetup}>{model.detail.text}</TextButton>
        ) : model.detail !== null ? (
          <span className={`gauge-card__detail gauge-card__detail--${model.detail.tone}`}>
            {model.detail.text}
          </span>
        ) : null}
      </div>
    </Card>
  );
}
