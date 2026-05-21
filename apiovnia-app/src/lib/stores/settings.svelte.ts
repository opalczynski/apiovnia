/**
 * Settings store — user preferences persisted to localStorage.
 *
 * Mirrors the `panels.svelte.ts` shape: a `$state` blob loaded once, plain
 * getters/setters that persist on write, and a separate transient `$state`
 * for the modal's open flag (never persisted).
 *
 * Theme is applied to `document.documentElement` so the CSS-variable bundles
 * in `app.css` (`:root[data-theme="..."]`) take effect. The default
 * `apiovnia` theme lives directly on `:root`, so an unknown/missing theme id
 * still renders correctly.
 */

export type ThemeId =
  | "apiovnia"
  | "atomic-dark"
  | "tokyo-night"
  | "monokai"
  | "light";

export type ThemeMeta = {
  id: ThemeId;
  label: string;
  blurb: string;
  /** Light color-scheme — flips native widgets (scrollbars, form controls). */
  light: boolean;
  /** Preview swatch: [background, surface, accent]. */
  swatch: [string, string, string];
};

/** Catalogue, in display order. Palettes drawn from the well-known sources
 *  (Tokyo Night by enkia, Monokai from Sublime) so they read as expected. */
export const THEMES: ThemeMeta[] = [
  {
    id: "apiovnia",
    label: "Apiovnia",
    blurb: "Warm amber on near-black — the original.",
    light: false,
    swatch: ["#0b0b0d", "#16161a", "#f59e0b"],
  },
  {
    id: "atomic-dark",
    label: "Atomic Dark",
    blurb: "Monochrome black, neutral accents, zero colour noise.",
    light: false,
    swatch: ["#0c0c0c", "#181818", "#d4d4d4"],
  },
  {
    id: "tokyo-night",
    label: "Tokyo Night",
    blurb: "Deep navy with blue-violet accents.",
    light: false,
    swatch: ["#16161e", "#1f2335", "#7aa2f7"],
  },
  {
    id: "monokai",
    label: "Monokai",
    blurb: "The classic Sublime look — pink on warm charcoal.",
    light: false,
    swatch: ["#1d1e19", "#2d2e27", "#f92672"],
  },
  {
    id: "light",
    label: "Light",
    blurb: "White base, dark ink — the only non-dark theme.",
    light: true,
    swatch: ["#ffffff", "#f0f0f2", "#b06f00"],
  },
];

/** Options for the history-panel retention cap. */
export const HISTORY_LIMITS = [100, 200, 500, 1000] as const;
export type HistoryLimit = (typeof HISTORY_LIMITS)[number];

const STORAGE_KEY = "apiovnia.settings.v1";

type SettingsState = {
  theme: ThemeId;
  /** How many executions `list_history` fetches into the History panel. */
  historyLimit: HistoryLimit;
};

const DEFAULT: SettingsState = {
  theme: "apiovnia",
  historyLimit: 200,
};

function loadPersisted(): SettingsState {
  if (typeof localStorage === "undefined") return { ...DEFAULT };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT };
    const parsed = JSON.parse(raw) as Partial<SettingsState>;
    const theme = THEMES.some((t) => t.id === parsed.theme)
      ? (parsed.theme as ThemeId)
      : DEFAULT.theme;
    const historyLimit =
      typeof parsed.historyLimit === "number" &&
      (HISTORY_LIMITS as readonly number[]).includes(parsed.historyLimit)
        ? (parsed.historyLimit as HistoryLimit)
        : DEFAULT.historyLimit;
    return { theme, historyLimit };
  } catch {
    return { ...DEFAULT };
  }
}

const store = $state<SettingsState>(loadPersisted());

/** Transient — Settings modal visibility. Deliberately not persisted. */
const ui = $state<{ open: boolean }>({ open: false });

function persist() {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
}

/** Push the active theme onto <html> so the CSS bundles switch. */
function applyTheme() {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.theme = store.theme;
}

// Apply at module init — runs while the import graph evaluates, before the
// Svelte tree mounts, so a non-default theme doesn't flash the amber default.
applyTheme();

export const settings = {
  get theme() {
    return store.theme;
  },
  set theme(v: ThemeId) {
    store.theme = v;
    applyTheme();
    persist();
  },

  get historyLimit() {
    return store.historyLimit;
  },
  set historyLimit(v: HistoryLimit) {
    store.historyLimit = v;
    persist();
  },

  get open() {
    return ui.open;
  },
  set open(v: boolean) {
    ui.open = v;
  },
  toggle() {
    ui.open = !ui.open;
  },

  readonly: { DEFAULT, THEMES, HISTORY_LIMITS },
};
