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
      if (event.key === "Escape") onClose();
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
      <div className="game-overlay__group">
        <h2 className="game-overlay__display">{copy.display}</h2>
        {copy.extra !== null ? <span className="mono-value live-number muted">{copy.extra}</span> : null}
      </div>
      {copy.newTitle !== null ? (
        <div className="game-overlay__group">
          <span className="caption">New title</span>
          <span className="game-overlay__heading">{copy.newTitle}</span>
        </div>
      ) : null}
      {copy.heading !== null ? <span className="game-overlay__heading">{copy.heading}</span> : null}
      {copy.detail !== null ? <span className="mono-value live-number muted">{copy.detail}</span> : null}
      <button ref={button} type="button" className="game-button" onClick={onClose}>
        {copy.button}
      </button>
    </div>
  );
}
