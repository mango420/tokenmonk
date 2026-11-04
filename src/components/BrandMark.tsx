interface BrandMarkProps {
  /** Rendered size in px. */
  size: number;
  /** CSS colour of the arc; omitted uses the current accent. */
  color?: string;
}

/**
 * The static brand mark (docs/DESIGN.md, "Brand mark"): an arc on a circle and
 * a centre dot. Not a gauge. Drawn inline; see docs/DESIGN.md, "Brand mark".
 */
export function BrandMark({ size, color = "var(--accent)" }: BrandMarkProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true" className="brand-mark">
      <circle
        cx="16"
        cy="16"
        r="12"
        fill="none"
        stroke={color}
        strokeWidth="3.2"
        strokeLinecap="round"
        strokeDasharray="58 100"
        transform="rotate(-70 16 16)"
      />
      <circle cx="16" cy="16" r="3.4" fill="var(--ink)" />
    </svg>
  );
}
