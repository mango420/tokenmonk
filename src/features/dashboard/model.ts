/**
 * Pure view models for the popup dashboard. Copy and rules follow
 * docs/DESIGN.md (popup layout, note line, dashboard states). "Now" is always
 * `DashboardSnapshot.generatedAt`, so the output is a pure function of the
 * snapshot.
 */
import type {
  DashboardSnapshot,
  GameState,
  LimitWindow,
  LimitsView,
  Mode,
} from "../../lib/contracts";
import {
  clamp01,
  formatClock,
  formatCompact,
  formatInt,
  formatResetIn,
  percentLeft,
} from "../../lib/format";

export type PopupMode = "max" | "diy";

/** Before onboarding (`noneYet`) the popup falls back to Max it out. */
export function toPopupMode(mode: Mode): PopupMode {
  return mode === "diy" ? "diy" : "max";
}

export const MODE_LABEL: Record<PopupMode, string> = {
  max: "Max it out",
  diy: "Do it yourself",
};

export const MODE_DESCRIPTION: Record<PopupMode, string> = {
  max: "Use the whole limit, every window.",
  diy: "Use less. Keep it calm.",
};

/** The 5 hours / Week segment. */
export type WindowChoice = "fiveHour" | "weekly";

export const WINDOW_LABEL: Record<WindowChoice, string> = {
  fiveHour: "5 hours",
  weekly: "Week",
};

// -------------------------------------------------------------------- ring

export interface RingModel {
  /** Hero text in the ring centre. */
  hero: string;
  /** `%` after the hero; only with a percent hero. */
  unit: string | null;
  caption: string;
  /** Percent left 0..100 for the arcs; `null` shows the track only. */
  percentLeft: number | null;
  /** Last-known value: the hero is `ink-2`. */
  stale: boolean;
  /** Smaller hero (tokens today instead of percent). */
  small: boolean;
  status: string;
  /** Reset line; empty string keeps its height. */
  reset: string;
}

const STATUS: Record<PopupMode, readonly [string, string, string, string]> = {
  // 60 and more, 30 to 59, 10 to 29, below 10
  max: ["Barely touched. Use it.", "Good pace.", "Nearly there. As planned.", "Almost gone. Make it count."],
  diy: ["Light day. Nicely done.", "Mid-range. Breathe.", "Getting thin. Slow down.", "Nearly out. Go for a walk."],
};

/** Status line under the ring from percent left and the mode. */
export function statusLine(mode: PopupMode, left: number): string {
  const lines = STATUS[mode];
  if (left >= 60) return lines[0];
  if (left >= 30) return lines[1];
  if (left >= 10) return lines[2];
  return lines[3];
}

/** `limits.status = unavailable`: no limit data at all. */
export function hasNoLimitData(limits: LimitsView): boolean {
  return limits.status === "unavailable";
}

export function selectedWindow(limits: LimitsView, choice: WindowChoice): LimitWindow | null {
  return choice === "fiveHour" ? limits.fiveHour : limits.weekly;
}

function tokensRing(snap: DashboardSnapshot, status: string): RingModel {
  return {
    hero: formatCompact(snap.today.total),
    unit: null,
    caption: "tokens today",
    percentLeft: null,
    stale: false,
    small: true,
    status,
    reset: "",
  };
}

export function buildRing(mode: PopupMode, snap: DashboardSnapshot, choice: WindowChoice): RingModel {
  const { limits } = snap;
  if (hasNoLimitData(limits)) return tokensRing(snap, "No limit data. Showing tokens only.");
  const window = selectedWindow(limits, choice);
  if (window === null) return tokensRing(snap, "Fresh window. Numbers on your next reply.");

  const left = percentLeft(window.utilization);
  const stale = limits.status === "stale";
  let reset = "";
  if (stale) {
    if (limits.fetchedAt !== null) {
      reset = `As of ${formatClock(limits.fetchedAt)}. Updates on your next reply.`;
    }
  } else if (window.resetsAt !== null) {
    reset = `Resets in ${formatResetIn(window.resetsAt - snap.generatedAt)}`;
  }
  return {
    hero: String(left),
    unit: "%",
    caption: "left",
    percentLeft: left,
    stale,
    small: false,
    status: statusLine(mode, left),
    reset,
  };
}

// ------------------------------------------------------------------- stats

export interface StatModel {
  label: string;
  value: string;
  unit: string;
}

export function buildStats(snap: DashboardSnapshot): StatModel[] {
  return [
    {
      label: "Burn rate",
      value: formatCompact(snap.burnRate.tokensPerMinute),
      unit: "tokens per minute",
    },
    budgetStat(snap.dailyBudget),
  ];
}

function budgetStat(budget: DashboardSnapshot["dailyBudget"]): StatModel {
  const label = "Daily budget";
  if (budget === null) return { label, value: "n/a", unit: "needs limit data" };
  if (budget.basis === "tokenHistory") {
    // Allowance is in tokens: the average active day of the past week.
    const left = Math.max(budget.allowance - budget.usedToday, 0);
    return { label, value: formatCompact(left), unit: "tokens left today" };
  }
  if (budget.tokensLeftPerDay === null) return { label, value: "n/a", unit: "no pace yet" };
  return { label, value: formatCompact(budget.tokensLeftPerDay), unit: "tokens left per day" };
}

// ------------------------------------------------------------- level block

export interface LevelModel {
  title: string;
  /** "340 / 500 XP"; `null` while old XP is being counted. */
  xp: string | null;
  /** "Level 7", or the replay line. */
  line: string;
  /** `null` shows the track only. */
  ratio: number | null;
}

export function buildLevel(game: GameState): LevelModel {
  if (!game.retroDone) {
    return { title: game.title, xp: null, line: "Counting old XP. Hold still.", ratio: null };
  }
  return {
    title: game.title,
    xp: `${formatInt(game.xpIntoLevel)} / ${formatInt(game.xpForLevel)} XP`,
    line: `Level ${game.level}`,
    ratio: clamp01(game.progress),
  };
}

// -------------------------------------------------------------------- note

/** A piece of the note line; numbers are mono. */
export type NoteSegment = { text: string; mono?: boolean };

export type NoteModel =
  | { kind: "text"; segments: NoteSegment[] }
  | { kind: "limitSetup"; text: string };

/** One line, first match wins (docs/DESIGN.md, "Note line"). */
export function buildNote(snap: DashboardSnapshot): NoteModel {
  const { data, limits } = snap;
  if (data.state === "importing") {
    return {
      kind: "text",
      segments: [
        { text: "Reading logs, " },
        { text: formatInt(data.filesDone), mono: true },
        { text: " of " },
        { text: formatInt(data.filesTotal), mono: true },
        { text: " files." },
      ],
    };
  }
  if (data.state === "noLogs") {
    return { kind: "text", segments: [{ text: "No Claude Code logs yet. Start a session." }] };
  }
  if (data.state === "error") {
    return { kind: "text", segments: [{ text: "Can’t read the logs. Retrying in the background." }] };
  }
  if (limits.reason === "noSource") {
    return { kind: "limitSetup", text: "No limit data yet. Setup is in Settings." };
  }
  return {
    kind: "text",
    segments: [{ text: "Limit % includes claude.ai chats. Token stats are Claude Code only." }],
  };
}
