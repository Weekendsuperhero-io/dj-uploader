"use client";

import * as React from "react";
import type { GlassCustomization } from "@/lib/glass-utils";
import { type HoverEffect, hoverEffects } from "@/lib/hover-effects";
import { cn } from "@/lib/utils";
import { DatePickerInput as BaseDatePickerInput } from "../date-picker-input";

export interface DatePickerInputProps extends React.ComponentProps<typeof BaseDatePickerInput> {
  effect?: HoverEffect;
  glass?: GlassCustomization;
}

/**
 * Sistine Date Picker Input - A beautifully designed date picker with glassy effects
 * Built on top of the base DatePickerInput component with enhanced visual styling
 */
export const DatePickerInput = React.forwardRef<HTMLButtonElement, DatePickerInputProps>(
  ({ className, variant = "glass", effect = "none", glass, ...props }, ref) => {
    return (
      <BaseDatePickerInput
        ref={ref}
        variant={variant}
        glass={glass}
        className={cn(
          hoverEffects({
            hover: effect,
          }),
          className,
        )}
        {...props}
      />
    );
  },
);
DatePickerInput.displayName = "DatePickerInput";
