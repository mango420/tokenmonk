import type { ReactNode } from "react";

interface TextButtonProps {
  onClick?: () => void;
  children: ReactNode;
}

/** Caption-sized link-style button: `accent-text`, underlined, `hit-target` tall. */
export function TextButton({ onClick, children }: TextButtonProps) {
  return (
    <button type="button" className="text-button" onClick={onClick}>
      {children}
    </button>
  );
}
