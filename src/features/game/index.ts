/**
 * Game UI for the `Popup` overlay slot: achievement toast and level-up
 * screen, typed by the contract types.
 *
 * Wiring (frontend, #13):
 *   const notices = useGameNotices({ game, sound: settings.sound, popupVisible, blocked: ghostOpen });
 *   // on "achievement-unlocked": notices.pushUnlock(payload)
 *   // on "level-up":             notices.pushLevelUp(payload)
 *   <Popup overlay={<>{ghost}{notices.node}</>}>...</Popup>
 */
export { AchievementToast } from "./AchievementToast";
export { LevelUpScreen } from "./LevelUpScreen";
export { useGameNotices, type GameNotices, type GameNoticesOptions } from "./useGameNotices";
export { FADE_MS, TOAST_VISIBLE_MS, TOAST_COLLAPSE_AT } from "./constants";
export { playTone, type Tone } from "./sound";
export {
  isToastable,
  levelScreenCopy,
  nextToast,
  retroSummary,
  toastCopy,
  type LevelScreenCopy,
  type LevelScreenModel,
  type ToastCopy,
  type ToastItem,
} from "./model";
