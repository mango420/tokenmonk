import markSource from "../../assets/brand/tokenmonk-mark-mono.svg?raw";

/**
 * The two-tone monk mark (docs/DESIGN.md, "Icon system"): the mono drawing in
 * `assets/brand/tokenmonk-mark-mono.svg`, head and sleeve in `--ink`, robe in
 * the `--accent` of the surrounding mode. The eyes are cut-outs of the nonzero
 * fill and stay untouched; only the fills are rebound.
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
  // Paths in file order: head, robe, sleeve. Head and sleeve are ink, the robe the accent.
  const fills = ["var(--ink)", "var(--accent)", "var(--ink)"];
  let i = 0;
  svg = svg.replace(/fill="#000"/g, () => `style="fill:${fills[i++]}"`);
  if (i !== fills.length) {
    throw new Error("tokenmonk-mark-mono.svg changed: expected 3 paths");
  }
  return svg;
}

const MARK = buildMark();

interface LogoMarkProps {
  /** `brand` is the 26 px top-row size, `lg` (default) the 52 px one. */
  size?: "brand" | "lg";
}

export function LogoMark({ size = "lg" }: LogoMarkProps) {
  return (
    <span
      className={size === "brand" ? "logo-mark logo-mark--brand" : "logo-mark"}
      aria-hidden="true"
      // Static, bundled asset; no user input.
      dangerouslySetInnerHTML={{ __html: MARK }}
    />
  );
}
