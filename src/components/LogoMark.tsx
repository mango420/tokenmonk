import markSource from "../../assets/brand/tokenmonk-mark.svg?raw";

/**
 * The brand mark with a live halo. The drawing stays in
 * `assets/brand/tokenmonk-mark.svg`; the colours the design system owns are
 * rebound to token variables and the halo arc length is set from `ratio`.
 * Only the halo follows the mode; the robe stays the brand saffron.
 */

/** Halo circle radius in the mark's 128 x 128 viewBox (DESIGN.md, "Logo and halo"). */
const HALO_RADIUS = 54;
const HALO_LENGTH = 2 * Math.PI * HALO_RADIUS;

function swap(source: string, from: string, to: string): string {
  if (!source.includes(from)) {
    throw new Error(`tokenmonk-mark.svg changed: "${from}" not found`);
  }
  return source.split(from).join(to);
}

const ARC = 'stroke="#F5A142" stroke-width="8" stroke-linecap="round" stroke-dasharray="244 340"';

function buildMark(ratio: number | null): string {
  let svg = markSource;
  svg = swap(svg, 'width="128" height="128"', 'class="logo-mark__svg"');
  svg = swap(svg, 'id="c"', 'id="logo-mark-clip"');
  svg = swap(svg, "url(#c)", "url(#logo-mark-clip)");
  const arc =
    ratio === null || ratio <= 0
      ? 'style="stroke:none"'
      : `style="stroke:var(--halo)" stroke-width="8" stroke-linecap="round" stroke-dasharray="${(
          HALO_LENGTH * ratio
        ).toFixed(1)} ${HALO_LENGTH.toFixed(1)}"`;
  svg = swap(svg, ARC, arc);
  svg = swap(svg, 'stroke="#2A2631"', 'style="stroke:var(--gauge-track)"');
  svg = swap(svg, 'fill="#1F1C25"', 'style="fill:var(--surface-200)"');
  svg = swap(svg, 'stroke="#1A1206"', 'style="stroke:var(--on-accent)"');
  return svg;
}

interface LogoMarkProps {
  /** Halo fill 0..1; `null` shows the track only. */
  ratio: number | null;
}

export function LogoMark({ ratio }: LogoMarkProps) {
  return (
    <span
      className="logo-mark"
      aria-hidden="true"
      // Static, bundled asset with computed numbers only; no user input.
      dangerouslySetInnerHTML={{ __html: buildMark(ratio) }}
    />
  );
}
