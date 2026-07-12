import { cva, type VariantProps } from "class-variance-authority";
import type * as React from "react";
import { type GlassCustomization, getGlassStyles } from "@/lib/glass-utils";
import { cn } from "@/lib/utils";

const cardVariants = cva("flex flex-col gap-6 rounded-xl py-6", {
  variants: {
    variant: {
      default: "bg-card text-card-foreground border shadow-sm",
      glass: "glass-bg text-foreground",
      frosted: "glass-frosted text-foreground",
      crystal: "glass-crystal text-foreground",
      opaque: "glass-opaque text-foreground",
      surface: "glass-surface text-foreground",
      solid: "glass-solid text-foreground",
    },
  },
  defaultVariants: {
    variant: "glass",
  },
});

function Card({
  className,
  variant = "glass",
  glass,
  style,
  ...props
}: React.ComponentProps<"div"> &
  VariantProps<typeof cardVariants> & {
    glass?: GlassCustomization;
  }) {
  // Custom glass props apply on top of the base glass-bg surface (so getGlassStyles can override),
  // which is exactly the "glass" variant; "default" opts out of glass entirely.
  const hasCustomGlass = glass !== undefined;
  const effectiveVariant = hasCustomGlass && variant !== "default" ? "glass" : variant;
  const glassStyles = variant !== "default" ? getGlassStyles(glass) : {};

  return (
    <div
      data-slot="card"
      className={cn(
        cardVariants({
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

function CardHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-header"
      className={cn(
        "@container/card-header grid auto-rows-min grid-rows-[auto_auto] items-start gap-1.5 px-6 has-data-[slot=card-action]:grid-cols-[1fr_auto] [.border-b]:pb-6",
        className,
      )}
      {...props}
    />
  );
}

function CardTitle({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="card-title" className={cn("leading-none font-semibold text-balance", className)} {...props} />;
}

function CardDescription({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="card-description" className={cn("text-muted-foreground text-sm text-pretty", className)} {...props} />;
}

function CardAction({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="card-action" className={cn("col-start-2 row-span-2 row-start-1 self-start justify-self-end", className)} {...props} />;
}

function CardContent({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="card-content" className={cn("px-6", className)} {...props} />;
}

function CardFooter({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="card-footer" className={cn("flex items-center px-6 [.border-t]:pt-6", className)} {...props} />;
}

export { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle };
