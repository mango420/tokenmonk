import { Bar } from "../../components/Bar";
import { Card } from "../../components/Card";
import { Icon, type IconName } from "../../components/Icon";
import type { AchievementId } from "../../lib/contracts";
import { formatInt } from "../../lib/format";
import type { AchievementRowModel } from "./model";

const GLYPH: Record<AchievementId, IconName> = {
  firstBlood: "drop",
  nightOwl: "moon",
  rateLimitRambo: "bolt",
  soClose: "target",
  tokenMonk: "person",
  boo: "ghost",
};

interface AchievementRowProps {
  model: AchievementRowModel;
}

export function AchievementRow({ model }: AchievementRowProps) {
  return (
    <Card className="achievement">
      <div className="badge">
        <Icon name={GLYPH[model.id]} size="sm" />
      </div>
      {model.kind === "unlocked" ? (
        <>
          <div className="achievement__text">
            <span className="label-strong">{model.name} unlocked</span>
            {model.detail !== null ? <span className="caption">{model.detail}</span> : null}
          </div>
          <span className="mono-value live-number caption--accent">+{formatInt(model.xp)} XP</span>
        </>
      ) : (
        <div className="achievement__text achievement__text--bar">
          <span className="label-strong">{model.name}</span>
          <Bar ratio={model.kind === "progress" ? model.ratio : null} small />
          <span className={model.kind === "progress" && model.mono ? "mono-value muted" : "caption"}>
            {model.caption}
          </span>
        </div>
      )}
    </Card>
  );
}
