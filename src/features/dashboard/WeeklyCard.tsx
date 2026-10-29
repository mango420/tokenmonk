import { Bar } from "../../components/Bar";
import { Card } from "../../components/Card";
import type { WeeklyModel } from "./model";

interface WeeklyCardProps {
  model: WeeklyModel;
}

export function WeeklyCard({ model }: WeeklyCardProps) {
  return (
    <Card className="stack-card">
      <div className="row-between">
        <span className="label-strong">{model.label}</span>
        <span className={`mono-value live-number${model.ratio === null ? " muted" : ""}`}>
          {model.value}
        </span>
      </div>
      <Bar ratio={model.ratio} />
      <span className="caption">{model.caption}</span>
    </Card>
  );
}
