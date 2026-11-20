import { TextLink } from "../../components/TextLink";
import { TopRow } from "../../components/TopRow";

interface HeaderProps {
  /** Omitted: the corner only (loading). */
  onOpenSettings?: () => void;
}

/** Top row of the dashboard: the corner (monk mark, wordmark) and the "Settings" link. */
export function Header({ onOpenSettings }: HeaderProps) {
  return (
    <TopRow>{onOpenSettings === undefined ? null : <TextLink onClick={onOpenSettings}>Settings</TextLink>}</TopRow>
  );
}
