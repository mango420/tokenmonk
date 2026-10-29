import type { ReactNode } from "react";

interface CardProps {
  className?: string;
  children: ReactNode;
}

/** `surface-200` block with `radius-md`; the base of every popup card. */
export function Card({ className, children }: CardProps) {
  return <section className={className ? `card ${className}` : "card"}>{children}</section>;
}
