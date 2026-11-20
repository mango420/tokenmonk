/**
 * The popup monk (docs/DESIGN.md, "Icon system", role B): drawn from the parts
 * table in the 280 x 280 gauge coordinate system, colours from tokens through
 * the `.monk__*` classes. The same drawing is the corner mark, the large monk
 * and the figure inside the five-bead gauge.
 */
export function MonkFigure() {
  return (
    <g className="monk">
      <path className="monk__robe" d="M82 216c2-46 26-74 58-74s56 28 58 74z" />
      <path className="monk__line" d="M112 152c10 22 30 40 62 46" />
      <path className="monk__line" d="M116 216c6-10 16-14 24-14s18 4 24 14" />
      <circle className="monk__head" cx="140" cy="106" r="24" />
      <path className="monk__line" d="M128 108c2 3 6 3 8 0M144 108c2 3 6 3 8 0" />
      <path className="monk__line" d="M135 118c3 2 7 2 10 0" />
    </g>
  );
}

/** Corner mark: the drawing cropped to `76 76 128 146`, shown at 26 x 30. */
export function MonkMark() {
  return (
    <svg className="corner__mark" viewBox="76 76 128 146" aria-hidden="true">
      <MonkFigure />
    </svg>
  );
}

/** Large monk with its ground shadow: `60 76 160 160`, shown at 120 x 120. */
export function MonkLarge() {
  return (
    <svg className="monk-lg" viewBox="60 76 160 160" aria-hidden="true">
      <ellipse className="monk__ground" cx="140" cy="220" rx="74" ry="13" />
      <MonkFigure />
    </svg>
  );
}
