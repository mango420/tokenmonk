import { GAUGE } from "./constants";

const BEADS = Array.from({ length: GAUGE.beads }, (_, i) => {
  const angle = ((GAUGE.startDeg + i * GAUGE.stepDeg) * Math.PI) / 180;
  return {
    cx: +(GAUGE.centre + GAUGE.radius * Math.cos(angle)).toFixed(2),
    cy: +(GAUGE.centre + GAUGE.radius * Math.sin(angle)).toFixed(2),
  };
});

/**
 * Static full bead string with the monk inside (DESIGN.md, Level-up screen):
 * track, 20 full beads, monk in the gauge coordinates. No motion.
 */
export function BeadString() {
  return (
    <svg
      className="game-beads"
      viewBox={`0 0 ${GAUGE.viewBox} ${GAUGE.viewBox}`}
      aria-hidden="true"
      focusable="false"
    >
      <circle className="game-beads__track" cx={GAUGE.centre} cy={GAUGE.centre} r={GAUGE.radius} />
      {BEADS.map((bead, i) => (
        <circle key={i} className="game-beads__bead" cx={bead.cx} cy={bead.cy} />
      ))}
      <path className="game-monk__robe" d="M82 216c2-46 26-74 58-74s56 28 58 74z" />
      <path className="game-monk__line" d="M112 152c10 22 30 40 62 46" />
      <path className="game-monk__line" d="M116 216c6-10 16-14 24-14s18 4 24 14" />
      <circle className="game-monk__head" cx="140" cy="106" r="24" />
      <path className="game-monk__line" d="M128 108c2 3 6 3 8 0M144 108c2 3 6 3 8 0" />
      <path className="game-monk__line" d="M135 118c3 2 7 2 10 0" />
    </svg>
  );
}
