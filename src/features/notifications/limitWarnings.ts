/**
 * Limit warnings for OS notifications (docs/DESIGN.md, "While hidden").
 * Pure: feed it snapshots in order, it says which warnings just fired.
 * Each warning fires once per upward crossing: per 5-hour window
 * (`fiveHour.resetsAt`) for Max it out, per local day for the budget.
 */
import type { DashboardSnapshot, Mode } from "../../lib/contracts";

/** "Limit in ~25 min at this pace": N rounded to 5, at least 5. */
export function limitEtaText(etaMinutes: number): string {
  const rounded = Math.max(5, Math.round(etaMinutes / 5) * 5);
  return `Limit in ~${rounded} min at this pace`;
}

export interface OsNotice {
  title: string;
  body: string | null;
}

export interface WarningState {
  /** Last seen 5-hour reading; `null` until a trusted reading was seen. */
  window: { key: string; utilization: number } | null;
  /** Last seen budget reading; `null` until a trusted reading was seen. */
  budget: { day: string; usedPercent: number } | null;
}

export const initialWarningState: WarningState = { window: null, budget: null };

function localDay(epochMs: number): string {
  const d = new Date(epochMs);
  return `${d.getFullYear()}-${d.getMonth() + 1}-${d.getDate()}`;
}

/** Crossing from below `threshold` to at or above it. */
const crossed = (previous: number, current: number, threshold: number): boolean =>
  previous < threshold && current >= threshold;

export function evaluateWarnings(
  state: WarningState,
  snap: DashboardSnapshot,
  mode: Mode,
): { state: WarningState; notices: OsNotice[] } {
  // Importing, unreadable or empty data says nothing about the present.
  if (snap.data.state !== "ready") return { state, notices: [] };

  const notices: OsNotice[] = [];
  let next = state;

  if (mode === "max" && snap.limits.status === "ok" && snap.limits.fiveHour !== null) {
    const { utilization, resetsAt } = snap.limits.fiveHour;
    const key = String(resetsAt);
    // First reading is a baseline; a fresh window starts from zero.
    const previous =
      state.window === null ? utilization : state.window.key === key ? state.window.utilization : 0;
    if (crossed(previous, utilization, 100)) {
      notices.push({ title: "5h limit at 100 %", body: "Limit reached. Breathe." });
    } else if (crossed(previous, utilization, 90)) {
      const eta = snap.burnRate.limitEtaMinutes;
      notices.push({
        title: "5h limit at 90 %",
        body: eta === null || eta <= 0 ? null : `${limitEtaText(eta)}.`,
      });
    }
    next = { ...next, window: { key, utilization } };
  }

  const budget = snap.dailyBudget;
  const budgetTrusted =
    mode === "diy" && budget !== null && (budget.basis !== "weeklyLimit" || snap.limits.status === "ok");
  if (budgetTrusted && budget !== null) {
    const day = localDay(snap.generatedAt);
    const previous =
      state.budget === null ? budget.usedPercent : state.budget.day === day ? state.budget.usedPercent : 0;
    if (crossed(previous, budget.usedPercent, 100)) {
      notices.push({ title: "Today’s budget at 100 %", body: "Over budget. The AI wins today." });
    }
    next = { ...next, budget: { day, usedPercent: budget.usedPercent } };
  }

  return { state: next, notices };
}
