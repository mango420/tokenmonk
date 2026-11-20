interface SwitchProps {
  checked: boolean;
  label: string;
  onChange: (next: boolean) => void;
}

/** 44 x 26 switch, outlined; the track takes the accent when on. */
export function Switch({ checked, label, onChange }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className="switch"
      onClick={() => onChange(!checked)}
    />
  );
}
