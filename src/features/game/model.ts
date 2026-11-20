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
  /** The one line under it: the detail, else the description. */
  line: string | null;
  /** The XP on the right ("+50 XP"). */
  xp: string;
}

/** `description` is the achievement's description, used when the unlock has no detail. */
export function toastCopy(item: ToastItem, description: string | null): ToastCopy {
  if (item.kind === "summary") {
    return {
      title: `${item.count} achievements unlocked`,
      line: `Newest: ${item.newest.name}.`,
      xp: `+${formatInt(item.xp)} XP`,
    };
  }
  const { unlock } = item;
  const line = (unlock.detail ?? description)?.trim() ?? "";
  return { title: unlock.name, line: line === "" ? null : line, xp: `+${formatInt(unlock.xp)} XP` };
}

// -------------------------------------------------------- level-up screen

export type LevelScreenModel =
  | { kind: "levelUp"; levelUp: LevelUp }
  | { kind: "retro"; level: number; unlockedCount: number; xpTotal: number };

export interface LevelScreenCopy {
  /** Plain text of the title, for the accessible name. */
  display: string;
  /** The level number, set in mono next to the word "Level". */
  level: number;
  /** "+2 levels" under the title when several levels passed. */
  extra: string | null;
  /** Title line: "Now a <title>.", "Still a <title>." or the retro line. */
  titleLine: string;
  /** Body line under it. */
  body: string;
  /** Meta line above the level bar (mono); `null` hides it and the bar. */
  meta: string | null;
  /** Level bar fill 0..1; `null` hides the bar. */
  progress: number | null;
  button: string;
}

const LEVEL_UP_LINE = "Keep burning.";

/** "an" before a vowel sound ("an Abbot"), else "a". */
function article(title: string): "a" | "an" {
  return /^[aeiou]/i.test(title) ? "an" : "a";
}

export function levelScreenCopy(model: LevelScreenModel, game: GameState): LevelScreenCopy {
  if (model.kind === "retro") {
    const noun = model.unlockedCount === 1 ? "achievement" : "achievements";
    return {
      display: `Level ${model.level}`,
      level: model.level,
      extra: null,
      titleLine: "Straight from your history.",
      body: `${model.unlockedCount} ${noun}, ${formatInt(model.xpTotal)} XP`,
      meta: null,
      progress: null,
      button: "Back to work",
    };
  }
  const { levelUp } = model;
  const gained = levelUp.level - levelUp.previousLevel;
  // The XP line comes from the state that follows the level-up; skip it if the
  // state has moved on (several level-ups queued).
  const current = game.level === levelUp.level;
  return {
    display: `Level ${levelUp.level}`,
    level: levelUp.level,
    extra: gained > 1 ? `+${gained} levels` : null,
    titleLine: `${levelUp.titleChanged ? "Now" : "Still"} ${article(levelUp.title)} ${levelUp.title}.`,
    body: LEVEL_UP_LINE,
    meta: current
      ? `Level ${levelUp.level}, ${formatInt(game.xpIntoLevel)} of ${formatInt(game.xpForLevel)} XP`
      : null,
    progress: current ? Math.min(1, Math.max(0, game.progress)) : null,
    button: "Back to work",
  };
}

/** The first-start summary: skipped for level 1 without unlocks. */
export function retroSummary(game: GameState): LevelScreenModel | null {
  const unlockedCount = game.achievements.filter((a) => a.unlockedAt !== null).length;
  if (game.level <= 1 && unlockedCount === 0) return null;
  return { kind: "retro", level: game.level, unlockedCount, xpTotal: game.xpTotal };
}
