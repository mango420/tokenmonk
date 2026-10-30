import type { ReactNode } from "react";

/** Line icons on a 24 x 24 grid; stroke, caps and joins come from CSS (`.icon`). */
const PATHS = {
  flame: <path d="M12 3c1 3 5 5 5 10a5 5 0 0 1-10 0c0-2 1-3.5 2-4.5.3 1.8 1.3 2.8 2.5 3C11 9 11 6 12 3z" />,
  wrench: (
    <path d="M14.7 6.3a4 4 0 0 0-5.4 5.4L3 18l3 3 6.3-6.3a4 4 0 0 0 5.4-5.4l-2.6 2.6-2.4-.6-.6-2.4z" />
  ),
  gear: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1" />
    </>
  ),
  chevronLeft: <path d="M15 5l-7 7 7 7" />,
  check: <path d="M5 12.5l4.5 4.5L19 7.5" />,
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
