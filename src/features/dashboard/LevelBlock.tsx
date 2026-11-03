import { LevelBar } from "../../components/LevelBar";
import type { LevelModel } from "./model";

interface LevelBlockProps {
  model: LevelModel;
  /** Opens the achievements screen. */
  onOpen: () => void;
}

/** Full-width button at the bottom: title, XP, level line and the level bar. */
export function LevelBlock({ model, onOpen }: LevelBlockProps) {
  return (
    <button type="button" className="level-block" onClick={onOpen}>
      <span className="level-block__row">
        <span className="level-block__title">{model.title}</span>
        {model.xp === null ? null : <span className="level-block__xp live-number">{model.xp}</span>}
      </span>
      <span className="level-block__line">{model.line}</span>
      <LevelBar ratio={model.ratio} />
    </button>
  );
}
