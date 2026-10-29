/**
 * Pure view models for the popup dashboard. Copy and rules follow
 * docs/DESIGN.md (popup layout, limits unavailable and stale, achievement row).
 * "Now" is always `DashboardSnapshot.generatedAt`, so the output is a pure
 * function of the snapshot.
 */
import type {
  AchievementId,
  DashboardSnapshot,
  GameState,
  LimitReason,
  LimitsView,
  Mode,
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

export type GaugeDetailTone = "muted" | "xp" | "ember";

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
  /** Right column, second line. */
  detail: { text: string; tone: GaugeDetailTone } | null;
}

const REASON_CAPTION: Record<LimitReason, string> = {
  noSource: "This version has no limit source.",
  notSignedIn: "Not signed in to Claude.",
  network: "Offline. Trying again in 5 min.",
  invalidResponse: "The limit source talked nonsense. Trying again in 5 min.",
  other: "The limit source talked nonsense. Trying again in 5 min.",
};

function limitsAvailable(limits: LimitsView): boolean {
  return limits.status !== "unavailable";
}

function burnLine(percent: number, etaMinutes: number | null): string {
  if (percent >= 100 || etaMinutes === 0) return "Limit reached. Breathe.";
  if (etaMinutes === null) return "Nothing burning right now.";
  const rounded = Math.max(5, Math.round(etaMinutes / 5) * 5);
  return `Limit in ~${rounded} min at this pace`;
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
  if (!limitsAvailable(limits) || fiveHour === null) {
    return {
      value: formatShort(burnRate.tokensPerMinute),
      caption: "tokens / min",
      ratio: null,
      danger: false,
      stale: false,
      heading: "No limit data. Running on tokens.",
      detail: { text: REASON_CAPTION[limits.reason ?? "other"], tone: "muted" },
    };
  }
  const stale = limits.status === "stale";
  let detailText: string | null = null;
  if (stale && limits.fetchedAt !== null) {
    detailText = `As of ${formatClock(limits.fetchedAt)}. Retrying.`;
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

export function buildGauge(mode: PopupMode, snap: DashboardSnapshot, game: GameState): GaugeModel {
  return mode === "max" ? maxGauge(snap) : diyGauge(snap, game);
}

// ------------------------------------------------------------- stat tiles

export interface StatTile {
  label: string;
  value: string;
}

export function buildStatTiles(snap: DashboardSnapshot): StatTile[] {
  return [
    { label: "Tokens today", value: formatInt(snap.today.total) },
    {
      label: "This session",
      value: snap.session === null ? "n/a" : formatInt(snap.session.tokens.total),
    },
  ];
}

// ------------------------------------------------------------ weekly card

export interface WeeklyModel {
  label: string;
  /** `n/a` without limit data. */
  value: string;
  ratio: number | null;
  caption: string;
}

export function buildWeekly(mode: PopupMode, limits: LimitsView): WeeklyModel {
  const label = mode === "max" ? "Weekly limit" : "Weekly limit used";
  const weekly = limits.weekly;
  if (weekly === null) {
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
  ratio: number;
  caption: string;
}

export function buildLevel(game: GameState): LevelModel {
  return {
    heading: `Level ${game.level}`,
    title: game.title,
    ratio: clamp01(game.progress),
    caption: `${formatInt(game.xpIntoLevel)} / ${formatInt(game.xpForLevel)} XP to level ${game.level + 1}`,
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
