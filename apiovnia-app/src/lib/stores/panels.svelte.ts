/**
 * Panel sizing store — persists left/middle widths and the right-side
 * request/response split ratio to localStorage.
 *
 * Uses Svelte 5 runes ($state) and exposes plain getters/setters so
 * components can do `panels.leftWidth = 250`.
 */

const STORAGE_KEY = "apiovnia.panels.v1";

type PanelState = {
  leftWidth: number;
  middleWidth: number;
  /** Request/response split, 0..1 — fraction of the right panel given to the request editor. */
  requestSplit: number;
};

const MIN = {
  left: 180,
  middle: 220,
  right: 360,
} as const;

const DEFAULT: PanelState = {
  leftWidth: 240,
  middleWidth: 280,
  requestSplit: 0.5,
};

function loadPersisted(): PanelState {
  if (typeof localStorage === "undefined") return { ...DEFAULT };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT };
    const parsed = JSON.parse(raw) as Partial<PanelState>;
    return {
      leftWidth: clamp(parsed.leftWidth ?? DEFAULT.leftWidth, MIN.left, 600),
      middleWidth: clamp(
        parsed.middleWidth ?? DEFAULT.middleWidth,
        MIN.middle,
        600,
      ),
      requestSplit: clamp(
        parsed.requestSplit ?? DEFAULT.requestSplit,
        0.2,
        0.8,
      ),
    };
  } catch {
    return { ...DEFAULT };
  }
}

function clamp(n: number, min: number, max: number): number {
  return Math.min(Math.max(n, min), max);
}

const initial = loadPersisted();

const store = $state<PanelState>(initial);

let saveTimer: ReturnType<typeof setTimeout> | undefined;
function persist() {
  if (typeof localStorage === "undefined") return;
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
  }, 150);
}

export const panels = {
  get leftWidth() {
    return store.leftWidth;
  },
  set leftWidth(v: number) {
    store.leftWidth = clamp(v, MIN.left, 600);
    persist();
  },

  get middleWidth() {
    return store.middleWidth;
  },
  set middleWidth(v: number) {
    store.middleWidth = clamp(v, MIN.middle, 600);
    persist();
  },

  get requestSplit() {
    return store.requestSplit;
  },
  set requestSplit(v: number) {
    store.requestSplit = clamp(v, 0.2, 0.8);
    persist();
  },

  readonly: { MIN, DEFAULT },
};
