import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import type { EasterEggResult, GameState } from "../../lib/contracts";
import { fadeOutDelay } from "../game/motion";
import { NO_CLICKS, registerClick, type ClickStreak } from "./clicks";
import type { GhostSession } from "./copy";
import { GhostOverlay } from "./GhostOverlay";

export interface GhostOptions {
  /** Latest `GameState`; Boo's `unlockedAt` picks the text before the call answers. */
  game: GameState;
  /** `trigger_easter_egg`. */
  trigger: () => Promise<EasterEggResult>;
}

export interface Ghost {
  /** Wire to every logo click. */
  onLogoClick: () => void;
  /** The overlay is mounted (open or fading out). Feeds `useGameNotices({ blocked })`. */
  active: boolean;
  /** Put this into the `Popup` overlay slot. */
  node: ReactNode;
}

/**
 * Three logo clicks, each within 800 ms of the last, open the ghost overlay
 * without waiting for the backend, then ask the core to unlock Boo. A failed
 * call leaves the overlay up and drops the XP line.
 */
export function useGhost({ game, trigger }: GhostOptions): Ghost {
  const [session, setSession] = useState<GhostSession | null>(null);
  const [open, setOpen] = useState(false);
  const streak = useRef<ClickStreak>(NO_CLICKS);
  const latest = useRef({ game, trigger });
  latest.current = { game, trigger };
  /** Identifies the current overlay so a late answer never lands on a newer one. */
  const run = useRef(0);
  const closeTimer = useRef<number | null>(null);

  useEffect(
    () => () => {
      if (closeTimer.current !== null) window.clearTimeout(closeTimer.current);
    },
    [],
  );

  const onLogoClick = useCallback(() => {
    if (session !== null) return;
    const next = registerClick(streak.current, performance.now());
    streak.current = next.streak;
    if (!next.triggered) return;

    const boo = latest.current.game.achievements.find((a) => a.id === "boo");
    const id = ++run.current;
    setSession({
      predictedFirst: boo?.unlockedAt == null,
      predictedXp: boo?.xp ?? null,
      outcome: "pending",
    });
    setOpen(true);

    const settle = (outcome: GhostSession["outcome"]) =>
      setSession((current) => (current !== null && run.current === id ? { ...current, outcome } : current));
    latest.current.trigger().then(settle, () => settle("failed"));
  }, [session]);

  const close = useCallback(() => {
    setOpen(false);
    closeTimer.current = window.setTimeout(() => {
      closeTimer.current = null;
      setSession(null);
    }, fadeOutDelay());
  }, []);

  return {
    onLogoClick,
    active: session !== null,
    node: session === null ? null : <GhostOverlay session={session} open={open} onClose={close} />,
  };
}
