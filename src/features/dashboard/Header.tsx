import { Icon } from "../../components/Icon";
import { LogoMark } from "../../components/LogoMark";
import { SeasonChip } from "../../components/SeasonChip";
import type { Season } from "../../lib/contracts";
import { MODE_LABEL, type PopupMode } from "./model";

interface HeaderProps {
  /** `null` hides the mode label (onboarding). */
  mode: PopupMode | null;
  /** Halo fill 0..1, same value as the gauge ring; `null` shows the track only. */
  haloRatio: number | null;
  /** Omitted: no settings button (onboarding). */
  onOpenSettings?: () => void;
  /** Click counting for the ghost easter egg lives with the caller. */
  onLogoClick?: () => void;
  /** `halloween` shows the "Spooky season" chip between label and settings. */
  season?: Season;
}

export function Header({ mode, haloRatio, onOpenSettings, onLogoClick, season }: HeaderProps) {
  return (
    // "deep": the whole header drags the window; its buttons stay clickable.
    <header className="header" data-tauri-drag-region="deep">
      <button type="button" className="header__logo" aria-label="Tokenmonk" onClick={onLogoClick}>
        <LogoMark ratio={haloRatio} />
      </button>
      <div className="header__title">
        <span className="header__wordmark">Tokenmonk</span>
        {mode === null ? null : (
          <span className="header__mode">
            <Icon name={mode === "max" ? "flame" : "wrench"} size="xs" />
            {MODE_LABEL[mode]}
          </span>
        )}
      </div>
      {season === "halloween" ? <SeasonChip /> : null}
      {onOpenSettings === undefined ? null : (
        <button type="button" className="icon-button" aria-label="Settings" onClick={onOpenSettings}>
          <Icon name="gear" />
        </button>
      )}
    </header>
  );
}
