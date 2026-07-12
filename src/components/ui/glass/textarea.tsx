"use client";

import * as React from "react";
import { cn } from "@/lib/utils";
import { Textarea as BaseTextarea } from "../textarea";

export interface TextareaProps extends React.ComponentProps<typeof BaseTextarea> {
  icon?: React.ReactNode;
  error?: boolean;
}

/**
 * Sistine Textarea - A beautifully designed textarea component with glassy effects
 * Built on top of the base Textarea component with enhanced visual styling
 */
export const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(({ className, variant = "glass", icon, error, ...props }, ref) => {
  return (
    <div className="relative">
      {icon && <div className="absolute left-3 top-3 text-muted-foreground">{icon}</div>}
      <BaseTextarea
        ref={ref}
        variant={variant}
        className={cn(
          icon && "pl-10",
          error && "border-destructive focus-visible:ring-destructive",
          "transition duration-200 focus-visible:scale-[1.01]",
          className,
        )}
        {...props}
      />
    </div>
  );
});
Textarea.displayName = "Textarea";
