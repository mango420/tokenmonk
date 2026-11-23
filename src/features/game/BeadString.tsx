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
 * track, 5 full beads, monk in the gauge coordinates. No motion.
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
      <circle className="game-monk__head" cx="108.3" cy="116.1" r="7.9" />
      <circle className="game-monk__head" cx="171.7" cy="116.1" r="7.9" />
      <circle className="game-monk__head" cx="140" cy="113.5" r="33" />
      <path className="game-monk__eyes" d="M118.9 115.2a7.9 7.9 0 0 0 15.8 0zM145.3 115.2a7.9 7.9 0 0 0 15.8 0z" />
      <path
        className="game-monk__robe"
        d="M85 184.3c0-23.7 23.1-33 55-33 9.6 0 18.3.9 25.9 2.7l14.8 47.1A63.8 63.8 0 0 1 85 184.3z"
      />
      <path
        className="game-monk__shoulder"
        d="M169.7 155c15.5 4.6 25.3 13.9 25.3 29.3a63.8 63.8 0 0 1-9.5 12.4z"
      />
    </svg>
  );
}
