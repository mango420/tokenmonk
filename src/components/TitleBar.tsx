import { useCallback, useEffect, useState, type CSSProperties } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { FADE_MS } from "../features/game";
import { describeError, insideTauri } from "../lib/backend";
import { Icon } from "./Icon";

const FADE_STYLE = { "--fade-ms": `${FADE_MS}ms` } as CSSProperties;

/**
 * Hover title bar for the undecorated popup. A slim strip at the top edge
 * reveals the bar (fade on opacity only). The bar is the drag region, the
 * buttons are not. Close hides the popup; the backend decides what close means.
 */
export function TitleBar() {
  const [maximized, setMaximized] = useState(false);
  const native = insideTauri();

  const sync = useCallback(async () => {
    try {
      setMaximized(await getCurrentWindow().isMaximized());
    } catch (error) {
      console.warn("window state unavailable:", describeError(error));
    }
  }, []);

  useEffect(() => {
    if (!native) return;
    void sync();
    const unlisten = getCurrentWindow().onResized(() => void sync());
    return () => {
      void unlisten.then((off) => off());
    };
  }, [native, sync]);

  // Plain browser (dev preview): there is no window to control.
  if (!native) return null;

  const toggleMaximize = async () => {
    try {
      await getCurrentWindow().toggleMaximize();
      await sync();
    } catch (error) {
      console.warn("maximize failed:", describeError(error));
    }
  };

  const close = async () => {
    try {
      await getCurrentWindow().close();
    } catch (error) {
      console.warn("close failed:", describeError(error));
    }
  };

  return (
    <div className="titlebar" style={FADE_STYLE} data-tauri-drag-region>
      <div className="titlebar__bar" data-tauri-drag-region>
        <button
          type="button"
          className="titlebar__button"
          aria-label={maximized ? "Restore" : "Maximize"}
          onClick={() => void toggleMaximize()}
        >
          <Icon name={maximized ? "windowRestore" : "windowMaximize"} size="sm" />
        </button>
        <button
          type="button"
          className="titlebar__button titlebar__button--close"
          aria-label="Close"
          onClick={() => void close()}
        >
          <Icon name="windowClose" size="sm" />
        </button>
      </div>
    </div>
  );
}
