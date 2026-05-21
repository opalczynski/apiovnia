/**
 * Global application state — current selection + cached lists for the three
 * panels. Uses Svelte 5 runes ($state).
 *
 * Components import this as `import { app } from "$lib/stores/app.svelte"`.
 * The exported `app` object proxies the underlying $state, so reading
 * `app.projects` inside templates is reactive.
 *
 * Mutations go through async actions (`loadAll`, `createProject`, etc.) that
 * call IPC and refresh the cached lists. The active IDs persist to
 * localStorage so a restart returns to the last open request.
 */

import { save } from "@tauri-apps/plugin-dialog";

import * as ipc from "$lib/api/ipc";
import { dialogs } from "$lib/stores/dialogs.svelte";
import { settings } from "$lib/stores/settings.svelte";
import type {
  Collection,
  CollectionId,
  Environment,
  EnvironmentId,
  EnvOverride,
  EnvVariable,
  ExecutionResult,
  ExportResult,
  HistoryRow,
  Project,
  ProjectId,
  Request,
  RequestId,
  RequestSummary,
  SnippetFormat,
} from "$lib/types/domain";

const STORAGE_KEY = "apiovnia.active.v1";
const ENV_STORAGE_KEY = "apiovnia.activeEnvByProject.v1";

type Persisted = {
  activeProjectId: ProjectId | null;
  activeCollectionId: CollectionId | null;
  activeRequestId: RequestId | null;
};

/** One row in the OpLog table — narrow enough to share between import
 *  (`{name, method, path}`) and export (adds `redactions`). */
export type OpLogRow = {
  name: string;
  method: string;
  path: string;
  /** Only present for export rows; undefined for import. */
  redactions?: number;
};

/** Bottom-right tabular notification driven by the Phase 7 OpenAPI flows
 *  (and reusable for any future "long-lived feedback" need). Stays open
 *  until the user dismisses; "Download log" saves `logText` to disk. */
export type OpLog = {
  kind: "import" | "export";
  title: string;
  /** Compact stats line under the title — e.g. "12 requests · 2 envs · 1 warning". */
  subtitle: string;
  rows: OpLogRow[];
  warnings: string[];
  logText: string;
  logFilename: string;
};

function loadPersisted(): Persisted {
  if (typeof localStorage === "undefined") {
    return { activeProjectId: null, activeCollectionId: null, activeRequestId: null };
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return { activeProjectId: null, activeCollectionId: null, activeRequestId: null };
    }
    const parsed = JSON.parse(raw) as Partial<Persisted>;
    return {
      activeProjectId: parsed.activeProjectId ?? null,
      activeCollectionId: parsed.activeCollectionId ?? null,
      activeRequestId: parsed.activeRequestId ?? null,
    };
  } catch {
    return { activeProjectId: null, activeCollectionId: null, activeRequestId: null };
  }
}

/** Map<projectId, envId> — last active env per project, persisted. */
function loadActiveEnvByProject(): Record<string, EnvironmentId> {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(ENV_STORAGE_KEY);
    if (!raw) return {};
    return JSON.parse(raw) as Record<string, EnvironmentId>;
  } catch {
    return {};
  }
}

const persisted = loadPersisted();
const activeEnvByProject = loadActiveEnvByProject();

const state = $state({
  projects: [] as Project[],
  collections: [] as Collection[],
  requests: [] as RequestSummary[],
  activeProjectId: persisted.activeProjectId,
  activeCollectionId: persisted.activeCollectionId,
  activeRequestId: persisted.activeRequestId,
  /** The currently opened request body (lazy-loaded on selection). */
  activeRequest: null as Request | null,
  /** Environments for the active project. */
  envs: [] as Environment[],
  /** Active env per project; null = "no env" (base request only). */
  activeEnvId: null as EnvironmentId | null,
  /** Variables for the active env, keyed by env id (so switching envs
   *  doesn't immediately clear what we've cached). */
  envVarsByEnv: {} as Record<string, EnvVariable[]>,
  /** Override for `(activeRequest, activeEnv)`, if any. */
  activeOverride: null as EnvOverride | null,
  /** Env IDs whose session key is currently loaded in the Rust backend.
   *  Starts empty on every app launch (session-only). */
  unlockedEnvIds: new Set<string>(),
  /** When the backend bounces with `ENV_LOCKED:{envId}`, we stash the id here
   *  so the UI can pop the UnlockEnvModal at the right moment, and remember
   *  what to retry after unlock. */
  unlockPrompt: null as {
    envId: EnvironmentId;
    /** Optional retry callback to fire on successful unlock. */
    retry?: () => void | Promise<void>;
  } | null,
  /** ⌘P command palette visibility. The component owns its own search /
   *  catalog state; we only flip this here so any module (keymap, store
   *  action, button click) can toggle. */
  commandPaletteOpen: false,
  /** Transient one-liner shown bottom-right of the shell (~2 s auto-dismiss).
   *  Used by Copy-as-curl and any other "did the thing" feedback that
   *  doesn't warrant a modal. */
  toast: null as { text: string; kind: "ok" | "err"; seq: number } | null,
  /** Persistent tabular feedback for OpenAPI import / export operations.
   *  Stays open until the user dismisses (no auto-fade). Holds the
   *  `logText` + filename so the Download-log button can save it. */
  opLog: null as OpLog | null,
  /** Lifted from `DetailPanel` so the command palette + future entry
   *  points can open the env manager without prop-drilling. */
  envManageOpen: false,
  /** When set, App.svelte renders `SetEnvPasswordModal` for this env id.
   *  Triggered from EnvManageModal's "Enable encryption" button AND from
   *  the command palette's per-env action. */
  envPasswordSetupId: null as EnvironmentId | null,
  /** History panel (Phase 9) — slide-in overlay from the left, shows the
   *  last N executions (`settings.historyLimit`, default 200). Loaded lazily
   *  on first open. */
  historyPanelOpen: false,
  historyEntries: [] as HistoryRow[],
  historyLoading: false,
  loading: false,
  error: null as string | null,
  /**
   * Save-cycle state machine for the active request:
   *   idle    → nothing pending, no recent save (clean / never edited yet)
   *   editing → user typed/pasted/etc; waiting out the debounce window
   *   saving  → IPC update_request in flight
   *   saved   → just landed; UI shows the green confirmation for ~800ms,
   *             then we fall back to idle
   */
  saveState: "idle" as "idle" | "editing" | "saving" | "saved",
  /** True while an HTTP request is in flight. */
  executing: false,
  /** Last execution result for the active request (cleared on selection change). */
  currentResponse: null as ExecutionResult | null,
  /** Non-HTTP execution failure (network, invalid URL, etc). */
  executionError: null as string | null,
});

