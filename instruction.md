# Apiovnia — Implementation Brief

## Tech Stack (zdecydowane, nie negocjowalne na start)
- **Tauri 2.x** (nie 1.x — 2.x ma stabilne API, lepsze pluginy, mobile-ready jakbyśmy 
  kiedyś chcieli)
- **Svelte 5** z **runes API** ($state, $derived, $effect) — NIE Svelte 4 stores 
  jako default. Stores tylko gdy naprawdę cross-component shared state.
- **Vite** standalone (nie SvelteKit — desktop app, SSR zbędne)
- **Tailwind v4** (nowy engine, CSS-first config przez @theme)
- **shadcn-svelte** dla bazowych komponentów (Button, Dialog, Tabs, etc.)
- **SQLite** przez `tauri-plugin-sql`
- **Rust** dla backendu Tauri — HTTP klient: `reqwest`, JSON: `serde_json`,
  szyfrowanie: `aes-gcm` + `argon2`

## Filozofia kodu
1. **Solo-dev tool, lokalne dane.** Żadnego sync, żadnego cloud, żadnych user accounts.
2. **SQLite jako single source of truth.** Pliki OpenAPI tylko import/export (best-effort export).
3. **Master-password opcjonalny per environment.** Plain text dla dev/stage default, 
   szyfrowanie wymuszone tam gdzie user oznaczy `requires_unlock: true`.
4. **Klawiaturowo first.** Cmd+K palette, Cmd+Enter send, Cmd+N new request, Cmd+1/2/3 switch panels.
5. **Małe, testowalne moduły.** Rust crates per domain (auth, env, http, storage).

## Model danych (SQLite schema, pierwsza wersja)

```sql
CREATE TABLE projects (
    id TEXT PRIMARY KEY,           -- UUID
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE collections (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE requests (
    id TEXT PRIMARY KEY,
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    method TEXT NOT NULL,          -- GET, POST, etc.
    url TEXT NOT NULL,
    headers_json TEXT NOT NULL DEFAULT '[]',     -- array of {key, value, enabled}
    params_json TEXT NOT NULL DEFAULT '[]',
    body_type TEXT,                -- 'json' | 'form' | 'raw' | 'none'
    body_content TEXT,
    auth_json TEXT,                -- {type: 'bearer'|'basic'|'apikey'|'none', ...}
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE environments (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,            -- 'dev', 'stage', 'prod', custom...
    requires_unlock INTEGER NOT NULL DEFAULT 0,
    is_encrypted INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

-- KLUCZOWA TABELA: patche na requesty per env
CREATE TABLE request_env_overrides (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL REFERENCES requests(id) ON DELETE CASCADE,
    environment_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    -- Każde pole opcjonalne. NULL = używaj wartości bazowej.
    method_override TEXT,
    url_override TEXT,
    headers_override_json TEXT,    -- pełny replacement headers
    params_override_json TEXT,
    body_type_override TEXT,
    body_content_override TEXT,
    auth_override_json TEXT,       -- ENCRYPTED jeśli env.requires_unlock=1
    UNIQUE(request_id, environment_id)
);

CREATE TABLE request_history (
    id TEXT PRIMARY KEY,
    request_id TEXT REFERENCES requests(id) ON DELETE SET NULL,
    environment_id TEXT,
    executed_at INTEGER NOT NULL,
    status_code INTEGER,
    duration_ms INTEGER,
    response_size_bytes INTEGER,
    response_headers_json TEXT,
    response_body TEXT,            -- może być duży, rozważyć limit/truncate
    error_message TEXT
);
```

## Architektura Rust (Tauri backend)

Crates wewnętrzne (workspace):
- `apiovnia-core` — modele domenowe, zero zależności od Tauri
- `apiovnia-storage` — SQLite layer, migracje, queries
- `apiovnia-http` — wykonywanie requestów (reqwest wrapper)
- `apiovnia-crypto` — AES-GCM + Argon2id, master password
- `apiovnia-openapi` — import/export OpenAPI 3.x
- `apiovnia-tauri` — Tauri commands (cienka warstwa, tylko ekspozycja)

Tauri commands (lista MVP):
- Projects: create_project, list_projects, rename_project, delete_project
- Collections: create_collection, list_collections, rename, delete
- Requests: create_request, get_request, update_request, delete_request, list_in_collection
- Environments: create_env, list_envs, delete_env, set_active_env
- Overrides: upsert_override, get_overrides_for_request, delete_override
- Execution: execute_request(request_id, env_id) → ExecutionResult
- Crypto: unlock_env(env_id, password), lock_env(env_id), is_env_unlocked
- OpenAPI: import_openapi(file_path, project_id), export_collection(collection_id, file_path)

## Resolver — kluczowa logika

Funkcja `resolve_request(base: Request, override: Option<EnvOverride>) -> ResolvedRequest`:
- Każde pole: jeśli override ma `Some(value)` → użyj override; jeśli `None` → użyj base.
- Headers/params: override JEŻELI present, ZASTĘPUJE całą listę (nie merge per-key, bo to 
  rodzi confusing edge cases). User może łatwo skopiować bazową listę i edytować — UI to ułatwi.
- Auth: zastępowane w całości (typowy case: dev = no auth, prod = bearer token).
- To musi być pure function, idealnie w `apiovnia-core`, łatwa do testów.

## Crypto — master password

Per environment z `requires_unlock=1`:
1. User ustawia hasło raz przy oznaczaniu env jako encrypted
2. Z hasła + losowy salt (zapisany razem z env) derywujemy klucz: Argon2id (params: 
   m_cost=19456, t_cost=2, p_cost=1 — OWASP 2024 baseline)
