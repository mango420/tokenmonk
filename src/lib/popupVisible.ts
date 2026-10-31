import { useEffect, useState } from "react";

function isShown(): boolean {
  return document.visibilityState === "visible" && document.hasFocus();
}

/**
 * True while the popup is on screen and has focus. The tray popup hides on
 * blur, so focus is the dependable signal; `visibilitychange` covers the rest.
 */
export function usePopupVisible(): boolean {
  const [shown, setShown] = useState(isShown);
  useEffect(() => {
    const update = () => setShown(isShown());
    window.addEventListener("focus", update);
    window.addEventListener("blur", update);
    document.addEventListener("visibilitychange", update);
    update();
    return () => {
      window.removeEventListener("focus", update);
      window.removeEventListener("blur", update);
      document.removeEventListener("visibilitychange", update);
    };
  }, []);
  return shown;
}
