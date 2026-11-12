interface LevelBarProps {
  /** Fill 0..1; `null` shows the track only. */
  ratio: number | null;
}

/** Level bar: `line` track, `--accent` fill, `--bar-height` everywhere. */
export function LevelBar({ ratio }: LevelBarProps) {
  return (
    <div className="level-bar" aria-hidden="true">
      {ratio !== null && ratio > 0 ? (
        <div className="level-bar__fill" style={{ width: `${ratio * 100}%` }} />
      ) : null}
    </div>
  );
}