/**
 * Debounced save plumbing — coalesces rapid edits into a single IPC call.
 *
 * Linear/Notion-style cadence: 250 ms after the last keystroke (or paste,
 * etc) we fire the save. SQLite writes are essentially free; the debounce
 * exists only to dodge a write per character.
 */
const SAVE_DEBOUNCE_MS = 250;
const SAVED_FLASH_MS = 800;

let saveTimer: ReturnType<typeof setTimeout> | undefined;
let savedFlashTimer: ReturnType<typeof setTimeout> | undefined;
let pendingSave: Request | null = null;

function scheduleSave(next: Request) {
  pendingSave = next;
  state.saveState = "editing";
  if (saveTimer) clearTimeout(saveTimer);
  if (savedFlashTimer) {
    clearTimeout(savedFlashTimer);
    savedFlashTimer = undefined;
  }
  saveTimer = setTimeout(() => {
    void flushSave();
  }, SAVE_DEBOUNCE_MS);
}

async function flushSave(): Promise<void> {
  if (saveTimer) {
    clearTimeout(saveTimer);
    saveTimer = undefined;
  }
  const toSave = pendingSave;
  if (!toSave) return;
  pendingSave = null;
  state.saveState = "saving";
  try {
    const saved = await ipc.updateRequest(toSave.id, toSave);
    // Keep the canonical body in sync (server may normalise sort_order etc).
    if (state.activeRequestId === saved.id) state.activeRequest = saved;
    // Refresh summary list so name/method/url changes show in the middle panel.
    if (state.activeCollectionId) await refreshRequests(state.activeCollectionId);

    // Brief "saved" flash, then back to idle — unless the user kept typing,
    // in which case scheduleSave already moved us back to "editing".
    if (state.saveState === "saving") {
      state.saveState = "saved";
      if (savedFlashTimer) clearTimeout(savedFlashTimer);
      savedFlashTimer = setTimeout(() => {
        if (state.saveState === "saved") state.saveState = "idle";
      }, SAVED_FLASH_MS);
    }
  } catch (e) {
    setError(e);
    state.saveState = "idle";
  }
}

function persist() {
  if (typeof localStorage === "undefined") return;
  const snap: Persisted = {
    activeProjectId: state.activeProjectId,
    activeCollectionId: state.activeCollectionId,
    activeRequestId: state.activeRequestId,
  };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(snap));
}

function persistActiveEnv() {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(ENV_STORAGE_KEY, JSON.stringify(activeEnvByProject));
}

function setError(e: unknown) {
  state.error = e instanceof Error ? e.message : String(e);
  // eslint-disable-next-line no-console
  console.error("[apiovnia]", e);
}

// ---------------------------------------------------------------------------
// Loaders — each refreshes a cached list from the backend.
// ---------------------------------------------------------------------------

async function refreshProjects(): Promise<void> {
  state.projects = await ipc.listProjects();
}

async function refreshCollections(projectId: ProjectId): Promise<void> {
  state.collections = await ipc.listCollections(projectId);
}

async function refreshRequests(collectionId: CollectionId): Promise<void> {
  state.requests = await ipc.listRequests(collectionId);
}

async function refreshActiveRequest(id: RequestId): Promise<void> {
  state.activeRequest = await ipc.getRequest(id);
}

async function refreshEnvs(projectId: ProjectId): Promise<void> {
  state.envs = await ipc.listEnvs(projectId);
}

async function refreshEnvVars(envId: EnvironmentId): Promise<void> {
  try {
    const vars = await ipc.listEnvVariables(envId);
    state.envVarsByEnv = { ...state.envVarsByEnv, [envId]: vars };
  } catch (e) {
    // ENV_LOCKED is expected for sealed envs that aren't unlocked yet —
    // the variables UI key off `isEnvLocked` and renders an unlock CTA.
    // Anything else gets surfaced through the global error bar.
    if (isEnvLockedError(e)) {
      state.envVarsByEnv = { ...state.envVarsByEnv, [envId]: [] };
    } else {
      throw e;
    }
  }
}

async function refreshActiveOverride(): Promise<void> {
  if (!state.activeRequestId || !state.activeEnvId) {
    state.activeOverride = null;
    return;
  }
  try {
    state.activeOverride = await ipc.getOverride(state.activeRequestId, state.activeEnvId);
  } catch (e) {
    if (isEnvLockedError(e)) {
      state.activeOverride = null;
    } else {
      throw e;
    }
  }
}

