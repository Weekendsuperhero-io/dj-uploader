"use client";

import { CheckCircleIcon, CircleNotchIcon, InfoIcon, WarningIcon, WarningOctagonIcon } from "@phosphor-icons/react";
import { Toaster as Sonner } from "sonner";

type ToasterProps = React.ComponentProps<typeof Sonner>;

/**
 * Sistine Toaster - Toast notifications with glassmorphism effects
 *
 * Features:
 * - Glass background with backdrop blur
 * - Glass border and shadow effects
 * - Smooth animations and transitions
 * - Light/dark mode support
 *
 * @example
 * ```tsx
 * import { Toaster } from "@/components/ui/sonner"
 * import { toast } from "sonner"
 *
 * // Add to root layout
 * <Toaster />
 *
 * // Use in components
 * toast.success('Success!', { description: 'Your action completed' })
 * ```
 */
const Toaster = ({ theme = "system", ...props }: ToasterProps) => {
  return (
    <Sonner
      theme={theme as ToasterProps["theme"]}
      className="toaster group"
      icons={{
        success: <CheckCircleIcon className="size-4" />,
        info: <InfoIcon className="size-4" />,
        warning: <WarningIcon className="size-4" />,
        error: <WarningOctagonIcon className="size-4" />,
        loading: <CircleNotchIcon className="size-4 animate-spin" />,
      }}
      toastOptions={{
        classNames: {
          toast: "group toast group-[.toaster]:rounded-xl group-[.toaster]:glass-surface group-[.toaster]:text-foreground",
          description: "group-[.toast]:text-muted-foreground",
          actionButton: "group-[.toast]:rounded-md group-[.toast]:glass-surface group-[.toast]:text-foreground group-[.toast]:hover:opacity-90",
          cancelButton: "group-[.toast]:rounded-md group-[.toast]:glass-surface group-[.toast]:text-muted-foreground group-[.toast]:hover:opacity-90",
          success:
            "group-[.toast]:border-green-500/30 [&_[data-title]]:text-green-600 dark:[&_[data-title]]:text-green-400 [&_[data-description]]:text-green-600/80! dark:[&_[data-description]]:text-green-400/80! [&_[data-icon]]:text-green-600! dark:[&_[data-icon]]:text-green-400!",
          error:
            "group-[.toast]:border-red-500/30 [&_[data-title]]:text-red-600 dark:[&_[data-title]]:text-red-400 [&_[data-description]]:text-red-600/80! dark:[&_[data-description]]:text-red-400/80! [&_[data-icon]]:text-red-600! dark:[&_[data-icon]]:text-red-400!",
          warning:
            "group-[.toast]:border-yellow-500/30 [&_[data-title]]:text-yellow-600 dark:[&_[data-title]]:text-yellow-400 [&_[data-description]]:text-yellow-600/80! dark:[&_[data-description]]:text-yellow-400/80! [&_[data-icon]]:text-yellow-600! dark:[&_[data-icon]]:text-yellow-400!",
          info: "group-[.toast]:border-blue-500/30 [&_[data-title]]:text-blue-600 dark:[&_[data-title]]:text-blue-400 [&_[data-description]]:text-blue-600/80! dark:[&_[data-description]]:text-blue-400/80! [&_[data-icon]]:text-blue-600! dark:[&_[data-icon]]:text-blue-400!",
        },
      }}
      {...props}
    />
  );
};

export { Toaster };
