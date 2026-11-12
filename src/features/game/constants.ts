/** Fade duration of the level-up screen (DESIGN.md, Derived screens: Motion). Not a design token. */
export const FADE_MS = 150;

/** How long an achievement toast stays up (tokens.css `--toast-visible`). */
export const TOAST_VISIBLE_MS = 3400;

/** Toast slide duration (tokens.css `--motion-toast`); the toast is removed once it has slid out. */
export const TOAST_SLIDE_MS = 280;

/** This many waiting unlocks collapse into one summary toast. */
export const TOAST_COLLAPSE_AT = 3;

/** Bead gauge geometry, mirrors the gauge tokens in tokens.css (SVG units). */
export const GAUGE = { viewBox: 280, centre: 140, radius: 112, beads: 20, startDeg: -90, stepDeg: 18 } as const;
