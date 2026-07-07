import type { RetroFocusThemeDefinition } from "./themes";

export type RetroFocusThemeCustomization = {
  fontFamily?: string;
  fontSizePx?: number;
  fontColor?: string;
};

export type RetroFocusResolvedEditorStyle = {
  fontFamily: string;
  fontSize: string;
  textColor: string;
  promptColor: string;
  gutterColor: string;
  gutterActiveColor: string;
  placeholderColor: string;
};

const RETRO_FOCUS_MIN_FONT_SIZE_PX = 12;
const RETRO_FOCUS_MAX_FONT_SIZE_PX = 36;

function toHexColor(value: unknown): string | undefined {
  const raw = String(value ?? "").trim();
  if (!raw) {
    return undefined;
  }

  const withHash = raw.startsWith("#") ? raw : `#${raw}`;
  return /^#[0-9a-fA-F]{6}$/.test(withHash) ? withHash.toLowerCase() : undefined;
}

function hexToRgb(value: string): { r: number; g: number; b: number } | null {
  const normalized = toHexColor(value);
  if (!normalized) {
    return null;
  }

  return {
    r: Number.parseInt(normalized.slice(1, 3), 16),
    g: Number.parseInt(normalized.slice(3, 5), 16),
    b: Number.parseInt(normalized.slice(5, 7), 16),
  };
}

function colorWithAlpha(value: string, alpha: number): string | undefined {
  const rgb = hexToRgb(value);
  if (!rgb) {
    return undefined;
  }

  return `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${Math.max(0, Math.min(1, alpha))})`;
}

function normalizeFontFamily(value: unknown): string | undefined {
  const candidate = String(value ?? "").trim();
  return candidate ? candidate : undefined;
}

function normalizeFontSizePx(value: unknown): number | undefined {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return undefined;
  }

  return Math.max(
    RETRO_FOCUS_MIN_FONT_SIZE_PX,
    Math.min(RETRO_FOCUS_MAX_FONT_SIZE_PX, Math.round(parsed)),
  );
}

function normalizeRetroFocusThemeCustomization(
  value: RetroFocusThemeCustomization | undefined,
): RetroFocusThemeCustomization {
  if (!value) {
    return {};
  }

  return {
    fontFamily: normalizeFontFamily(value.fontFamily),
    fontSizePx: normalizeFontSizePx(value.fontSizePx),
    fontColor: toHexColor(value.fontColor),
  };
}

export function resolveRetroFocusEditorStyle(
  theme: RetroFocusThemeDefinition,
  customization?: RetroFocusThemeCustomization,
): RetroFocusResolvedEditorStyle {
  const normalizedCustomization = normalizeRetroFocusThemeCustomization(customization);
  const gutter = theme.editorLayout.gutter;
  const fontColor = normalizedCustomization.fontColor;
  const dimmedFontColor = fontColor
    ? colorWithAlpha(fontColor, 0.58) ?? theme.chrome.dimTextColor
    : undefined;

  return {
    fontFamily: normalizedCustomization.fontFamily ?? theme.editorLayout.fontFamily,
    fontSize:
      normalizedCustomization.fontSizePx !== undefined
        ? `${normalizedCustomization.fontSizePx}px`
        : theme.editorLayout.fontSize,
    textColor: fontColor ?? theme.chrome.accentColor,
    promptColor: fontColor ?? theme.editorLayout.promptColor ?? theme.chrome.accentColor,
    gutterColor: fontColor
      ? dimmedFontColor ?? theme.chrome.dimTextColor
      : gutter?.color ?? theme.chrome.dimTextColor,
    gutterActiveColor: fontColor
      ? fontColor
      : gutter?.activeColor ?? theme.chrome.accentColor,
    placeholderColor: fontColor
      ? dimmedFontColor ?? theme.chrome.dimTextColor
      : theme.chrome.dimTextColor,
  };
}
