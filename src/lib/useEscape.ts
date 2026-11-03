import { useEffect } from "react";

/**
 * Calls `handler` when Esc is pressed while the screen is open. Ignores Esc
 * that an overlay already handled: a dialog (the level-up screen) owns the key
 * while focus is inside it.
 */
export function useEscape(handler: () => void): void {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      if (event.target instanceof Element && event.target.closest('[role="dialog"]') !== null) return;
      handler();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handler]);
}
