import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { AchievementUnlocked, GameState, LevelUp } from "../../lib/contracts";
import { AchievementToast } from "./AchievementToast";
import { TOAST_VISIBLE_MS } from "./constants";
import { LevelUpScreen } from "./LevelUpScreen";
import {
  isToastable,
  nextToast,
  retroSummary,
  type LevelScreenModel,
  type ToastItem,
} from "./model";
import { fadeOutDelay } from "./motion";
import { playTone } from "./sound";

export interface GameNoticesOptions {
  /** Latest `GameState` (`game-updated`). */
  game: GameState;
  /** `Settings.sound`. */
  sound: boolean;
  /** The popup window is shown. Events are kept while it is hidden. Default `true`. */
  popupVisible?: boolean;
  /** A higher-priority overlay is open: toasts and the level-up wait. */
  blocked?: boolean;
}

export interface GameNotices {
  /** Feed every `achievement-unlocked` event here. */
  pushUnlock: (unlock: AchievementUnlocked) => void;
  /** Feed every `level-up` event here. */
  pushLevelUp: (levelUp: LevelUp) => void;
  /** Put this into the `Popup` overlay slot. */
  node: ReactNode;
}

/**
 * Queueing, timing and sound for the achievement toast and the level-up
 * screen (DESIGN.md, Achievement toast and Level-up screen).
 *
 * - The tone plays once per live event when `sound` is on, whether the
 *   popup is open or hidden. Retroactive events are ignored entirely.
 * - Toasts: one at a time, 4 s, click dismisses, hover pauses, three or more
 *   waiting collapse into one. The legacy Boo badge never gets a toast.
 * - Priority: another overlay (`blocked`), then the level-up screen, then toasts.
 * - `game.retroDone` turning true shows the first-start summary once.
 */
export function useGameNotices({
  game,
  sound,
  popupVisible = true,
  blocked = false,
}: GameNoticesOptions): GameNotices {
  const soundRef = useRef(sound);
  useEffect(() => {
    soundRef.current = sound;
  }, [sound]);

  const [unlockQueue, setUnlockQueue] = useState<AchievementUnlocked[]>([]);
  const [levelQueue, setLevelQueue] = useState<LevelUp[]>([]);
  const [retro, setRetro] = useState<LevelScreenModel | null>(null);
  const [toast, setToast] = useState<ToastItem | null>(null);
  const [toastClosing, setToastClosing] = useState(false);
  const [screenClosing, setScreenClosing] = useState(false);
  const [hover, setHover] = useState(false);
  const remaining = useRef(TOAST_VISIBLE_MS);
  const timers = useRef(new Set<number>());

  const later = useCallback((fn: () => void, ms: number) => {
    const id = window.setTimeout(() => {
      timers.current.delete(id);
      fn();
    }, ms);
    timers.current.add(id);
  }, []);

  useEffect(() => {
    const pending = timers.current;
    return () => {
      pending.forEach((id) => window.clearTimeout(id));
      pending.clear();
    };
  }, []);

  const pushUnlock = useCallback((unlock: AchievementUnlocked) => {
    if (unlock.retroactive) return;
    if (soundRef.current) playTone("unlock");
    if (isToastable(unlock)) setUnlockQueue((q) => [...q, unlock]);
  }, []);

  const pushLevelUp = useCallback((levelUp: LevelUp) => {
    if (levelUp.retroactive) return;
    if (soundRef.current) playTone("levelUp");
    setLevelQueue((q) => [...q, levelUp]);
  }, []);

  // First-start summary: once, when the replay finishes during this run.
  const wasRetroDone = useRef(game.retroDone);
  useEffect(() => {
    if (!wasRetroDone.current && game.retroDone) {
      const summary = retroSummary(game);
      if (summary !== null) setRetro(summary);
    }
    wasRetroDone.current = game.retroDone;
  }, [game]);

  const head = levelQueue[0];
  const screen: LevelScreenModel | null =
    retro ?? (head !== undefined ? { kind: "levelUp", levelUp: head } : null);
  const screenOpen = popupVisible && !blocked && !screenClosing;

  const closeScreen = useCallback(() => {
    setScreenClosing(true);
    later(() => {
      setScreenClosing(false);
      if (retro !== null) setRetro(null);
      else setLevelQueue((q) => q.slice(1));
    }, fadeOutDelay());
  }, [later, retro]);

  // Next toast, only when nothing with higher priority is up.
  const toastAllowed = popupVisible && !blocked && screen === null;
  useEffect(() => {
    if (toast !== null || unlockQueue.length === 0 || !toastAllowed) return;
    const { item, rest } = nextToast(unlockQueue);
    remaining.current = TOAST_VISIBLE_MS;
    setToast(item);
    setUnlockQueue(rest);
  }, [toast, unlockQueue, toastAllowed]);

  const dismissToast = useCallback(() => {
    setToastClosing(true);
    later(() => {
      setToast(null);
      setToastClosing(false);
      setHover(false);
    }, fadeOutDelay());
  }, [later]);

  // The 4 s timer runs only while the toast is visible and not hovered.
  const timerRunning = toast !== null && toastAllowed && !hover && !toastClosing;
  useEffect(() => {
    if (!timerRunning) return undefined;
    const startedAt = Date.now();
    const id = window.setTimeout(dismissToast, Math.max(0, remaining.current));
    return () => {
      window.clearTimeout(id);
      remaining.current -= Date.now() - startedAt;
    };
  }, [timerRunning, dismissToast]);

  const description = useMemo(
    () =>
      toast === null
        ? null
        : (game.achievements.find(
            (a) => a.id === (toast.kind === "single" ? toast.unlock.id : toast.newest.id),
          )?.description ?? null),
    [toast, game.achievements],
  );

  const node = (
    <>
      {toast !== null ? (
        <AchievementToast
          key={toast.key}
          item={toast}
          description={description}
          open={toastAllowed && !toastClosing}
          onDismiss={dismissToast}
          onHoverChange={setHover}
        />
      ) : null}
      {screen !== null ? (
        <LevelUpScreen
          key={screen.kind === "retro" ? "retro" : `level-${screen.levelUp.level}`}
          model={screen}
          game={game}
          open={screenOpen}
          onClose={closeScreen}
        />
      ) : null}
    </>
  );

  return { pushUnlock, pushLevelUp, node };
}
