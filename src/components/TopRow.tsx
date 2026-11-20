import type { ReactNode } from "react";
import { MonkMark } from "./Monk";

interface TopRowProps {
  /**
   * The corner (docs/DESIGN.md, "Corner rule"): monk mark plus wordmark, or the
   * wordmark alone where the large monk is on screen.
   */
  corner?: "mark" | "wordmark";
  /** The text link on the right ("Settings", "Back"). */
  children?: ReactNode;
}

/** Top row of every screen. The whole row drags the window; links stay clickable. */
export function TopRow({ corner = "mark", children }: TopRowProps) {
  return (
    <header className="top-row" data-tauri-drag-region="deep">
      <span className="corner">
        {corner === "mark" ? <MonkMark /> : null}
        <span className="brand">Tokenmonk</span>
      </span>
      {children}
    </header>
  );
}
