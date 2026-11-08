import { useEffect, useState, type CSSProperties } from "react";
import type { RingModel } from "./model";

/** Reads a pixel token (for example `--ring-size`) from the document root once. */
function useTokenPx(name: string): number {
  const [value] = useState(() => {
    const raw = getComputedStyle(document.documentElement).getPropertyValue(name);
    const parsed = Number.parseFloat(raw);
    return Number.isFinite(parsed) ? parsed : 0;
  });
  return value;
}

/** The arcs draw only the first time the dashboard shows them after start. */
let ringDrawn = false;

/** Hand-drawn wobble (docs/DESIGN.md, "Dashboard", ring). */
const ROUGH_FILTER = "tokenmonk-rough";

interface HaloGaugeProps {
  model: RingModel;
}

/**
 * The ring: dotted track, a main arc that drains as the window
 * is used, the hero number in the centre. Never changes colour.
 */
export function HaloGauge({ model }: HaloGaugeProps) {
  const size = useTokenPx("--ring-size");
  const radius = useTokenPx("--ring-radius");
  const wobble = useTokenPx("--ring-wobble-scale");
  const centre = size / 2;
  const circumference = 2 * Math.PI * radius;

  const left = model.percentLeft;
  const arcs = left !== null && left > 0;
  const length = arcs ? (circumference * Math.min(100, left)) / 100 : 0;

  const [animate, setAnimate] = useState(() => !ringDrawn);
  useEffect(() => {
    if (arcs) ringDrawn = true;
  }, [arcs]);

  const arcClass = animate ? "ring__arc ring__arc--draw" : "ring__arc";
  const gap = circumference.toFixed(1);

  return (
    <div className="ring" style={{ "--ring-gap": gap } as CSSProperties}>
      <svg viewBox={`0 0 ${size} ${size}`} aria-hidden="true">
        <defs>
          <filter id={ROUGH_FILTER} x="-10%" y="-10%" width="120%" height="120%">
            <feTurbulence type="fractalNoise" baseFrequency="0.045" numOctaves="2" seed="4" result="noise" />
            <feDisplacementMap in="SourceGraphic" in2="noise" scale={wobble} />
          </filter>
        </defs>
        <circle className="ring__track" cx={centre} cy={centre} r={radius} />
        {arcs ? (
          <g filter={`url(#${ROUGH_FILTER})`}>
            <g transform={`rotate(-90 ${centre} ${centre})`}>
              <circle
                className={`${arcClass} ring__arc--main`}
                cx={centre}
                cy={centre}
                r={radius}
                strokeDasharray={`${length.toFixed(1)} ${gap}`}
                onAnimationEnd={() => setAnimate(false)}
              />
            </g>
          </g>
        ) : null}
      </svg>
      <div className="ring__centre">
        <div className="ring__hero-row">
          <span
            className={`ring__hero live-number${model.small ? " ring__hero--sm" : ""}${
              model.stale ? " ring__hero--stale" : ""
            }`}
          >
            {model.hero}
          </span>
          {model.unit === null ? null : <span className="ring__unit live-number">{model.unit}</span>}
        </div>
        <span className="ring__caption">{model.caption}</span>
      </div>
    </div>
  );
}
