import type { ReactNode } from "react";
import { LogoMark } from "./LogoMark";

interface TopRowProps {
  /** Brand on the left (main screens); sub-screens hold only their link. */
  brand?: boolean;
  /** The text link, on the right next to the brand or alone on the left. */
  children?: ReactNode;
}

/** Top row of every screen. The whole row drags the window; links stay clickable. */
export function TopRow({ brand = false, children }: TopRowProps) {
  return (
    <header className="top-row" data-tauri-drag-region="deep">
      {brand ? (
        <div className="brand">
          <LogoMark size="brand" />
          <span>Tokenmonk</span>
        </div>
      ) : null}
      {children}
    </header>
  );
}