3. Każda zaszyfrowana wartość: AES-256-GCM z losowym nonce per encryption
4. Format storage: `base64(salt || nonce || ciphertext || tag)` w polu *_override_json
5. Session: po unlock, klucz trzymany w `Arc<RwLock<HashMap<EnvId, Key>>>` w stanie Tauri,
   wyczyszczony przy lock_env lub close app

WAŻNE: nigdy nie loguj odszyfrowanych wartości. Nigdy nie wysyłaj klucza do frontu — 
deszyfracja zawsze po stronie Rust, frontend dostaje gotową ResolvedRequest.

## OpenAPI import — pierwsza wersja

Tylko OpenAPI 3.0 i 3.1. Crate `openapiv3` lub `oas3`.
Mapping:
- info.title → nowa Collection name
- paths → Requesty (path + method = jeden Request)
- parameters → params/headers
- requestBody example/examples → body_content
- securitySchemes → auth_json (best effort: http+bearer → bearer, http+basic → basic, 
  apiKey → apikey, oauth2 → import jako manual TODO header)
- servers → tworzy domyślne environments (server[0]=dev, server[1]=stage, ...) z odpowiednim 
  url_override per env. To realizuje feel "env nadpisuje URL".

Export: best effort, dokumentujemy że nie round-trippuje wszystkiego. Pomijamy: auth secrets,
history, env-specific overrides poza URL.

## Frontend struktura (Svelte 5)
src/
routes/
+page.svelte              # Główny shell (trzy panele)
lib/
components/
panels/
ProjectsPanel.svelte
RequestsPanel.svelte
DetailPanel.svelte
request/
UrlBar.svelte
TabsParams.svelte
TabsHeaders.svelte
TabsBody.svelte
TabsAuth.svelte
TabsEnvOverrides.svelte
response/
ResponseViewer.svelte
JsonPretty.svelte
ResponseHeaders.svelte
modals/
UnlockEnvModal.svelte
CommandPalette.svelte
ui/                     # shadcn-svelte komponenty
stores/
app.svelte.ts           # $state runes: activeProjectId, activeRequestId, activeEnvId
panels.svelte.ts        # rozmiary paneli, persistowane do localStorage
api/
ipc.ts                  # wrappery Tauri invoke()
types/
domain.ts               # TS types odpowiadające Rust modelom

Stan: jeden globalny `app.svelte.ts` z runes, eksportuje getters i actions. Komponenty 
robią `import { app } from '$lib/stores/app.svelte.ts'` i czytają `app.activeRequest` 
reaktywnie.

## Plan implementacji w fazach (sugerowane PRs)

**PR 1 — Setup & shell** (1-2h)
- Tauri 2 + Svelte 5 + Tailwind v4 + shadcn-svelte
- Trójpanelowy layout z hardcoded contentem
- Resizery paneli + persistence do localStorage
- Dark mode jako default

**PR 2 — Storage layer**
- Tauri-plugin-sql + migracje
- CRUD commands: projects, collections, requests
- Frontend listy w lewym i środkowym panelu, live z DB

**PR 3 — Request editor**
- URL bar + method selector
- Tabs: Params, Headers, Body, Auth (bez Env Overrides jeszcze)
- Save on blur / debounced

**PR 4 — HTTP execution**
- Rust: execute_request command (reqwest)
- Frontend: Send button, loading state, error handling
- Response viewer: Raw mode + status/timing

**PR 5 — Pretty JSON response**
- JSON parser + syntax highlighting
- Collapsible nodes
- Search w response
- Copy values

**PR 6 — Environments + Overrides** (core feature)
- Schema env + overrides
- UI Env Overrides tab
- Resolver function w Rust
- Env selector w prawym panelu

**PR 7 — Crypto / Master password**
- Argon2id + AES-GCM
- Unlock modal
- Session key management
- Mark env as encrypted

**PR 8 — OpenAPI import**
- Import flow z file picker
- Mapowanie OpenAPI → collection + requests + envs
- Error handling dla niepoprawnych speców

**PR 9 — Cmd palette + skróty**
- Wszystkie shortcuts
- Cmd+K z fuzzy search po requestach

**PR 10 — Polish**
- History panel
- Export OpenAPI (best effort)
- Empty states
- Onboarding (puste DB → "Create your first project")

## Testy

Rust: unit testy dla resolvera i crypto OBOWIĄZKOWE. Storage: integration testy z 
:memory: SQLite. HTTP: skip dla MVP (i tak głównie reqwest).

Frontend: testy tylko dla resolvera/transformerów po stronie TS jeśli takie powstaną. 
Komponenty: skip dla MVP. Jeśli kiedyś dodamy: Vitest + @testing-library/svelte.

## Co świadomie pomijamy w MVP
- WebSocket / Server-Sent Events / gRPC (REST only, zgodnie z briefem)
- GraphQL
- Pre-request scripts (to droga do Postmana, nie chcemy)
- Test assertions na response (może w v2)
- Sync, sharing, team features (filozofia produktu)
- Mobile (Tauri 2 to wspiera, ale później)

## Definicja "done" dla MVP (przed pierwszym release)
1. Mogę stworzyć projekt → kolekcję → request, wykonać go, zobaczyć ładny response
2. Mogę zdefiniować env "prod" z innym URL i auth, przełączyć się, wykonać request z patchem
3. Mogę zablokować env "prod" hasłem, restartować app, odblokować
4. Mogę zaimportować OpenAPI 3 i mieć działający zestaw requestów
5. Trzy panele resizable, layout się zapisuje
6. Skróty klawiaturowe działają
7. Działa na macOS i Linux (Windows nice-to-have na start jeśli czasu mało)
