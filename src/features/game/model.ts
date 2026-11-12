/**
 * Pure view models and copy for the toast and the level-up screen. Copy is
 * final per docs/DESIGN.md (Achievement toast, Level-up screen, First-start
 * import); numbers in it are live.
 */
import type { AchievementUnlocked, GameState, LevelUp } from "../../lib/contracts";
import { formatInt } from "../../lib/format";
import { TOAST_COLLAPSE_AT } from "./constants";

// ------------------------------------------------------------------- toasts

export type ToastItem =
  | { kind: "single"; key: string; unlock: AchievementUnlocked }
  | { kind: "summary"; key: string; count: number; newest: AchievementUnlocked; xp: number };

/** Retroactive unlocks are not toasted one by one and the legacy Boo badge (not earnable any more) never gets a toast. */
export function isToastable(unlock: AchievementUnlocked): boolean {
  return !unlock.retroactive && unlock.id !== "boo";
}

/**
 * Takes the next toast off a queue ordered oldest first. Three or more
 * waiting collapse into one summary that consumes them all.
 */
export function nextToast(queue: readonly AchievementUnlocked[]): {
  item: ToastItem | null;
  rest: AchievementUnlocked[];
} {
  const [first, ...others] = queue;
  if (first === undefined) return { item: null, rest: [] };
  if (queue.length < TOAST_COLLAPSE_AT) {
    return {
      item: { kind: "single", key: `${first.id}:${first.unlockedAt}`, unlock: first },
      rest: others,
    };
  }
  const newest = queue.reduce((a, b) => (b.unlockedAt >= a.unlockedAt ? b : a));
  const xp = queue.reduce((sum, u) => sum + u.xp, 0);
  return {
    item: {
      kind: "summary",
      key: `summary:${newest.id}:${newest.unlockedAt}:${queue.length}`,
      count: queue.length,
      newest,
      xp,
    },
    rest: [],
  };
}

export interface ToastCopy {
  /** Toast title: the achievement name. */
  title: string;
  /** The line under it: text, then the XP ("Coding after midnight. +50 XP"). */
  line: string;
}

/** One sentence: the text with a closing full stop, then the XP. */
function withXp(text: string | null, xp: number): string {
  const amount = `+${formatInt(xp)} XP`;
  const trimmed = text?.trim() ?? "";
  if (trimmed === "") return amount;
  return `${/[.!?]$/.test(trimmed) ? trimmed : `${trimmed}.`} ${amount}`;
}

/** `description` is the achievement's description, used when the unlock has no detail. */
export function toastCopy(item: ToastItem, description: string | null): ToastCopy {
  if (item.kind === "summary") {
    return {
      title: `${item.count} achievements unlocked`,
      line: withXp(`Newest: ${item.newest.name}`, item.xp),
    };
  }
  const { unlock } = item;
  return { title: unlock.name, line: withXp(unlock.detail ?? description, unlock.xp) };
}

// -------------------------------------------------------- level-up screen

export type LevelScreenModel =
  | { kind: "levelUp"; levelUp: LevelUp }
  | { kind: "retro"; level: number; unlockedCount: number; xpTotal: number };

export interface LevelScreenCopy {
  display: string;
  /** "+2 levels" under the display when several levels passed. */
  extra: string | null;
  /** Title line: "Still a <title>." or "Now a <title>.". */
  titleLine: string | null;
  /** Mode line (status style), or the retro line. */
  heading: string | null;
  /** Mono line. */
  detail: string | null;
  /** Level bar fill 0..1; `null` hides the bar. */
  progress: number | null;
  button: string;
}

const MODE_LINE = { max: "Keep burning.", diy: "Well earned. Breathe." } as const;

/** "an" before a vowel sound ("an Abbot of Restraint"), else "a". */
function article(title: string): "a" | "an" {
  return /^[aeiou]/i.test(title) ? "an" : "a";
}

export function levelScreenCopy(model: LevelScreenModel, game: GameState): LevelScreenCopy {
  if (model.kind === "retro") {
    const noun = model.unlockedCount === 1 ? "achievement" : "achievements";
    return {
      display: `Level ${model.level}`,
      extra: null,
      titleLine: null,
      heading: "Straight from your history.",
      detail: `${model.unlockedCount} ${noun}, ${formatInt(model.xpTotal)} XP`,
      progress: null,
      button: "Back to work",
    };
  }
  const { levelUp } = model;
  const gained = levelUp.level - levelUp.previousLevel;
  // The XP line comes from the state that follows the level-up; skip it if the
  // state has moved on (several level-ups queued).
  const detail =
    game.level === levelUp.level
      ? `Level ${levelUp.level}, ${formatInt(game.xpIntoLevel)} of ${formatInt(game.xpForLevel)} XP`
      : null;
  return {
    display: `Level ${levelUp.level}`,
    extra: gained > 1 ? `+${gained} levels` : null,
    titleLine: `${levelUp.titleChanged ? "Now" : "Still"} ${article(levelUp.title)} ${levelUp.title}.`,
    heading: game.mode === "diy" ? MODE_LINE.diy : MODE_LINE.max,
    detail,
    progress: detail === null ? null : Math.min(1, Math.max(0, game.progress)),
    button: "Back to work",
  };
}

/** The first-start summary: skipped for level 1 without unlocks. */
export function retroSummary(game: GameState): LevelScreenModel | null {
  const unlockedCount = game.achievements.filter((a) => a.unlockedAt !== null).length;
  if (game.level <= 1 && unlockedCount === 0) return null;
  return { kind: "retro", level: game.level, unlockedCount, xpTotal: game.xpTotal };
}
