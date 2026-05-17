/**
 * TypeScript mirror of `apiovnia-core` domain types.
 *
 * Kept in sync by hand for now — we don't generate from Rust because the
 * cost (ts-rs / specta) outweighs the ceremony for a < 20-type surface.
 * When this file drifts from `crates/apiovnia-core/src/model.rs`, the
 * compiler will surface it via the IPC wrappers in `$lib/api/ipc.ts`.
 *
 * `id` fields are branded strings so the type system catches mix-ups
 * (passing a CollectionId where a ProjectId is expected). The brand is
 * erased at runtime — IPC sees plain strings.
 */

declare const __brand: unique symbol;
type Brand<T, B> = T & { readonly [__brand]: B };

export type ProjectId = Brand<string, "ProjectId">;
export type CollectionId = Brand<string, "CollectionId">;
export type RequestId = Brand<string, "RequestId">;
export type EnvironmentId = Brand<string, "EnvironmentId">;

export const asProjectId = (s: string): ProjectId => s as ProjectId;
export const asCollectionId = (s: string): CollectionId => s as CollectionId;
export const asRequestId = (s: string): RequestId => s as RequestId;
export const asEnvironmentId = (s: string): EnvironmentId => s as EnvironmentId;

export type HttpMethod =
  | "GET"
  | "POST"
  | "PUT"
  | "PATCH"
  | "DELETE"
  | "HEAD"
  | "OPTIONS";

export type BodyType = "none" | "json" | "form" | "multipart" | "raw";

/**
 * Multipart row — text part or file part. Serialised as an array inside
 * `request.bodyContent` (same trick as form-encoded). The Rust executor
 * decodes the same shape via serde — keep these in sync.
 */
export type MultipartField = {
  key: string;
  /** Text body when `kind === "text"`. */
  value: string;
  kind: "text" | "file";
  /** Absolute path to the file when `kind === "file"`; empty otherwise. */
  filePath: string;
  /** Optional MIME override. Empty → guessed from the file extension. */
  contentType: string;
  enabled: boolean;
};

export type KeyValue = {
  key: string;
  value: string;
  enabled: boolean;
};

export type AuthConfig =
  | { type: "none" }
  | { type: "bearer"; token: string }
  | { type: "basic"; username: string; password: string }
  | { type: "apikey"; name: string; value: string; in: "header" | "query" };

export type Project = {
  id: ProjectId;
  name: string;
  createdAt: number;
  updatedAt: number;
  sortOrder: number;
};

export type Collection = {
  id: CollectionId;
  projectId: ProjectId;
  name: string;
  createdAt: number;
  updatedAt: number;
  sortOrder: number;
};

export type Request = {
  id: RequestId;
  collectionId: CollectionId;
  name: string;
  method: HttpMethod;
  url: string;
  headers: KeyValue[];
  params: KeyValue[];
  bodyType: BodyType;
  bodyContent: string;
  auth: AuthConfig;
  createdAt: number;
  updatedAt: number;
  sortOrder: number;
};

/** Lightweight row for the middle panel — no headers/body/auth. */
export type RequestSummary = {
  id: RequestId;
  collectionId: CollectionId;
  name: string;
  method: HttpMethod;
  url: string;
  sortOrder: number;
};

// ---------------------------------------------------------------------------
// Environments — Phase 5. Encryption (Phase 6) just flips `isEncrypted` /
// `requiresUnlock` and adds salt + password_check fields on the Rust side
// that we never see over IPC.
// ---------------------------------------------------------------------------

export type Environment = {
  id: EnvironmentId;
  projectId: ProjectId;
  name: string;
  requiresUnlock: boolean;
  isEncrypted: boolean;
  createdAt: number;
};

export type EnvVariable = {
  id: string;
  environmentId: EnvironmentId;
  name: string;
  value: string;
  isSecret: boolean;
};

