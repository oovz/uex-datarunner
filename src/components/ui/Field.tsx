import React from "react";

export type FieldProps = {
  label: string;
  hint?: string;
  children: React.ReactNode;
};

/**
 * A labelled form field. The control is nested inside the `<label>`, so the
 * label is implicitly associated with it (accessible by label text) without
 * fragile id wiring.
 */
export function Field({ label, hint, children }: FieldProps) {
  return (
    <label className="flex w-full flex-col gap-1">
      <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </span>
      {children}
      {hint ? <span className="text-[10px] leading-snug text-muted-foreground">{hint}</span> : null}
    </label>
  );
}
