/**
 * The popup monk (docs/DESIGN.md, "Icon system", role B): drawn from the parts
 * table in the 280 x 280 gauge coordinate system, colours from tokens through
 * the `.monk__*` classes. The same drawing sits inside the bead gauge and,
 * cropped, as the standalone mark.
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
