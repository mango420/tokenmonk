import type { ReactNode } from "react";

/** Line icons on a 24 x 24 grid; stroke, caps and joins come from CSS (`.icon`). */
const PATHS = {
  windowMaximize: <rect x="5" y="5" width="14" height="14" rx="1" />,
  windowRestore: (
    <>
      <rect x="4" y="9" width="11" height="11" rx="1" />
      <path d="M9 9V5h11v11h-5" />
    </>
  ),
  windowClose: <path d="M6 6l12 12M18 6L6 18" />,
  moon: <path d="M20 14.5A8 8 0 0 1 9.5 4 8 8 0 1 0 20 14.5z" />,
  drop: <path d="M12 3s6 6.2 6 10.5a6 6 0 0 1-12 0C6 9.2 12 3 12 3z" />,
  bolt: <path d="M13 2 4 14h7l-1 8 9-12h-7z" />,
  target: (
    <>
      <circle cx="12" cy="12" r="9" />
      <circle cx="12" cy="12" r="4" />
    </>
  ),
  person: (
    <>
      <circle cx="12" cy="8" r="3" />
      <path d="M5 20c1.5-4 4-6 7-6s5.5 2 7 6" />
    </>
  ),
  ghost: (
    <>
      <path d="M6 20v-9a6 6 0 0 1 12 0v9l-3-2-3 2-3-2z" />
      <path d="M10 11h.01M14 11h.01" />
    </>
  ),
} satisfies Record<string, ReactNode>;

export type IconName = keyof typeof PATHS;
export type IconSize = "md" | "sm" | "xs";

interface IconProps {
  name: IconName;
  /** md = `--icon-size`, sm = `--icon-size-sm`, xs = `--icon-size-xs`. */
  size?: IconSize;
}

export function Icon({ name, size = "md" }: IconProps) {
  return (
    <svg className={`icon icon--${size}`} viewBox="0 0 24 24" aria-hidden="true">
      {PATHS[name]}
    </svg>
  );
}
