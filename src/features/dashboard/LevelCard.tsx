import { Bar } from "../../components/Bar";
import { Card } from "../../components/Card";
import type { LevelModel } from "./model";

interface LevelCardProps {
  model: LevelModel;
}

export function LevelCard({ model }: LevelCardProps) {
  return (
    <Card className="stack-card">
      <div className="row-between">
        <span className="level-card__level">{model.heading}</span>
        <span className="caption caption--accent">{model.title}</span>
      </div>
      <Bar ratio={model.ratio} fill="ink" />
      <span className="mono-value live-number muted">{model.caption}</span>
    </Card>
  );
}
