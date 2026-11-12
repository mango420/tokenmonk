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
  UpdateStatus,
} from "../../lib/contracts";
import {
  clamp01,
  DAY_MS,
  formatClock,
  formatCompact,
  formatInt,
  formatResetIn,
  formatWeekday,
  percentLeft,
} from "../../lib/format";
import { roundEtaMinutes } from "../notifications/limitWarnings";

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

// ------------------------------------------------------------------ gauge

const WEEK_FIRST_STATUS = "The week runs out first.";

export interface GaugeModel {
  caption: string;
  /** Percent left 0..100 for the beads; `null` shows all beads empty. */
  percentLeft: number | null;
  /** Last-known value: the hero is `ink-2`, full beads are dimmed. */
  stale: boolean;
  /** Hero text: "73%" or the compact tokens today. */
  hero: string;
  /** Smaller hero (tokens today instead of percent). */
  small: boolean;
  heroLabel: string;
  status: string;
  /** Meta row, left; empty string keeps the row's height. */
  metaLeft: string;
  metaRight: string;
}

export interface WeekModel {
  /** Week bar width, percent. */
  percentLeft: number;
  /** "81% left · resets Thu" */
  figure: string;
  /** The week runs out first: the block becomes a card with one more line. */
  low: boolean;
  stale: boolean;
}

export interface DashboardModel {
  gauge: GaugeModel;
  /** `null` leaves the week block out. */
  week: WeekModel | null;
}

type StatusRows = readonly [string, string, string, string, string];

const STATUS: Record<PopupMode, StatusRows> = {
  // 60 and more, 30 to 59, 10 to 29, 1 to 9, exactly 0
  max: [
    "Barely touched. Use it.",
    "Good pace.",
    "Nearly there. As planned.",
    "Almost gone. Make it count.",
    "Limit reached. Breathe.",
  ],
  diy: [
    "Light day. Nicely done.",
    "Mid-range. Breathe.",
    "Getting thin. Slow down.",
    "Nearly out. Go for a walk.",
    "All out. Good time for a walk.",
  ],
};

/** Status line under the hero from percent left and the mode. */
export function statusLine(mode: PopupMode, left: number): string {
  const lines = STATUS[mode];
  if (left >= 60) return lines[0];
  if (left >= 30) return lines[1];
  if (left >= 10) return lines[2];
  if (left >= 1) return lines[3];
  return lines[4];
}

/** `limits.status = unavailable`: no limit data at all. */
export function hasNoLimitData(limits: LimitsView): boolean {
  return limits.status === "unavailable";
}

const tokensPerMinute = (snap: DashboardSnapshot) => `${formatCompact(snap.burnRate.tokensPerMinute)} tokens/min`;

/** Meta row, right: tokens per minute; in Do it yourself the daily budget replaces it. */
function metaRight(mode: PopupMode, snap: DashboardSnapshot): string {
  const budget = snap.dailyBudget;
  if (mode === "diy" && budget !== null) {
    if (budget.basis === "tokenHistory") {
      return `${formatCompact(Math.max(budget.allowance - budget.usedToday, 0))} left today`;
    }
    if (budget.tokensLeftPerDay !== null) return `~${formatCompact(budget.tokensLeftPerDay)} left today`;
  }
  return tokensPerMinute(snap);
}

/** Meta row, left: the limit ETA when it applies, else the reset. */
function metaLeft(snap: DashboardSnapshot, window: LimitWindow): string {
  const { limits, burnRate } = snap;
  const toReset = window.resetsAt === null ? null : window.resetsAt - snap.generatedAt;
  const eta = burnRate.limitEtaMinutes;
  if (limits.status === "ok" && eta !== null && eta >= 1) {
    const n = roundEtaMinutes(eta);
    const minutesToReset = toReset === null ? null : Math.floor(Math.max(0, toReset) / 60_000);
    if (minutesToReset === null || n < minutesToReset) return `Limit in ~${n} min`;
  }
  return toReset === null ? "" : `resets in ${formatResetIn(toReset)}`;
}

function tokensGauge(
  snap: DashboardSnapshot,
  caption: string,
  status: string,
  left: string,
  right: string,
): GaugeModel {
  return {
    caption,
    percentLeft: null,
    stale: false,
    hero: formatCompact(snap.today.total),
    small: true,
    heroLabel: "tokens today",
    status,
    metaLeft: left,
    metaRight: right,
  };
}

function weekModel(snap: DashboardSnapshot): WeekModel | null {
  const { limits } = snap;
  if (hasNoLimitData(limits) || limits.weekly === null) return null;
  const weekly = limits.weekly;
  const left = percentLeft(weekly.utilization);
  let figure = `${left}% left`;
  if (weekly.resetsAt !== null) {
    const ms = weekly.resetsAt - snap.generatedAt;
    figure += ` · resets ${ms < DAY_MS ? `in ${formatResetIn(ms)}` : formatWeekday(weekly.resetsAt)}`;
  }
  const five = limits.fiveHour === null ? null : percentLeft(limits.fiveHour.utilization);
  return {
    percentLeft: left,
    figure,
    low: five !== null && left < five && left < 20,
    stale: limits.status === "stale",
  };
}

