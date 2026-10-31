import { useEffect, useRef } from "react";
import ghostUrl from "../../../assets/brand/tokenmonk-ghost.svg";
import { fadeStyle, useFadeShown } from "../game/motion";
import { ghostCopy, type GhostSession } from "./copy";
import "./ghost.css";

interface GhostOverlayProps {
  session: GhostSession;
  /** `false` fades the overlay out. */
  open: boolean;
  /** "Back to work" button and Esc. */
  onClose: () => void;
}

/**
 * The easter egg (DESIGN.md, Ghost overlay). Covers the whole popup; render it
 * in the popup overlay slot. Pumpkin in every mode and season.
 */
export function GhostOverlay({ session, open, onClose }: GhostOverlayProps) {
  const shown = useFadeShown(open);
  const copy = ghostCopy(session);
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
      className="ghost-overlay"
      role="dialog"
      aria-modal="true"
      aria-label={copy.title}
      style={{ ...fadeStyle(shown), pointerEvents: shown ? "auto" : "none" }}
    >
      <img className="ghost-overlay__mark" src={ghostUrl} alt="" />
      <h2 className="ghost-overlay__title">{copy.title}</h2>
      {copy.line !== null ? <p className="ghost-overlay__line">{copy.line}</p> : null}
      <button ref={button} type="button" className="ghost-overlay__button" onClick={onClose}>
        {copy.button}
      </button>
    </div>
  );
}
