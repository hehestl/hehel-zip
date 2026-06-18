interface Props {
  size?: "thin" | "normal";
  className?: string;
  ariaLabel: string;
}

export function ProgressBar({
  size = "normal",
  className = "",
  ariaLabel,
}: Props) {
  const heightClass = size === "thin" ? "progress-bar--thin" : "progress-bar--normal";

  return (
    <div
      className={`progress-bar progress-bar--indeterminate ${heightClass} ${className}`.trim()}
      role="progressbar"
      aria-label={ariaLabel}
      aria-busy="true"
    />
  );
}
