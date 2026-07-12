"use client";

import { cn } from "@/lib/utils";
import { Calendar as BaseCalendar, type CalendarProps as BaseCalendarProps } from "../calendar";

export type CalendarProps = BaseCalendarProps & {
  glow?: boolean;
};

/**
 * Sistine Calendar - Enhanced calendar with glassy effects
 */
export function Calendar({ className, variant = "glass", glow = false, ...props }: CalendarProps) {
  return (
    <div className={cn(glow && "glass-glow", className)}>
      <BaseCalendar variant={variant} {...props} />
    </div>
  );
}
