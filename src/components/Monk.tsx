/**
 * The popup monk (docs/DESIGN.md, "Icon system", role B): drawn from the parts
 * table in the 280 x 280 gauge coordinate system, colours from tokens through
 * the `.monk__*` classes. The same drawing is the corner mark, the large monk
 * and the figure inside the five-bead gauge.
 */
export function MonkFigure() {
  return (
    <g className="monk">
      <circle className="monk__head" cx="108.3" cy="116.1" r="7.9" />
      <circle className="monk__head" cx="171.7" cy="116.1" r="7.9" />
      <circle className="monk__head" cx="140" cy="113.5" r="33" />
      <path className="monk__eyes" d="M118.9 115.2a7.9 7.9 0 0 0 15.8 0zM145.3 115.2a7.9 7.9 0 0 0 15.8 0z" />
      <path
        className="monk__robe"
        d="M85 184.3c0-23.7 23.1-33 55-33 9.6 0 18.3.9 25.9 2.7l14.8 47.1A63.8 63.8 0 0 1 85 184.3z"
      />
      <path
        className="monk__shoulder"
        d="M169.7 155c15.5 4.6 25.3 13.9 25.3 29.3a63.8 63.8 0 0 1-9.5 12.4z"
      />
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
