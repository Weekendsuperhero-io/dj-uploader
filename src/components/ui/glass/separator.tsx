"use client";

import * as React from "react";
import { cn } from "@/lib/utils";
import { Separator as BaseSeparator } from "../separator";

export interface SeparatorProps extends React.ComponentProps<typeof BaseSeparator> {
  glow?: boolean;
}

/**
 * Sistine Separator - Enhanced separator with glassy effects
 */
export const Separator = React.forwardRef<React.ElementRef<typeof BaseSeparator>, SeparatorProps>(
  ({ className, variant = "glass", glow = false, ...props }, ref) => {
    return <BaseSeparator ref={ref} variant={variant} className={cn(glow && "shadow-sm shadow-(color:--glass-glow)", className)} {...props} />;
  },
);
Separator.displayName = "Separator";
