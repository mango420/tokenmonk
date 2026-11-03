import { useEffect, useRef } from "react";
import { LogoMark } from "../../components/LogoMark";
import type { GameState } from "../../lib/contracts";
import { fadeStyle, useFadeShown } from "./motion";
import { levelScreenCopy, type LevelScreenModel } from "./model";
import "./game.css";

interface LevelUpScreenProps {
  model: LevelScreenModel;
  /** The state that follows the level-up (XP line, mode line). */
  game: GameState;
  /** `false` fades the screen out. */
  open: boolean;
  /** "Back to work" button and Esc. */
  onClose: () => void;
}

/**
 * Level-up overlay (DESIGN.md, Level-up screen), also the first-start
 * summary (`model.kind === "retro"`). Covers the whole popup; render it in
 * the popup overlay slot.
 */
export function LevelUpScreen({ model, game, open, onClose }: LevelUpScreenProps) {
  const shown = useFadeShown(open);
  const copy = levelScreenCopy(model, game);
  const button = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (open) button.current?.focus();
  }, [open]);

  useEffect(() => {
    if (!open) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  return (
    <div
      className="game-overlay"
      role="dialog"
      aria-modal="true"
      aria-label={copy.display}
      style={{ ...fadeStyle(shown), pointerEvents: shown ? "auto" : "none" }}
    >
      <LogoMark ratio={1} />
      <h2 className="game-overlay__title">{copy.display}</h2>
      {copy.extra !== null ? <span className="game-overlay__mono game-overlay__extra">{copy.extra}</span> : null}
      {copy.newTitle !== null ? (
        <>
          <span className="game-overlay__caption">New title</span>
          <span className="game-overlay__status">{copy.newTitle}</span>
        </>
      ) : null}
      {copy.heading !== null ? <span className="game-overlay__status">{copy.heading}</span> : null}
      {copy.detail !== null ? (
        <span className="game-overlay__mono game-overlay__xp">{copy.detail}</span>
      ) : null}
      {copy.progress !== null ? (
        <div className="game-overlay__bar" aria-hidden="true">
          <div className="game-overlay__bar-fill" style={{ width: `${(copy.progress * 100).toFixed(1)}%` }} />
        </div>
      ) : null}
      <button ref={button} type="button" className="game-link" onClick={onClose}>
        {copy.button}
      </button>
    </div>
  );
}
