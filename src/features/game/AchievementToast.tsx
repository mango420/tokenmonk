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
 * Bar anchored to the bottom of the popup (DESIGN.md, Achievement toast):
 * name over one line of text plus XP, accent bar on the left. Render it
 * inside the popup overlay slot. Timing and queueing live in
 * `useGameNotices`; the slide is CSS (off under reduced motion).
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
      className={shown ? "game-toast-slot game-toast-slot--shown" : "game-toast-slot"}
      role="status"
      aria-live="polite"
    >
      <button
        type="button"
        className="game-toast"
        onClick={onDismiss}
        onPointerEnter={() => onHoverChange(true)}
        onPointerLeave={() => onHoverChange(false)}
      >
        <span className="game-toast__title">{copy.title}</span>
        <span className="game-toast__line">{copy.line}</span>
      </button>
    </div>
  );
}
