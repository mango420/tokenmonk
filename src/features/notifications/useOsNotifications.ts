import { useEffect, useRef } from "react";
import type { AppData } from "../../lib/appData";
import { useBackendEvent } from "../../lib/backend";
import { Events } from "../../lib/contracts";
import { evaluateWarnings, initialWarningState, type WarningState } from "./limitWarnings";
import { isPopupHidden, notify } from "./osNotify";

/**
 * Sends OS notifications while the popup is hidden (docs/DESIGN.md, "While
 * hidden"): non-retroactive unlocks and limit warnings. The
 * in-popup toast queue is separate (#12) and still shows on the next open.
 */
export function useOsNotifications(data: AppData, enabled: boolean): void {
  const latest = useRef(data);
  latest.current = data;
  const warnings = useRef<WarningState>(initialWarningState);

  useBackendEvent(
    Events.achievementUnlocked,
    (unlock) => {
      if (unlock.retroactive) return;
      void isPopupHidden().then((hidden) => {
        if (!hidden) return;
        const description = latest.current.game.achievements.find((a) => a.id === unlock.id)?.description;
        void notify(`${unlock.name} unlocked · +${unlock.xp} XP`, unlock.detail ?? description ?? null);
      });
    },
    enabled,
  );

  const { dashboard, settings } = data;
  useEffect(() => {
    if (!enabled) return;
    const result = evaluateWarnings(warnings.current, dashboard, settings.mode);
    warnings.current = result.state;
    if (result.notices.length === 0) return;
    void isPopupHidden().then((hidden) => {
      if (!hidden) return;
      for (const notice of result.notices) void notify(notice.title, notice.body);
    });
  }, [dashboard, settings.mode, enabled]);
}
