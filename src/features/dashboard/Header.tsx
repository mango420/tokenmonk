import { TextLink } from "../../components/TextLink";
import { TopRow } from "../../components/TopRow";

interface HeaderProps {
  /** Omitted: brand only (loading). */
  onOpenSettings?: () => void;
}

/** Top row of the main screens: brand and the "Settings" link. */
export function Header({ onOpenSettings }: HeaderProps) {
  return (
    <TopRow brand>
      {onOpenSettings === undefined ? null : <TextLink onClick={onOpenSettings}>Settings</TextLink>}
    </TopRow>
  );
}
