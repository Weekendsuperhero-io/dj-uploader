"use client";

import { CalendarIcon } from "@phosphor-icons/react";
import { format } from "date-fns";
import * as React from "react";
import { cn } from "@/lib/utils";
import { Button } from "./button";
import { Calendar } from "./calendar";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";

export interface DatePickerInputProps extends Omit<React.ComponentProps<typeof Button>, "value" | "onChange" | "variant"> {
  value?: Date;
  onChange?: (date: Date | undefined) => void;
  placeholder?: string;
  variant?: "default" | "glass" | "frosted" | "crystal" | "opaque";
}

const DatePickerInput = React.forwardRef<HTMLButtonElement, DatePickerInputProps>(
  ({ className, value, onChange, placeholder = "Pick a date", variant = "glass", ...props }, ref) => {
    const [open, setOpen] = React.useState(false);

    return (
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            ref={ref}
            variant={variant}
            className={cn(
              "w-full min-w-0 justify-start overflow-hidden text-left font-normal",
              !value && "text-muted-foreground",
              className,
            )}
            {...props}
          >
            <CalendarIcon className="mr-2 h-4 w-4 shrink-0" />
            <span className="min-w-0 truncate">
              {value ? format(value, "MMM d, yyyy") : placeholder}
            </span>
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-auto p-0" variant={variant}>
          <Calendar
            mode="single"
            selected={value}
            onSelect={(date) => {
              onChange?.(date);
              setOpen(false);
            }}
            autoFocus
          />
        </PopoverContent>
      </Popover>
    );
  },
);
DatePickerInput.displayName = "DatePickerInput";

export { DatePickerInput };
