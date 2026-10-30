import { Bar } from "../../components/Bar";
import { Card } from "../../components/Card";
import { TextButton } from "../../components/TextButton";
import type { WeeklyModel } from "./model";

interface WeeklyCardProps {
  model: WeeklyModel;
  /** Opens Settings at "Limit data". */
  onOpenLimitSetup?: () => void;
}

export function WeeklyCard({ model, onOpenLimitSetup }: WeeklyCardProps) {
  return (
    <Card className="stack-card">
      <div className="row-between">
        <span className="label-strong">{model.label}</span>
        <span className={`mono-value live-number${model.ratio === null ? " muted" : ""}`}>
          {model.value}
        </span>
      </div>
      <Bar ratio={model.ratio} />
      {model.action === "openLimitSetup" ? (
        <TextButton onClick={onOpenLimitSetup}>{model.caption}</TextButton>
      ) : (
        <span className="caption">{model.caption}</span>
      )}
    </Card>
  );
}
