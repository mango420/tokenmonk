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

export type PopupMode = "max" | "diy";

// Kept until the Settings mode picker is gone (TKM-307).
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
  /** The status line opens Settings ("Connect limits in Settings."). */
  statusLink: boolean;
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

const STATUS = [
  // 60 and more, 30 to 59, 10 to 29, 1 to 9, exactly 0
  "Barely touched. Use it.",
  "Good pace.",
  "Nearly there. As planned.",
  "Almost gone. Make it count.",
  "Limit reached. Breathe.",
] as const;

/** Status line under the hero from percent left. */
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

const tokensPerMinute = (snap: DashboardSnapshot) => `${formatCompact(snap.burnRate.tokensPerMinute)} tokens/min`;

/** Meta row, left: the reset time. */
function metaLeft(snap: DashboardSnapshot, window: LimitWindow): string {
  if (window.resetsAt === null) return "";
  return `resets in ${formatResetIn(window.resetsAt - snap.generatedAt)}`;
}

function tokensGauge(
  snap: DashboardSnapshot,
  caption: string,
  status: string,
  statusLink = false,
): GaugeModel {
  return {
    caption,
    percentLeft: null,
    stale: false,
    hero: formatCompact(snap.today.total),
    small: true,
    heroLabel: "tokens today",
    status,
    statusLink,
    metaLeft: "",
    metaRight: tokensPerMinute(snap),
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

const LIMITS_NEXT_REPLY = "Limits after your next reply.";
const LIMITS_CONNECT = "Connect limits in Settings.";

/** No limits: tokens today is the full view, no warning, one calm line. */
function tokensOnly(snap: DashboardSnapshot, week: WeekModel | null): DashboardModel {
  const notConnected = snap.limits.reason === "noSource";
  return {
    gauge: tokensGauge(
      snap,
      "Your Claude Code tokens, today",
      notConnected ? LIMITS_CONNECT : LIMITS_NEXT_REPLY,
      notConnected,
    ),
    week,
  };
}

export function buildDashboard(snap: DashboardSnapshot): DashboardModel {
  const { limits } = snap;
  if (hasNoLimitData(limits)) return tokensOnly(snap, null);

  const week = weekModel(snap);
  const window = limits.fiveHour;
  if (window === null) return tokensOnly(snap, week);

  const caption = "Your Claude Code limit, this 5-hour window";
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
      status: week !== null && week.low ? WEEK_FIRST_STATUS : statusLine(left),
      statusLink: false,
      metaLeft: metaLeftText,
      metaRight: tokensPerMinute(snap),
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
  /** Update line: `action` is the text link after the sentence, if any. */
  | { kind: "update"; segments: NoteSegment[]; action: string | null };

/** One line, first match wins (docs/DESIGN.md, "Note line"). */
export function buildNote(snap: DashboardSnapshot, update: UpdateStatus): NoteModel {
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
  return {
    kind: "text",
    segments: [{ text: "Limit % includes claude.ai chats. Token stats are Claude Code only." }],
  };
}
