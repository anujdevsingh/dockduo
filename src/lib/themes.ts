/**
 * Theme system — 4 palettes as CSS variable bundles, applied to :root
 * live when the user picks a theme from the tray menu.
 *
 * Bubble colors and picker-pill styling throughout the app reference
 * `var(--bubble-bg)` etc., so a single call to `applyTheme(id)` recolors
 * everything on the next frame without reload.
 */

export type ThemeId = "midnight" | "daylight" | "pastel" | "retro";

export interface ThemeVars {
  bubbleBg: string;
  bubbleBorder: string;
  bubbleText: string;
  bubbleCompletionText: string;
  bubbleCompletionBorder: string;
  bubbleErrorBg: string;
  bubbleErrorBorder: string;
  bubbleErrorText: string;
  pickerBg: string;
  pickerBorder: string;
  pickerText: string;
  shadow: string;
}

/** Dark glassy (default) — matches the current hand-rolled styling. */
const midnight: ThemeVars = {
  bubbleBg: "rgba(24,24,28,0.92)",
  bubbleBorder: "rgba(255,255,255,0.25)",
  bubbleText: "#FFFFFF",
  bubbleCompletionText: "#7CFFB2",
  bubbleCompletionBorder: "#3AA76A",
  bubbleErrorBg: "rgba(60,10,10,0.95)",
  bubbleErrorBorder: "#D9534F",
  bubbleErrorText: "#FFB3B3",
  pickerBg: "#111418",
  pickerBorder: "rgba(255,255,255,0.55)",
  pickerText: "#FFFFFF",
  shadow: "0 4px 14px rgba(0,0,0,0.55)",
};

/** Bright, paper-like — for users who keep the desktop in light mode. */
const daylight: ThemeVars = {
  bubbleBg: "rgba(255,255,255,0.96)",
  bubbleBorder: "rgba(0,0,0,0.18)",
  bubbleText: "#111418",
  bubbleCompletionText: "#0F7A3D",
  bubbleCompletionBorder: "#0F7A3D",
  bubbleErrorBg: "rgba(253,235,235,0.98)",
  bubbleErrorBorder: "#C7382F",
  bubbleErrorText: "#A41E16",
  pickerBg: "#FFFFFF",
  pickerBorder: "rgba(0,0,0,0.35)",
  pickerText: "#111418",
  shadow: "0 4px 14px rgba(0,0,0,0.18)",
};

/** Soft pastel — playful, matches the cartoon sprites. */
const pastel: ThemeVars = {
  bubbleBg: "rgba(255,245,230,0.96)",
  bubbleBorder: "#E4A5B9",
  bubbleText: "#5A3A4B",
  bubbleCompletionText: "#2E7F5E",
  bubbleCompletionBorder: "#6DC7A4",
  bubbleErrorBg: "rgba(255,226,226,0.98)",
  bubbleErrorBorder: "#E38B86",
  bubbleErrorText: "#8A2F2B",
  pickerBg: "#FFEFDC",
  pickerBorder: "#D39AB0",
  pickerText: "#5A3A4B",
  shadow: "0 4px 12px rgba(150,100,120,0.3)",
};

/** Retro CRT green — fun dev aesthetic. */
const retro: ThemeVars = {
  bubbleBg: "rgba(8,20,12,0.95)",
  bubbleBorder: "#3DF57A",
  bubbleText: "#B6FFD0",
  bubbleCompletionText: "#E9FFB0",
  bubbleCompletionBorder: "#E9FFB0",
  bubbleErrorBg: "rgba(40,8,8,0.96)",
  bubbleErrorBorder: "#FF6B6B",
  bubbleErrorText: "#FFC4C4",
  pickerBg: "#0C1A10",
  pickerBorder: "#3DF57A",
  pickerText: "#B6FFD0",
  shadow: "0 0 14px rgba(61,245,122,0.35)",
};

export const THEMES: Record<ThemeId, ThemeVars> = {
  midnight,
  daylight,
  pastel,
  retro,
};

/** Write the theme's CSS vars onto `:root`. Next paint picks them up. */
export function applyTheme(id: ThemeId) {
  const vars = THEMES[id] ?? THEMES.midnight;
  const r = document.documentElement.style;
  r.setProperty("--bubble-bg", vars.bubbleBg);
  r.setProperty("--bubble-border", vars.bubbleBorder);
  r.setProperty("--bubble-text", vars.bubbleText);
  r.setProperty("--bubble-completion-text", vars.bubbleCompletionText);
  r.setProperty("--bubble-completion-border", vars.bubbleCompletionBorder);
  r.setProperty("--bubble-error-bg", vars.bubbleErrorBg);
  r.setProperty("--bubble-error-border", vars.bubbleErrorBorder);
  r.setProperty("--bubble-error-text", vars.bubbleErrorText);
  r.setProperty("--picker-bg", vars.pickerBg);
  r.setProperty("--picker-border", vars.pickerBorder);
  r.setProperty("--picker-text", vars.pickerText);
  r.setProperty("--shadow", vars.shadow);
}
