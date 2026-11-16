/**
 * Pure view model of the achievements screen (docs/DESIGN.md, "Achievements").
 */
import type { GameState } from "../../lib/contracts";
import { clamp01, formatInt } from "../../lib/format";

export interface AchievementRowModel {
  id: string;
  name: string;
  description: string;
  unlocked: boolean;
}

export interface AchievementsModel {
  title: string;
  /** "Level 7, 340 of 500 XP", or the replay line while old XP is counted. */
  levelLine: string;
  /** `null` shows the track only. */
  ratio: number | null;
  /** "Achievements, 2 of 5" */
  heading: string;
  rows: AchievementRowModel[];
}

/** Every achievement except secret ones that are still locked, in display order. */
export function buildAchievements(game: GameState): AchievementsModel {
  const rows = game.achievements
    .filter((a) => !a.secret || a.unlockedAt !== null)
    .map((a) => ({
      id: a.id,
      name: a.name,
      description: a.description,
      unlocked: a.unlockedAt !== null,
    }));
  const unlockedCount = rows.filter((r) => r.unlocked).length;
  return {
    title: game.title,
    levelLine: game.retroDone
      ? `Level ${game.level}, ${formatInt(game.xpIntoLevel)} of ${formatInt(game.xpForLevel)} XP`
      : "Counting old XP. Hold still.",
    ratio: game.retroDone ? clamp01(game.progress) : null,
    heading: `Achievements, ${unlockedCount} of ${rows.length}`,
    rows,
  };
}
