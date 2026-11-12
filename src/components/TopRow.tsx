import type { ReactNode } from "react";

interface TopRowProps {
  /** The "Tokenmonk" wordmark on the left (main screens); sub-screens hold only their link. */
  brand?: boolean;
  /** The text link, next to the brand or alone on the left. */
  children?: ReactNode;
}

/** Top row of every screen. The whole row drags the window; links stay clickable. */
export function TopRow({ brand = false, children }: TopRowProps) {
  return (
    <header className="top-row" data-tauri-drag-region="deep">
      {brand ? <span className="brand">Tokenmonk</span> : null}
      {children}
    </header>
  );
}
