/**
 * Pure view models for the popup dashboard. Copy and rules follow
 * docs/DESIGN.md (popup layout, limits unavailable and stale, achievement row).
 * "Now" is always `DashboardSnapshot.generatedAt`, so the output is a pure
 * function of the snapshot.
 */
import type { Fill } from "../../components/ChoiceTile";
import type {
  AchievementId,
  DashboardSnapshot,
  DataStatus,
  GameState,
  LimitReason,
  LimitsView,
  Mode,
  Season,
} from "../../lib/contracts";
import {
  clamp01,
  formatClock,
  formatDuration,
  formatInt,
  formatPercent,
  formatShort,
  formatWeekdayClock,
} from "../../lib/format";

export type PopupMode = "max" | "diy";

/** Fill of a mode's tile and button; Max it out turns pumpkin during Halloween. */
export function modeFill(mode: PopupMode, season: Season): Fill {
  if (mode === "diy") return "jade";
  return season === "halloween" ? "pumpkin" : "saffron";
}

/** Before onboarding (`noneYet`) the popup falls back to Max it out. */
export function toPopupMode(mode: Mode): PopupMode {
  return mode === "diy" ? "diy" : "max";
}

export const MODE_LABEL: Record<PopupMode, string> = {
  max: "Max it out",
  diy: "Do it yourself",
};

const DAY_MS = 24 * 60 * 60 * 1000;
const DANGER_MAX_PERCENT = 90;
const CAREFUL_PERCENT = 75;

// ------------------------------------------------------------------- gauge

export type GaugeDetailTone = "muted" | "mono" | "xp" | "ember";

export interface GaugeModel {
  /** Counter text in the ring centre. */
  value: string;
  caption: string;
  /** Ring fill 0..1; `null` shows the track only. */
  ratio: number | null;
  /** Ring switches to `--gauge-danger`. */
  danger: boolean;
  /** Last-known value; the counter is muted. */
  stale: boolean;
  /** Right column, heading style. */
  heading: string;
  /** Heading in `--ember` (logs unreadable). */
  headingAlert?: boolean;
  /** Right column, second line. */
  detail: {
    text: string;
    tone: GaugeDetailTone;
    /** Renders the text as a button that opens Settings at "Limit data". */
    action?: "openLimitSetup";
  } | null;
}

const REASON_CAPTION: Record<LimitReason, string> = {
  noSource: "Hook up Claude Code’s status line in Settings.",
  notSignedIn: "Not signed in to Claude.",
  network: "Offline. Trying again in 5 min.",
  invalidResponse: "The limit file talked nonsense. Next reply fixes it.",
  other: "The limit file talked nonsense. Next reply fixes it.",
};

function reasonDetail(reason: LimitReason | null): GaugeModel["detail"] {
  if (reason === null) return null;
  return reason === "noSource"
    ? { text: REASON_CAPTION[reason], tone: "muted", action: "openLimitSetup" }
    : { text: REASON_CAPTION[reason], tone: "muted" };
}

function limitsAvailable(limits: LimitsView): boolean {
  return limits.status !== "unavailable" && (limits.fiveHour !== null || limits.weekly !== null);
}

/** "Limit in ~25 min at this pace": N rounded to 5, at least 5. */
export function limitEtaText(etaMinutes: number): string {
  const rounded = Math.max(5, Math.round(etaMinutes / 5) * 5);
  return `Limit in ~${rounded} min at this pace`;
}

function burnLine(percent: number, etaMinutes: number | null): string {
  if (percent >= 100 || etaMinutes === 0) return "Limit reached. Breathe.";
  if (etaMinutes === null) return "Nothing burning right now.";
  return limitEtaText(etaMinutes);
}

function moodLine(usedPercent: number): string {
  if (usedPercent <= 0) return "No AI yet today. Pure so far.";
  if (usedPercent > 100) return "Over budget. The AI wins today.";
  if (usedPercent >= CAREFUL_PERCENT) return "Careful. The AI is creeping in.";
  const left = Math.max(0, 100 - Math.round(usedPercent));
  return `Calm. ${left} % of today’s budget left.`;
}

