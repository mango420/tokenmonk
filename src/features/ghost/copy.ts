import type { EasterEggResult } from "../../lib/contracts";

/** What the overlay knows: the guess made before the call, then its answer. */
export interface GhostSession {
  /** Boo was still locked in `GameState` when the overlay opened. */
  predictedFirst: boolean;
  /** XP of the Boo achievement from `GameState`, when known. */
  predictedXp: number | null;
  /** `pending` until `trigger_easter_egg` answers; `failed` if it rejected. */
  outcome: "pending" | "failed" | EasterEggResult;
}

export interface GhostCopy {
  title: string;
  /** `null` when the call failed: the line is omitted (DESIGN.md, Ghost overlay, Boo already unlocked). */
  line: string | null;
  button: string;
}

/** Overlay text; `EasterEggResult.newlyUnlocked` confirms or corrects the early guess. */
export function ghostCopy(session: GhostSession): GhostCopy {
  const { outcome } = session;
  const button = "Back to work";
  if (outcome === "failed") {
    return { title: session.predictedFirst ? "Hoho, you found me!" : "Hoho, you again!", line: null, button };
  }
  const first = outcome === "pending" ? session.predictedFirst : outcome.newlyUnlocked;
  const title = first ? "Hoho, you found me!" : "Hoho, you again!";
  if (!first) return { title, line: "Boo is already yours · +0 XP", button };
  const xp = outcome === "pending" ? session.predictedXp : outcome.achievement.xp;
  return {
    title,
    line: xp === null ? null : `Secret achievement unlocked: Boo · +${xp} XP`,
    button,
  };
}