// ---------------------------------------------------------------------------
// Crypto / unlock helpers
// ---------------------------------------------------------------------------

/** Match the `ENV_LOCKED:<envId>` sentinel produced by the Rust storage layer. */
function isEnvLockedError(e: unknown): boolean {
  const msg = e instanceof Error ? e.message : String(e);
  return msg.startsWith("ENV_LOCKED:");
}

function envIdFromLockedError(e: unknown): EnvironmentId | null {
  const msg = e instanceof Error ? e.message : String(e);
  if (!msg.startsWith("ENV_LOCKED:")) return null;
  return msg.slice("ENV_LOCKED:".length) as EnvironmentId;
}

async function refreshUnlockedSet(): Promise<void> {
  try {
    const ids = await ipc.listUnlockedEnvs();
    state.unlockedEnvIds = new Set(ids as unknown as string[]);
  } catch (e) {
    // Non-fatal — the set defaults empty.
    // eslint-disable-next-line no-console
    console.warn("[apiovnia] failed to fetch unlocked envs", e);
  }
}

// ---------------------------------------------------------------------------
// Cascade resolvers — walking the selection chain on load / after deletes.
// ---------------------------------------------------------------------------

async function loadCollectionsForActive(): Promise<void> {
  if (!state.activeProjectId) {
    state.collections = [];
    return;
  }
  await refreshCollections(state.activeProjectId);

  // Keep activeCollectionId pointing at *something* whenever the project has
  // collections — avoids the empty-middle-panel flash when the user picks a
  // different project. Triggers in three cases:
  //   1) no collection was selected (project switch nulled it),
  //   2) the previously-active collection no longer exists,
  //   3) it belonged to a different project.
  const stillValid =
    state.activeCollectionId != null &&
    state.collections.some((c) => c.id === state.activeCollectionId);
  if (!stillValid) {
    state.activeCollectionId = state.collections[0]?.id ?? null;
  }
}

async function loadEnvsForActiveProject(): Promise<void> {
  if (!state.activeProjectId) {
    state.envs = [];
    state.activeEnvId = null;
    return;
  }
  await refreshEnvs(state.activeProjectId);

  // Restore the per-project active env from localStorage. If it's gone
  // (deleted or belongs to a different project), fall back to null.
  const remembered = activeEnvByProject[state.activeProjectId];
  const valid = remembered && state.envs.some((e) => e.id === remembered);
  state.activeEnvId = valid ? (remembered as EnvironmentId) : null;

  // Eagerly fetch variables for the active env so interpolation in the
  // overrides UI shows live preview without an extra await.
  if (state.activeEnvId) await refreshEnvVars(state.activeEnvId);
}

async function loadRequestsForActive(): Promise<void> {
  if (!state.activeCollectionId) {
    state.requests = [];
    return;
  }
  await refreshRequests(state.activeCollectionId);

  // Mirror the collection-level cascade: keep activeRequestId pointing at
  // the first request whenever the active collection has any — avoids the
  // empty-right-panel flash when switching projects/collections. Null OR
  // invalid both pick the first available request.
  const stillValid =
    state.activeRequestId != null &&
    state.requests.some((r) => r.id === state.activeRequestId);
  if (!stillValid) {
    state.activeRequestId = state.requests[0]?.id ?? null;
  }
}

async function loadActiveRequestBody(): Promise<void> {
  if (!state.activeRequestId) {
    state.activeRequest = null;
    state.currentResponse = null;
    state.activeOverride = null;
    return;
  }
  await refreshActiveRequest(state.activeRequestId);
  await refreshActiveOverride();
  // Restore the last successful response from history so the right pane
  // isn't empty after a restart / request switch.
  try {
    state.currentResponse = await ipc.getLastResponse(state.activeRequestId);
    state.executionError = null;
  } catch (e) {
    // Restoration is best-effort — don't block the editor on this.
    // eslint-disable-next-line no-console
    console.warn("[apiovnia] failed to restore last response", e);
    state.currentResponse = null;
  }
}

// ---------------------------------------------------------------------------
// Public actions
// ---------------------------------------------------------------------------

async function loadAll(): Promise<void> {
  state.loading = true;
  state.error = null;
  try {
    await refreshUnlockedSet();
    await refreshProjects();

    if (
      state.activeProjectId &&
      !state.projects.some((p) => p.id === state.activeProjectId)
    ) {
      state.activeProjectId = state.projects[0]?.id ?? null;
    } else if (!state.activeProjectId) {
      state.activeProjectId = state.projects[0]?.id ?? null;
    }

    await loadCollectionsForActive();
    await loadEnvsForActiveProject();
    await loadRequestsForActive();
    await loadActiveRequestBody();
    persist();
  } catch (e) {
    setError(e);
  } finally {
    state.loading = false;
  }
}

async function selectProject(id: ProjectId): Promise<void> {
  if (state.activeProjectId === id) return;
  state.activeProjectId = id;
  state.activeCollectionId = null;
  state.activeRequestId = null;
  state.activeRequest = null;
  try {
    await loadCollectionsForActive();
    await loadEnvsForActiveProject();
    await loadRequestsForActive();
    await loadActiveRequestBody();
    persist();
  } catch (e) {
    setError(e);
  }
}

