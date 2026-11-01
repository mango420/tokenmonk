import { Icon, type IconName } from "./Icon";

/** Fill colour family of a mode. */
export type Fill = "saffron" | "jade";

interface ChoiceTileProps {
  icon: IconName;
  name: string;
  /** Omitted in the compact (settings) variant. */
  description?: string;
  fill: Fill;
  selected: boolean;
  compact?: boolean;
  disabled?: boolean;
  onSelect: () => void;
}

/** Radio-style tile. Selection is a ring plus a check icon, never colour alone. */
export function ChoiceTile({
  icon,
  name,
  description,
  fill,
  selected,
  compact = false,
  disabled = false,
  onSelect,
}: ChoiceTileProps) {
  const classes = ["choice-tile", `fill--${fill}`];
  if (compact) classes.push("choice-tile--compact");
  if (selected) classes.push("choice-tile--selected");
  return (
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      className={classes.join(" ")}
      disabled={disabled}
      onClick={onSelect}
    >
      <span className="choice-tile__head">
        <span className="choice-tile__icon">
          <Icon name={icon} />
        </span>
        <span className="choice-tile__name">{name}</span>
        {selected ? (
          <span className="choice-tile__check">
            <Icon name="check" />
          </span>
        ) : null}
      </span>
      {description === undefined ? null : <span className="choice-tile__description">{description}</span>}
    </button>
  );
}
