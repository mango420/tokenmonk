import markSource from "../../assets/brand/tokenmonk-mark-mono.svg?raw";

/**
 * The monk mark for the level-up screen (docs/DESIGN.md, "Monk mark"): the
 * mono drawing in `assets/brand/tokenmonk-mark-mono.svg`, filled with the
 * `--accent` of the current mode. The eyes are cut-outs of the nonzero fill
 * and stay untouched; only the fill is rebound.
 */

function swap(source: string, from: string, to: string): string {
  if (!source.includes(from)) {
    throw new Error(`tokenmonk-mark-mono.svg changed: "${from}" not found`);
  }
  return source.split(from).join(to);
}

function buildMark(): string {
  let svg = markSource;
  svg = swap(svg, 'width="1024" height="1024"', 'class="logo-mark__svg"');
  svg = swap(svg, 'fill="#000"', 'style="fill:var(--accent)"');
  return svg;
}

const MARK = buildMark();

interface LogoMarkProps {
  /** Unused since the mark lost its halo; kept so existing callers compile. */
  ratio?: number | null;
}

export function LogoMark(_props: LogoMarkProps) {
  return (
    <span
      className="logo-mark"
      aria-hidden="true"
      // Static, bundled asset; no user input.
      dangerouslySetInnerHTML={{ __html: MARK }}
    />
  );
}
