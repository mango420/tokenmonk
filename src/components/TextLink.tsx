import type { ReactNode } from "react";

interface TextLinkProps {
  onClick?: () => void;
  children: ReactNode;
  /** Quiet inline variant for note lines and copy links. */
  small?: boolean;
  /** Accessible name when the visible text alone is ambiguous. */
  label?: string;
}

/** Text action: `ink-2`, `ink` on hover. No icons, no cards. */
export function TextLink({ onClick, children, small = false, label }: TextLinkProps) {
  return (
    <button type="button" className={small ? "link link--small" : "link"} aria-label={label} onClick={onClick}>
      {children}
    </button>
  );
}
