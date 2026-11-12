import { LevelBar } from "../../components/LevelBar";
import type { LevelModel } from "./model";

interface LevelBlockProps {
  model: LevelModel;
  /** Opens the achievements screen. */
  onOpen: () => void;
}

/** The level card: a full-width button with title and level, XP and the level bar. */
export function LevelBlock({ model, onOpen }: LevelBlockProps) {
  return (
    <button type="button" className="card level-card" onClick={onOpen}>
      <span className="level-card__row">
        <span className="level-card__title">
          {model.title}
          {model.level === null ? null : (
            <>
              {" · Level "}
              <span className="live-number">{model.level}</span>
            </>
          )}
        </span>
        {model.xp === null ? null : <span className="meta live-number">{model.xp}</span>}
      </span>
      <LevelBar ratio={model.ratio} />
    </button>
  );
}
