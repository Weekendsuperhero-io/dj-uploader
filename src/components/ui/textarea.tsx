import { cva, type VariantProps } from "class-variance-authority";
import type * as React from "react";
import { type GlassCustomization, getGlassStyles } from "@/lib/glass-utils";
import { cn } from "@/lib/utils";

const textareaVariants = cva("", {
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

function Textarea({
  className,
  variant = "glass",
  glass,
  style,
  ...props
}: React.ComponentProps<"textarea"> &
  VariantProps<typeof textareaVariants> & {
    glass?: GlassCustomization;
  }) {
  const hasCustomGlass = glass !== undefined;
  const effectiveVariant = hasCustomGlass && variant !== "default" ? "glass" : variant;

  const glassStyles = variant !== "default" ? getGlassStyles(glass) : {};

  return (
    <textarea
      data-slot="textarea"
      className={cn(
        "placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive flex field-sizing-content min-h-16 w-full rounded-md px-3 py-2 text-base transition-[color,box-shadow] outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
        textareaVariants({
          variant: effectiveVariant,
        }),
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

export { Textarea };
