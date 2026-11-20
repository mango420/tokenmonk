import { useFadeShown } from "./motion";
import { toastCopy, type ToastItem } from "./model";
import "./game.css";

interface AchievementToastProps {
  item: ToastItem;
  /** Description of the achievement, shown when the unlock has no detail. */
  description: string | null;
  /** `false` slides the toast out. */
  open: boolean;
  /** Click on the toast. */
  onDismiss: () => void;
  /** Pointer hover pauses the 3.4 s timer (kept by the caller). */
  onHoverChange: (hovering: boolean) => void;
}

/**
 * Toast over a scrim (DESIGN.md, Achievement toast): a bead on a string, the
 * name over one line, "+50 XP" on the right. Render it inside the popup
 * overlay slot. Timing and queueing live in `useGameNotices`; the slide is
 * CSS (off under reduced motion). A click on the toast or the scrim dismisses.
 */
export function AchievementToast({
  item,
  description,
  open,
  onDismiss,
  onHoverChange,
}: AchievementToastProps) {
  const shown = useFadeShown(open);
  const copy = toastCopy(item, description);
  return (
    <div
      className={shown ? "game-toast-layer game-toast-layer--shown" : "game-toast-layer"}
      role="status"
      aria-live="polite"
    >
      <div className="game-toast-scrim" onClick={onDismiss} aria-hidden="true" />
      <button
        type="button"
        className="game-toast"
        onClick={onDismiss}
        onPointerEnter={() => onHoverChange(true)}
        onPointerLeave={() => onHoverChange(false)}
      >
        <svg className="game-toast__bead" width="24" height="48" viewBox="0 0 24 48" aria-hidden="true" focusable="false">
          <path className="game-toast__string" d="M12 0V48" />
          <circle className="game-toast__knot" cx="12" cy="24" />
        </svg>
        <span className="game-toast__text">
          <span className="game-toast__title">{copy.title}</span>
          {copy.line !== null ? <span className="game-toast__line">{copy.line}</span> : null}
        </span>
        <span className="game-toast__xp">{copy.xp}</span>
      </button>
    </div>
  );
}
