/**
 * Typed wrappers around `@tauri-apps/api/core` `invoke`.
 *
 * One function per Rust command. Each function:
 *   - Names parameters explicitly (Tauri serialises the kwargs object).
 *   - Returns the typed domain model.
 *
 * Any Rust-side change to `commands/*.rs` lands here as a type error.
 */

import { invoke } from "@tauri-apps/api/core";

import type {
  Collection,
  CollectionId,
  Environment,
  EnvironmentId,
  EnvOverride,
  EnvVariable,
  ExecutionResult,
  ExportResult,
  ImportResult,
  PasswordStrength,
  Project,
  ProjectId,
  Request,
  RequestId,
  RequestSummary,
  SnippetFormat,
} from "$lib/types/domain";

// ---------- Health ----------

export const ping = (): Promise<string> => invoke("ping");

// ---------- Projects ----------

export const listProjects = (): Promise<Project[]> => invoke("list_projects");

export const createProject = (name: string): Promise<Project> =>
  invoke("create_project", { name });

export const renameProject = (id: ProjectId, name: string): Promise<Project> =>
  invoke("rename_project", { id, name });

export const deleteProject = (id: ProjectId): Promise<void> =>
  invoke("delete_project", { id });

// ---------- Collections ----------

export const listCollections = (projectId: ProjectId): Promise<Collection[]> =>
  invoke("list_collections", { projectId });

export const createCollection = (
  projectId: ProjectId,
  name: string,
): Promise<Collection> => invoke("create_collection", { projectId, name });

export const renameCollection = (
  id: CollectionId,
  name: string,
): Promise<Collection> => invoke("rename_collection", { id, name });

export const deleteCollection = (id: CollectionId): Promise<void> =>
  invoke("delete_collection", { id });

// ---------- Requests ----------

export const listRequests = (
  collectionId: CollectionId,
): Promise<RequestSummary[]> => invoke("list_requests", { collectionId });

export const getRequest = (id: RequestId): Promise<Request> =>
  invoke("get_request", { id });

export const createRequest = (
  collectionId: CollectionId,
  name: string,
): Promise<Request> => invoke("create_request", { collectionId, name });

export const renameRequest = (id: RequestId, name: string): Promise<Request> =>
  invoke("rename_request", { id, name });

export const updateRequest = (id: RequestId, patch: Request): Promise<Request> =>
  invoke("update_request", { id, patch });

export const deleteRequest = (id: RequestId): Promise<void> =>
  invoke("delete_request", { id });

// ---------- Environments ----------

export const listEnvs = (projectId: ProjectId): Promise<Environment[]> =>
  invoke("list_envs", { projectId });

export const createEnv = (projectId: ProjectId, name: string): Promise<Environment> =>
  invoke("create_env", { projectId, name });

export const renameEnv = (id: EnvironmentId, name: string): Promise<Environment> =>
  invoke("rename_env", { id, name });

export const deleteEnv = (id: EnvironmentId): Promise<void> =>
  invoke("delete_env", { id });

export const listEnvVariables = (envId: EnvironmentId): Promise<EnvVariable[]> =>
  invoke("list_env_variables", { envId });

export const upsertEnvVariable = (
  envId: EnvironmentId,
  name: string,
  value: string,
  isSecret: boolean,
): Promise<EnvVariable> =>
  invoke("upsert_env_variable", { envId, name, value, isSecret });

export const deleteEnvVariable = (envId: EnvironmentId, name: string): Promise<void> =>
  invoke("delete_env_variable", { envId, name });

// ---------- Overrides ----------

export const getOverride = (
  requestId: RequestId,
  envId: EnvironmentId,
): Promise<EnvOverride | null> => invoke("get_override", { requestId, envId });

export const listOverridesForRequest = (
  requestId: RequestId,
): Promise<EnvOverride[]> => invoke("list_overrides_for_request", { requestId });

