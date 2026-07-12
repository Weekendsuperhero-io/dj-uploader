"use client";

import * as React from "react";
import { cn } from "@/lib/utils";
import { Checkbox as BaseCheckbox } from "../checkbox";

export interface CheckboxProps extends React.ComponentProps<typeof BaseCheckbox> {
  glow?: boolean;
}

/**
 * Sistine Checkbox - Enhanced checkbox with glassy effects
 */
export const Checkbox = React.forwardRef<React.ElementRef<typeof BaseCheckbox>, CheckboxProps>(
  ({ className, variant = "glass", glow = false, ...props }, ref) => {
    return (
      <BaseCheckbox
        ref={ref}
        variant={variant}
        className={cn(glow && "data-[state=checked]:glass-glow", "transition duration-200", className)}
        {...props}
      />
    );
  },
);
Checkbox.displayName = "Checkbox";
