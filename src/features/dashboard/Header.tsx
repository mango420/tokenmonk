import type { ReactNode } from "react";
import { Icon } from "../../components/Icon";
import { LogoMark } from "../../components/LogoMark";
import { MODE_LABEL, type PopupMode } from "./model";

interface HeaderProps {
  /** `null` hides the mode label (onboarding). */
  mode: PopupMode | null;
  /** Halo fill 0..1, same value as the gauge ring; `null` shows the track only. */
  haloRatio: number | null;
  /** Omitted: no settings button (onboarding). */
  onOpenSettings?: () => void;
  /** Seam for the ghost easter egg (#16): click counting lives with the caller. */
  onLogoClick?: () => void;
  /** Seam for the "Spooky season" chip (#16), rendered between label and settings. */
  badge?: ReactNode;
}

export function Header({ mode, haloRatio, onOpenSettings, onLogoClick, badge }: HeaderProps) {
  return (
    <header className="header">
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
      {badge}
      {onOpenSettings === undefined ? null : (
        <button type="button" className="icon-button" aria-label="Settings" onClick={onOpenSettings}>
          <Icon name="gear" />
        </button>
      )}
    </header>
  );
}