/** Upserts the override. Returns `null` when every field is unset (the row
 *  gets deleted in that case). */
export const upsertOverride = (patch: EnvOverride): Promise<EnvOverride | null> =>
  invoke("upsert_override", { patch });

export const deleteOverride = (
  requestId: RequestId,
  envId: EnvironmentId,
): Promise<void> => invoke("delete_override", { requestId, envId });

// ---------- Execution ----------

export const executeRequest = (
  requestId: RequestId,
  envId: EnvironmentId | null = null,
): Promise<ExecutionResult> =>
  invoke("execute_request", { requestId, envId });

/** Last successful response from history for this request, or null. */
export const getLastResponse = (
  requestId: RequestId,
): Promise<ExecutionResult | null> =>
  invoke("get_last_response", { requestId });

/**
 * Build a paste-ready code snippet (curl / Python requests / HTTPie /
 * JS fetch / PowerShell) for the resolved request. Same resolution +
 * decryption path as `executeRequest`. Returns a string ready for the
 * clipboard.
 *
 * Throws `ENV_LOCKED:{envId}` if the active env is locked.
 */
export const buildRequestSnippet = (
  requestId: RequestId,
  envId: EnvironmentId | null,
  format: SnippetFormat,
): Promise<string> =>
  invoke("build_request_snippet", { requestId, envId, format });

// ---------- Crypto / encrypted envs (Phase 6) ----------

/**
 * Seal a plaintext env behind a master password. Encrypts every existing
 * variable value + override field in a single transaction and loads the
 * derived session key. The frontend never sees the key.
 *
 * `bypassPolicy` lets pro users override the zxcvbn/length floor — the
 * backend still rejects empty strings.
 */
export const enableEnvEncryption = (
  envId: EnvironmentId,
  password: string,
  bypassPolicy = false,
): Promise<void> =>
  invoke("enable_env_encryption", { envId, password, bypassPolicy });

/** Inverse of `enableEnvEncryption` — verifies the password, decrypts back to plaintext. */
export const disableEnvEncryption = (
  envId: EnvironmentId,
  password: string,
): Promise<void> => invoke("disable_env_encryption", { envId, password });

/** Derive + verify the env's master key, load it into the session store. */
export const unlockEnv = (
  envId: EnvironmentId,
  password: string,
): Promise<void> => invoke("unlock_env", { envId, password });

/** Drop the session key. Idempotent. */
export const lockEnv = (envId: EnvironmentId): Promise<void> =>
  invoke("lock_env", { envId });

export const isEnvUnlocked = (envId: EnvironmentId): Promise<boolean> =>
  invoke("is_env_unlocked", { envId });

export const listUnlockedEnvs = (): Promise<EnvironmentId[]> =>
  invoke("list_unlocked_envs");

/**
 * Live password strength scoring (zxcvbn). Cheap — debounce ~120 ms in the
 * UI to avoid IPC churn but otherwise call freely. Stateless on the Rust
 * side: no password is logged or stored.
 */
export const scorePassword = (password: string): Promise<PasswordStrength> =>
  invoke("score_password", { password });

// ---------- OpenAPI import / export (Phase 7) ----------

/** Parse an OpenAPI 3.x YAML/JSON file and persist a new collection. */
export const importOpenapi = (
  projectId: ProjectId,
  filePath: string,
): Promise<ImportResult> =>
  invoke("import_openapi", { projectId, filePath });

/** Build the OpenAPI YAML for a collection (secrets pre-stripped). */
export const exportCollectionOpenapi = (
  collectionId: CollectionId,
): Promise<ExportResult> =>
  invoke("export_collection_openapi", { collectionId });

/** Generic "save this text to disk" — used by OpLog "Download log" + the
 *  YAML save itself (frontend picks the path via tauri-plugin-dialog). */
export const saveTextFile = (path: string, contents: string): Promise<void> =>
  invoke("save_text_file", { path, contents });
