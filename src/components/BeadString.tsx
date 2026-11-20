import { useEffect, useId, useMemo, useState, type CSSProperties } from "react";
import { readTokenNumber } from "../lib/tokens";

/** The beads fill in one after another only the first time a string shows after start. */
let introPlayed = false;

interface BeadStringProps {
  /** Percent left 0..100, as shown. */
  percentLeft: number;
  /** Last-known value: the accent fills are dimmed. */
  stale?: boolean;
}

/** Fill of bead `i` (0 based): `clamp((p - 20 i) / 20, 0, 1)`, five beads at 20 % each. */
export function beadFill(percentLeft: number, i: number, count: number): number {
  const share = 100 / count;
  return Math.min(1, Math.max(0, (percentLeft - share * i) / share));
}

/**
 * One limit window as five beads on a string (docs/DESIGN.md, "Shared parts").
 * Each bead fills from the bottom to its share; the string drains from the
 * right. The accent part is the bead circle clipped to a rect that slides
 * up from the bead's bottom edge.
 */
export function BeadString({ percentLeft, stale = false }: BeadStringProps) {
  const clipBase = useId();
  const g = useMemo(() => {
    const count = readTokenNumber("--bead-count");
    const pitch = readTokenNumber("--bead-pitch");
    const width = readTokenNumber("--bead-string-width");
    const height = readTokenNumber("--bead-string-height");
    const first = (width - pitch * (count - 1)) / 2;
    return {
      count,
      width,
      height,
      radius: readTokenNumber("--bead-radius"),
      cy: height / 2,
      xs: Array.from({ length: count }, (_, i) => first + i * pitch),
      stroke: readTokenNumber("--bead-stroke"),
    };
  }, []);

  const [intro] = useState(() => !introPlayed);
  const [started, setStarted] = useState(!intro);

  const [introActive, setIntroActive] = useState(intro);

  useEffect(() => {
    if (!intro) return;
    // Paint the empty string first, then let the beads transition in.
    const frame = requestAnimationFrame(() => setStarted(true));
    const done = window.setTimeout(
      () => setIntroActive(false),
      (g.count - 1) * readTokenNumber("--motion-bead-stagger") + readTokenNumber("--motion-bead") + 100,
    );
    return () => {
      cancelAnimationFrame(frame);
      window.clearTimeout(done);
    };
  }, [intro, g.count]);

  useEffect(() => {
    introPlayed = true;
  }, []);

  const classes = ["bead-string"];
  if (stale) classes.push("bead-string--stale");
  if (introActive) classes.push("bead-string--intro");

  return (
    <svg className={classes.join(" ")} viewBox={`0 0 ${g.width} ${g.height}`} aria-hidden="true">
      <path className="bead-string__line" d={`M${g.stroke + 0.5} ${g.cy}H${g.width - g.stroke - 0.5}`} />
      {g.xs.map((cx, i) => {
        const fill = started ? beadFill(percentLeft, i, g.count) : 0;
        const id = `${clipBase}-${i}`;
        return (
          <g key={i} style={{ "--bead-index": i } as CSSProperties}>
            <clipPath id={id}>
              <rect
                className="bead__clip"
                x={cx - g.radius}
                y={g.cy - g.radius}
                width={g.radius * 2}
                height={g.radius * 2}
                style={{ transform: `translateY(${(1 - fill) * g.radius * 2}px)` }}
              />
            </clipPath>
            <circle className="bead__base" cx={cx} cy={g.cy} r={g.radius} />
            <circle className="bead__fill" cx={cx} cy={g.cy} r={g.radius} clipPath={`url(#${id})`} />
            <circle
              className={fill > 0 ? "bead__edge" : "bead__edge bead__edge--empty"}
              cx={cx}
              cy={g.cy}
              r={g.radius}
            />
          </g>
        );
      })}
    </svg>
  );
}