function maxGauge(snap: DashboardSnapshot): GaugeModel {
  const { limits, burnRate } = snap;
  const fiveHour = limits.fiveHour;
  if (limitsAvailable(limits) && fiveHour === null) {
    // Fresh 5-hour window: the old one reset, no reading has arrived since.
    return {
      value: formatShort(burnRate.tokensPerMinute),
      caption: "tokens / min",
      ratio: null,
      danger: false,
      stale: false,
      heading: "Fresh window.",
      detail: { text: "Numbers on your next reply.", tone: "muted" },
    };
  }
  if (!limitsAvailable(limits) || fiveHour === null) {
    return {
      value: formatShort(burnRate.tokensPerMinute),
      caption: "tokens / min",
      ratio: null,
      danger: false,
      stale: false,
      heading: "No limit data. Running on tokens.",
      detail: reasonDetail(limits.reason),
    };
  }
  const stale = limits.status === "stale";
  let detailText: string | null = null;
  if (stale && limits.fetchedAt !== null) {
    detailText = `As of ${formatClock(limits.fetchedAt)}. Updates on your next reply.`;
  } else if (fiveHour.resetsAt !== null) {
    detailText = `Resets in ${formatDuration(fiveHour.resetsAt - snap.generatedAt)}`;
  }
  return {
    value: formatPercent(fiveHour.utilization),
    caption: "5h limit",
    ratio: clamp01(fiveHour.utilization / 100),
    danger: fiveHour.utilization >= DANGER_MAX_PERCENT,
    stale,
    heading: burnLine(fiveHour.utilization, burnRate.limitEtaMinutes),
    detail: detailText === null ? null : { text: detailText, tone: "muted" },
  };
}

function diyGauge(snap: DashboardSnapshot, game: GameState): GaugeModel {
  const budget = snap.dailyBudget;
  if (budget === null) {
    return {
      value: formatShort(snap.today.total),
      caption: "tokens today",
      ratio: null,
      danger: false,
      stale: false,
      heading: "Learning your normal.",
      detail: { text: "A budget needs a few days of history.", tone: "muted" },
    };
  }
  const over = budget.usedPercent > 100;
  let detail: GaugeModel["detail"] = null;
  if (over) {
    detail = { text: "+0 XP today", tone: "ember" };
  } else if (game.dailyXpPreview !== null) {
    detail = { text: `+${formatInt(game.dailyXpPreview)} XP if you stop now`, tone: "xp" };
  }
  return {
    value: formatPercent(budget.usedPercent),
    caption: "of today’s budget",
    ratio: clamp01(budget.usedPercent / 100),
    danger: over,
    stale: snap.limits.status === "stale" && budget.basis === "weeklyLimit",
    heading: moodLine(budget.usedPercent),
    detail,
  };
}

/** Import progress 0..1 while importing (0 without files), otherwise `null`. */
export function importRatio(data: DataStatus): number | null {
  if (data.state !== "importing") return null;
  return data.filesTotal > 0 ? clamp01(data.filesDone / data.filesTotal) : 0;
}

/** First-start log import: both modes, never danger (DESIGN.md, "First-start import"). */
function importGauge(data: DataStatus): GaugeModel {
  const ratio = importRatio(data) ?? 0;
  return {
    value: formatPercent(ratio * 100),
    caption: "logs read",
    ratio,
    danger: false,
    stale: false,
    heading: "Reading your history.",
    detail: {
      text: `${formatInt(data.filesDone)} / ${formatInt(data.filesTotal)} files`,
      tone: "mono",
    },
  };
}

function noLogsGauge(): GaugeModel {
  return {
    value: "0",
    caption: "tokens today",
    ratio: null,
    danger: false,
    stale: false,
    heading: "No logs yet.",
    detail: { text: "Start a Claude Code session. I pick it up by myself.", tone: "muted" },
  };
}

function logErrorGauge(snap: DashboardSnapshot): GaugeModel {
  return {
    value: formatShort(snap.today.total),
    caption: "tokens today",
    ratio: null,
    danger: false,
    stale: false,
    heading: "Can’t read the logs.",
    headingAlert: true,
    detail: { text: "Retrying in the background.", tone: "muted" },
  };
}

export function buildGauge(mode: PopupMode, snap: DashboardSnapshot, game: GameState): GaugeModel {
  switch (snap.data.state) {
    case "importing":
      return importGauge(snap.data);
    case "noLogs":
      return noLogsGauge();
    case "error":
      return logErrorGauge(snap);
    case "ready":
      return mode === "max" ? maxGauge(snap) : diyGauge(snap, game);
  }
}

// ------------------------------------------------------------- stat tiles

export interface StatTile {
  label: string;
  value: string;
  /** Muted value such as "Idle". */
  muted?: boolean;
}

