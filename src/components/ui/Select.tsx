import React from "react";

export type SelectProps = React.SelectHTMLAttributes<HTMLSelectElement>;

export function Select({ className = "", children, ...props }: SelectProps) {
  const base =
    "flex h-8 w-full rounded-md border border-input bg-background px-2.5 text-xs text-foreground transition-colors cursor-pointer focus-visible:outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40 disabled:cursor-not-allowed disabled:opacity-50";

  return (
    <select className={`${base} ${className}`} {...props}>
      {children}
    </select>
  );
}
