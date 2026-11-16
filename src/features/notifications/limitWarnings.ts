/**
 * Limit warnings for OS notifications (docs/DESIGN.md, "While hidden").
 * Pure: feed it snapshots in order, it says which warnings just fired.
 * Each warning fires once per upward crossing: per 5-hour window
 * (`fiveHour.resetsAt`).
 */
import type { DashboardSnapshot } from "../../lib/contracts";

export interface OsNotice {
  title: string;
  body: string | null;
}

export interface WarningState {
  /** Last seen 5-hour reading; `null` until a trusted reading was seen. */
  window: { key: string; utilization: number } | null;
}

export const initialWarningState: WarningState = { window: null };

/** Crossing from below `threshold` to at or above it. */
const crossed = (previous: number, current: number, threshold: number): boolean =>
  previous < threshold && current >= threshold;

export function evaluateWarnings(
  state: WarningState,
  snap: DashboardSnapshot,
): { state: WarningState; notices: OsNotice[] } {
  // Importing, unreadable or empty data says nothing about the present.
  if (snap.data.state !== "ready") return { state, notices: [] };

  const notices: OsNotice[] = [];
  let next = state;

  if (snap.limits.status === "ok" && snap.limits.fiveHour !== null) {
    const { utilization, resetsAt } = snap.limits.fiveHour;
    const key = String(resetsAt);
    // First reading is a baseline; a fresh window starts from zero.
    const previous =
      state.window === null ? utilization : state.window.key === key ? state.window.utilization : 0;
    if (crossed(previous, utilization, 100)) {
      notices.push({ title: "5h limit at 100 %", body: "Limit reached. Breathe." });
    } else if (crossed(previous, utilization, 90)) {
      notices.push({ title: "5h limit at 90 %", body: null });
    }
    next = { ...next, window: { key, utilization } };
  }

  return { state: next, notices };
}
