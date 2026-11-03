import type { StatModel } from "./model";

interface StatTilesProps {
  stats: StatModel[];
}

/** Two equal columns: label, compact value, unit. No cards. */
export function StatTiles({ stats }: StatTilesProps) {
  return (
    <div className="stats">
      {stats.map((stat) => (
        <div key={stat.label} className="stat">
          <span className="stat__label">{stat.label}</span>
          <span className="stat__value live-number">{stat.value}</span>
          <span className="stat__unit">{stat.unit}</span>
        </div>
      ))}
    </div>
  );
}
