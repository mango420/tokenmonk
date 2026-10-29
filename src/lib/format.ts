/** Display formatting. House style: `72 %`, `184,230`, 24 h clock, no em dashes. */

const intFormat = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });
const weekdayFormat = new Intl.DateTimeFormat("en-US", { weekday: "long" });

/** Thousands with a comma: `184,230`. */
export function formatInt(n: number): string {
  return intFormat.format(Math.round(n));
}

/** Percent with a space: `72 %`. */
export function formatPercent(n: number): string {
  return `${formatInt(n)} %`;
}

/** Full number below 10,000, otherwise one decimal with `k` (`12.4k`). Used inside the gauge. */
export function formatShort(n: number): string {
  const rounded = Math.round(n);
  return rounded >= 10_000 ? `${(rounded / 1000).toFixed(1)}k` : formatInt(rounded);
}

function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

/** Local 24 h time: `14:05`. */
export function formatClock(epochMs: number): string {
  const d = new Date(epochMs);
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
}

/** Local weekday and time: `Thursday, 09:00`. */
export function formatWeekdayClock(epochMs: number): string {
  return `${weekdayFormat.format(new Date(epochMs))}, ${formatClock(epochMs)}`;
}

/** Remaining time as `2 h 14 min` or `38 min`; never below `1 min`. */
export function formatDuration(ms: number): string {
  const totalMin = Math.max(1, Math.round(ms / 60_000));
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  if (h === 0) return `${m} min`;
  return `${h} h ${m} min`;
}

/** Clamp to 0..1 for rings and bars. */
export function clamp01(n: number): number {
  return Math.min(1, Math.max(0, n));
}
