import { MonkFigure } from "./Monk";

/**
 * The standalone monk at `--mark-lg-size` (onboarding rows, About): the
 * gauge drawing cropped to the standalone viewBox (docs/DESIGN.md, "Icon
 * system"). The robe takes the `--accent` of the surrounding mode.
 */
export function LogoMark() {
  return (
    <span className="logo-mark" aria-hidden="true">
      <svg className="logo-mark__svg" viewBox="76 76 128 146">
        <MonkFigure />
      </svg>
    </span>
  );
}
