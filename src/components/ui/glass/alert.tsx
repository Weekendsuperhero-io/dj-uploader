"use client";

import * as React from "react";
import { type HoverEffect, hoverEffects } from "@/lib/hover-effects";
import { cn } from "@/lib/utils";
import { AlertDescription, AlertTitle, Alert as BaseAlert } from "../alert";

export interface AlertProps extends React.ComponentProps<typeof BaseAlert> {
  glow?: boolean;
  hover?: HoverEffect;
}

/**
 * Sistine Alert - Enhanced alert with glassy effects and hover animations
 *
 * @example
 * ```tsx
 * <Alert variant="glass" hover="glow">
 *   <AlertTitle>Heads up!</AlertTitle>
 *   <AlertDescription>You have new notifications</AlertDescription>
 * </Alert>
 * ```
 */
export const Alert = React.forwardRef<HTMLDivElement, AlertProps>(({ className, variant = "glass", glow = false, hover = "none", ...props }, ref) => {
  return (
    <BaseAlert
      ref={ref}
      variant={variant}
      className={cn(
        "relative overflow-hidden",
        glow && "glass-glow",
        "transition duration-200",
        hoverEffects({
          hover,
        }),
        className,
      )}
      {...props}
    />
  );
});
Alert.displayName = "Alert";

export { AlertDescription, AlertTitle };
