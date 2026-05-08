import type { DashboardTheme, ThemeTypography, ThemeLayout } from "./types";

/**
 * Built-in dashboard themes.
 *
 * Each theme defines its own palette, typography, and layout so switching
 * themes produces visible changes beyond just color — fonts, density, and
 * corner-radius all shift to match the theme's personality.
 *
 * Theme names must stay in sync with the backend's
 * `_BUILTIN_DASHBOARD_THEMES` list in `hermes_cli/web_server.py`.
 */

// ---------------------------------------------------------------------------
// Shared typography / layout presets
// ---------------------------------------------------------------------------

/** Default system stack — neutral, safe fallback for every platform. */
const SYSTEM_SANS =
  'system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif';
const SYSTEM_MONO =
  'ui-monospace, "SF Mono", "Cascadia Mono", Menlo, Consolas, monospace';

const DEFAULT_TYPOGRAPHY: ThemeTypography = {
  fontSans: SYSTEM_SANS,
  fontMono: SYSTEM_MONO,
  baseSize: "15px",
  lineHeight: "1.55",
  letterSpacing: "0",
};

const DEFAULT_LAYOUT: ThemeLayout = {
  radius: "0.5rem",
  density: "comfortable",
};

// ---------------------------------------------------------------------------
// Themes
// ---------------------------------------------------------------------------

export const defaultTheme: DashboardTheme = {
  name: "default",
  label: "Workbench",
  description: "Quiet Paperclip-inspired control panel — dense, stable, readable",
  palette: {
    background: { hex: "#101114", alpha: 1 },
    midground: { hex: "#f4f4f5", alpha: 1 },
    foreground: { hex: "#ffffff", alpha: 0 },
    warmGlow: "rgba(245, 158, 11, 0.12)",
    noiseOpacity: 0.18,
  },
  typography: {
    ...DEFAULT_TYPOGRAPHY,
    fontSans:
      '"Aptos", "Segoe UI Variable", "Segoe UI", "Helvetica Neue", sans-serif',
    fontMono:
      '"Cascadia Code", "JetBrains Mono", "SF Mono", Consolas, monospace',
    baseSize: "14px",
    lineHeight: "1.5",
  },
  layout: {
    radius: "0.375rem",
    density: "compact",
  },
  colorOverrides: {
    card: "#17181c",
    cardForeground: "#f4f4f5",
    popover: "#18191e",
    popoverForeground: "#f4f4f5",
    primary: "#f59e0b",
    primaryForeground: "#111111",
    secondary: "#22242a",
    secondaryForeground: "#e4e4e7",
    muted: "#202126",
    mutedForeground: "#a1a1aa",
    accent: "#272a31",
    accentForeground: "#f4f4f5",
    success: "#22c55e",
    warning: "#f59e0b",
    destructive: "#ef4444",
    border: "rgba(244,244,245,0.12)",
    input: "rgba(244,244,245,0.16)",
    ring: "#f59e0b",
  },
  componentStyles: {
    backdrop: {
      fillerOpacity: "0",
      fillerBlendMode: "normal",
    },
    sidebar: {
      background:
        "linear-gradient(180deg, rgba(16,17,20,0.98) 0%, rgba(13,14,17,0.98) 100%)",
    },
    card: {
      background:
        "linear-gradient(180deg, rgba(255,255,255,0.035) 0%, rgba(255,255,255,0.015) 100%), var(--color-card)",
      boxShadow: "0 1px 0 rgba(255,255,255,0.04) inset",
    },
  },
  customCSS: `
    :root {
      color-scheme: dark;
    }
    body {
      background:
        radial-gradient(circle at 18% 0%, rgba(245,158,11,0.10), transparent 28rem),
        linear-gradient(180deg, #101114 0%, #0d0e11 100%);
    }
    [data-layout-variant] {
      text-transform: none;
    }
    html body .font-mondwest,
    html body .font-expanded,
    html body .font-compressed,
    html body .font-courier {
      font-family: var(--theme-font-sans);
      letter-spacing: 0;
    }
    html body aside .font-mondwest,
    html body aside .font-expanded {
      letter-spacing: 0.025em;
    }
    aside nav a {
      text-transform: none;
      font-size: 0.88rem;
      letter-spacing: 0;
    }
    aside nav a[aria-current="page"] {
      background: rgba(245,158,11,0.10);
    }
    table, input, textarea, select, button {
      font-feature-settings: "tnum" 1, "cv02" 1;
    }
  `,
};

export const hermesClassicTheme: DashboardTheme = {
  name: "hermes-classic",
  label: "Hermes Classic",
  description: "Classic dark teal — the original Hermes look",
  palette: {
    background: { hex: "#041c1c", alpha: 1 },
    midground: { hex: "#ffe6cb", alpha: 1 },
    foreground: { hex: "#ffffff", alpha: 0 },
    warmGlow: "rgba(255, 189, 56, 0.35)",
    noiseOpacity: 1,
  },
  typography: DEFAULT_TYPOGRAPHY,
  layout: DEFAULT_LAYOUT,
};

export const midnightTheme: DashboardTheme = {
  name: "midnight",
  label: "Midnight",
  description: "Deep blue-violet with cool accents",
  palette: {
    background: { hex: "#0a0a1f", alpha: 1 },
    midground: { hex: "#d4c8ff", alpha: 1 },
    foreground: { hex: "#ffffff", alpha: 0 },
    warmGlow: "rgba(167, 139, 250, 0.32)",
    noiseOpacity: 0.8,
  },
  typography: {
    ...DEFAULT_TYPOGRAPHY,
    fontSans: `"Inter", ${SYSTEM_SANS}`,
    fontMono: `"JetBrains Mono", ${SYSTEM_MONO}`,
    fontUrl:
      "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;700&display=swap",
    letterSpacing: "-0.005em",
  },
  layout: {
    ...DEFAULT_LAYOUT,
    radius: "0.75rem",
  },
};