async function selectCollection(id: CollectionId): Promise<void> {
  if (state.activeCollectionId === id) return;
  state.activeCollectionId = id;
  state.activeRequestId = null;
  state.activeRequest = null;
  try {
    await loadRequestsForActive();
    await loadActiveRequestBody();
    persist();
  } catch (e) {
    setError(e);
  }
}

/**
 * Cross-collection / cross-project request navigation — used by the
 * command palette where the selected request might live anywhere.
 *
 * Walks the chain top-down, only triggering the heavy cascades when the
 * level actually changed (no point reloading collections if we're already
 * on the right project).
 */
async function navigateToRequest(
  projectId: ProjectId,
  collectionId: CollectionId,
  requestId: RequestId,
): Promise<void> {
  if (state.activeProjectId !== projectId) {
    await selectProject(projectId);
  }
  if (state.activeCollectionId !== collectionId) {
    await selectCollection(collectionId);
  }
  if (state.activeRequestId !== requestId) {
    await selectRequest(requestId);
  }
}

// ---------------------------------------------------------------------------
// Toast (transient bottom-right notification)
// ---------------------------------------------------------------------------

let toastTimer: ReturnType<typeof setTimeout> | undefined;
let toastSeq = 0;

function showToast(text: string, kind: "ok" | "err" = "ok"): void {
  toastSeq += 1;
  const seq = toastSeq;
  state.toast = { text, kind, seq };
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    // Only clear if no newer toast has displaced us.
    if (state.toast?.seq === seq) state.toast = null;
  }, 2200);
}

function dismissToast(): void {
  state.toast = null;
}

// ---------------------------------------------------------------------------
// Copy as curl
// ---------------------------------------------------------------------------

function showOpLog(entry: OpLog): void {
  state.opLog = entry;
}

function dismissOpLog(): void {
  state.opLog = null;
}

// ---------------------------------------------------------------------------
// OpenAPI import / export (Phase 7)
// ---------------------------------------------------------------------------

/**
 * Run the OpenAPI import for the active project. `filePath` comes from
 * the native file picker (caller's job). On success, opens the OpLog
 * panel + navigates to the new collection. On failure, surfaces a toast.
 */
async function importOpenapiForProject(
  projectId: ProjectId,
  filePath: string,
): Promise<void> {
  try {
    const result = await ipc.importOpenapi(projectId, filePath);
    // Refresh the caches the user is about to see.
    await refreshProjects();
    if (state.activeProjectId === projectId) {
      await refreshCollections(projectId);
      await refreshEnvs(projectId);
    }
    // Jump to the new collection — best UX, no "where did it go" moment.
    await selectProject(projectId);
    state.activeCollectionId = result.collectionId;
    await loadRequestsForActive();
    await loadActiveRequestBody();
    persist();

    const subtitleBits: string[] = [
      `${result.requestCount} request${result.requestCount === 1 ? "" : "s"}`,
    ];
    if (result.environmentCount > 0) {
      subtitleBits.push(
        `${result.environmentCount} env${result.environmentCount === 1 ? "" : "s"}`,
      );
    }
    if (result.warningCount > 0) {
      subtitleBits.push(
        `${result.warningCount} warning${result.warningCount === 1 ? "" : "s"}`,
      );
    }
    showOpLog({
      kind: "import",
      title: `Imported "${result.collectionName}"`,
      subtitle: subtitleBits.join(" · "),
      rows: result.rows.map((r) => ({
        name: r.name,
        method: r.method,
        path: r.path,
      })),
      warnings: result.warnings,
      logText: result.logText,
      logFilename: result.logFilename,
    });
  } catch (e) {
    showToast(
      `Import failed: ${e instanceof Error ? e.message : String(e)}`,
      "err",
    );
  }
}

/**
 * Run the OpenAPI export for a collection — interactive flow.
 *
 * Sequence: build YAML (which gives us the suggested
 * `{project}_{collection}.yaml` filename) → native save dialog defaulted
 * to that name → write the file → pop the OpLog panel. Cancelling the
 * dialog is a no-op (no toast, no log).
 *
 * Building before opening the dialog is what lets the suggested filename
 * include the project name — the frontend doesn't always have project
 * context for an arbitrary collection (e.g. when called from the palette
 * with whatever is active), and the backend already knows the full
 * project → collection chain.
 */
async function exportCollectionInteractive(collectionId: CollectionId): Promise<void> {
  let result: ExportResult;
  try {
    result = await ipc.exportCollectionOpenapi(collectionId);
  } catch (e) {
    // ExportError::Collision is the most useful case to surface verbatim —
    // the message includes the colliding path + method.
    showToast(
      `Export failed: ${e instanceof Error ? e.message : String(e)}`,
      "err",
    );
    return;
  }

  const yamlSavePath = await save({
    title: "Export collection as OpenAPI",
    defaultPath: result.yamlFilename,
    filters: [{ name: "OpenAPI YAML", extensions: ["yaml", "yml"] }],
  });
  if (!yamlSavePath) return; // user cancelled

  try {
    await ipc.saveTextFile(yamlSavePath, result.yaml);
  } catch (e) {
    showToast(
      `Couldn't write YAML: ${e instanceof Error ? e.message : String(e)}`,
      "err",
    );
    return;
  }

  const subtitleBits: string[] = [
    `${result.requestCount} request${result.requestCount === 1 ? "" : "s"}`,
  ];
  if (result.redactionCount > 0) {
    subtitleBits.push(
      `${result.redactionCount} secret${result.redactionCount === 1 ? "" : "s"} stripped`,
    );
  }
  if (result.warningCount > 0) {
    subtitleBits.push(
      `${result.warningCount} warning${result.warningCount === 1 ? "" : "s"}`,
    );
  }
  showOpLog({
    kind: "export",
    title: `Exported to ${yamlSavePath.split("/").pop() ?? yamlSavePath}`,
    subtitle: subtitleBits.join(" · "),
    rows: result.rows.map((r) => ({
      name: r.name,
      method: r.method,
      path: r.path,
      redactions: r.redactions,
    })),
    warnings: result.warnings,
    logText: result.logText,
    logFilename: result.logFilename,
  });
}