export function buildDashboard(mode: PopupMode, snap: DashboardSnapshot): DashboardModel {
  const { limits } = snap;
  if (hasNoLimitData(limits)) {
    const budget = snap.dailyBudget;
    if (budget !== null && budget.basis === "tokenHistory") {
      const left = Math.max(0, Math.round(100 - budget.usedPercent));
      const status = budget.usedPercent > 100 ? "Over budget. The AI wins today." : statusLine(mode, left);
      const average = `average day ${formatCompact(budget.allowance)}`;
      return {
        gauge: {
          ...tokensGauge(snap, "Your token budget, today", status, average, metaRight(mode, snap)),
          percentLeft: left,
        },
        week: null,
      };
    }
    return {
      gauge: tokensGauge(
        snap,
        "Your Claude Code tokens, today",
        "No limit data. Showing tokens only.",
        "",
        tokensPerMinute(snap),
      ),
      week: null,
    };
  }

  const week = weekModel(snap);
  const caption = "Your Claude Code limit, this 5-hour window";
  const window = limits.fiveHour;
  if (window === null) {
    const status = "Fresh window. Numbers on your next reply.";
    return { gauge: tokensGauge(snap, caption, status, "", metaRight(mode, snap)), week };
  }

  const left = percentLeft(window.utilization);
  const stale = limits.status === "stale";
  let metaLeftText = metaLeft(snap, window);
  if (stale) metaLeftText = limits.fetchedAt === null ? "" : `as of ${formatClock(limits.fetchedAt)}`;
  return {
    gauge: {
      caption,
      percentLeft: left,
      stale,
      hero: `${left}%`,
      small: false,
      heroLabel: "of your limit left",
      status: week !== null && week.low ? WEEK_FIRST_STATUS : statusLine(mode, left),
      metaLeft: metaLeftText,
      metaRight: metaRight(mode, snap),
    },
    week,
  };
}

// ------------------------------------------------------------- level card

export interface LevelModel {
  /** Left of the card row: the title, or the replay line. */
  title: string;
  /** " · Level N" follows the title; `null` while old XP is being counted. */
  level: number | null;
  /** "340 / 500 XP"; `null` while old XP is being counted. */
  xp: string | null;
  /** `null` shows the track only. */
  ratio: number | null;
}

export function buildLevel(game: GameState): LevelModel {
  if (!game.retroDone) {
    return { title: "Counting old XP. Hold still.", level: null, xp: null, ratio: null };
  }
  return {
    title: game.title,
    level: game.level,
    xp: `${formatInt(game.xpIntoLevel)} / ${formatInt(game.xpForLevel)} XP`,
    ratio: clamp01(game.progress),
  };
}

// -------------------------------------------------------------------- note

/** A piece of the note line; numbers are mono. */
export type NoteSegment = { text: string; mono?: boolean };

export type NoteModel =
  | { kind: "text"; segments: NoteSegment[] }
  | { kind: "limitSetup"; text: string }
  /** Update line: `action` is the text link after the sentence, if any. */
  | { kind: "update"; segments: NoteSegment[]; action: string | null };

/** One line, first match wins (docs/DESIGN.md, "Note line"). */
export function buildNote(snap: DashboardSnapshot, update: UpdateStatus): NoteModel {
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
  if (update.kind === "available") {
    return {
      kind: "update",
      segments: [{ text: update.version, mono: true }, { text: " is out." }],
      action: "Update",
    };
  }
  if (update.kind === "downloading") {
    return {
      kind: "update",
      segments:
        update.percent === null
          ? [{ text: "Downloading " }, { text: update.version, mono: true }, { text: "." }]
          : [
              { text: "Downloading " },
              { text: update.version, mono: true },
              { text: ", " },
              { text: `${Math.round(update.percent)}%`, mono: true },
              { text: "." },
            ],
      action: null,
    };
  }
  if (update.kind === "installing") {
    return {
      kind: "update",
      segments: [
        { text: "Installing " },
        { text: update.version, mono: true },
        { text: ". Back in a moment." },
      ],
      action: null,
    };
  }
  if (update.kind === "installFailed") {
    return {
      kind: "update",
      segments: [{ text: update.version, mono: true }, { text: " didn’t install." }],
      action: "Try again",
    };
  }
  if (limits.reason === "noSource") {
    return { kind: "limitSetup", text: "No limit data. It comes from the status line, see Settings." };
  }
  return {
    kind: "text",
    segments: [{ text: "Limit % includes claude.ai chats. Token stats are Claude Code only." }],
  };
}
