import { TextLink } from "../../components/TextLink";
import { TopRow } from "../../components/TopRow";
import { MODE_LABEL, type PopupMode } from "./model";

interface HeaderProps {
  /** Omitted together with `onOpenSettings`: brand only (onboarding, loading). */
  mode?: PopupMode;
  onOpenSettings?: () => void;
}

/** Top row of the main screens: brand, mode dot and name, the "Settings" link. */
export function Header({ mode, onOpenSettings }: HeaderProps) {
  return (
    <TopRow brand>
      {mode === undefined ? null : (
        <span className="mode-name">
          <span className="mode-dot" aria-hidden="true" />
          {MODE_LABEL[mode]}
        </span>
      )}
      {onOpenSettings === undefined ? null : <TextLink onClick={onOpenSettings}>Settings</TextLink>}
    </TopRow>
  );
}
