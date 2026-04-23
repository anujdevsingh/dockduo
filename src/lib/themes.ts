import type { ITheme } from "@xterm/xterm";

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
  /** Embedded terminal window: outer padding area */
  terminalChromeBg: string;
  /** Rounded card behind header + xterm */
  terminalSurfaceBg: string;
  terminalSurfaceBorder: string;
  terminalHeaderBg: string;
  terminalTitleText: string;
  terminalTitleMuted: string;
  terminalCardShadow: string;
}

/** Soft accent for Bruce / Jazz chrome (pill strip); not full-window borders. */
export const TERMINAL_CHARACTER_ACCENT: Record<"bruce" | "jazz", string> = {
  bruce: "#d4a06a",
  jazz: "#5eb8a8",
};

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
  terminalChromeBg: "#0c0d10",
  terminalSurfaceBg: "#13141a",
  terminalSurfaceBorder: "rgba(255,255,255,0.07)",
  terminalHeaderBg: "rgba(255,255,255,0.03)",
  terminalTitleText: "#c8cad0",
  terminalTitleMuted: "#6b6f78",
  terminalCardShadow: "0 8px 32px rgba(0,0,0,0.45)",
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
  terminalChromeBg: "#e8e6e1",
  terminalSurfaceBg: "#fdfcfa",
  terminalSurfaceBorder: "rgba(0,0,0,0.06)",
  terminalHeaderBg: "rgba(0,0,0,0.03)",
  terminalTitleText: "#2c2e33",
  terminalTitleMuted: "#8a8f98",
  terminalCardShadow: "0 8px 28px rgba(0,0,0,0.08)",
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
  terminalChromeBg: "#f5ebe2",
  terminalSurfaceBg: "#fffbfa",
  terminalSurfaceBorder: "rgba(90,58,75,0.12)",
  terminalHeaderBg: "rgba(212,165,185,0.12)",
  terminalTitleText: "#4a3545",
  terminalTitleMuted: "#9a7d88",
  terminalCardShadow: "0 8px 26px rgba(120,80,100,0.15)",
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
  terminalChromeBg: "#060a08",
  terminalSurfaceBg: "#0c1510",
  terminalSurfaceBorder: "rgba(61,245,122,0.18)",
  terminalHeaderBg: "rgba(61,245,122,0.06)",
  terminalTitleText: "#a8e8c0",
  terminalTitleMuted: "rgba(182,255,208,0.45)",
  terminalCardShadow: "0 0 24px rgba(61,245,122,0.12)",
};

export const THEMES: Record<ThemeId, ThemeVars> = {
  midnight,
  daylight,
  pastel,
  retro,
};

/** Write the theme's CSS vars onto `:root`. Next paint picks them up. */
/** xterm.js palette aligned with each DockDuo theme. */
export function xtermThemeFor(id: ThemeId): ITheme {
  switch (id) {
    case "daylight":
      return {
        background: "#faf9f7",
        foreground: "#1a1a1c",
        cursor: "#1a1a1c",
        cursorAccent: "#faf9f7",
        selectionBackground: "rgba(0,0,0,0.12)",
        black: "#1a1a1c",
        red: "#c7382f",
        green: "#0f7a3d",
        yellow: "#8a6a00",
        blue: "#1e63d6",
        magenta: "#8b3db8",
        cyan: "#007a8a",
        white: "#dde0e5",
        brightBlack: "#6b6f76",
        brightRed: "#e45c54",
        brightGreen: "#2ea05a",
        brightYellow: "#c9a227",
        brightBlue: "#4a8ef0",
        brightMagenta: "#b56ede",
        brightCyan: "#2aacc0",
        brightWhite: "#ffffff",
      };
    case "pastel":
      return {
        background: "#fff7ee",
        foreground: "#4a3545",
        cursor: "#4a3545",
        cursorAccent: "#fff7ee",
        selectionBackground: "rgba(180,120,140,0.25)",
        black: "#3a2a38",
        red: "#c45a5a",
        green: "#2e7f5e",
        yellow: "#c9a052",
        blue: "#5a7bc8",
        magenta: "#b565a8",
        cyan: "#4a9e9e",
        white: "#ead9d0",
        brightBlack: "#887080",
        brightRed: "#e07878",
        brightGreen: "#42b082",
        brightYellow: "#ddc060",
        brightBlue: "#7a9eee",
        brightMagenta: "#d090cc",
        brightCyan: "#6ccccc",
        brightWhite: "#ffffff",
      };
    case "retro":
      return {
        background: "#0a120e",
        foreground: "#b6ffd0",
        cursor: "#3df57a",
        cursorAccent: "#0a120e",
        selectionBackground: "rgba(61,245,122,0.25)",
        black: "#041008",
        red: "#ff6b6b",
        green: "#3df57a",
        yellow: "#e9ffb0",
        blue: "#6ecfff",
        magenta: "#ff8cff",
        cyan: "#5dffc8",
        white: "#d0ffe4",
        brightBlack: "#2a5040",
        brightRed: "#ff9494",
        brightGreen: "#7affa8",
        brightYellow: "#f5ffbe",
        brightBlue: "#9eddff",
        brightMagenta: "#ffc4ff",
        brightCyan: "#8fffec",
        brightWhite: "#ffffff",
      };
    case "midnight":
    default:
      return {
        background: "#141416",
        foreground: "#e8e8ec",
        cursor: "#e8e8ec",
        cursorAccent: "#141416",
        selectionBackground: "rgba(255,255,255,0.14)",
        black: "#000000",
        red: "#e06c75",
        green: "#98c379",
        yellow: "#e5c07b",
        blue: "#61afef",
        magenta: "#c678dd",
        cyan: "#56b6c2",
        white: "#abb2bf",
        brightBlack: "#5c6370",
        brightRed: "#f5949a",
        brightGreen: "#b5d49f",
        brightYellow: "#f0d48a",
        brightBlue: "#8fc7fa",
        brightMagenta: "#deb3f0",
        brightCyan: "#8fd4dc",
        brightWhite: "#ffffff",
      };
  }
}

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
  r.setProperty("--terminal-chrome-bg", vars.terminalChromeBg);
  r.setProperty("--terminal-surface-bg", vars.terminalSurfaceBg);
  r.setProperty("--terminal-surface-border", vars.terminalSurfaceBorder);
  r.setProperty("--terminal-header-bg", vars.terminalHeaderBg);
  r.setProperty("--terminal-title-text", vars.terminalTitleText);
  r.setProperty("--terminal-title-muted", vars.terminalTitleMuted);
  r.setProperty("--terminal-card-shadow", vars.terminalCardShadow);
}
