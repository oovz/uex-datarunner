import React from "react";

export type BadgeProps = React.HTMLAttributes<HTMLSpanElement> & {
  tone?: "success" | "warning" | "default" | "info" | "danger";
};

export function Badge({ tone = "default", className = "", children, ...props }: BadgeProps) {
  const baseStyle =
    "inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-semibold tracking-wider transition-colors border";

  const toneStyles = {
    default: "bg-secondary text-secondary-foreground border-transparent",
    success: "bg-emerald-50 text-emerald-700 border-emerald-200",
    warning: "bg-amber-50 text-amber-700 border-amber-200",
    info: "bg-sky-50 text-sky-700 border-sky-200",
    danger: "bg-red-50 text-red-700 border-red-200",
  };

  return (
    <span className={`${baseStyle} ${toneStyles[tone]} ${className}`} {...props}>
      {children}
    </span>
  );
}