/**
 * Build a code snippet for the request (with the currently-active env's
 * resolution + decryption) and write it to the clipboard. EnvLocked
 * propagates to the unlock modal with a retry that re-runs this op.
 */
async function copyRequestAsSnippet(
  requestId: RequestId,
  format: SnippetFormat,
): Promise<void> {
  // Flush any pending request edits so the snippet reflects what's on screen.
  await flushSave();
  try {
    const snippet = await ipc.buildRequestSnippet(
      requestId,
      state.activeEnvId,
      format,
    );
    await navigator.clipboard.writeText(snippet);
    showToast(`Copied ${snippetFormatLabel(format)} to clipboard`);
  } catch (e) {
    const lockedEnv = envIdFromLockedError(e);
    if (lockedEnv) {
      state.unlockPrompt = {
        envId: lockedEnv,
        retry: () => copyRequestAsSnippet(requestId, format),
      };
      return;
    }
    showToast(
      `Couldn't copy snippet: ${e instanceof Error ? e.message : String(e)}`,
      "err",
    );
  }
}

/** Display name for each format — used in toasts + menu labels. */
export function snippetFormatLabel(f: SnippetFormat): string {
  switch (f) {
    case "curl":
      return "curl";
    case "pythonRequests":
      return "Python (requests)";
    case "httpie":
      return "HTTPie";
    case "javaScriptFetch":
      return "JavaScript (fetch)";
    case "powerShell":
      return "PowerShell";
  }
}

/** All formats in a stable display order, for menus / palette. */
export const SNIPPET_FORMATS: readonly SnippetFormat[] = [
  "curl",
  "pythonRequests",
  "httpie",
  "javaScriptFetch",
  "powerShell",
];

function openPalette(): void {
  state.commandPaletteOpen = true;
}

function closePalette(): void {
  state.commandPaletteOpen = false;
}

function openEnvManage(): void {
  state.envManageOpen = true;
}

function closeEnvManage(): void {
  state.envManageOpen = false;
}

function openEnvPasswordSetup(id: EnvironmentId): void {
  state.envPasswordSetupId = id;
}

function closeEnvPasswordSetup(): void {
  state.envPasswordSetupId = null;
}

/**
 * Interactive "disable encryption" flow — same UX whether triggered from
 * EnvManageModal or the command palette. Prompts for the current master
 * password, then asks the backend to decrypt every variable + override
 * row in one transaction.
 */
async function disableEncryptionWithPrompt(
  envId: EnvironmentId,
  envName: string,
): Promise<void> {
  const password = await dialogs.prompt({
    title: `Disable encryption for "${envName}"?`,
    message:
      "Enter the current master password. Every variable + override will be decrypted and stored as plaintext.",
    placeholder: "current master password",
    confirmLabel: "Decrypt environment",
    kind: "password",
  });
  if (!password) return;
  try {
    await disableEnvEncryption(envId, password);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    await dialogs.confirm({
      title: "Couldn't disable encryption",
      message: msg.includes("wrong password")
        ? "That password didn't match. Try again from the menu."
        : msg,
      confirmLabel: "OK",
    });
  }
}

async function selectRequest(id: RequestId): Promise<void> {
  if (state.activeRequestId === id) return;
  // Flush pending edits to the *previous* request before switching. Same
  // for any in-flight override edits.
  await flushSave();
  await flushOverride();
  state.activeRequestId = id;
  state.saveState = "idle";
  // Reset transient response state; loadActiveRequestBody will repopulate
  // from history if there's anything on file.
  state.currentResponse = null;
  state.executionError = null;
  try {
    await loadActiveRequestBody();
    persist();
  } catch (e) {
    setError(e);
  }
}

/**
 * Patch the active request in-place and schedule a debounced save. Components
 * call this on every keystroke; we coalesce them through `scheduleSave`.
 */
function updateActiveRequest(patch: Partial<Request>): void {
  if (!state.activeRequest) return;
  const next: Request = { ...state.activeRequest, ...patch };
  state.activeRequest = next;
  scheduleSave(next);
}

/**
 * Execute the active request against the active environment. Flushes any
 * pending save first so the wire payload matches what the user sees in
 * the editor. The env id (or null) gets forwarded to the backend, which
 * folds the override + interpolates `{{vars}}` before sending.
 */
async function executeActive(): Promise<void> {
  if (!state.activeRequestId || state.executing) return;
  await flushSave();
  state.executing = true;
  state.executionError = null;
  try {
    state.currentResponse = await ipc.executeRequest(
      state.activeRequestId,
      state.activeEnvId,
    );
  } catch (e) {
    const lockedEnv = envIdFromLockedError(e);
    if (lockedEnv) {
      // Pop the unlock modal; on success it will retry execute via the
      // `retry` callback wired here.
      state.unlockPrompt = {
        envId: lockedEnv,
        retry: () => executeActive(),
      };
    } else {
      state.executionError = e instanceof Error ? e.message : String(e);
      state.currentResponse = null;
    }
  } finally {
    state.executing = false;
  }
}