/**
 * Per-`(request, env)` patch. Every override field is optional — `null` /
 * absent = inherit from the underlying request. Headers and params are
 * **full replacements** when set, not per-key merges (the brief asks for
 * this; per-key merge is a footgun).
 */
export type EnvOverride = {
  requestId: RequestId;
  environmentId: EnvironmentId;
  method?: HttpMethod | null;
  url?: string | null;
  headers?: KeyValue[] | null;
  params?: KeyValue[] | null;
  bodyType?: BodyType | null;
  bodyContent?: string | null;
  auth?: AuthConfig | null;
};

// ---------------------------------------------------------------------------
// Execution — mirrors `apiovnia-http::ExecutionResult`.
// ---------------------------------------------------------------------------

export type ResponseBodyKind = "text" | "binarybase64" | "empty";

export type HeaderEntry = {
  name: string;
  value: string;
};

export type SentRequest = {
  method: string;
  url: string;
  headers: HeaderEntry[];
  /** First 16 KiB of the outgoing body, UTF-8 lossy. Empty for GET/HEAD. */
  bodyPreview: string;
  bodySizeBytes: number;
};

// ---------------------------------------------------------------------------
// Snippet formats — mirrors `apiovnia_core::SnippetFormat`. Drives the
// "Copy as…" submenu in the request context menu + command palette.
// ---------------------------------------------------------------------------

export type SnippetFormat =
  | "curl"
  | "pythonRequests"
  | "httpie"
  | "javaScriptFetch"
  | "powerShell";

// ---------------------------------------------------------------------------
// Password strength — mirrors `apiovnia_crypto::PasswordStrength`. Used by
// the encrypted-env setup flow's live meter.
// ---------------------------------------------------------------------------

export type PasswordStrength = {
  /** 0..=4, zxcvbn's score. */
  score: number;
  /** Localised label matching the score ("Very weak" → "Excellent"). */
  label: string;
  /**
   * Human crack-time line: "less than a second", "3 days", "centuries".
   * Modelled against a slow offline attacker (Argon2id-class).
   */
  crackTimeDisplay: string;
  /** One-line description of the main weakness (low scores only). */
  warning: string | null;
  /** Up to ~3 suggestions; UI shows the first. */
  suggestions: string[];
  /** True when both the length floor and score threshold are cleared. */
  meetsPolicy: boolean;
  /** True iff input is at least `MIN_LENGTH` characters. */
  longEnough: boolean;
};

// ---------------------------------------------------------------------------
// OpenAPI import / export — mirrors the IPC DTOs in commands/openapi.rs.
// ---------------------------------------------------------------------------

export type ImportRow = {
  name: string;
  method: string;
  path: string;
};

export type ImportResult = {
  collectionId: CollectionId;
  collectionName: string;
  requestCount: number;
  environmentCount: number;
  warningCount: number;
  rows: ImportRow[];
  warnings: string[];
  logText: string;
  logFilename: string;
};

export type ExportRow = {
  name: string;
  method: string;
  path: string;
  /** Number of secrets stripped from this request before export. */
  redactions: number;
};

export type ExportResult = {
  /** The OpenAPI 3.0.3 document, ready to write to disk. */
  yaml: string;
  yamlFilename: string;
  requestCount: number;
  redactionCount: number;
  warningCount: number;
  rows: ExportRow[];
  warnings: string[];
  logText: string;
  logFilename: string;
};

export type ExecutionResult = {
  status: number;
  statusText: string;
  headers: HeaderEntry[];
  contentType: string | null;
  bodyKind: ResponseBodyKind;
  /** UTF-8 text or base64 of binary; empty if `bodyKind === "empty"`. */
  body: string;
  bodyTruncated: boolean;
  durationMs: number;
  sizeBytes: number;
  finalUrl: string;
  /** Snapshot of what actually went on the wire — debug aid for auth/redirect issues. */
  sent: SentRequest;
};
