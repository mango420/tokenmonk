import { EASTER_EGG_CLICKS, EASTER_EGG_WINDOW_MS } from "../../lib/contracts";

/** Where the logo-click streak stands. */
export interface ClickStreak {
  count: number;
  /** Timestamp of the last counted click, in ms; `null` before the first. */
  lastAt: number | null;
}

export const NO_CLICKS: ClickStreak = { count: 0, lastAt: null };

/**
 * Counts a logo click. A click within `EASTER_EGG_WINDOW_MS` of the previous
 * one extends the streak, a later one starts a new streak. `triggered` turns
 * true on the click that completes `EASTER_EGG_CLICKS`; the streak then resets.
 */
export function registerClick(
  streak: ClickStreak,
  now: number,
): { streak: ClickStreak; triggered: boolean } {
  const chained = streak.lastAt !== null && now - streak.lastAt <= EASTER_EGG_WINDOW_MS;
  const count = chained ? streak.count + 1 : 1;
  if (count >= EASTER_EGG_CLICKS) return { streak: NO_CLICKS, triggered: true };
  return { streak: { count, lastAt: now }, triggered: false };
}