// ---------------------------------------------------------------------------
// Environments
// ---------------------------------------------------------------------------

async function selectEnv(id: EnvironmentId | null): Promise<void> {
  if (state.activeEnvId === id) return;
  state.activeEnvId = id;
  if (state.activeProjectId) {
    if (id) {
      activeEnvByProject[state.activeProjectId] = id;
    } else {
      delete activeEnvByProject[state.activeProjectId];
    }
    persistActiveEnv();
  }
  // Fetch variables for the new active env (cached per env id).
  if (id && !state.envVarsByEnv[id]) {
    try {
      await refreshEnvVars(id);
    } catch (e) {
      setError(e);
    }
  }
  // Refresh the override binding for the current request under the new env.
  try {
    await refreshActiveOverride();
  } catch (e) {
    setError(e);
  }
}

async function createEnv(name: string): Promise<Environment | null> {
  if (!state.activeProjectId) return null;
  try {
    const created = await ipc.createEnv(state.activeProjectId, name);
    await refreshEnvs(state.activeProjectId);
    await selectEnv(created.id);
    return created;
  } catch (e) {
    setError(e);
    return null;
  }
}

async function renameEnv(id: EnvironmentId, name: string): Promise<void> {
  try {
    await ipc.renameEnv(id, name);
    if (state.activeProjectId) await refreshEnvs(state.activeProjectId);
  } catch (e) {
    setError(e);
  }
}

async function deleteEnv(id: EnvironmentId): Promise<void> {
  try {
    await ipc.deleteEnv(id);
    if (state.activeEnvId === id) {
      await selectEnv(null);
    }
    // Clear cached variables for the dropped env.
    if (state.envVarsByEnv[id]) {
      const { [id]: _, ...rest } = state.envVarsByEnv;
      state.envVarsByEnv = rest;
    }
    if (state.activeProjectId) await refreshEnvs(state.activeProjectId);
  } catch (e) {
    setError(e);
  }
}

async function upsertEnvVariable(
  envId: EnvironmentId,
  name: string,
  value: string,
  isSecret: boolean,
): Promise<void> {
  try {
    await ipc.upsertEnvVariable(envId, name, value, isSecret);
    await refreshEnvVars(envId);
  } catch (e) {
    setError(e);
  }
}

async function deleteEnvVariable(envId: EnvironmentId, name: string): Promise<void> {
  try {
    await ipc.deleteEnvVariable(envId, name);
    await refreshEnvVars(envId);
  } catch (e) {
    setError(e);
  }
}

// ---------------------------------------------------------------------------
// Encryption: enable / disable / unlock / lock
// ---------------------------------------------------------------------------

/**
 * Seal an env with a master password. On success the env's session key is
 * loaded so the user can keep editing without re-typing. Throws on conflict
 * (already encrypted), empty password, or — when `bypassPolicy=false` — a
 * password below the zxcvbn floor.
 */
async function enableEnvEncryption(
  envId: EnvironmentId,
  password: string,
  bypassPolicy = false,
): Promise<void> {
  await ipc.enableEnvEncryption(envId, password, bypassPolicy);
  state.unlockedEnvIds = new Set([...state.unlockedEnvIds, envId as unknown as string]);
  if (state.activeProjectId) await refreshEnvs(state.activeProjectId);
  await refreshEnvVars(envId);
  if (state.activeEnvId === envId) await refreshActiveOverride();
}

/** Verify password, decrypt everything, flip env back to plaintext. */
async function disableEnvEncryption(
  envId: EnvironmentId,
  password: string,
): Promise<void> {
  await ipc.disableEnvEncryption(envId, password);
  const next = new Set(state.unlockedEnvIds);
  next.delete(envId as unknown as string);
  state.unlockedEnvIds = next;
  if (state.activeProjectId) await refreshEnvs(state.activeProjectId);
  await refreshEnvVars(envId);
  if (state.activeEnvId === envId) await refreshActiveOverride();
}

/** Derive + load the session key. Throws on wrong password. */
async function unlockEnv(envId: EnvironmentId, password: string): Promise<void> {
  await ipc.unlockEnv(envId, password);
  state.unlockedEnvIds = new Set([...state.unlockedEnvIds, envId as unknown as string]);
  // Refresh anything that was previously blocked.
  await refreshEnvVars(envId);
  if (state.activeEnvId === envId) await refreshActiveOverride();
}

async function lockEnv(envId: EnvironmentId): Promise<void> {
  try {
    await ipc.lockEnv(envId);
  } catch (e) {
    setError(e);
    return;
  }
  const next = new Set(state.unlockedEnvIds);
  next.delete(envId as unknown as string);
  state.unlockedEnvIds = next;
  // Re-fetch — variables/override will now show as locked.
  await refreshEnvVars(envId);
  if (state.activeEnvId === envId) await refreshActiveOverride();
}

/** Check whether an env is currently sealed AND not unlocked in this session. */
function isEnvLocked(envId: EnvironmentId | null | undefined): boolean {
  if (!envId) return false;
  const env = state.envs.find((e) => e.id === envId);
  if (!env || !env.isEncrypted) return false;
  return !state.unlockedEnvIds.has(envId as unknown as string);
}

/** Request the unlock modal for this env. Optional retry runs after success. */
function promptUnlock(
  envId: EnvironmentId,
  retry?: () => void | Promise<void>,
): void {
  state.unlockPrompt = { envId, retry };
}

