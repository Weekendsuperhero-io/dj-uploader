import { cva, type VariantProps } from "class-variance-authority";
import { Slot as SlotPrimitive } from "radix-ui";
import type * as React from "react";
import { type GlassCustomization, getGlassStyles } from "@/lib/glass-utils";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive active:scale-[0.96] active:transition-transform",
  {
    variants: {
      variant: {
        default:
          "bg-primary text-primary-foreground [background-image:var(--glass-bg)] shadow-md hover:bg-primary/90 transition active:opacity-90 active:shadow-[var(--press-shadow)]",
        glass: "glass-bg text-foreground hover:opacity-90 transition active:opacity-80 active:shadow-[var(--press-shadow-strong)]",
        gradient: "glass-gradient text-foreground hover:opacity-90 transition active:opacity-80 active:shadow-[var(--press-shadow)]",
        frosted: "glass-frosted text-foreground hover:opacity-90 transition active:opacity-85 active:shadow-[var(--press-shadow-strong)]",
        crystal: "glass-crystal text-foreground transition active:opacity-90 active:shadow-[var(--press-shadow-deep)]",
        opaque: "glass-opaque text-foreground hover:opacity-90 transition active:opacity-85 active:shadow-[var(--press-shadow)]",
        surface: "glass-surface text-foreground",
        solid: "glass-solid text-foreground",
        destructive:
          "glass-bg text-destructive border border-destructive/60 hover:opacity-90 transition active:opacity-80 focus-visible:ring-destructive/20 active:shadow-[var(--press-shadow-strong)]",
        outline:
          "glass-bg backdrop-blur-[var(--blur-sm)] text-foreground border-2 border-foreground/20 hover:border-foreground/40 dark:border-white/40 dark:hover:border-white/60 dark:text-white transition active:border-foreground/50 active:shadow-[var(--press-shadow-sm)]",
        secondary: "glass-surface text-foreground hover:opacity-90 transition active:opacity-80 active:shadow-[var(--press-shadow-strong)]",
        ghost:
          "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50 active:bg-accent/80 dark:active:bg-accent/60 active:shadow-[var(--press-shadow-sm)]",
        link: "text-primary underline-offset-4 hover:underline active:opacity-80",
      },
      size: {
        xs: "h-6 gap-1 rounded-md px-2 text-xs has-[>svg]:px-1.5 [&_svg:not([class*='size-'])]:size-3",
        default: "h-9 px-4 py-2 has-[>svg]:px-3",
        sm: "h-8 rounded-md gap-1.5 px-3 has-[>svg]:px-2.5",
        lg: "h-10 rounded-md px-6 has-[>svg]:px-4",
        icon: "size-9",
        "icon-xs": "size-6 rounded-md [&_svg:not([class*='size-'])]:size-3",
        "icon-sm": "size-8",
        "icon-lg": "size-10",
      },
    },
    defaultVariants: {
      variant: "glass",
      size: "default",
    },
  },
);

function Button({
  className,
  variant = "glass",
  size = "default",
  asChild = false,
  glass,
  style,
  ...props
}: React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean;
    glass?: GlassCustomization;
  }) {
  const Comp = asChild ? SlotPrimitive.Slot : "button";

  // Apply glass styles for glass variants when custom glass props are provided
  const hasCustomGlass = glass !== undefined;
  const isGlassVariant =
    variant === "glass" ||
    variant === "gradient" ||
    variant === "frosted" ||
    variant === "crystal" ||
    variant === "opaque" ||
    variant === "surface" ||
    variant === "solid" ||
    variant === "outline";

  const glassStyles = isGlassVariant && hasCustomGlass ? getGlassStyles(glass) : {};

  return (
    <Comp
      data-slot="button"
      data-variant={variant}
      data-size={size}
      data-glass-tint={variant === "destructive" ? "destructive" : undefined}
      className={cn(
        buttonVariants({
          variant,
          size,
          className,
        }),
      )}
      style={{
        ...glassStyles,
        ...style,
      }}
      {...props}
    />
  );
}

export { Button, buttonVariants };
