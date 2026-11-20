import { useMemo } from "react";
import { readTokenNumber } from "../lib/tokens";
import { MonkFigure } from "./Monk";

/**
 * Five-bead gauge (docs/DESIGN.md, "Shared parts"): a ring with five full
 * beads and the monk inside. About shows it at `--gauge-size-sm`.
 */
export function FiveBeadGauge() {
  const geometry = useMemo(() => {
    const count = readTokenNumber("--bead-count");
    const start = readTokenNumber("--bead-start-angle");
    const step = readTokenNumber("--bead-step-angle");
    const view = readTokenNumber("--gauge-viewbox");
    const centre = view / 2;
    const radius = readTokenNumber("--gauge-radius");
    const beads = Array.from({ length: count }, (_, i) => {
      const angle = ((start + i * step) * Math.PI) / 180;
      return { cx: centre + radius * Math.cos(angle), cy: centre + radius * Math.sin(angle) };
    });
    return { view, centre, radius, beadRadius: readTokenNumber("--bead-radius"), beads };
  }, []);

  return (
    <svg className="five-gauge" viewBox={`0 0 ${geometry.view} ${geometry.view}`} aria-hidden="true">
      <circle className="five-gauge__track" cx={geometry.centre} cy={geometry.centre} r={geometry.radius} />
      <MonkFigure />
      {geometry.beads.map((bead, i) => (
        <circle key={i} className="five-gauge__bead" cx={bead.cx} cy={bead.cy} r={geometry.beadRadius} />
      ))}
    </svg>
  );
}