export function buildStatTiles(snap: DashboardSnapshot): StatTile[] {
  return [
    { label: "Tokens today", value: formatInt(snap.today.total) },
    snap.session === null
      ? { label: "This session", value: "Idle", muted: true }
      : { label: "This session", value: formatInt(snap.session.tokens.total) },
  ];
}

// ------------------------------------------------------------ weekly card

export interface WeeklyModel {
  label: string;
  /** `n/a` without limit data. */
  value: string;
  ratio: number | null;
  caption: string;
  /** The caption is a button that opens Settings at "Limit data". */
  action?: "openLimitSetup";
}

export function buildWeekly(mode: PopupMode, limits: LimitsView): WeeklyModel {
  const label = mode === "max" ? "Weekly limit" : "Weekly limit used";
  const weekly = limits.weekly;
  if (weekly === null) {
    if (limits.reason === "noSource") {
      return {
        label,
        value: "n/a",
        ratio: null,
        caption: "No limit data yet. Setup is in Settings.",
        action: "openLimitSetup",
      };
    }
    return {
      label,
      value: "n/a",
      ratio: null,
      caption: "No limit data. Token stats still count.",
    };
  }
  const parts: string[] = [];
  if (mode === "diy") parts.push("Lower is better here.");
  if (weekly.resetsAt !== null) parts.push(`Resets ${formatWeekdayClock(weekly.resetsAt)}`);
  return {
    label,
    value: formatPercent(weekly.utilization),
    ratio: clamp01(weekly.utilization / 100),
    caption: parts.join(" "),
  };
}

// ------------------------------------------------------------- level card

export interface LevelModel {
  heading: string;
  title: string;
  /** `null` shows the track only (old XP still being counted). */
  ratio: number | null;
  caption: string;
  /** Plain muted caption instead of the mono XP line. */
  counting: boolean;
}

export function buildLevel(game: GameState): LevelModel {
  if (!game.retroDone) {
    return {
      heading: `Level ${game.level}`,
      title: game.title,
      ratio: null,
      caption: "Counting old XP. Hold still.",
      counting: true,
    };
  }
  return {
    heading: `Level ${game.level}`,
    title: game.title,
    ratio: clamp01(game.progress),
    caption: `${formatInt(game.xpIntoLevel)} / ${formatInt(game.xpForLevel)} XP to level ${game.level + 1}`,
    counting: false,
  };
}

// ---------------------------------------------------------- achievement row

export type AchievementRowModel =
  | { kind: "unlocked"; id: AchievementId; name: string; detail: string | null; xp: number }
  | { kind: "progress"; id: AchievementId; name: string; ratio: number; caption: string; mono: boolean }
  | { kind: "next"; id: AchievementId; name: string; caption: string };

/** Picks the single achievement to show (DESIGN.md, "Achievement row rule"). */
export function buildAchievementRow(game: GameState, now: number): AchievementRowModel | null {
  if (!game.retroDone) return null;

  const unlocked = (u: GameState["recentUnlocks"][number]): AchievementRowModel => ({
    kind: "unlocked",
    id: u.id,
    name: u.name,
    detail: u.detail,
    xp: u.xp,
  });

  const latest = game.recentUnlocks[0];
  if (latest !== undefined && now - latest.unlockedAt <= DAY_MS) return unlocked(latest);

  const eligible = game.achievements.filter(
    (a) => a.unlockedAt === null && !a.secret && (a.mode === null || a.mode === game.mode),
  );

  let best: { ratio: number; row: AchievementRowModel } | null = null;
  for (const a of eligible) {
    if (a.current === null || a.target === null || a.target <= 0) continue;
    const ratio = clamp01(a.current / a.target);
    if (best === null || ratio > best.ratio) {
      best = {
        ratio,
        row: {
          kind: "progress",
          id: a.id,
          name: a.name,
          ratio,
          caption: a.progressLabel ?? `${formatInt(a.current)} / ${formatInt(a.target)}`,
          mono: a.progressLabel === null,
        },
      };
    }
  }
  if (best !== null) return best.row;

  const next = eligible[0];
  if (next !== undefined) {
    return { kind: "next", id: next.id, name: next.name, caption: next.description };
  }

  return latest === undefined ? null : unlocked(latest);
}

// ------------------------------------------------------------------ footer

export function footerNote(limits: LimitsView): string {
  return limitsAvailable(limits)
    ? "Limit % includes claude.ai chats · token stats from Claude Code only"
    : "No limit data · token stats from Claude Code only";
}
