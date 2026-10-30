/**
 * Pure view models and copy for the toast and the level-up screen. Copy is
 * final per docs/DESIGN.md (Achievement toast, Level-up screen, First-start
 * import); numbers in it are live.
 */
import type { AchievementId, AchievementUnlocked, GameState, LevelUp } from "../../lib/contracts";
import { formatInt } from "../../lib/format";
import { TOAST_COLLAPSE_AT } from "./constants";

// ------------------------------------------------------------------- toasts

export type ToastItem =
  | { kind: "single"; key: string; unlock: AchievementUnlocked }
  | { kind: "summary"; key: string; count: number; newest: AchievementUnlocked; xp: number };

/** Retroactive unlocks are not toasted one by one and Boo never gets a toast. */
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
  glyph: AchievementId;
  title: string;
  detail: string | null;
  xp: string;
}

/** `description` is the achievement's description, used when the unlock has no detail. */
export function toastCopy(item: ToastItem, description: string | null): ToastCopy {
  if (item.kind === "summary") {
    return {
      glyph: item.newest.id,
      title: `${item.count} achievements unlocked`,
      detail: `Newest: ${item.newest.name}`,
      xp: `+${formatInt(item.xp)} XP`,
    };
  }
  const { unlock } = item;
  return {
    glyph: unlock.id,
    title: `${unlock.name} unlocked`,
    detail: unlock.detail ?? description,
    xp: `+${formatInt(unlock.xp)} XP`,
  };
}

// -------------------------------------------------------- level-up screen

export type LevelScreenModel =
  | { kind: "levelUp"; levelUp: LevelUp }
  | { kind: "retro"; level: number; unlockedCount: number; xpTotal: number };

export interface LevelScreenCopy {
  display: string;
  /** "+2 levels" under the display when several levels passed. */
  extra: string | null;
  /** New title (caption "New title" over it); replaces the mode line. */
  newTitle: string | null;
  /** Heading line in the accent colour. */
  heading: string | null;
  /** Mono line. */
  detail: string | null;
  button: string;
}

const MODE_LINE = { max: "Keep burning \u{1F525}", diy: "Well earned. Breathe." } as const;

export function levelScreenCopy(model: LevelScreenModel, game: GameState): LevelScreenCopy {
  if (model.kind === "retro") {
    const noun = model.unlockedCount === 1 ? "achievement" : "achievements";
    return {
      display: `Level ${model.level}`,
      extra: null,
      newTitle: null,
      heading: "Straight from your history.",
      detail: `${model.unlockedCount} ${noun} · ${formatInt(model.xpTotal)} XP`,
      button: "Back to work",
    };
  }
  const { levelUp } = model;
  const gained = levelUp.level - levelUp.previousLevel;
  // The XP line comes from the state that follows the level-up; skip it if the
  // state has moved on (several level-ups queued).
  const detail =
    game.level === levelUp.level
      ? `${formatInt(game.xpIntoLevel)} / ${formatInt(game.xpForLevel)} XP to level ${levelUp.level + 1}`
      : null;
  return {
    display: `Level ${levelUp.level}`,
    extra: gained > 1 ? `+${gained} levels` : null,
    newTitle: levelUp.titleChanged ? levelUp.title : null,
    heading: levelUp.titleChanged ? null : game.mode === "diy" ? MODE_LINE.diy : MODE_LINE.max,
    detail,
    button: "Back to work",
  };
}

/** The first-start summary: skipped for level 1 without unlocks. */
export function retroSummary(game: GameState): LevelScreenModel | null {
  const unlockedCount = game.achievements.filter((a) => a.unlockedAt !== null).length;
  if (game.level <= 1 && unlockedCount === 0) return null;
  return { kind: "retro", level: game.level, unlockedCount, xpTotal: game.xpTotal };
}
