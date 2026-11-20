import { LevelBar } from "../../components/LevelBar";
import { TextLink } from "../../components/TextLink";
import { TopRow } from "../../components/TopRow";
import type { GameState } from "../../lib/contracts";
import { useEscape } from "../../lib/useEscape";
import { buildAchievements } from "./model";

interface AchievementsViewProps {
  game: GameState;
  onBack: () => void;
}

/** Title, level line, level bar and the achievements: the locked ones at half strength. */
export function AchievementsView({ game, onBack }: AchievementsViewProps) {
  useEscape(onBack);
  const model = buildAchievements(game);

  return (
    <div className="screen">
      <TopRow>
        <TextLink onClick={onBack}>Back</TextLink>
      </TopRow>
      <div className="scroll">
        <h1 className="title title--tight">{model.title}</h1>
        <p className="meta meta--tight">{model.levelLine}</p>
        <LevelBar ratio={model.ratio} />
        <h2 className="section-title">{model.heading}</h2>
        <ul className="ach-list hairlist">
          {model.rows.map((row) => (
            <li key={row.id} className={row.unlocked ? "ach" : "ach ach--locked"}>
              <div className="ach__text">
                <span className="ach__name">{row.name}</span>
                <span className="ach__description">{row.description}</span>
              </div>
              <span className="ach__state">{row.unlocked ? "Unlocked" : "Locked"}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
