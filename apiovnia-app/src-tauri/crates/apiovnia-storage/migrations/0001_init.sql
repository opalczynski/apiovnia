-- Apiovnia initial schema.
--
-- We deliberately create every table the MVP needs in a single migration so
-- the early phases don't have to amend it. Subsequent migrations stay
-- additive (new tables / new nullable columns only).

PRAGMA foreign_keys = ON;

-- ---------------------------------------------------------------------------
-- Projects · Collections · Requests
-- ---------------------------------------------------------------------------

CREATE TABLE projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_projects_sort ON projects(sort_order);

CREATE TABLE collections (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
CREATE INDEX idx_collections_project ON collections(project_id, sort_order);

CREATE TABLE requests (
    id              TEXT PRIMARY KEY,
    collection_id   TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    method          TEXT NOT NULL,
    url             TEXT NOT NULL,
    headers_json    TEXT NOT NULL DEFAULT '[]',
    params_json     TEXT NOT NULL DEFAULT '[]',
    body_type       TEXT NOT NULL DEFAULT 'none',
    body_content    TEXT NOT NULL DEFAULT '',
    auth_json       TEXT NOT NULL DEFAULT '{"type":"none"}',
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
CREATE INDEX idx_requests_collection ON requests(collection_id, sort_order);

-- ---------------------------------------------------------------------------
-- Environments · Variables · Overrides
-- ---------------------------------------------------------------------------

CREATE TABLE environments (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    requires_unlock INTEGER NOT NULL DEFAULT 0,
    is_encrypted    INTEGER NOT NULL DEFAULT 0,
    -- Filled in Phase 6 once the env is marked encrypted.
    salt            BLOB,
    password_check  BLOB,
    created_at      INTEGER NOT NULL,
    UNIQUE (project_id, name)
);
CREATE INDEX idx_environments_project ON environments(project_id);

-- Named variables consumable as `{{name}}` in URL/headers/body/auth.
-- Value is plaintext when env.is_encrypted = 0, ciphertext otherwise.
CREATE TABLE environment_variables (
    id              TEXT PRIMARY KEY,
    environment_id  TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    value           TEXT NOT NULL,
    is_secret       INTEGER NOT NULL DEFAULT 0,
    UNIQUE (environment_id, name)
);
CREATE INDEX idx_envvars_env ON environment_variables(environment_id);

-- Per-request patches applied when a given env is active. Every column
-- bar (request_id, environment_id) is nullable: NULL means "inherit base".
CREATE TABLE request_env_overrides (
    id                      TEXT PRIMARY KEY,
    request_id              TEXT NOT NULL REFERENCES requests(id) ON DELETE CASCADE,
    environment_id          TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    method_override         TEXT,
    url_override            TEXT,
    headers_override_json   TEXT,
    params_override_json    TEXT,
    body_type_override      TEXT,
    body_content_override   TEXT,
    auth_override_json      TEXT,
    UNIQUE (request_id, environment_id)
);
CREATE INDEX idx_overrides_request ON request_env_overrides(request_id);

-- ---------------------------------------------------------------------------
-- History
-- ---------------------------------------------------------------------------

CREATE TABLE request_history (
    id                      TEXT PRIMARY KEY,
    request_id              TEXT REFERENCES requests(id) ON DELETE SET NULL,
    environment_id          TEXT,
    executed_at             INTEGER NOT NULL,
    status_code             INTEGER,
    duration_ms             INTEGER,
    response_size_bytes     INTEGER,
    response_headers_json   TEXT,
    response_body           TEXT,
    error_message           TEXT
);
CREATE INDEX idx_history_request ON request_history(request_id, executed_at DESC);
