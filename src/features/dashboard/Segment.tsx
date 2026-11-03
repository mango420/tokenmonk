import { WINDOW_LABEL, type WindowChoice } from "./model";

interface SegmentProps {
  value: WindowChoice;
  onChange: (next: WindowChoice) => void;
  /** Without limit data: invisible, but it keeps its height. */
  hidden: boolean;
}

const CHOICES: readonly WindowChoice[] = ["fiveHour", "weekly"];

/** "5 hours" / "Week": text buttons with a rule under the selected one. */
export function Segment({ value, onChange, hidden }: SegmentProps) {
  return (
    <div
      className={hidden ? "segment segment--hidden" : "segment"}
      role="group"
      aria-label="Limit window"
    >
      {CHOICES.map((choice) => (
        <button
          key={choice}
          type="button"
          className="segment__button"
          aria-pressed={value === choice}
          onClick={() => onChange(choice)}
        >
          {WINDOW_LABEL[choice]}
        </button>
      ))}
    </div>
  );
}
