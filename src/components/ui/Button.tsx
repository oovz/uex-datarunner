import React from "react";

export type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "default" | "secondary" | "ghost" | "danger";
};

export function Button({
  variant = "default",
  className = "",
  children,
  type = "button",
  ...props
}: ButtonProps) {
  const base =
    "inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-md text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-3.5 [&_svg]:shrink-0 h-8 px-3 cursor-pointer";

  const variantStyles = {
    default:
      "bg-primary text-primary-foreground font-semibold hover:bg-primary/90 active:bg-primary/80",
    secondary:
      "border border-border bg-background text-foreground hover:bg-accent hover:text-accent-foreground",
    ghost: "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
    danger: "border border-destructive/30 bg-transparent text-destructive hover:bg-destructive/10",
  };

  const finalClass = `${base} ${variantStyles[variant]} ${className}`;

  return (
    <button type={type} className={finalClass} {...props}>
      {children}
    </button>
  );
}
