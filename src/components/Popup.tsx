import type { ReactNode } from "react";
import { TitleBar } from "./TitleBar";

interface PopupProps {
  children: ReactNode;
  /**
   * Covers the whole popup. Seam for game toasts (#12); the frame is `position: relative`.
   */
  overlay?: ReactNode;
}

/** The 360 x 680 popup frame every view renders into. */
export function Popup({ children, overlay }: PopupProps) {
  return (
    <div className="popup">
      <TitleBar />
      {children}
      {overlay}
    </div>
  );
}
