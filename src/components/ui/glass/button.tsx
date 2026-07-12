"use client";

import * as React from "react";
import type { GlassCustomization } from "@/lib/glass-utils";
import { type HoverEffect, hoverEffects } from "@/lib/hover-effects";
import { cn } from "@/lib/utils";
import { Button as BaseButton } from "../button";

export interface ButtonProps extends Omit<React.ComponentProps<typeof BaseButton>, "glass"> {
  effect?: HoverEffect;
  glass?: GlassCustomization;
}

/**
 * Sistine Button - A beautifully designed button component with glassy effects
 * Built on top of the base Button component with enhanced visual effects
 *
 * @example
 * ```tsx
 * <Button
 *   glass={{
 *     color: "oklch(62.3083% 0.188015 259.814527 / 0.2)",
 *     blur: 25,
 *     outline: "oklch(62.3083% 0.188015 259.814527 / 0.4)"
 *   }}
 * >
 *   Click me
 * </Button>
 * ```
 */
export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(({ className, effect = "glow", variant = "glass", glass, ...props }, ref) => {
  return (
    <BaseButton
      ref={ref}
      variant={variant}
      glass={glass}
      className={cn(
        "relative overflow-hidden",
        hoverEffects({
          hover: effect,
        }),
        className,
      )}
      {...props}
    />
  );
});
Button.displayName = "Button";
