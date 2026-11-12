/**
 * Reads a unitless or px design token (for example `--gauge-radius`) from the
 * document root. SVG geometry needs numbers; the values still live only in
 * tokens.css.
 */
export function readTokenNumber(name: string): number {
  const raw = getComputedStyle(document.documentElement).getPropertyValue(name);
  return Number.parseFloat(raw);
}
