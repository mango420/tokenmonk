/**
 * OS notifications through the Tauri notification plugin (JS API).
 *
 * Findings from the plugin docs and source (v2.8): `sendNotification` takes
 * title, body, icon, sound, channel and more. `silent` exists but is iOS only.
 * On desktop the only sound control is `sound` (a name or file path), so there
 * is no way to ask for a silent toast. We never pass `sound`; the OS then uses
 * its own default for toasts. The in-app tone follows `Settings.sound`, the
 * notification itself cannot.
 */
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { describeError, insideTauri } from "../../lib/backend";

let permission: Promise<boolean> | null = null;

/** Asks the user once per run; a denial is remembered and never asked again. */
function ensurePermission(): Promise<boolean> {
  permission ??= (async () => {
    if (await isPermissionGranted()) return true;
    return (await requestPermission()) === "granted";
  })();
  return permission;
}

/** True while the popup window is not shown (tray toggled away or lost focus). */
export async function isPopupHidden(): Promise<boolean> {
  if (!insideTauri()) return document.visibilityState === "hidden";
  try {
    return !(await getCurrentWindow().isVisible());
  } catch {
    return document.visibilityState === "hidden";
  }
}

/** Shows an OS notification if the user allows it. Never throws. */
export async function notify(title: string, body: string | null): Promise<void> {
  if (!insideTauri()) return;
  try {
    if (!(await ensurePermission())) return;
    sendNotification(body === null ? { title } : { title, body });
  } catch (error) {
    console.warn(`notification failed: ${describeError(error)}`);
  }
}