function dismissUnlockPrompt(): void {
  state.unlockPrompt = null;
}

// ---------------------------------------------------------------------------
// Overrides — debounced like the request editor.
// ---------------------------------------------------------------------------

let overrideSaveTimer: ReturnType<typeof setTimeout> | undefined;
let pendingOverride: EnvOverride | null = null;

async function flushOverride(): Promise<void> {
  if (overrideSaveTimer) {
    clearTimeout(overrideSaveTimer);
    overrideSaveTimer = undefined;
  }
  const patch = pendingOverride;
  if (!patch) return;
  pendingOverride = null;
  try {
    const saved = await ipc.upsertOverride(patch);
    // Refresh the active binding if this was for the same (req, env) pair.
    if (
      state.activeRequestId === patch.requestId &&
      state.activeEnvId === patch.environmentId
    ) {
      state.activeOverride = saved;
    }
  } catch (e) {
    setError(e);
  }
}

function scheduleOverrideSave(patch: EnvOverride) {
  pendingOverride = patch;
  if (overrideSaveTimer) clearTimeout(overrideSaveTimer);
  overrideSaveTimer = setTimeout(() => {
    void flushOverride();
  }, SAVE_DEBOUNCE_MS);
}

/**
 * Patch the active override in-place and schedule a debounced save.
 * Caller passes a partial map (e.g. `{ url: "https://prod…" }`); we merge
 * onto the existing override (or create a fresh empty one) and dispatch.
 */
function updateActiveOverride(patch: Partial<EnvOverride>): void {
  if (!state.activeRequestId || !state.activeEnvId) return;
  const base: EnvOverride = state.activeOverride ?? {
    requestId: state.activeRequestId,
    environmentId: state.activeEnvId,
  };
  const next: EnvOverride = { ...base, ...patch };
  state.activeOverride = next;
  scheduleOverrideSave(next);
}

async function resetActiveOverride(): Promise<void> {
  if (!state.activeRequestId || !state.activeEnvId) return;
  if (overrideSaveTimer) {
    clearTimeout(overrideSaveTimer);
    overrideSaveTimer = undefined;
  }
  pendingOverride = null;
  try {
    await ipc.deleteOverride(state.activeRequestId, state.activeEnvId);
    state.activeOverride = null;
  } catch (e) {
    setError(e);
  }
}

async function createProject(name: string): Promise<void> {
  try {
    const created = await ipc.createProject(name);
    await refreshProjects();
    await selectProject(created.id);
  } catch (e) {
    setError(e);
  }
}

async function createCollection(name: string): Promise<void> {
  if (!state.activeProjectId) return;
  try {
    const created = await ipc.createCollection(state.activeProjectId, name);
    await refreshCollections(state.activeProjectId);
    await selectCollection(created.id);
  } catch (e) {
    setError(e);
  }
}

async function createRequest(name: string): Promise<void> {
  if (!state.activeCollectionId) return;
  try {
    const created = await ipc.createRequest(state.activeCollectionId, name);
    await refreshRequests(state.activeCollectionId);
    await selectRequest(created.id);
  } catch (e) {
    setError(e);
  }
}

async function renameProject(id: ProjectId, name: string): Promise<void> {
  try {
    await ipc.renameProject(id, name);
    await refreshProjects();
  } catch (e) {
    setError(e);
  }
}

async function renameCollection(id: CollectionId, name: string): Promise<void> {
  try {
    await ipc.renameCollection(id, name);
    if (state.activeProjectId) await refreshCollections(state.activeProjectId);
  } catch (e) {
    setError(e);
  }
}

async function renameRequest(id: RequestId, name: string): Promise<void> {
  try {
    await ipc.renameRequest(id, name);
    if (state.activeCollectionId) await refreshRequests(state.activeCollectionId);
    if (state.activeRequestId === id) await loadActiveRequestBody();
  } catch (e) {
    setError(e);
  }
}

async function deleteProject(id: ProjectId): Promise<void> {
  try {
    await ipc.deleteProject(id);
    if (state.activeProjectId === id) {
      state.activeProjectId = null;
      state.activeCollectionId = null;
      state.activeRequestId = null;
      state.activeRequest = null;
    }
    await loadAll();
  } catch (e) {
    setError(e);
  }
}

async function deleteCollection(id: CollectionId): Promise<void> {
  try {
    await ipc.deleteCollection(id);
    if (state.activeCollectionId === id) {
      state.activeCollectionId = null;
      state.activeRequestId = null;
      state.activeRequest = null;
    }
    await loadCollectionsForActive();
    await loadRequestsForActive();
    await loadActiveRequestBody();
    persist();
  } catch (e) {
    setError(e);
  }
}

async function deleteRequest(id: RequestId): Promise<void> {
  try {
    await ipc.deleteRequest(id);
    if (state.activeRequestId === id) {
      state.activeRequestId = null;
      state.activeRequest = null;
    }
    await loadRequestsForActive();
    await loadActiveRequestBody();
    persist();
  } catch (e) {
    setError(e);
  }
}

// ---------------------------------------------------------------------------
// History (Phase 9)
// ---------------------------------------------------------------------------

async function refreshHistory(): Promise<void> {
  state.historyLoading = true;
  try {
    state.historyEntries = await ipc.listHistory(settings.historyLimit);
  } catch (e) {
    setError(e);
  } finally {
    state.historyLoading = false;
  }
}

async function openHistoryPanel(): Promise<void> {
  state.historyPanelOpen = true;
  await refreshHistory();
}

