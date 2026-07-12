export interface GlassCustomization {
  /**
   * Background color for the glass effect (e.g., "oklch(100% 0 0 / 0.1)")
   * Default: uses CSS variable --glass-bg
   */
  color?: string;

  /**
   * Transparency/opacity for the background (0-1)
   * If provided, will override the alpha channel in color
   */
  transparency?: number;

  /**
   * Overall glass solidity (0–1). 0 = the variant's natural sheer glass (default), 1 = fully solid. Sets the
   * `--glass-opacity` floor the glass utilities read — keeps the tint / gradient / blur, just makes the
   * surface less see-through. (Unlike `transparency`, which replaces the whole background with flat white.)
   */
  opacity?: number;

  /**
   * Blur amount in pixels
   * Default: uses CSS variable --blur (20px)
   */
  blur?: number | string;

  /**
   * Border/outline color (e.g., "oklch(100% 0 0 / 0.25)")
   * Default: uses CSS variable --glass-border
   */
  outline?: string;

  /**
   * Border/outline width in pixels
   * Default: 1px
   */
  outlineWidth?: number | string;

  /**
   * Shadow for the glass effect
   * Default: uses CSS variable --glass-shadow
   */
  shadow?: string;

  /**
   * Inner glow color and intensity (e.g., "oklch(100% 0 0 / 0.2)")
   * Creates an inset shadow for a glowing effect inside the element
   */
  innerGlow?: string;

  /**
   * Inner glow blur/spread radius in pixels
   * Default: 20px
   */
  innerGlowBlur?: number | string;
}

/**
 * Set or replace the alpha channel on any CSS color string.
 * Handles oklch(), hex, and rgb/rgba formats — always outputs oklch.
 */
function setAlpha(color: string, alpha: number): string {
  // oklch(...) — replace or append alpha
  const oklchMatch = color.match(/^oklch\(([^/)]+?)(?:\s*\/\s*[\d.]+)?\)$/);
  if (oklchMatch) {
    return `oklch(${oklchMatch[1].trim()} / ${alpha})`;
  }

  // hex — convert to oklch with alpha
  if (color.startsWith("#")) {
    const hex = color.replace("#", "");
    const r = parseInt(hex.substring(0, 2), 16) / 255;
    const g = parseInt(hex.substring(2, 4), 16) / 255;
    const b = parseInt(hex.substring(4, 6), 16) / 255;
    // Approximate lightness for simple hex fallback
    const l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    return `oklch(${(l * 100).toFixed(1)}% 0 0 / ${alpha})`;
  }

  // rgb/rgba — convert to oklch with alpha
  const rgbaMatch = color.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
  if (rgbaMatch) {
    const l = (0.2126 * +rgbaMatch[1] + 0.7152 * +rgbaMatch[2] + 0.0722 * +rgbaMatch[3]) / 255;
    return `oklch(${(l * 100).toFixed(1)}% 0 0 / ${alpha})`;
  }

  // Unknown format — wrap with color-mix as fallback
  return `color-mix(in oklch, ${color} ${Math.round(alpha * 100)}%, transparent)`;
}

/**
 * Converts glass customization props to CSS style object
 */
export function getGlassStyles(customization?: GlassCustomization): React.CSSProperties {
  if (!customization) return {};

  const styles = {} as React.CSSProperties & Record<`--${string}`, string | number>;

  // Handle background color and transparency
  if (customization.color || customization.transparency !== undefined) {
    let bgColor = customization.color || "oklch(100% 0 0 / 0.1)";

    if (customization.transparency !== undefined) {
      bgColor = setAlpha(bgColor, customization.transparency);
    }

    styles.backgroundColor = bgColor;
  }

  // Overall solidity — sets the --glass-opacity floor the glass utilities lay behind the surface (0 = the
  // variant's natural sheer glass, 1 = fully solid). Keeps the tint / gradient / blur intact.
  if (customization.opacity !== undefined) {
    styles["--glass-opacity"] = customization.opacity;
  }

  // Handle blur
  if (customization.blur !== undefined) {
    const blurValue = typeof customization.blur === "number" ? `${customization.blur}px` : customization.blur;
    styles.backdropFilter = `blur(${blurValue})`;
    styles.WebkitBackdropFilter = `blur(${blurValue})`; // Safari support
  }

  // Handle outline/border - always apply border for glassmorphism effect
  if (customization.outline !== undefined) {
    const width = customization.outlineWidth || "1px";
    styles.borderColor = customization.outline;
    styles.borderWidth = typeof width === "number" ? `${width}px` : width;
    styles.borderStyle = "solid";
  } else if (!customization.outline && (customization.color || customization.transparency !== undefined || customization.blur !== undefined)) {
    // Apply default border if glass customization is provided but outline is not
    styles.borderColor = "oklch(100% 0 0 / 0.3)";
    styles.borderWidth = "1px";
    styles.borderStyle = "solid";
  }

  // Handle shadow and inner glow - combine both if provided
  const shadows: string[] = [];

  // Add outer shadow
  if (customization.shadow !== undefined) {
    shadows.push(customization.shadow);
  } else if (customization.color || customization.transparency !== undefined || customization.blur !== undefined) {
    // Apply default glass shadow for depth
    shadows.push("0 8px 32px oklch(0% 0 0 / 0.1), 0 2px 8px oklch(0% 0 0 / 0.05)");
  }

  // Add inner glow as inset shadow
  if (customization.innerGlow !== undefined) {
    const glowBlur =
      customization.innerGlowBlur !== undefined
        ? typeof customization.innerGlowBlur === "number"
          ? `${customization.innerGlowBlur}px`
          : customization.innerGlowBlur
        : "20px";
    shadows.push(`inset 0 0 ${glowBlur} ${customization.innerGlow}`);
  }

  if (shadows.length > 0) {
    styles.boxShadow = shadows.join(", ");
  }

  return styles;
}

/**
 * Generates CSS custom properties for glass customization
 * Useful for components that need to pass styles to child elements
 */
export function getGlassCSSVars(customization?: GlassCustomization): Record<string, string> {
  if (!customization) return {};

  const vars: Record<string, string> = {};

  if (customization.color || customization.transparency !== undefined) {
    let bgColor = customization.color || "oklch(100% 0 0 / 0.1)";

    if (customization.transparency !== undefined) {
      bgColor = setAlpha(bgColor, customization.transparency);
    }

    vars["--glass-bg-custom"] = bgColor;
  }

  if (customization.opacity !== undefined) {
    vars["--glass-opacity"] = String(customization.opacity);
  }

  if (customization.blur !== undefined) {
    const blurValue = typeof customization.blur === "number" ? `${customization.blur}px` : customization.blur;
    vars["--blur-custom"] = blurValue;
  }

  if (customization.outline !== undefined) {
    vars["--glass-border-custom"] = customization.outline;
  }

  if (customization.outlineWidth !== undefined) {
    const width = typeof customization.outlineWidth === "number" ? `${customization.outlineWidth}px` : customization.outlineWidth;
    vars["--glass-border-width-custom"] = width;
  }

  if (customization.shadow !== undefined) {
    vars["--glass-shadow-custom"] = customization.shadow;
  }

  if (customization.innerGlow !== undefined) {
    vars["--glass-inner-glow-custom"] = customization.innerGlow;
  }

  if (customization.innerGlowBlur !== undefined) {
    const blurValue = typeof customization.innerGlowBlur === "number" ? `${customization.innerGlowBlur}px` : customization.innerGlowBlur;
    vars["--glass-inner-glow-blur-custom"] = blurValue;
  }

  return vars;
}
