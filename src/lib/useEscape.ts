import { useEffect } from "react";

/** Calls `handler` when Esc is pressed while the screen is open. */
export function useEscape(handler: () => void): void {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") handler();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handler]);
}