export const emberTheme: DashboardTheme = {
  name: "ember",
  label: "Ember",
  description: "Warm crimson and bronze — forge vibes",
  palette: {
    background: { hex: "#1a0a06", alpha: 1 },
    midground: { hex: "#ffd8b0", alpha: 1 },
    foreground: { hex: "#ffffff", alpha: 0 },
    warmGlow: "rgba(249, 115, 22, 0.38)",
    noiseOpacity: 1,
  },
  typography: {
    ...DEFAULT_TYPOGRAPHY,
    fontSans: `"Spectral", Georgia, "Times New Roman", serif`,
    fontMono: `"IBM Plex Mono", ${SYSTEM_MONO}`,
    fontUrl:
      "https://fonts.googleapis.com/css2?family=Spectral:wght@400;500;600;700&family=IBM+Plex+Mono:wght@400;500;700&display=swap",
  },
  layout: {
    ...DEFAULT_LAYOUT,
    radius: "0.25rem",
  },
  colorOverrides: {
    destructive: "#c92d0f",
    warning: "#f97316",
  },
};

export const monoTheme: DashboardTheme = {
  name: "mono",
  label: "Mono",
  description: "Clean grayscale — minimal and focused",
  palette: {
    background: { hex: "#0e0e0e", alpha: 1 },
    midground: { hex: "#eaeaea", alpha: 1 },
    foreground: { hex: "#ffffff", alpha: 0 },
    warmGlow: "rgba(255, 255, 255, 0.1)",
    noiseOpacity: 0.6,
  },
  typography: {
    ...DEFAULT_TYPOGRAPHY,
    fontSans: `"IBM Plex Sans", ${SYSTEM_SANS}`,
    fontMono: `"IBM Plex Mono", ${SYSTEM_MONO}`,
    fontUrl:
      "https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@400;500;600&family=IBM+Plex+Mono:wght@400;500&display=swap",
  },
  layout: {
    ...DEFAULT_LAYOUT,
    radius: "0",
  },
};

export const cyberpunkTheme: DashboardTheme = {
  name: "cyberpunk",
  label: "Cyberpunk",
  description: "Neon green on black — matrix terminal",
  palette: {
    background: { hex: "#040608", alpha: 1 },
    midground: { hex: "#9bffcf", alpha: 1 },
    foreground: { hex: "#ffffff", alpha: 0 },
    warmGlow: "rgba(0, 255, 136, 0.22)",
    noiseOpacity: 1.2,
  },
  typography: {
    ...DEFAULT_TYPOGRAPHY,
    fontSans: `"Share Tech Mono", "JetBrains Mono", ${SYSTEM_MONO}`,
    fontMono: `"Share Tech Mono", "JetBrains Mono", ${SYSTEM_MONO}`,
    fontUrl:
      "https://fonts.googleapis.com/css2?family=Share+Tech+Mono&family=JetBrains+Mono:wght@400;700&display=swap",
  },
  layout: {
    ...DEFAULT_LAYOUT,
    radius: "0",
  },
  colorOverrides: {
    success: "#00ff88",
    warning: "#ffd700",
    destructive: "#ff0055",
  },
};

export const roseTheme: DashboardTheme = {
  name: "rose",
  label: "Rosé",
  description: "Soft pink and warm ivory — easy on the eyes",
  palette: {
    background: { hex: "#1a0f15", alpha: 1 },
    midground: { hex: "#ffd4e1", alpha: 1 },
    foreground: { hex: "#ffffff", alpha: 0 },
    warmGlow: "rgba(249, 168, 212, 0.3)",
    noiseOpacity: 0.9,
  },
  typography: {
    ...DEFAULT_TYPOGRAPHY,
    fontSans: `"Fraunces", Georgia, serif`,
    fontMono: `"DM Mono", ${SYSTEM_MONO}`,
    fontUrl:
      "https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,400;9..144,500;9..144,600&family=DM+Mono:wght@400;500&display=swap",
  },
  layout: {
    ...DEFAULT_LAYOUT,
    radius: "1rem",
  },
};

/**
 * Same look as ``defaultTheme`` but with a larger root font size, looser
 * line-height, and ``spacious`` density so every rem-based size in the
 * dashboard scales up. For users who find the default 15px UI too dense.
 */
export const defaultLargeTheme: DashboardTheme = {
  name: "default-large",
  label: "Workbench Large",
  description: "Workbench with bigger fonts and roomier spacing",
  palette: defaultTheme.palette,
  typography: {
    ...defaultTheme.typography,
    baseSize: "17px",
    lineHeight: "1.6",
  },
  layout: {
    ...defaultTheme.layout,
    density: "spacious",
  },
  colorOverrides: defaultTheme.colorOverrides,
  componentStyles: defaultTheme.componentStyles,
  customCSS: defaultTheme.customCSS,
};

export const BUILTIN_THEMES: Record<string, DashboardTheme> = {
  default: defaultTheme,
  "default-large": defaultLargeTheme,
  "hermes-classic": hermesClassicTheme,
  midnight: midnightTheme,
  ember: emberTheme,
  mono: monoTheme,
  cyberpunk: cyberpunkTheme,
  rose: roseTheme,
};
