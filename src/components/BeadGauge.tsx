import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { readTokenNumber } from "../lib/tokens";
import { MonkFigure } from "./Monk";

/** The beads fill in one after another only the first time the gauge shows after start. */
let introPlayed = false;

interface BeadGaugeProps {
  /** Percent left 0..100; `null` shows all beads empty. */
  percentLeft: number | null;
  /** Last-known value: full beads are dimmed. */
  stale?: boolean;
  /** Level-up screen size. */
  large?: boolean;
  /** Week runs out first: 20 px smaller to make room for the week card. */
  compact?: boolean;
}

/** Bead count from percent left: 0 only at exactly 0, otherwise `ceil(percent / 5)`. */
function beadsFor(percentLeft: number | null, count: number): number {
  if (percentLeft === null || percentLeft <= 0) return 0;
  return Math.min(count, Math.ceil((percentLeft * count) / 100));
}

/**
 * The bead gauge (docs/DESIGN.md, "Shared parts"): 20 beads on a track, the
 * monk inside. Beads 1 to N are full, clockwise from 12 o'clock; the string
 * drains from the end.
 */
export function BeadGauge({ percentLeft, stale = false, large = false, compact = false }: BeadGaugeProps) {
  const geometry = useMemo(() => {
    const count = readTokenNumber("--bead-count");
    const radius = readTokenNumber("--gauge-radius");
    const start = readTokenNumber("--bead-start-angle");
    const step = readTokenNumber("--bead-step-angle");
    const centre = readTokenNumber("--gauge-viewbox") / 2;
    const beads = Array.from({ length: count }, (_, i) => {
      const angle = ((start + i * step) * Math.PI) / 180;
      return { cx: centre + radius * Math.cos(angle), cy: centre + radius * Math.sin(angle) };
    });
    return {
      count,
      centre,
      radius,
      view: centre * 2,
      beadRadius: readTokenNumber("--bead-radius"),
      introMs: (count - 1) * readTokenNumber("--motion-bead-stagger") + readTokenNumber("--motion-bead"),
      beads,
    };
  }, []);

  const target = beadsFor(percentLeft, geometry.count);
  const [intro] = useState(() => !introPlayed);
  const [started, setStarted] = useState(!intro);
  const [introActive, setIntroActive] = useState(intro);

  useEffect(() => {
    if (!intro) return;
    // Paint the empty look first, then let the full beads transition in.
    const frame = requestAnimationFrame(() => setStarted(true));
    const done = window.setTimeout(() => setIntroActive(false), geometry.introMs + 100);
    return () => {
      cancelAnimationFrame(frame);
      window.clearTimeout(done);
    };
  }, [intro, geometry.introMs]);

  useEffect(() => {
    if (target > 0) introPlayed = true;
  }, [target]);

  const shown = started ? target : 0;
  const classes = ["bead-gauge"];
  if (large) classes.push("bead-gauge--lg");
  if (compact) classes.push("bead-gauge--compact");
  if (stale) classes.push("bead-gauge--stale");
  if (introActive) classes.push("bead-gauge--intro");

  return (
    <svg
      className={classes.join(" ")}
      viewBox={`0 0 ${geometry.view} ${geometry.view}`}
      aria-hidden="true"
    >
      <circle className="bead-gauge__track" cx={geometry.centre} cy={geometry.centre} r={geometry.radius} />
      <MonkFigure />
      {geometry.beads.map((bead, i) => (
        <circle
          key={i}
          className={i < shown ? "bead bead--full" : "bead bead--empty"}
          cx={bead.cx}
          cy={bead.cy}
          r={geometry.beadRadius}
          style={{ "--bead-index": i } as CSSProperties}
        />
      ))}
    </svg>
  );
}
