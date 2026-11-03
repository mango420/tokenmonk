import { useEffect, useState, type CSSProperties } from "react";
import { FADE_MS, TOAST_SLIDE_MS } from "./constants";

/** `prefers-reduced-motion: reduce`: overlays then appear and disappear without a fade. */
export function prefersReducedMotion(): boolean {
  try {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  } catch {
    return false;
  }
}

/**
 * `true` once the element has been painted at opacity 0 (so the fade-in
 * runs); `false` again as soon as `open` turns false (fade-out). With
 * reduced motion it follows `open` immediately.
 */
export function useFadeShown(open: boolean): boolean {
  const [shown, setShown] = useState(false);
  useEffect(() => {
    if (!open || prefersReducedMotion()) {
      setShown(open);
      return undefined;
    }
    let inner = 0;
    const outer = requestAnimationFrame(() => {
      inner = requestAnimationFrame(() => setShown(true));
    });
    return () => {
      cancelAnimationFrame(outer);
      cancelAnimationFrame(inner);
    };
  }, [open]);
  return shown;
}

/** Opacity-only fade; no transition at all with reduced motion. */
export function fadeStyle(shown: boolean): CSSProperties {
  return {
    opacity: shown ? 1 : 0,
    transition: prefersReducedMotion() ? "none" : `opacity ${FADE_MS}ms linear`,
  };
}

/** Delay before the toast is removed after its slide-out started. */
export function slideOutDelay(): number {
  return prefersReducedMotion() ? 0 : TOAST_SLIDE_MS;
}

/** Delay before an item is removed after its fade-out started. */
export function fadeOutDelay(): number {
  return prefersReducedMotion() ? 0 : FADE_MS;
}
