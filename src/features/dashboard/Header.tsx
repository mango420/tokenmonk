import { TextLink } from "../../components/TextLink";
import { TopRow } from "../../components/TopRow";

interface HeaderProps {
  /** Omitted: brand only (onboarding, loading). */
  onOpenSettings?: () => void;
}

/** Top row of the main screens: brand on the left, the "Settings" link on the right. */
export function Header({ onOpenSettings }: HeaderProps) {
  return (
    <TopRow brand>
      {onOpenSettings === undefined ? null : <TextLink onClick={onOpenSettings}>Settings</TextLink>}
    </TopRow>
  );
}
