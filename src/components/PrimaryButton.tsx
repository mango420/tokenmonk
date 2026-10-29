import type { ReactNode } from "react";
import type { Fill } from "./ChoiceTile";

interface PrimaryButtonProps {
  children: ReactNode;
  onClick: () => void;
  disabled?: boolean;
  /** Fill colour family; omitted uses the current accent. */
  fill?: Fill;
  fullWidth?: boolean;
}

export function PrimaryButton({
  children,
  onClick,
  disabled = false,
  fill,
  fullWidth = false,
}: PrimaryButtonProps) {
  const classes = ["button-primary"];
  if (fill !== undefined) classes.push(`fill--${fill}`);
  if (fullWidth) classes.push("button-primary--full");
  return (
    <button type="button" className={classes.join(" ")} disabled={disabled} onClick={onClick}>
      {children}
    </button>
  );
}
