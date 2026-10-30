interface ToggleRowProps {
  label: string;
  caption: string;
  checked: boolean;
  onChange: (next: boolean) => void;
}

/** Settings row with a switch on the right; the whole row is the hit target. */
export function ToggleRow({ label, caption, checked, onChange }: ToggleRowProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      className={`toggle-row${checked ? " toggle-row--on" : ""}`}
      onClick={() => onChange(!checked)}
    >
      <span className="toggle-row__text">
        <span className="label-strong">{label}</span>
        <span className="caption">{caption}</span>
      </span>
      <span className="toggle" aria-hidden="true">
        <span className="toggle__knob" />
      </span>
    </button>
  );
}
