import { Icon, type IconName } from "../../components/Icon";
import type { AchievementId } from "../../lib/contracts";
import { fadeStyle, useFadeShown } from "./motion";
import { toastCopy, type ToastItem } from "./model";
import "./game.css";

/** Same glyphs as the achievement row on the dashboard. */
const GLYPH: Record<AchievementId, IconName> = {
  firstBlood: "drop",
  nightOwl: "moon",
  rateLimitRambo: "bolt",
  soClose: "target",
  tokenMonk: "person",
  boo: "ghost",
};

interface AchievementToastProps {
  item: ToastItem;
  /** Description of the achievement, shown when the unlock has no detail. */
  description: string | null;
  /** `false` fades the toast out. */
  open: boolean;
  /** Click on the toast. */
  onDismiss: () => void;
  /** Pointer hover pauses the 4 s timer (kept by the caller). */
  onHoverChange: (hovering: boolean) => void;
}

/**
 * Card floating over the top of the popup (DESIGN.md, Achievement toast):
 * the unlocked variant of the achievement row. Render it inside the popup
 * overlay slot. Timing and queueing live in `useGameNotices`.
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
      className="game-toast-slot"
      role="status"
      aria-live="polite"
      style={{ ...fadeStyle(shown), pointerEvents: shown ? "auto" : "none" }}
    >
      <button
        type="button"
        className="game-toast"
        onClick={onDismiss}
        onPointerEnter={() => onHoverChange(true)}
        onPointerLeave={() => onHoverChange(false)}
      >
        <span className="badge">
          <Icon name={GLYPH[copy.glyph]} size="sm" />
        </span>
        <span className="achievement__text">
          <span className="label-strong">{copy.title}</span>
          {copy.detail !== null ? <span className="caption">{copy.detail}</span> : null}
        </span>
        <span className="mono-value live-number caption--accent">{copy.xp}</span>
      </button>
    </div>
  );
}
