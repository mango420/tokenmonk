import { Card } from "../../components/Card";
import type { StatTile } from "./model";

interface StatTilesProps {
  tiles: StatTile[];
}

export function StatTiles({ tiles }: StatTilesProps) {
  return (
    <div className="stat-tiles">
      {tiles.map((tile) => (
        <Card key={tile.label} className="stat-tile">
          <span className="caption">{tile.label}</span>
          <span className={`stat-tile__value live-number${tile.muted ? " muted" : ""}`}>{tile.value}</span>
        </Card>
      ))}
    </div>
  );
}
