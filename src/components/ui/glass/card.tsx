"use client";

import * as React from "react";
import type { GlassCustomization } from "@/lib/glass-utils";
import { type HoverEffect, hoverEffects } from "@/lib/hover-effects";
import { cn } from "@/lib/utils";
import { Card as BaseCard, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "../card";

export interface CardProps extends React.ComponentProps<typeof BaseCard> {
  gradient?: boolean;
  animated?: boolean;
  hover?: HoverEffect;
  glass?: GlassCustomization;
}

/**
 * Sistine Card - A beautifully designed card component with glassy effects
 * Built on top of the base Card component with enhanced visual styling
 *
 * @example
 * ```tsx
 * // Standard glass variant (default)
 * <Card variant="glass">
 *   Content
 * </Card>
 *
 * // Frosted glass variant - enhanced blur and opacity
 * <Card variant="frosted">
 *   Content
 * </Card>
 *
 * // Crystal glass variant - clear with highlights and glow
 * <Card variant="crystal">
 *   Content
 * </Card>
 *
 * // Custom glass properties
 * <Card
 *   variant="glass"
 *   glass={{
 *     color: "oklch(60.5631% 0.218915 292.717225 / 0.2)",
 *     blur: 30,
 *     transparency: 0.3,
 *     outline: "oklch(60.5631% 0.218915 292.717225 / 0.5)",
 *     innerGlow: "oklch(100% 0 0 / 0.3)",
 *     innerGlowBlur: 25
 *   }}
 *   gradient
 *   animated
 * >
 *   Content
 * </Card>
 * ```
 */
export const Card = React.forwardRef<HTMLDivElement, CardProps>(
  ({ className, variant = "glass", gradient = false, animated = false, hover = "none", glass, children, ...props }, ref) => {
    return (
      <BaseCard
        ref={ref}
        variant={variant}
        glass={glass}
        className={cn(
          "relative overflow-hidden",
          gradient && "bg-gradient-to-br from-purple-500/10 via-blue-500/10 to-pink-500/10",
          animated && "transition duration-300 hover:scale-[1.02] hover:shadow-[var(--glass-shadow-lg)]",
          hoverEffects({
            hover,
          }),
          className,
        )}
        {...props}
      >
        {children}
      </BaseCard>
    );
  },
);
Card.displayName = "Card";

export { CardContent, CardDescription, CardFooter, CardHeader, CardTitle };
