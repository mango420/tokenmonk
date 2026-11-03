interface LevelBarProps {
  /** Fill 0..1; `null` shows the track only. */
  ratio: number | null;
  /** Taller variant of the achievements screen. */
  large?: boolean;
}

/** Level bar: `line` track, `--accent` fill. */
export function LevelBar({ ratio, large = false }: LevelBarProps) {
  return (
    <div className={large ? "level-bar level-bar--lg" : "level-bar"} aria-hidden="true">
      {ratio !== null && ratio > 0 ? (
        <div className="level-bar__fill" style={{ width: `${ratio * 100}%` }} />
      ) : null}
    </div>
  );
}
