/**
 * Pure view models for the popup dashboard. Copy and rules follow
 * docs/DESIGN.md (popup layout, low limit, note line, dashboard states). "Now"
 * is always `DashboardSnapshot.generatedAt`, so the output is a pure function
 * of the snapshot.
 */
import type {
  DashboardSnapshot,
  GameState,
  LimitWindow,
  LimitsConnection,
  LimitsView,
  UpdateStatus,
} from "../../lib/contracts";
import {
  DAY_MS,
  formatClock,
  formatCompact,
  formatInt,
  formatResetIn,
  formatWeekday,
  percentLeft,
} from "../../lib/format";

// ---------------------------------------------------------------- windows

/** Below this percent left a window becomes the low band (same 10 % as the notification). */
const LOW_BELOW = 10;
/** The week runs out first only under this percent left. */
const WEEK_FIRST_BELOW = 20;

export interface WindowModel {
  key: "fiveHour" | "weekly";
  /** "5 hours", or "5 hours, nearly out" in the low band. */
  label: string;
  /** "resets in 2h 14m"; empty while the reset time is unknown. */
  reset: string;
  /** Integer percent left. */
  percent: number;
  stale: boolean;
  /** The low band (0 to 9 percent left). */
  low: boolean;
}

export interface ClosingLine {
  text: string;
  /** The line opens Settings at "Limits". */
  link: boolean;
}

/** The hero run of the no-limits layout: "184.2" and "k". */
export interface TokensToday {
  number: string;
  unit: string;
}

export interface DashboardModel {
  /** Empty: the no-limits layout applies. */
  windows: WindowModel[];
  closing: ClosingLine;
  tokensToday: TokensToday;
}

const STATUS = [
  // 60 and more, 30 to 59, 10 to 29, 1 to 9, exactly 0
  "Barely touched. Use it.",
  "Good pace.",
  "Nearly there. As planned.",
  "Almost gone. Make it count.",
  "Limit reached. Breathe.",
] as const;

/** Closing line from the 5-hour percent left. */
export function statusLine(left: number): string {
  if (left >= 60) return STATUS[0];
  if (left >= 30) return STATUS[1];
  if (left >= 10) return STATUS[2];
  if (left >= 1) return STATUS[3];
  return STATUS[4];
}

/** `limits.status = unavailable`: no limit data at all. */
export function hasNoLimitData(limits: LimitsView): boolean {
  return limits.status === "unavailable";
}

/** Splits a compact count into the hero number and its unit: "184.2k" to "184.2" + "k". */
export function splitCompact(text: string): TokensToday {
  const unit = /[kM]$/.test(text) ? text.slice(-1) : "";
  return { number: unit === "" ? text : text.slice(0, -1), unit };
}

function windowLabel(base: string, left: number): string {
  return left >= 1 && left < LOW_BELOW ? `${base}, nearly out` : base;
}

function fiveHourModel(snap: DashboardSnapshot, window: LimitWindow): WindowModel {
  const { limits } = snap;
  const left = percentLeft(window.utilization);
  const stale = limits.status === "stale";
  let reset = window.resetsAt === null ? "" : `resets in ${formatResetIn(window.resetsAt - snap.generatedAt)}`;
  if (stale) reset = limits.fetchedAt === null ? "" : `as of ${formatClock(limits.fetchedAt)}`;
  return {
    key: "fiveHour",
    label: windowLabel("5 hours", left),
    reset,
    percent: left,
    stale,
    low: left < LOW_BELOW,
  };
}

function weeklyModel(snap: DashboardSnapshot, window: LimitWindow): WindowModel {
  const left = percentLeft(window.utilization);
  let reset = "";
  if (window.resetsAt !== null) {
    const ms = window.resetsAt - snap.generatedAt;
    reset = `resets ${ms < DAY_MS ? `in ${formatResetIn(ms)}` : formatWeekday(window.resetsAt)}`;
  }
  return {
    key: "weekly",
    label: windowLabel("7 days", left),
    reset,
    percent: left,
    stale: snap.limits.status === "stale",
    low: left < LOW_BELOW,
  };
}

function noLimitsClosing(connection: LimitsConnection): ClosingLine {
  if (connection.state === "connected") return { text: "Limits after your next reply.", link: false };
  if (connection.state === "outdated") return { text: "Reconnect limits in Settings.", link: true };
  return { text: "Connect limits in Settings.", link: true };
}

export function buildDashboard(snap: DashboardSnapshot, connection: LimitsConnection): DashboardModel {
  const { limits } = snap;
  const tokensToday = splitCompact(formatCompact(snap.today.total));
  const five = hasNoLimitData(limits) || limits.fiveHour === null ? null : fiveHourModel(snap, limits.fiveHour);
  const week = hasNoLimitData(limits) || limits.weekly === null ? null : weeklyModel(snap, limits.weekly);
  const windows = [five, week].filter((w): w is WindowModel => w !== null);

  if (windows.length === 0) return { windows, closing: noLimitsClosing(connection), tokensToday };

  let text: string;
  if (five !== null && week !== null && week.percent < five.percent && week.percent < WEEK_FIRST_BELOW) {
    text = "The week runs out first.";
  } else if (five === null) {
    text = "Fresh window. Numbers on your next reply.";
  } else {
    text = statusLine(five.percent);
  }
  return { windows, closing: { text, link: false }, tokensToday };
}

// ------------------------------------------------------------- rank block

export interface LevelModel {
  /** `GameState.title`. */
  title: string;
  /** "Level 7, 340 of 920 XP", or the replay line while old XP is counted. */
  meta: string;
}

export function buildLevel(game: GameState): LevelModel {
  return {
    title: game.title,
    meta: game.retroDone
      ? `Level ${game.level}, ${formatInt(game.xpIntoLevel)} of ${formatInt(game.xpForLevel)} XP`
      : "Counting old XP. Hold still.",
  };
}

// -------------------------------------------------------------------- note

/** A piece of the note line; numbers are mono. */
export type NoteSegment = { text: string; mono?: boolean };

export type NoteModel =
  | { kind: "text"; segments: NoteSegment[] }
  /** Update line: `action` is the text link after the sentence, if any. */
  | { kind: "update"; segments: NoteSegment[]; action: string | null };

/** One line, first match wins; `null` leaves the line out (docs/DESIGN.md, "Note line"). */
export function buildNote(snap: DashboardSnapshot, update: UpdateStatus): NoteModel | null {
  const { data } = snap;
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
  return null;
}
