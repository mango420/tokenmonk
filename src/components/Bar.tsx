interface BarProps {
  /** Fill 0..1; `null` shows the track only. */
  ratio: number | null;
  /** `accent` (weekly, achievement) or `ink` (XP). */
  fill?: "accent" | "ink";
  small?: boolean;
}

/** Pill progress bar on a `--gauge-track` track. */
export function Bar({ ratio, fill = "accent", small = false }: BarProps) {
  return (
    <div className={`bar${small ? " bar--sm" : ""}`} aria-hidden="true">
      {ratio !== null && ratio > 0 ? (
        <div className={`bar__fill bar__fill--${fill}`} style={{ width: `${ratio * 100}%` }} />
      ) : null}
    </div>
  );
}
