import { useEffect, useRef } from "react";
import type { GameState } from "../../lib/contracts";
import { LevelBar } from "../../components/LevelBar";
import { BeadString } from "./BeadString";
import { fadeStyle, useFadeShown } from "./motion";
import { levelScreenCopy, type LevelScreenModel } from "./model";
import "./game.css";

interface LevelUpScreenProps {
  model: LevelScreenModel;
  /** The state that follows the level-up (XP line). */
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
      <span className="game-brand">Tokenmonk</span>
      <div className="game-overlay__spacer" />
      <BeadString />
      <h2 className="game-overlay__title">
        <span>Level</span> <span className="game-overlay__number">{copy.level}</span>
      </h2>
      {copy.extra !== null ? <span className="game-overlay__extra">{copy.extra}</span> : null}
      <span className="game-overlay__line">{copy.titleLine}</span>
      <span className="game-overlay__body">{copy.body}</span>
      <div className="game-overlay__spacer" />
      {copy.meta !== null ? <span className="game-overlay__meta">{copy.meta}</span> : null}
      {copy.progress !== null ? <LevelBar ratio={copy.progress} /> : null}
      <button ref={button} type="button" className="game-button" onClick={onClose}>
        {copy.button}
      </button>
    </div>
  );
}