function closeHistoryPanel(): void {
  state.historyPanelOpen = false;
}

async function toggleHistoryPanel(): Promise<void> {
  if (state.historyPanelOpen) {
    closeHistoryPanel();
  } else {
    await openHistoryPanel();
  }
}

/**
 * Open a stored history entry: navigates to the originating request (if
 * still present) and rehydrates that row's response into the right pane.
 * If the request was deleted we still surface the saved response.
 */
async function openHistoryEntry(entry: HistoryRow): Promise<void> {
  closeHistoryPanel();

  // Navigate first — selectRequest reloads the body + last response from
  // history, which would otherwise blow away the row we're about to open.
  // We override `currentResponse` afterwards.
  if (entry.projectId && entry.collectionId && entry.requestId) {
    try {
      await navigateToRequest(entry.projectId, entry.collectionId, entry.requestId);
    } catch (e) {
      // Request/collection/project was deleted — non-fatal, we still show
      // the saved response below.
      // eslint-disable-next-line no-console
      console.warn("[apiovnia] couldn't navigate to history entry's request", e);
    }
  }

  try {
    const result = await ipc.getHistoryResponse(entry.id);
    if (result) {
      state.currentResponse = result;
      state.executionError = null;
    }
  } catch (e) {
    showToast(
      `Couldn't load history entry: ${e instanceof Error ? e.message : String(e)}`,
      "err",
    );
  }
}

// ---------------------------------------------------------------------------
// Public proxy — getters bind reactivity, actions are plain functions.
// ---------------------------------------------------------------------------

export const app = {
  get projects() {
    return state.projects;
  },
  get collections() {
    return state.collections;
  },
  get requests() {
    return state.requests;
  },
  get activeProjectId() {
    return state.activeProjectId;
  },
  get activeCollectionId() {
    return state.activeCollectionId;
  },
  get activeRequestId() {
    return state.activeRequestId;
  },
  get activeProject(): Project | null {
    return (
      state.projects.find((p) => p.id === state.activeProjectId) ?? null
    );
  },
  get activeCollection(): Collection | null {
    return (
      state.collections.find((c) => c.id === state.activeCollectionId) ?? null
    );
  },
  get activeRequest() {
    return state.activeRequest;
  },
  get envs() {
    return state.envs;
  },
  get activeEnvId() {
    return state.activeEnvId;
  },
  get activeEnv(): Environment | null {
    return state.envs.find((e) => e.id === state.activeEnvId) ?? null;
  },
  get activeEnvVariables(): EnvVariable[] {
    return state.activeEnvId ? (state.envVarsByEnv[state.activeEnvId] ?? []) : [];
  },
  envVariablesFor(envId: EnvironmentId): EnvVariable[] {
    return state.envVarsByEnv[envId] ?? [];
  },
  get activeOverride() {
    return state.activeOverride;
  },
  get loading() {
    return state.loading;
  },
  get error() {
    return state.error;
  },
  get saveState() {
    return state.saveState;
  },
  get executing() {
    return state.executing;
  },
  get currentResponse() {
    return state.currentResponse;
  },
  get executionError() {
    return state.executionError;
  },

  loadAll,
  selectProject,
  selectCollection,
  selectRequest,
  createProject,
  createCollection,
  createRequest,
  renameProject,
  renameCollection,
  renameRequest,
  deleteProject,
  deleteCollection,
  deleteRequest,
  updateActiveRequest,
  flushSave,
  executeActive,
  // Envs
  selectEnv,
  createEnv,
  renameEnv,
  deleteEnv,
  upsertEnvVariable,
  deleteEnvVariable,
  refreshEnvVars,
  // Overrides
  updateActiveOverride,
  resetActiveOverride,
  flushOverride,
  // Crypto / encrypted envs (Phase 6)
  get unlockedEnvIds() {
    return state.unlockedEnvIds;
  },
  get unlockPrompt() {
    return state.unlockPrompt;
  },
  isEnvLocked,
  enableEnvEncryption,
  disableEnvEncryption,
  unlockEnv,
  lockEnv,
  promptUnlock,
  dismissUnlockPrompt,
  // Command palette (Phase 8)
  get commandPaletteOpen() {
    return state.commandPaletteOpen;
  },
  openPalette,
  closePalette,
  navigateToRequest,
  // Toast (transient feedback)
  get toast() {
    return state.toast;
  },
  showToast,
  dismissToast,
  // OpLog (persistent feedback, used by OpenAPI flows)
  get opLog() {
    return state.opLog;
  },
  dismissOpLog,
  // Copy as snippet (curl / python / httpie / javascript / powershell)
  copyRequestAsSnippet,
  // OpenAPI (Phase 7)
  importOpenapiForProject,
  exportCollectionInteractive,
  // Env manage / password modals (lifted to global state so the palette
  // can open them without prop-drilling through DetailPanel)
  get envManageOpen() {
    return state.envManageOpen;
  },
  get envPasswordSetupId() {
    return state.envPasswordSetupId;
  },
  openEnvManage,
  closeEnvManage,
  openEnvPasswordSetup,
  closeEnvPasswordSetup,
  disableEncryptionWithPrompt,
  // History panel (Phase 9)
  get historyPanelOpen() {
    return state.historyPanelOpen;
  },
  get historyEntries() {
    return state.historyEntries;
  },
  get historyLoading() {
    return state.historyLoading;
  },
  openHistoryPanel,
  closeHistoryPanel,
  toggleHistoryPanel,
  openHistoryEntry,
  refreshHistory,
};
