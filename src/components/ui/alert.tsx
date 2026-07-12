import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";

import { cn } from "@/lib/utils";

const alertVariants = cva(
  "relative w-full rounded-lg border px-4 py-3 text-sm [&>svg+div]:translate-y-[-3px] [&>svg]:absolute [&>svg]:left-4 [&>svg]:top-4 [&>svg]:text-foreground [&>svg~*]:pl-7",
  {
    variants: {
      variant: {
        default: "bg-background text-foreground",
        glass: "glass-surface text-foreground",
        frosted: "glass-frosted text-foreground",
        crystal: "glass-crystal text-foreground",
        opaque: "glass-opaque text-foreground",
        surface: "glass-surface text-foreground",
        solid: "glass-solid text-foreground",
        info: "glass-bg backdrop-blur-[var(--blur)] border border-blue-500/60 text-blue-600 shadow-[var(--glass-shadow)] dark:text-blue-400 [&>svg]:text-blue-600 dark:[&>svg]:text-blue-400",
        success:
          "glass-bg backdrop-blur-[var(--blur)] border border-green-500/60 text-green-600 shadow-[var(--glass-shadow)] dark:text-green-400 [&>svg]:text-green-600 dark:[&>svg]:text-green-400",
        warning:
          "glass-bg backdrop-blur-[var(--blur)] border border-yellow-500/60 text-yellow-600 shadow-[var(--glass-shadow)] dark:text-yellow-400 [&>svg]:text-yellow-600 dark:[&>svg]:text-yellow-400",
        destructive:
          "glass-bg backdrop-blur-[var(--blur)] border border-destructive/60 text-destructive shadow-[var(--glass-shadow)] [&>svg]:text-destructive",
      },
    },
    defaultVariants: {
      variant: "glass",
    },
  },
);

const Alert = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement> & VariantProps<typeof alertVariants>>(
  ({ className, variant, ...props }, ref) => (
    <div
      ref={ref}
      data-slot="alert"
      role="alert"
      data-glass-tint={
        variant &&
        [
          "info",
          "success",
          "warning",
          "destructive",
        ].includes(variant)
          ? variant
          : undefined
      }
      className={cn(
        alertVariants({
          variant,
        }),
        className,
      )}
      {...props}
    />
  ),
);
Alert.displayName = "Alert";

const AlertTitle = React.forwardRef<HTMLParagraphElement, React.HTMLAttributes<HTMLHeadingElement>>(({ className, ...props }, ref) => (
  <h5 ref={ref} data-slot="alert-title" className={cn("mb-1 font-medium leading-none tracking-tight", className)} {...props} />
));
AlertTitle.displayName = "AlertTitle";

const AlertDescription = React.forwardRef<HTMLParagraphElement, React.HTMLAttributes<HTMLParagraphElement>>(({ className, ...props }, ref) => (
  <div ref={ref} data-slot="alert-description" className={cn("text-sm [&_p]:leading-relaxed", className)} {...props} />
));
AlertDescription.displayName = "AlertDescription";

export { Alert, AlertDescription, AlertTitle };
