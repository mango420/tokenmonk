/** Display formatting (docs/DESIGN.md, "Number formats"). No em dashes, 24 h clock. */

const intFormat = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;

/** Thousands with a comma: `184,230`. */
export function formatInt(n: number): string {
  return intFormat.format(Math.round(n));
}

/** One decimal, a trailing `.0` dropped: `1.1`, `120`. */
function oneDecimal(n: number): string {
  return n.toFixed(1).replace(/\.0$/, "");
}

/** Compact token value: `842`, `4.2k`, `120k`, `1.2M`. */
export function formatCompact(n: number): string {
  const rounded = Math.round(Math.max(0, n));
  if (rounded < 1_000) return String(rounded);
  const thousands = Math.round(rounded / 100) / 10;
  if (thousands < 1_000) return `${oneDecimal(thousands)}k`;
  return `${oneDecimal(rounded / 1_000_000)}M`;
}

/** Percent left of a window: `max(0, round(100 - utilization))`. */
export function percentLeft(utilization: number): number {
  return Math.max(0, Math.round(100 - utilization));
}

function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

/** Local 24 h time: `14:05`. */
export function formatClock(epochMs: number): string {
  const d = new Date(epochMs);
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** Local time on the same local day as `now` (`14:05`), else `3 Nov 14:05`. */
export function formatDayTime(epochMs: number, now: number = Date.now()): string {
  const d = new Date(epochMs);
  const n = new Date(now);
  const sameDay = d.getFullYear() === n.getFullYear() && d.getMonth() === n.getMonth() && d.getDate() === n.getDate();
  return sameDay ? formatClock(epochMs) : `${d.getDate()} ${MONTHS[d.getMonth()]} ${formatClock(epochMs)}`;
}

/** Reset duration: `2h 14m`, `0h 41m`; from 24 h `3d 4h`. Never negative. */
export function formatResetIn(ms: number): string {
  const clamped = Math.max(0, ms);
  if (clamped >= DAY_MS) {
    const days = Math.floor(clamped / DAY_MS);
    const hours = Math.floor((clamped % DAY_MS) / HOUR_MS);
    return `${days}d ${hours}h`;
  }
  const hours = Math.floor(clamped / HOUR_MS);
  const minutes = Math.floor((clamped % HOUR_MS) / MINUTE_MS);
  return `${hours}h ${minutes}m`;
}

/** Clamp to 0..1 for bars. */
export function clamp01(n: number): number {
  return Math.min(1, Math.max(0, n));
}
