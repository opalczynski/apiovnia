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
  Project,
  ProjectId,
  Request,
  RequestId,
  RequestSummary,
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
