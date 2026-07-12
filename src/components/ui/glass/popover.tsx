"use client";

import * as React from "react";
import { cn } from "@/lib/utils";
import { Popover as BasePopover, PopoverContent as BasePopoverContent, PopoverTrigger } from "../popover";

export interface PopoverContentProps extends React.ComponentProps<typeof BasePopoverContent> {
  glow?: boolean;
}

/**
 * Sistine Popover - Enhanced popover with glassy effects
 */
export const PopoverContent = React.forwardRef<React.ElementRef<typeof BasePopoverContent>, PopoverContentProps>(
  ({ className, variant = "glass", glow = false, ...props }, ref) => {
    return <BasePopoverContent ref={ref} variant={variant} className={cn(glow && "glass-glow", className)} {...props} />;
  },
);
PopoverContent.displayName = "PopoverContent";

export { BasePopover as Popover, PopoverTrigger };
