"use client";

import type * as React from "react";
import { type HoverEffect, hoverEffects } from "@/lib/hover-effects";
import { cn } from "@/lib/utils";
import { Badge as BaseBadge } from "../badge";

export interface BadgeProps extends React.ComponentProps<typeof BaseBadge> {
  glow?: boolean;
  hover?: HoverEffect;
}

/**
 * Sistine Badge - Enhanced badge with glassy effects and glow option
 */
export function Badge({ className, variant = "glass", glow = false, hover = "none", ...props }: BadgeProps) {
  return (
    <BaseBadge
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
}
