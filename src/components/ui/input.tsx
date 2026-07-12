import { cva, type VariantProps } from "class-variance-authority";
import type * as React from "react";
import { type GlassCustomization, getGlassStyles } from "@/lib/glass-utils";
import { cn } from "@/lib/utils";

const inputVariants = cva("", {
  variants: {
    variant: {
      default: "dark:bg-input/30 border-input bg-transparent shadow-xs",
      glass: "glass-surface-sm",
      frosted: "glass-frosted",
      crystal: "glass-crystal",
      opaque: "glass-opaque",
      surface: "glass-surface text-foreground",
      solid: "glass-solid text-foreground",
    },
  },
  defaultVariants: {
    variant: "glass",
  },
});

function Input({
  className,
  type,
  variant = "glass",
  glass,
  style,
  ...props
}: React.ComponentProps<"input"> &
  VariantProps<typeof inputVariants> & {
    glass?: GlassCustomization;
  }) {
  const hasCustomGlass = glass !== undefined;
  const effectiveVariant = hasCustomGlass && variant !== "default" ? "glass" : variant;

  const glassStyles = variant !== "default" ? getGlassStyles(glass) : {};

  return (
    <input
      type={type}
      data-slot="input"
      className={cn(
        "file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground flex h-9 w-full min-w-0 rounded-md px-3 py-1 text-base transition-[color,box-shadow] outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
        inputVariants({
          variant: effectiveVariant,
        }),
        "focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
        "aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
        className,
      )}
      style={{
        ...glassStyles,
        ...style,
      }}
      {...props}
    />
  );
}

export { Input };
