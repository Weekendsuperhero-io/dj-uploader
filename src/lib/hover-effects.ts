import { cva } from "class-variance-authority";

/**
 * Shared hover effect variants for all Sistine components
 *
 * Effects:
 * - none: No hover effect
 * - glow: Themed glow shadow (--glass-glow, follows the tint hue) that intensifies on hover
 * - shimmer: Moving shimmer effect across the component
 * - ripple: Ripple effect that scales outward on hover
 * - lift: Subtle lift effect with shadow increase
 * - scale: Scale up slightly on hover
 */
export const hoverEffects = cva("transition duration-300", {
  variants: {
    hover: {
      none: "",
      glow: "glass-glow hover:glass-glow-lg",
      shimmer:
        "relative overflow-hidden before:absolute before:inset-0 before:-translate-x-full before:bg-gradient-to-r before:from-transparent before:via-white/20 before:to-transparent hover:before:translate-x-full before:transition-transform before:duration-1000",
      ripple:
        "relative overflow-hidden after:absolute after:inset-0 after:scale-0 after:rounded-full after:bg-white/30 after:transition-transform after:duration-500 hover:after:scale-150",
      lift: "hover:-translate-y-1 hover:shadow-lg",
      scale: "hover:scale-105",
    },
  },
  defaultVariants: {
    hover: "none",
  },
});

export type HoverEffect = "none" | "glow" | "shimmer" | "ripple" | "lift" | "scale";
