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

import * as ipc from "$lib/api/ipc";
import type {
  Collection,
  CollectionId,
  Environment,
  EnvironmentId,
  EnvOverride,
  EnvVariable,
  ExecutionResult,
  Project,
  ProjectId,
  Request,
  RequestId,
  RequestSummary,
} from "$lib/types/domain";

const STORAGE_KEY = "apiovnia.active.v1";
const ENV_STORAGE_KEY = "apiovnia.activeEnvByProject.v1";

type Persisted = {
  activeProjectId: ProjectId | null;
  activeCollectionId: CollectionId | null;
  activeRequestId: RequestId | null;
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
  const vars = await ipc.listEnvVariables(envId);
  state.envVarsByEnv = { ...state.envVarsByEnv, [envId]: vars };
}

async function refreshActiveOverride(): Promise<void> {
  if (!state.activeRequestId || !state.activeEnvId) {
    state.activeOverride = null;
    return;
  }
  state.activeOverride = await ipc.getOverride(state.activeRequestId, state.activeEnvId);
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
    state.executionError = e instanceof Error ? e.message : String(e);
    state.currentResponse = null;
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
};
