# Apiovnia — Plan implementacji

Plan oparty na `instruction.md` i `design_artifacts/`. Każda faza kończy się czymś, co da się odpalić, kliknąć i zweryfikować — żadnych "wewnętrznych" faz, które nie produkują widocznego efektu. Estymaty są orientacyjne dla solo-dev.

Konwencja:
- **Cel** — co po fazie działa.
- **Backend** — Rust crates + Tauri commands.
- **Frontend** — Svelte components + store.
- **Schemat / migracje** — co trafia do SQLite.
- **Acceptance** — checkpointy do potwierdzenia, że faza skończona.

---

## Faza 0 — Bootstrap & shell wizualny (PR 1)

**Cel:** odpalalny `cargo tauri dev`, trójpanelowy layout w tym samym dark-mode look co design artifacts, ale z hardcoded contentem. Zero DB, zero backendu poza pustym Tauri.

### Setup
- `cargo create-tauri-app` → wybór: Vite + Svelte + TS, Rust workspace.
- Restrukturyzacja na workspace cargo:
  ```
  src-tauri/
    Cargo.toml                   # workspace root
    crates/
      apiovnia-core/             # puste lib + Cargo.toml
      apiovnia-storage/
      apiovnia-http/
      apiovnia-crypto/
      apiovnia-openapi/
    src/                         # apiovnia-tauri (binary)
      main.rs
  ```
- Dodać `tauri-plugin-sql`, `tauri-plugin-fs`, `tauri-plugin-dialog` w `Cargo.toml` (na zapas — używane od fazy 1).
- Vite config: alias `$lib` → `src/lib`.
- Tailwind v4 + `@theme` z tokenami z `design_artifacts/tokens.css` (przepisać do CSS-first config; zachować nazwy zmiennych `--bg`, `--surface`, `--accent`, `--m-get`, etc.).
- shadcn-svelte init (Button, Input, Tabs, Dialog, Tooltip, Command, Resizable).
- Inter + JetBrains Mono — albo via `@fontsource/...` (offline-friendly desktop), albo lokalne pliki w `src/assets/fonts/`.

### Layout
- `src/routes/+page.svelte` (singleton — desktop, nie SPA).
- `lib/components/layout/`:
  - `TitleBar.svelte` — wzór z `artifact-shell.jsx` (TrafficLights + breadcrumb + ⌘K hint).
  - `ThreePanelLayout.svelte` — `Resizable` z shadcn-svelte (lub własny resizer 4px), 3 panele (lewy 240px, środkowy 280px, prawy `flex-1`).
- Persistencja szerokości paneli do `localStorage` przez `panels.svelte.ts`.
- Dark mode jako jedyny mode (na MVP — light skip).

### Acceptance
- `pnpm tauri dev` otwiera natywne okno.
- Trzy panele resizable myszą, rozmiary po restarcie zachowane.
- Wygląd 1:1 z `artifact-shell.jsx` (puste rzędy + placeholder content w panelach).

---

## Faza 1 — Storage layer + projekty/kolekcje/requesty na żywo (PR 2)

**Cel:** lewy i środkowy panel czytają z SQLite. Można z UI dodać/zmienić nazwę/usunąć projekt, kolekcję, request. Edytor w prawym panelu nadal placeholder.

### Backend
- `apiovnia-core`:
  - struktury `Project`, `Collection`, `Request`, `Environment`, `EnvOverride`, `HttpMethod`, `BodyType`, `AuthConfig`, `KeyValue { key, value, enabled }`.
  - serde-friendly (camelCase do TS, snake_case w Rust).
  - **Zero zależności od Tauri/SQL** — czysty domain.
- `apiovnia-storage`:
  - `Db` wrapper na `sqlx::SqlitePool` (lub `rusqlite` jeśli nie chcemy async — preferuję sqlx, gra ładnie z tokio Tauri).
  - Migracje w `migrations/0001_init.sql` zgodne ze schematem z `instruction.md` (wszystkie tabele od razu, nawet jeśli używamy ich dopiero w późniejszych fazach — jedna migracja na MVP, mniej drama).
  - Repos: `ProjectRepo`, `CollectionRepo`, `RequestRepo` z metodami `list`, `get`, `create`, `update`, `delete`, `reorder`.
- `apiovnia-tauri` commands (cienkie, każdy <20 linii):
  - `create_project(name)`, `list_projects()`, `rename_project(id, name)`, `delete_project(id)`, `reorder_projects(ids)`
  - to samo dla `collection` (z `project_id`)
  - to samo dla `request` (z `collection_id`, ale na razie tylko `name`/`method`/`url` minimum, reszta default)
- Db path: `app_data_dir()/apiovnia.db` przez `tauri::path::PathResolver`.

### Frontend
- `lib/api/ipc.ts` — typowane wrappery na `invoke<T>`. Jeden plik, jedna funkcja per command.
- `lib/types/domain.ts` — TS mirror modeli z `apiovnia-core` (na razie ręcznie; **nie** generujemy z `ts-rs` na MVP, za dużo ceremonii).
- `lib/stores/app.svelte.ts` — runes `$state`:
  - `activeProjectId`, `activeCollectionId`, `activeRequestId`, `activeEnvId`
  - actions: `selectProject(id)`, `loadProjects()`, etc.
- `lib/components/panels/`:
  - `ProjectsPanel.svelte` — projekty + kolekcje (z sekcjami "Projects" i "Collections · {name}" jak w shellu), `+` button → inline rename / dialog.
  - `RequestsPanel.svelte` — lista requestów aktywnej kolekcji, MethodBadge, tytuł + URL podtytuł (matching design `MidPanel`).
- Drag-to-reorder w v2 — na MVP `↑↓` w kontekście (skip dla teraz, tylko `sort_order` ustawiamy timestampem przy create).

### Acceptance
- Świeża baza → empty state "Create your first project".
- Po klliku "+" mogę dodać projekt, dodać kolekcję, dodać request — wszystko persistent po restarcie.
- Usuwanie kaskadowo działa (delete project → kolekcje + requesty znikają — sprawdzić cascade w sqlite).
- `pnpm tauri dev` → operacje na liście są <50ms.

---

## Faza 2 — Edytor requestu (PR 3)

**Cel:** klikam request w środkowym panelu, w prawym widzę pełny edytor (URL bar + Method + 5 zakładek bez Env Overrides). Edycje zapisują się automatycznie (debounce na blur). Żadnego wykonywania jeszcze.

### Backend
- Rozszerzenie `update_request(id, patch)` o pełne pola: headers, params, body_type, body_content, auth.
- `get_request(id)` zwraca pełny `Request` z `headers`/`params` zdeserializowanym z JSON.

### Frontend
- `lib/components/request/`:
  - `UrlBar.svelte` — Method dropdown (shadcn Select), URL input z monospace, kolorowanie `{{var}}` jako `--accent` (regex highlight w spanach pod inputem albo CodeMirror — dla MVP **prosty contenteditable** z highlightem syntaktycznym, bez pełnego edytora). Send button wyłączony jeszcze.
  - `RequestTabs.svelte` — wrapper na `Tabs` z shadcn, taby: Params, Headers, Body, Auth, Env Overrides (disabled), Tests (disabled "soon").
  - `TabsParams.svelte` / `TabsHeaders.svelte` — tabela `key | value | enabled (checkbox) | × delete`, `+ Add` na dole. Reusable component `KeyValueTable.svelte`.
  - `TabsBody.svelte` — segmentowany picker (JSON / Form / Raw / None), niżej:
    - JSON / Raw → CodeMirror 6 (`@codemirror/lang-json`) z dark theme matchującym `--j-*` tokeny.
    - Form → `KeyValueTable` reuse.
    - "Beautify" button dla JSON.
  - `TabsAuth.svelte` — Type select (None / Bearer / Basic / API Key) + odpowiadające pola.
- Save flow:
  - W każdym komponencie `$effect` na zmianę → debounce 400ms → `ipc.updateRequest(id, patch)`.
  - Globalny "saved 2s ago" w footerze (cicha informacja, nic blokującego).

### Acceptance
- Wybieram request → wszystkie pola wczytane.
- Edytuję URL, headers, body → po 400ms zapisane, po restarcie te same wartości.
- Przełączenie requestu w środkowym panelu nie gubi pending changes (force-flush przed switchem).
- `{{base_url}}` w URL podświetla się amber.

---

## Faza 3 — HTTP execution + Response (raw) (PR 4)

**Cel:** Send button działa. Response widoczny jako raw text + status/timing/headers. Bez resolvera env jeszcze (env_id == None == base values).

### Backend
- `apiovnia-http`:
  - `Executor` na `reqwest::Client` (jeden globalny + redirect policy + timeout 30s).
  - `execute(resolved: ResolvedRequest) -> Result<ExecutionResult, ExecutionError>`.
  - `ExecutionResult { status, headers, body_bytes, duration_ms, size_bytes }`.
  - Body decoding: JSON content-type → string, inne → base64 (zwracamy też `is_binary`).
- `apiovnia-core`:
  - `ResolvedRequest` (na razie po prostu `From<Request>` — resolver przyjdzie w fazie 5).
  - Substytucja `{{var}}` jako pure function `interpolate(template: &str, ctx: &VarContext) -> String`, na MVP ctx pusty (zostawiamy literal `{{var}}` jeśli brak — z warningiem do logu) **albo** zostawiamy substytucję na fazę 5 (env vars dopiero z env). → **Decyzja: bez interpolacji w fazie 3**, dopiero w fazie 5 razem z env.
- Tauri command: `execute_request(request_id, env_id: Option<String>) -> ExecutionResult` (env_id ignorowany teraz).
- Zapis do `request_history` przy każdym wykonaniu (truncate body do 2 MB w MVP).

### Frontend
- `UrlBar`: Send button enabled, disabled state podczas in-flight (`$state isExecuting`).
- `lib/components/response/`:
  - `ResponseHeader.svelte` — status pill (`.ap-status.ok/warn/err` w zależności od kodu), time, size, content-type, taby (Pretty/Raw/Headers/Preview — Pretty na razie === Raw).
  - `ResponseRaw.svelte` — `<pre>` z body.
  - `ResponseHeaders.svelte` — tabela key/value.
- Splitter horyzontalny w prawym panelu (request góra / response dół), persistowany.
- Skrót: Cmd+Enter → Send.

### Acceptance
- Robię request do `https://httpbin.org/get` → widzę 200, body, headers, czas.
- Błąd sieciowy → status pill `err`, body pokazuje error message.
- 2 MB response nie wiesza UI (truncate + "showing first 2 MB" badge).

---

## Faza 4 — Pretty JSON viewer (PR 5)

**Cel:** Zakładka Pretty pokazuje response jak na `artifact-response.jsx` — collapsible, syntax highlight, search ⌘F, hover-copy.

### Frontend (zero backendu)
- `lib/components/response/JsonView.svelte`:
  - Parser: native `JSON.parse`, własny renderer (port `json-view.jsx` z artefaktów).
  - State: `$state` per ścieżka (`Set<JsonPath>`) dla collapsed.
  - Numeracja linii, `j-key`/`j-string`/`j-number`/`j-bool`/`j-null` klasy z tokens.
  - Hover: copy button → `navigator.clipboard.writeText(value)`.
- `JsonSearch.svelte`:
  - Input w toolbarze, po Enter → highlight matchy (`<mark>`), nawigacja Next/Prev (n/N), "1/3" counter.
  - Skrót: ⌘F gdy focus w response panelu.
- "Collapse all" / "Expand all" buttons.
- Toggle Tree / JSON / Schema — na MVP tylko **JSON tab działa**, Tree i Schema disabled "soon".

### Acceptance
- Response z fazy 3 wyświetla się ładnie w Pretty.
- Kliknięcie strzałki obok `{` zwija obiekt do `{ … 8 keys }`.
- ⌘F → input focus → wpisuję "user" → highlight + 1/3.
- Hover na value → copy icon → schowek.

---

## Faza 5 — Environments + Overrides + Resolver (PR 6, **core feature**)

**Cel:** definiuję env "dev"/"stage"/"prod", ustawiam w nich override URL/headers/body, przełączam env w prawym górnym rogu, Send wykonuje request z patchem. Visual zgodny z `artifact-env.jsx`.

### Backend
- `apiovnia-core`:
  - `EnvOverride { id, request_id, env_id, method_override, url_override, headers_override, params_override, body_type_override, body_content_override, auth_override }` — wszystkie `Option<...>`.
  - `resolve_request(base: &Request, override: Option<&EnvOverride>) -> ResolvedRequest`:
    - Pole-po-polu: `override.x.unwrap_or(base.x.clone())`.
    - Headers/params: replacement, **nie** merge per-key (zgodnie z briefem).
    - **Pure function**, w `apiovnia-core/src/resolver.rs`.
  - Variable interpolation: `interpolate(template, vars: &HashMap<String, String>) -> String`. Source of vars na MVP: `Environment` ma TODO `variables_json` (pomijamy w schemie, bo brief tego nie miał — zamiast tego `{{base_url}}` wpisuje user w URL bezpośrednio, a env override podmienia całą wartość URL).
  - **Tests obowiązkowe** (z briefu): pełne pokrycie kombinacji "all None / some Some / all Some" + headers replace.
- `apiovnia-storage`:
  - `EnvironmentRepo`, `OverrideRepo`.
  - `upsert_override(request_id, env_id, patch)`, `get_overrides_for_request(request_id) -> Vec<EnvOverride>`.
- Tauri commands:
  - `create_env(project_id, name)`, `list_envs(project_id)`, `delete_env(id)`.
  - `set_active_env(env_id)` — zapis do `app_state` (mała tabela `kv` na app preferences) lub po prostu w localStorage frontu, **wybieram localStorage**, mniej DB ruchu.
  - `upsert_override(request_id, env_id, patch)`, `get_overrides_for_request(request_id)`, `delete_override(id)`.
  - `execute_request(request_id, env_id)` — TERAZ używa resolvera.

### Frontend
- `lib/components/request/EnvSelector.svelte` — dropdown w UrlBar (po prawej Send), kolorowa kropka per env (zielona dev, żółta stage, czerwona prod), `🔒` jeśli encrypted+locked (faza 6).
- `lib/components/request/TabsEnvOverrides.svelte` — port `artifact-env.jsx`:
  - Env switcher row z `EnvTab`-ami (dev/stage/prod/custom).
  - Sekcje: Request (URL, method), Headers, Query params, Body fields.
  - `OverrideField` component:
    - Lewa kolumna: label + status ("inherits base" / "overridden in dev" + amber dot).
    - Środkowa kolumna: read-only base value (greyed).
    - Prawa kolumna: editable override (placeholder "override for dev…", border amber gdy filled).
    - `×` button → clear override (ustawia pole na NULL).
  - Top-right: licznik "N overrides in this env" + "Reset all in {env}".
- "Add field override" — dropdown z polami które można override'ować (URL, method, każdy header z base, każdy param z base, "custom header", "custom body field path").

### Acceptance
- Tworzę env "prod", w request "Login" override URL na `https://api.prod.com/auth/login`.
- Przełączam env na prod, Send → request leci na prod URL.
- Przełączam na dev → leci na base URL.
- Resolver tests: `cargo test -p apiovnia-core` zielono.
- Override URL ma amber dot + amber border w UI; po `×` znika.

---

## Faza 6 — Crypto / master password (PR 7)

**Cel:** mogę oznaczyć env jako `requires_unlock=1`, ustawić master password, encryptować wartości override. Po restarcie próba użycia env → modal unlock (matching `artifact-unlock.jsx`).

### Backend
- `apiovnia-crypto`:
  - `derive_key(password: &str, salt: &[u8]) -> [u8; 32]` — Argon2id, m_cost=19456, t_cost=2, p_cost=1 (z briefu).
  - `encrypt(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8>` — AES-256-GCM, random nonce, format `salt(16) || nonce(12) || ciphertext || tag(16)`. **Salt per env, nie per pole** — salt przechowujemy z env, format ciphertext: `nonce(12) || ciphertext || tag(16)` (16-bajtowy salt już w env table).
  - `decrypt(ciphertext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, CryptoError>`.
  - Pełne testy (round-trip, wrong key, tampered tag).
- Schema dodatki:
  - Migracja `0002_crypto.sql`: `ALTER TABLE environments ADD COLUMN salt BLOB`, `password_check BLOB` (encrypted "verification" string do walidacji hasła).
- `SessionKeyStore` w state Tauri: `Arc<RwLock<HashMap<EnvId, [u8; 32]>>>`. Klucz wpisywany przy `unlock_env`, usuwany przy `lock_env` lub na `app_quit`.
- Tauri commands:
  - `set_env_encrypted(env_id, password)` — generuje salt, derywuje klucz, zapisuje `password_check`.
  - `unlock_env(env_id, password) -> Result<(), CryptoError>` — derywuje klucz, weryfikuje przez `password_check`, wstawia do store.
  - `lock_env(env_id)` — usuwa ze store.
  - `is_env_unlocked(env_id) -> bool`.
- Modyfikacja `upsert_override`: jeśli `env.is_encrypted == 1`, każde pole z sekretem (na MVP: `auth_override_json` + każdy header zaczynający się od `Authorization`/`X-API-Key` — albo prościej: WSZYSTKIE override'y w encrypted env są szyfrowane) → encryptujemy przed zapisem.
- Modyfikacja `execute_request`: jeśli env encrypted → wyciągamy klucz ze store → deszyfrujemy override → resolver → execute. Jeśli klucza brak → zwracamy `ExecutionError::EnvLocked(env_id)`.
- **Frontend nigdy nie widzi plaintext sekretów** ani klucza.

### Frontend
- `lib/components/modals/UnlockEnvModal.svelte` — port `artifact-unlock.jsx` (header z ikoną kłódki, env summary card, password input, "Remember for this session" checkbox, Cancel/Unlock).
- `EnvSelector` pokazuje 🔒 obok zablokowanego env.
- Try-execute z lockowanym env → backend zwraca `EnvLocked` → frontend otwiera modal → po sukcesie ponawia execute.
- Settings (per env): "Mark as encrypted (requires master password)" toggle → flow ustawienia hasła (modal z confirm).
- W liście override'ów: encrypted wartości pokazują się jako `••••••••` (placeholder). Edycja możliwa tylko przy unlocked.

### Acceptance
- Ustawiam env "prod" jako encrypted, ustawiam hasło "test123".
- Override Authorization na "Bearer xyz" → w SQLite (sprawdzić sqlite cli) wartość zaszyfrowana, nie plaintext.
- Restart app → kliknięcie env "prod" → modal "Unlock production environment" → wpisuję "test123" → unlock OK → Send → request leci z bearerem.
- Wpisuję złe hasło → "Invalid password" inline error.
- `cargo test -p apiovnia-crypto` zielono (round-trip, wrong-key, tamper).

---

## Faza 7 — OpenAPI import (PR 8)

**Cel:** klikam "Import OpenAPI", wybieram plik `.yaml`/`.json`, dostaję nową kolekcję z requestami + envami z `servers`.

### Backend
- `apiovnia-openapi`:
  - Crate `oas3` (lepsze API niż `openapiv3`).
  - `parse(path) -> Result<OpenApiDoc, ImportError>`.
  - `to_collection(doc, project_id) -> (Collection, Vec<Request>, Vec<Environment>, Vec<EnvOverride>)`:
    - `info.title` → Collection name.
    - `paths × methods` → Requesty (`name = operation.summary || method + path`).
    - `parameters` (in: query/header/path) → params/headers (path params jako `{var}` w URL, na razie zostawiamy jako placeholder).
    - `requestBody.content["application/json"].example` → body_content (JSON pretty).
    - `securitySchemes` → auth (bearer/basic/apikey; oauth2 → komentarz "// OAuth: setup manually").
    - `servers[]` → tworzy environments `dev`, `stage`, `prod` (lub użyj `description`/`x-env-name` jeśli jest), z `url_override` per request (każdy request ma override URL = `server.url + path`).
- Tauri command: `import_openapi(file_path, project_id) -> ImportResult { collection_id, request_count, env_count, warnings: Vec<String> }`.

### Frontend
- "Import OpenAPI" w menu projektu (kontekstowe `…` na projekcie).
- `tauri-plugin-dialog` → file picker.
- Po imporcie: toast "Imported 23 requests into UDL · 3 envs created" + rozwinięta nowa kolekcja.
- Warningi → mały panel "Import notes" z listą.
- **Export** OpenAPI — best effort, w fazie 9.

### Acceptance
- Import petstore.yaml (standardowy sample) → kolekcja "Swagger Petstore" z requestami.
- Każdy request ma URL z placeholdera `{{base_url}}` lub konkretnym URL z `servers[0]`.
- Switch envu zmienia URL.
- Import wadliwego YAML → user-friendly error, bez crasha.

---

## Faza 8 — Cmd palette + skróty (PR 9)

**Cel:** ⌘K otwiera fuzzy search po wszystkich requestach, ⌘N nowy request, ⌘1/2/3 focus na panel, ⌘Enter Send (już z fazy 3), Esc zamyka modal.

### Frontend
- `lib/components/modals/CommandPalette.svelte` — shadcn `Command` (cmdk pod spodem).
- Sources: requesty (z badgem method + collection name), kolekcje, envy, akcje ("New request", "New project", "Settings").
- Fuzzy: cmdk built-in.
- Otwierane ⌘K, zamykane Esc.
- Globalny `lib/keymap.ts` — registracja w `+page.svelte` przez jeden `$effect` listener:
  - ⌘K → open palette
  - ⌘Enter → Send (jeśli request aktywny)
  - ⌘N → new request w aktywnej kolekcji
  - ⌘1/2/3 → focus panel
  - ⌘P → focus filter w lewym panelu (z designu)
  - ⌘F → focus search w response (jeśli response pokazany)
- Cross-platform: `Cmd` na macOS, `Ctrl` na Linux/Windows. Detekcja przez `navigator.platform`.

### Acceptance
- ⌘K → palette → wpisuję "log" → znajduje "Login", Enter otwiera.
- ⌘N w widoku requestu → nowy request "Untitled" w aktualnej kolekcji.
- Esc zamyka modal, wraca focus tam, gdzie był.

---

## Faza 9 — Polish & release (PR 10)

**Cel:** MVP gotowy do daily use'a. History, export OpenAPI, empty states, packaging.

### History
- Tabela `request_history` już jest. Dodaj panel "History" otwarty z lewego dolnego rogu (ikona w `LeftPanel` footer już jest w designie).
- `HistoryPanel.svelte`: lista ostatnich N=200 wykonań (data, method, URL, status, duration), klik → otwiera response viewer z zapisaną odpowiedzią.

### Export OpenAPI
- `apiovnia-openapi::export(collection: &Collection, requests: &[Request]) -> String` (YAML).
- Best-effort: skip secrets, history, encrypted overrides. Dokumentuj w UI ("Export omits secrets and env-specific overrides").

### Empty states & onboarding
- Świeża baza: full-screen empty state w shellu — "Welcome to Apiovnia · Create your first project" z dużym CTA.
- Pusta kolekcja w środkowym panelu: "No requests yet · ⌘N to create".
- Pusty request (brak URL): inline hint w response area "Hit Send to see the response".

### Packaging
- `tauri.conf.json`: bundle dla macOS (.dmg + .app), Linux (.deb + .AppImage), Windows (.msi) — Windows nice-to-have.
- App icon (provide placeholder + TODO real icon).
- About dialog z wersją.
- Auto-update — **skip dla MVP** (single-user, ręczny download wystarczy).

### Acceptance — pełna lista z briefu (Definicja "done"):
1. ✅ Tworzę projekt → kolekcję → request, wykonuję, widzę response.
2. ✅ Definiuję env "prod" z innym URL i auth, przełączam, wykonuję z patchem.
3. ✅ Blokuję "prod" hasłem, restartuję, odblokowuję.
4. ✅ Importuję OpenAPI 3, mam działający zestaw requestów.
5. ✅ Trzy panele resizable, layout się zapisuje.
6. ✅ Skróty klawiaturowe działają.
7. ✅ Działa na macOS i Linux (Windows nice-to-have).

---

## Co NIE robimy w MVP (recap z briefu)
- WebSocket / SSE / gRPC / GraphQL.
- Pre-request scripts.
- Test assertions.
- Sync, sharing, team features.
- Mobile.
- Auto-updater.
- Tree / Schema view dla response (tylko Pretty + Raw + Headers + Preview).
- Drag-to-reorder requestów (sort by `sort_order` + `↑↓` w v2).
- Light mode.

---

## Ścieżka krytyczna (zależności między fazami)

```
0 ── 1 ── 2 ── 3 ── 4
          │    │
          │    └── 5 ── 6
          │              │
          └────── 7 ─────┤
                         │
                         8 ── 9
```

- Faza 4 (Pretty JSON) jest **niezależna** od 5/6/7 — można robić równolegle gdyby było więcej rąk.
- Faza 7 (OpenAPI) nie wymaga env/crypto, ale ich istnienie pomaga (tworzy envy z servers).
- Faza 8 (palette) wymaga, żeby istniały requesty (faza 1).

---

## Wzajemne sanity-checki

- **Po każdej fazie** uruchom: `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `pnpm check` (svelte-check), `pnpm tauri build` (czy się buduje).
- **Smoke test** po fazie 3, 5, 6, 9 — pełny manual run-through happy path.
- **DB migration sanity**: każda nowa migracja musi być addytywna; nie modyfikujemy istniejących migracji po wydaniu.

---

## Otwarte pytania do potwierdzenia (przed startem)

1. **`sqlx` vs `rusqlite`** — preferuję `sqlx` (async, kompiluje queries na etapie build). OK?
2. **CodeMirror vs własny edytor** — CodeMirror dla body/JSON, ale to ~100KB do bundle'a. Akceptowalne?
3. **Variable interpolation** — `{{var}}` zostawiamy tylko w URL (nie w body/headers) na MVP, czy wszędzie? Brief sugeruje "wszędzie" przez podświetlenie w designie — propozycja: **wszędzie**, ale source vars to **tylko env override URL** (nie ma osobnej tabeli `env_variables`). Czy potrzebujesz osobnych zmiennych env (typowo `{{base_url}}`, `{{api_key}}` jako nazwane)? Jeśli tak → faza 5 dostaje +tabela `environment_variables`.
4. **Encrypted scope** — w fazie 6 propozycja: w encrypted env **wszystkie override fields** są szyfrowane (proste). Alternatywa: per-field flag "is secret" (więcej UI). Wybieram wszystko-lub-nic per env, OK?
5. **OpenAPI library** — `oas3` vs `openapiv3`. `oas3` aktywniej rozwijane, OK?

---

**Czekam na akceptację / korekty / odpowiedzi na pytania powyżej zanim ruszamy z fazą 0.**

---

# Postęp realizacji

## Decyzje podjęte (z odpowiedzi na pytania powyżej)

1. **sqlx** ✓
2. **CodeMirror** ✓ — instaluję modularnie w fazie 2.
3. **Interpolacja `{{var}}` wszędzie** — URL, headers, auth, body. **Wymaga osobnej tabeli `environment_variables(env_id, name, value, is_secret)`** — już w schemacie 0001_init.sql. Source vars per env.
4. **Toggle encrypted per env** — user wybiera, gdy włączony → wszystko w env zaszyfrowane (whole-or-nothing).
5. **oas3** ✓

## Faza 0 — DONE ✅ (2026-05-15)

**Setup zrealizowany:**
- Tauri 2.11 scaffold (`apiovnia-app/`), wymieniony SvelteKit na czysty Vite + Svelte 5 z runes.
- Tailwind v4 + `@theme` block (tokeny `--color-bg`, `--color-accent`, etc. — port z `design_artifacts/tokens.css`).
- Self-hosted fonts (`@fontsource/inter`, `@fontsource/jetbrains-mono`).
- Rust workspace `apiovnia-app/src-tauri/` z 5 wewnętrznymi crate'ami: `apiovnia-core`, `apiovnia-storage`, `apiovnia-http`, `apiovnia-crypto`, `apiovnia-openapi`. Workspace dependencies wycentralizowane.
- Tauri binary przemianowany `apiovnia-tauri`, identifier `tech.trurl.apiovnia`, window 1400x900 dark mode, CSP set, `tauri-plugin-opener` usunięty.
- Layout: `TitleBar`, `ThreePanelLayout`, `Resizer` (vertical + horizontal), `panels.svelte.ts` store z localStorage persistence.
- Hardcoded panele: `ProjectsPanel` (3 projekty + 5 kolekcji), `RequestsPanel` (8 requestów Auth), `DetailPanel` (URL bar + tabs + body editor + response viewer — wszystko statyczne).
- Komponenty primitive: `Icon.svelte`, `MethodBadge.svelte`, dictionary `icons.ts`.

**Acceptance — wszystkie zielone:**
- `pnpm build` ✓ (vite, 807 ms)
- `pnpm check` ✓ (svelte-check + tsc, 0 errors)
- `cargo check --workspace` ✓
- `cargo build --workspace` ✓
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test --workspace --no-run` ✓
- `pnpm tauri:dev` ✓ — okno realnie się otworzyło (PID 242993), GTK/webkit2gtk OK, hot-reload działa.

**Pliki:** patrz `apiovnia-app/` (Vite/Svelte 5/TS) + `apiovnia-app/src-tauri/` (Cargo workspace).

## Faza 1 — DONE ✅ (2026-05-16, oczekuje smoke testu z user side)

### Backend

- [x] `apiovnia-core`:
  - `ids.rs` — typed newtypes `ProjectId`/`CollectionId`/`RequestId`/`EnvironmentId` z prefixem (`prj_`, `col_`, `req_`, `env_`) + UUIDv4. Tests: unique, parse, serde round-trip.
  - `time.rs` — `Clock::System|Fixed`, `epoch_millis_now()`, `millis_to_datetime()`. Tests.
  - `model.rs` — `HttpMethod`, `BodyType`, `KeyValue`, `AuthConfig` (None/Bearer/Basic/ApiKey z `r#in`), `Project`, `Collection`, `Request` (z `new_blank()`), `Environment`. Wszystko `#[serde(rename_all = "camelCase")]`. Tests: serde tagi, defaulty.
- [x] `apiovnia-storage`:
  - `migrations/0001_init.sql` — **pełna schema** (projects, collections, requests, environments, environment_variables, request_env_overrides, request_history) z indeksami, FK cascade. Jedna migracja na cały MVP.
  - `error.rs` — `StorageError` (NotFound, Conflict, InvalidData, Sqlx, Migrate, Json, Io) + `Serialize` flatten do stringa dla IPC.
  - `db.rs` — `Db` wrapper na `SqlitePool`, WAL, FK, busy_timeout 5s. `Db::open(path)` + `Db::open_in_memory()` dla testów.
  - `repos/{projects,collections,requests}.rs` — list/get/create/rename/delete + `update_full` dla Request. Method/BodyType jako stringi w SQL, JSON columny dla headers/params/auth.
- [x] `apiovnia-storage/tests/crud.rs` — **5/5 zielone**: project CRUD, empty name rejection, cascade delete, request full round-trip, NotFound dla unknown ID.
- [x] `apiovnia-tauri`:
  - `app_state.rs` — `AppState { db: Arc<Db> }`, managed jako `tauri::State`.
  - `commands/{projects,collections,requests}.rs` — 4+4+7 = 15 IPC commands, cienkie wrappers na repos. `RequestSummaryDto` jako camelCase DTO dla middle panel listingu.
  - `lib.rs` — `setup()` z `block_on(Db::open(app_data_dir/apiovnia.db))`, `generate_handler![…]` dla wszystkich 15+ping.

### Frontend

- [x] `lib/types/domain.ts` — TS mirror modeli z `apiovnia-core` (Project/Collection/Request/RequestSummary/KeyValue/AuthConfig/HttpMethod/BodyType + branded IDs przez `unique symbol`).
- [x] `lib/api/ipc.ts` — typed wrappers na `invoke<T>` per command.
- [x] `lib/stores/app.svelte.ts` — `$state` z `projects/collections/requests/activeProjectId/activeCollectionId/activeRequestId/activeRequest/loading/error`. Actions: `loadAll/selectProject/selectCollection/selectRequest/create*/rename*/delete*`. Cascade loaders (selekcja project → załadowanie kolekcji → załadowanie requestów → załadowanie body aktywnego). Persistencja active IDs w `localStorage` (`apiovnia.active.v1`).
- [x] `lib/components/InlineRename.svelte` — dwuklik/F2 → edit mode, Enter commit, Esc cancel, blur commit.
- [x] `ProjectsPanel.svelte` — live, lista projektów + kolekcji aktywnego projektu, `+ New project/collection`, inline rename, right-click → delete (z confirm), empty state CTA.
- [x] `RequestsPanel.svelte` — live, lista requestów aktywnej kolekcji, `+` button disabled bez kolekcji, inline rename, right-click delete, empty states.
- [x] `DetailPanel.svelte` (Phase 1 widok read-only) — pokazuje aktywny request (method badge + URL + created/updated), Send disabled, taby disabled z label "soon" do Phase 2. Placeholder gdy brak active request.
- [x] `App.svelte` — `onMount(() => app.loadAll())`, breadcrumb w TitleBar = `[project, collection, request]`, error bar gdy `app.error`.

### Quality gates

- [x] `pnpm check` ✓ (0 errors, 0 warnings, 231 plików)
- [x] `cargo clippy --workspace --all-targets -- -D warnings` ✓
- [x] `cargo test -p apiovnia-storage` ✓ (5/5)
- [ ] **Smoke test GUI** (in progress — w trakcie pisania): odpalone `pnpm tauri:dev`, do zweryfikowania że można dodać projekt → kolekcję → request → restart → wszystko zostało.

### Tooling notes

- `source .envrc.local` w katalogu głównym przed czymkolwiek (rust + pnpm w PATH).
- Pełny dev: `cd apiovnia-app && pnpm tauri:dev`.
- DB w `~/.local/share/tech.trurl.apiovnia/apiovnia.db` (Linux XDG).

## Faza 1.5 — Dialogs + context menu polish — DONE ✅ (2026-05-16)

(zrealizowane między Fazą 1 a 2, na prośbę usera — natywne `window.prompt/confirm` zastąpione własnymi)

- `dialogs.svelte.ts` — globalny store, `dialogs.prompt({...}): Promise<string|null>` + `dialogs.confirm({...}): Promise<boolean>`. Kolejka — jeden dialog naraz.
- `PromptModal.svelte` / `ConfirmModal.svelte` — natywny `<dialog>` HTML (focus trap + Esc), karta 420px, viewport-flex centering, danger variant (czerwony przycisk).
- `DialogsHost.svelte` w `App.svelte` renderuje active dialog.
- `ContextMenu.svelte` — popover z `MenuItem[]`, anchor `{x,y}`, klik poza/Esc zamyka. Right-click na row + `⋯` button (hover-only) → wspólne menu z akcjami Rename / Delete.
- `ProjectsPanel`/`RequestsPanel` przepisane na dialogs + context menu (window.prompt/confirm wycięte).
- Plus przy "Projects" header (spójność z "Collections"), Bearer/`InlineRename` removed.

## Faza 2 — DONE ✅ (2026-05-16)

### UI infra (reusable)

- `Popover.svelte` — anchor-based positioning, click-outside, Esc, viewport clamp, `matchAnchorWidth`.
- `Select.svelte<T>` — custom dropdown z keyboard nav (↑↓/Enter/Esc, type-ahead), `triggerLabel`/`optionLabel` snippets.
- `Segment.svelte<T>` — radio horizontal (sm/md).
- `Tabs.svelte<T>` — header rail z badge/count/soon, ←/→ keyboard nav.
- `CodeMirrorEditor.svelte` — wrapper na CodeMirror 6, lang `json|html|xml|plain`, dark theme z naszymi tokenami, `Annotation` marker dla external-sync (eliminuje race przy paste/drag), `lint` + `onLintChange` callback.

### Request editor

- `UrlBar.svelte` — method picker (Select z `MethodBadge` w trigger+option), URL input mono, Send (disabled do Phase 3), env picker placeholder (Phase 5).
- `KeyValueTable.svelte` — generyczna tabela {key, value, enabled}, always-empty draft row na dole, `×` delete (hover).
- `RequestTabs` (przez `Tabs`): Params / Headers / Body (badge type) / Auth (badge type) / Env Overrides (soon) / Tests (soon). Counts/badges derived z live request.
- `ParamsTab` / `HeadersTab` — KeyValueTable.
- `BodyTab` — Segment (None/JSON/Form/Raw), JSON/Raw → CodeMirror z syntax highlight, Form → KeyValueTable (serialized do `bodyContent` JSON). "Beautify" dla JSON.
- `AuthTab` — Select dla typu, pola dla Bearer (token), Basic (user+pass), ApiKey (name+value+location).

### Save flow

- Store `updateActiveRequest(patch)` + debounced save **250 ms** (Linear-style).
- `selectRequest()` flushuje pending save przed switchem.
- **4-state machine** `idle | editing | saving | saved`:
  - `editing` — amber dot + "editing" (po edycji, czekamy na debounce)
  - `saving` — amber pulse + "saving…" (IPC w locie)
  - `saved` — **green dot z poświatą** + "saved" przez 800ms, potem fade do idle
  - `idle` — szary dot, brak labela

## Faza 3 — DONE ✅ (2026-05-16, z polish 3.5/3.6)

### Backend (`apiovnia-http`)

- `Executor` z `reqwest::Client` (rustls-tls, gzip, 30s timeout, redirect 10, User-Agent `Apiovnia/0.1.0`).
- `ExecutionResult { status, statusText, headers, contentType, bodyKind: text|binarybase64|empty, body, bodyTruncated, durationMs, sizeBytes, finalUrl, sent: SentRequest }`.
- `SentRequest { method, url, headers, bodyPreview (16 KiB), bodySizeBytes }` — snapshot tego co poszło na wire, debug aid.
- Text/binary detection po content-type (`+json`, `+xml`, `text/*`, JS/JSON/XML/YAML).
- Body cap **2 MiB** truncate, base64 dla binary.
- Auth: Bearer (`bearer_auth`), Basic (`basic_auth`), ApiKey w header / query param.
- Body: JSON/Raw → `.body()`, Form → `.form()` (deserializuje z `bodyContent` JSON serialized z KeyValueTable).
- 2/2 unit tests (content-type classification + method translation).

### Storage rozszerzone

- Migracja `0002_history_full.sql` — ALTER TABLE `request_history` ADD: `sent_json`, `final_url`, `content_type`, `body_kind` (wszystko nullable, legacy rows zachowane).
- `HistoryRepo::insert` zapisuje pełny snapshot ExecutionResult.
- `HistoryRepo::latest_success_for(request_id)` — najnowszy successful (`error_message IS NULL AND status_code IS NOT NULL`).
- `HistoryRepo::list_recent(limit)` — gotowe pod history panel z Phase 9.

### IPC

- `execute_request(request_id, env_id?)` — load → execute → zapis do history (sukces lub error) → result. Body w history truncated do 64 KiB.
- `get_last_response(request_id)` — rehydratuje `ExecutionResult` z history (z fallbackami dla legacy rows bez sent_json).

### Frontend

- `app.executeActive()` — flushSave → IPC → `currentResponse | executionError`. State `executing`.
- `loadActiveRequestBody()` po `selectRequest()` / `loadAll()` ładuje też **last response z history** → prawy panel nie jest pusty po restarcie ani po switchu requestu.
- `UrlBar`: Send live (disabled gdy executing lub pusty URL), spinner, **⌘Enter** shortcut, "Sending…" label.
- `response/ResponseViewer.svelte` — state machine: idle / sending (spinner) / error / result.
- `response/ResponseHeader.svelte` — status pill (200=ok / 3xx-4xx=warn / 5xx=err), time / size / content-type / `truncated` badge, "Copy response body" button.
- Tabs: **Pretty (default) | Headers | Request | Raw** (Preview wyrzucone — designer artifact, niski value dla API clientu).
- `ResponsePretty.svelte` — read-only CodeMirror, **auto-language z content-type**:
  - `application/json` / `*+json` → JSON (auto-format JSON, note "Auto-formatted JSON · the original byte stream is on the Raw tab")
  - `text/html` → HTML
  - `application/xml` / `text/xml` / `*+xml` → XML
  - inne text → plain
  - binary → notatka "switch to Raw"
- `ResponseHeaders.svelte` + `ResponseSent.svelte` używają wspólnego `HeadersGrid.svelte` — jednolite stylowanie (greyed name na `var(--surface)` tle, mono value).
- `ResponseSent.svelte` (Request tab): method badge + final URL (po redirectach) + headers + **body preview z content-type-aware syntax highlight** (CodeMirror read-only, JSON pretty-printed). Content-type wyświetlony jako badge w nagłówku sekcji.
- `format.ts` helpery: `formatBytes`, `formatDuration`, `statusKind`, `langFromContentType`, `findHeader`, `contentTypeOf`.

### JSON lint w body editor (3.6 polish)

- `@codemirror/lint` + `jsonParseLinter()` z `@codemirror/lang-json`, **delay 250 ms** (zgodne z save debounce).
- `tooltipFilter: () => []` w `linter()` → eliminuje "drugi tooltip" odpalany przy hoverze nad inline range. Diagnostics w state zostają (banner / `diagnosticCount` działa).
- Inline mark (`.cm-lintRange*`, `.cm-diagnostic*`) — `display: none + pointer-events: none` via CSS. Pojedynczy wskaźnik = gutter marker.
- **Gutter marker** w `app.css`: 12px **czerwone kółko z białym `!`** w środku (background `var(--err)`, 3px poświata, `::after` content "!"), default CodeMirror SVG/trójkącik wycięty przez `display: none !important` na wszystkich dzieciach.
- Tooltip na hover marker: `var(--elevated)` bg + czerwona lewa krawędź (3px) + naszą typografią + max 360px.
- **Banner nad edytorem** (`BodyTab`): subtle czerwony pasek z pulsującą małą kropką + ikona X + tekst `N JSON parse error(s) — hover the indicator in the gutter for details.` Pojawia się tylko gdy `lintCount > 0`.

### Detale UX

- `time 1.70 s` / `size 1.2 kB` — flex `.metric` z `gap: 5px` (HTML whitespace collapse fixed).
- Copy button: ikona + label `Copy response body` (jasne komu/co kopiuje).
- Subtitle requestu w środkowym panelu: `r.url` + italic placeholder `add URL in editor` gdy puste.

### Quality gates

- `pnpm check` ✓ (0 errors, 0 warnings)
- `pnpm build` ✓ (~400 kB JS / ~130 kB gzip — CodeMirror dominuje)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test -p apiovnia-storage` ✓ (5/5)
- `cargo test -p apiovnia-http` ✓ (2/2)

---

## Faza 4 — DONE ✅ (2026-05-16)

**Decyzja:** custom renderer (nie CodeMirror foldGutter). Powód: hover-copy per value
i nawigacja matchy (`1/3` z next/prev) są naturalne w własnym DOMie; CSS w `tokens.css`
już istniał (`.jv-*`), więc port był ~1:1.

### Frontend

- `lib/components/response/JsonView.svelte` — port `design_artifacts/json-view.jsx`
  na Svelte 5 runes. Flatten tree → płaska lista linii (`{ no, depth, parts, path,
  foldable, copyable, copyValue }`). Hard cap **8 000 linii** (truncation indicator
  na końcu); w praktyce response cap to 2 MiB, więc rzadko bije.
- **Collapse state**: `Set<string>` paths + bumped `collapsedVersion` (Svelte $state
  trackuje Set tylko po identity, więc po mutacji bumpujemy licznik żeby `$derived`
  lines się przeliczyło). Per-line fold-button + foldlabel `…N items / N keys`
  klikalne.
- **Search**: input + counter `N/M` + Next/Prev (Enter / Shift+Enter / przyciski) +
  Esc clears. Highlight przez `<mark>` w renderowanym `Part.v` (case-insensitive,
  wszystkie wystąpienia w jednej linii). **Active match** dostaje silniejszy
  background (`rgba(245,158,11,0.18)` + lewa krawędź akcentowa) i smooth-scrollIntoView.
- **Hover-copy**: button po prawej każdej primitive line, kopiuje wartość (raw, bez
  cudzysłowów dla stringów). Toast "Copied" w prawym dolnym rogu na ~1.2 s.
- **Expand all / Collapse all**: collapseAll zwija każdy non-root foldable jeden poziom
  niżej; expandAll czyści Set.
- **⌘F / Ctrl+F** capture globalnie przy mount — preventDefault + focus search input.
  Działa zawsze gdy Pretty tab + JSON content-type aktywny.

### Integracja w `ResponsePretty.svelte`

- JSON content-type → `JsonView` (po `JSON.parse`).
- Parse fail → fallback do read-only CodeMirror + amber warning bar "Could not parse
  as JSON — see Raw tab for the unmodified body".
- HTML / XML / plain text → bez zmian (CodeMirror).
- Binary / empty → friendly note.

### Styling (port do `app.css`)

Skopiowane z `design_artifacts/tokens.css`:
- `.jv`, `.jv-line` (+ `.match`, `.match-active`), `.jv-no`, `.jv-gutter`, `.jv-fold-btn`,
  `.jv-content`, `.jv-foldlabel`, `.jv-copy`, `.jv mark`
- `.j-key`, `.j-string`, `.j-number`, `.j-bool`, `.j-null`, `.j-punct`, `.j-ws`

### Quality gates

- `pnpm check` ✓ (0 errors, 0 warnings, 268 plików)
- `pnpm build` ✓ (~616 kB JS / ~212 kB gzip — custom renderer +1 kB vs CM only)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test --workspace` ✓ (17/17: 10 core + 2 http + 5 storage)

### Otwarte poprawki na potem (nie blokery Phase 4)

- Smooth scroll może być nadgorliwy przy szybkim cyklowaniu matchy. Jeśli zacznie
  irytować — zmienić `behavior: 'smooth'` na `'auto'`.
- Search po obecnej implementacji to per-line `indexOf` — przy ~8k linii i query
  na każdy keystroke jest OK (kilka ms), ale jeśli kiedyś podniesiemy MAX_LINES,
  warto zdebouncować input.
- Collapse-all robi się po stronie JS przez obejście drzewa data → buduje pełny
  Set ścieżek. Dla bardzo głębokich JSONów (Petstore-class) to chwila — można
  to leniwie zrobić na request, ale nie problem teraz.

---

## Faza 5 — DONE ✅ (2026-05-16)

**Cel zrealizowany:** env-aware execution. Tworzę env, override URL/method/headers/params/body/auth per env, definiuję `{{var}}` w env variables, przełączam env w UrlBar, Send leci ze zresolvowanym requestem (interpolacja + fold override → base). Brak crypto (Phase 6).

### Backend (`apiovnia-core`)

- `model.rs`: `EnvVariable`, `EnvOverride` (każde override-pole `Option<...>` z `is_empty()` helperem).
- `interpolate.rs`: `interpolate(template, vars)` — `{{name}}` → wartość; nazwy `[A-Za-z0-9_.-]+`; nieznane placeholdery zostają verbatim (żeby user widział lukę). Helpery `interpolate_pairs` (tylko values, klucze nietknięte — `{{var}}` w nazwie nagłówka to footgun) + `interpolate_auth`. Generic `<S: BuildHasher>` wszędzie (clippy `implicit_hasher`).
- `resolver.rs`: `resolve_request(base, Option<&EnvOverride>, vars) -> ResolvedRequest`. Pole-po-polu: `Some(v)` → użyj v, `None` → keep base. Headers/params: **full replacement**, nie merge (z briefu). Po fold → interpolacja na wszystkich polach (url, headers values, params values, body, auth).
- **Tests**: 33/33 zielone — wszystkie kombinacje all-None / some-Some / all-Some + interpolacja w base, w override, missing var verbatim, headers replace nie merge.

### Backend (`apiovnia-storage`)

- `repos/environments.rs` — `EnvironmentRepo`: `list_for_project`, `get`, `create`, `rename`, `delete`. UNIQUE(project_id, name) → mapowane do `StorageError::Conflict`.
- `repos/env_variables.rs` — `EnvVariableRepo`: `list_for_env`, `upsert` (ON CONFLICT po (env_id, name)), `delete`.
- `repos/overrides.rs` — `OverrideRepo`: `get`, `list_for_request`, `upsert` (ON CONFLICT po (request_id, environment_id)), `delete`. Mapping pól <-> SQL (method/body_type jako stringi, kolekcje jako JSON).

### Backend (`apiovnia-tauri` commands)

11 nowych IPC:
- env: `list_envs`, `create_env`, `rename_env`, `delete_env`
- env vars: `list_env_variables`, `upsert_env_variable`, `delete_env_variable`
- overrides: `get_override`, `list_overrides_for_request`, `upsert_override` (empty patch → backend delete), `delete_override`

`execute_request(req, env)` teraz: `RequestRepo::get` → jeśli env: ładuj override + env vars → `resolve_request(base, ovr, vars)` → `executor.execute(resolved)`. History dostaje env_id jak zwykle.

### Frontend

- TS mirror: `Environment`, `EnvVariable`, `EnvOverride` w `types/domain.ts`. IPC wrappery w `api/ipc.ts`.
- Store `app.svelte.ts` rozszerzony:
  - `envs[]`, `activeEnvId`, `envVarsByEnv: Record<envId, EnvVariable[]>`, `activeOverride`
  - Persistencja per-project active env w `localStorage` (`apiovnia.activeEnvByProject.v1` — `Record<projectId, envId>`)
  - Cascade load: `selectProject` → `loadEnvsForActiveProject` → restore remembered env z mapy → `refreshEnvVars(active)`
  - `selectRequest` flushuje pending override save przed switchem (analogiczne do request save)
  - Actions: `selectEnv`, `createEnv`, `renameEnv`, `deleteEnv`, `upsertEnvVariable`, `deleteEnvVariable`, `updateActiveOverride` (debounced 250 ms, identyczna cadencja jak request editor), `resetActiveOverride`, `flushOverride`
  - `executeActive()` przekazuje `state.activeEnvId` do backendu
- `EnvSelector.svelte` (w `UrlBar`):
  - Pill z kolorowaną kropką (red=prod, yellow=stage, green=dev/local, amber=custom) + nazwą + 🔒 jeśli `requiresUnlock`
  - Popover: "No environment" + lista envów (active dostaje check + bold) + "+ New environment…" + "Manage envs & variables…"
  - "+ New env" inline (`dialogs.prompt`) — nie wymaga otwierania modala
- `EnvManageModal.svelte` (760×540 dialog, opened from EnvSelector lub z Env Overrides tab):
  - Lewy rail: lista envów + Rename / Delete per row + "+ New"
  - Prawy pane: variables aktywnego envu — read-only widok + inline edit on click + draft row na dole (Enter commit) + delete per row
  - Footer "Lock env… (soon)" — placeholder pod Phase 6
- `EnvOverridesTab.svelte` (port `artifact-env.jsx`, uproszczony do whole-field overrides):
  - Empty states: "No environments yet" + "+ New env" button / "No environment selected"
  - Bar: 🌐 + ekspiner ("Patches on this request when {env} is active... resolution: request > env override > base") + "Reset all in {env}"
  - 7 field rows (URL, Method, Headers, Params, Body type, Body content, Auth) — każde z checkbox toggle ("inherits base" gdy off, akcent border + dot gdy on), kontrolą edycji (input / Select+MethodBadge / KeyValueTable / Segment / CodeMirror / Select+inputy dla auth), oraz greyed `base` linią pokazującą wartość bazową
  - Headers/Params override = full replace — explicit "replaces the base list entirely" note (mirror brief)
- `DetailPanel`:
  - "Env Overrides" tab już nie soon/disabled — z count badgem (liczba ustawionych pól w aktywnym overridzie)
  - `EnvManageModal` zamontowany na końcu, `envManageOpen` state

### Quality gates

- `pnpm check` ✓ (0/0/0, 271 plików)
- `pnpm build` ✓ (~646 kB JS / ~221 kB gzip)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test --workspace` ✓ (40/40: 33 core + 2 http + 5 storage)

### Otwarte tematy / nice-to-have (nie blokery Phase 5)

- **Per-body-field dotted-path overrides** (`device.platform` jak w designie) — design canvas to pokazuje; MVP robi whole-body content override (prostsze, dla większości użyć wystarczy). Jak ktoś poprosi → patch resolver + UI.
- **"Add field override" popover** z briefu — obecnie checkboxy są zawsze widoczne, działa tak samo, mniej klików.
- **"Diff base" button** z designu — TODO Phase 9 polish.
- **Env color persistence** — heurystyka po nazwie (prod=red, dev=green, ...) zamiast pełnego pickera. Wystarczy na MVP.
- **No-env vs deleted-env path** — jeśli user usunął activeEnv, fall back na null (`use base values`); persistencja per-project map zostaje, ale invalid id się ignoruje przy load.

---

## Faza 5.5 — Multipart body + TitleBar controls — DONE ✅ (2026-05-16)

Drobne, ale przydatne dla codziennego użycia, w przerwie przed Phase 6.

### Multipart `BodyType` (text + file parts)

- `apiovnia-core::BodyType` rozszerzony o wariant `Multipart`. Storage parser
  `body_type` (req + override repos) gada nowy literal `"multipart"`.
- `apiovnia-http::executor`:
  - `apply_body` zmieniony na async (wymagane przez fs read).
  - Nowy `apply_multipart` parsuje `body_content` jako JSON `Vec<MultipartField>`
    (`{key, value, kind: text|file, file_path, content_type, enabled}`),
    buduje `reqwest::multipart::Form` per row — `.text()` dla tekstu, `.part()`
    z `Part::bytes` + `file_name` + `mime_str` dla pliku.
  - MIME pliku: explicit `content_type` > guess z extension (`mime_for_path`,
    ręczna mapa popularnych typów) > `application/octet-stream`. Nie ciągniemy
    `mime_guess` żeby nie pęcznieć zależnościami.
  - Reqwest dostał feature `"multipart"`, tokio dostał `"fs"`.
- Frontend:
  - `MultipartField` w `types/domain.ts`.
  - `MultipartTable.svelte` — KeyValueTable's cousin: per-row text/file kind
    toggle, file picker przez `@tauri-apps/plugin-dialog` (`open(...)`),
    basename display + full path w `title`, optional MIME override input.
  - `BodyTab.svelte` — dodany 5. segment "Multipart" + nowa gałąź renderingu.
- Tauri capability: `dialog:allow-open` + `core:window:allow-close/minimize/set-fullscreen/is-fullscreen`.

### macOS-style traffic lights → real window actions

- `TitleBar.svelte`:
  - `<span>` → `<button>`, podpięte przez `@tauri-apps/api/window`
    `getCurrentWindow().close() / minimize() / setFullscreen(toggle)`.
  - Green light: tytuł/aria reflektuje stan ("Enter full screen" vs "Exit
    full screen"), zsync'owany via `onResized` listener (Tauri nie ma
    bezpośredniego `onFullscreenChange`).
- `app.css`:
  - `.tl` dostaje `cursor: pointer; padding: 0; outline: none` + focus-visible
    ring.
  - Glify (`× − ⤢`) via `::after`, ujawniane na `.ap-traffic:hover .tl::after`
    — mirror native macOS feeling.
- Cross-platform: działa też na Linux/Windows (rysujemy własny chrome).
  Jeśli ktoś kiedyś włączy native macOS traffic lights via `titleBarStyle`,
  trzeba będzie nasze ukryć — TODO Phase 9 packaging.

### Quality gates

- `pnpm check` ✓ (287 plików, 0/0/0)
- `pnpm build` ✓
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test --workspace` ✓ (40/40)

### Otwarte tematy

- Multipart file size — żadnego capa nad rozmiarem; cały file ląduje w RAM
  podczas execute. Jak ktoś wrzuci 500 MB ZIP, dostaniemy pamięciowy bump
  i drawcall do JSON history (cap 64 KiB i tak truncuje text body, ale plik
  binary trzyma się w RAM tylko podczas send). Akceptowalne na MVP.
- File path persistowany w DB. Jak user przeniesie plik, multipart Send
  zwróci `failed to read multipart file ...` z czytelnym message.

**Update (2026-05-16):** TitleBar traffic lights cofnięte z poziomu Phase 5.5
— OS i tak rysuje natywny chrome (`decorations: true`), nasze fake'owe
duplikowały + confused na Linux/Windows. Zostawiamy native, mniej kodu,
zero ambiguity. `TitleBar.svelte` ma teraz tylko crumbs + actions.

---

## Faza 5.6 — Polishe i bugfixy z dnia (2026-05-16) — DONE ✅

Zebrane drobne rzeczy z sesji już po fazach 5/5.5, każda mała, ale poprawia
real-world UX dziennego użycia.

### Filtry (lewy + środkowy panel)

- `ProjectsPanel`: text filter nad listą projektów I kolekcji jednocześnie,
  ⌘P / Ctrl+P focusuje input, Esc czyści. Empty matches → "No projects match
  …" zamiast pustej listy. Selekcja active project/collection persistowana
  nawet jak filter je schowa (visual-only).
- `RequestsPanel`: text filter (match na `name + url`) + filter button (lejek)
  → popover z method multi-select (GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS)
  z `MethodBadge`-ami. Active filter → akcent button + amber-numer badge.
  Sekcja "{collection} · M of N requests" + "Clear filter" link. Designer
  intent z `artifact-shell.jsx` (`title="Filter by method"`) — nie był to
  artefakt.

### JSON Pretty viewer (Phase 4)

- Już DONE wcześniej; tu tylko nota że `JsonView` z collapsible nodes +
  ⌘F search + hover-copy + Expand/Collapse all jest w produkcji.

### Send + ResponseViewer

- Spinner "Sending request…" dostał elapsed timer obok, tickujący co 200 ms
  (tabular-nums, `formatDuration`) — sygnalizuje że aplikacja żyje przy
  wolnych endpointach. Reset na 0 przy każdym Send.

### Cascade auto-pick (store)

- `loadCollectionsForActive` + `loadRequestsForActive` rozszerzone z
  "if invalid → first" do "if null OR invalid → first". Zmiana projektu →
  auto-aktywna pierwsza kolekcja → auto-aktywny pierwszy request → body
  + last response. Żadnego empty-flash w middle/right panelu.
  Persisted activeRequestId zostaje jeśli pasuje do nowej listy.

### Smart empty states w `DetailPanel`

Trzy poziomy z dedykowanym CTA buttonem (mirror flow create-project →
create-collection → create-request):
- **Brak projektu**: 📁 hero icon + "Welcome to Apiovnia" + amber "New project"
- **Projekt bez kolekcji**: 🗂️ hero + "No collections in {project}" + "New collection"
- **Kolekcja bez requestów**: 📤 hero + "No requests in {collection}" + "New request"
- **Edge** (z cascade prawie nie wystąpi): krótkie "Pick a request from the middle panel"

Każdy CTA używa tego samego `dialogs.prompt` + `app.create*` flow co
buttony w lewym/środkowym panelu. Bugfix: dekoracyjne hero iconki dostały
klasę `.hero-icon` po tym jak `:global(svg)` selector przypadkiem zlewał
ikonę `IC.plus` wewnątrz CTA buttona z amber background.

### Multipart snapshot (Request tab po Send)

- Bug: `RequestBuilder::try_clone()` zwraca `None` dla `Body::stream(...)` —
  reqwest właśnie tak stora multipart. Snapshot wpadał w empty fallback
  (`headers: []`, `body_preview: ""`). Inne body types (JSON/Form/Raw)
  używają in-memory bodies — `try_clone` działa.
- Fix w `executor.rs`:
  - Pre-body probe (auth applied, no body) buildowany przed `apply_body`
  - Dla `BodyType::Multipart` → `synthesize_multipart_snapshot()` ręcznie
    rekonstruuje SentRequest:
    - method/url/headers z pre-probe (Bearer/Basic/ApiKey-header zachowane)
    - dropujemy any user Content-Type, dodajemy nasze
      `multipart/form-data; boundary=----apiovnia-snapshot-boundary`
    - body_preview w RFC-7578 formacie: `--boundary` separator,
      `Content-Disposition: form-data; name="..."`, dla plików `Content-Type`
      + placeholder `[N bytes — file contents omitted from preview]`
    - body_size_bytes liczy real bajty plików (`fs::metadata`) + tekst party
  - Boundary cosmetic — reqwest na wire używa losowego; nasz boundary
    sygnalizuje że to reconstruction, nie capture.

### Collections header (lewy panel)

- Rozbity z 1 linii "Collections · {project name}" (łamało się brzydko dla
  długich nazw projektów) na **2 stacked linie**:
  - `COLLECTIONS` — tiny uppercase label (jak inne `.ap-sec`)
  - `{project name}` — 11.5px, `fg-dim`, `overflow-wrap: anywhere` (clean
    break nawet bardzo długich nazw bez spacji), full nazwa w `title`
- `+` button zakotwiczony do góry przez `align-items: flex-start` — nie
  ginie nawet jak nazwa zawinie się na 2-3 linie.

### Quality gates (po wszystkich poprawkach)

- `pnpm check` ✓ (273 plików, 0/0/0)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test --workspace` ✓ (40/40)

---

## Faza 6-9 — PENDING

- **Faza 6 — Crypto / master password** (Argon2id + AES-GCM, unlock modal — `artifact-unlock.jsx`).
- **Faza 7 — OpenAPI import** (`oas3`, mapping do collections + envs).
- **Faza 8 — Cmd palette + global shortcuts** (⌘K, ⌘N, ⌘1/2/3, ⌘P, ⌘F).
- **Faza 9 — Polish & release** (History panel widoczny w UI, OpenAPI export, packaging dla macOS/Linux).

### Co konkretnie zostało (snapshot stanu)

**Działa end-to-end:**
- Pełny CRUD na projektach/kolekcjach/requestach z persisted active selection
- Request editor (URL/method/params/headers/body/auth/env-overrides) z debounced save
- Body types: None/JSON/Form/Multipart(text+file)/Raw — wszystkie wysyłane poprawnie
- HTTP execute z reqwest, full response viewer (Pretty JSON / Headers / Request / Raw)
- Env CRUD + variables + per-(req,env) overrides + resolver + `{{var}}` interpolacja
- Filtry (text + method), cascade auto-pick, smart empty states z CTA

**Backend gotowy, brak UI:**
- History panel: `HistoryRepo::list_recent()` istnieje, każdy execute zapisuje
  pełny snapshot ExecutionResult, ale w UI nie ma jeszcze panelu — Phase 9.

**Brak feature'u w ogóle:**
- Crypto/master password — `apiovnia-crypto` crate scaffold tylko, brak impl
- OpenAPI import/export — `apiovnia-openapi` crate scaffold tylko
- Command palette — żaden globalny shortcut poza ⌘P (focus filter), ⌘Enter
  (Send), ⌘F (search w JSON viewer)
- Packaging dla release — `tauri.conf.json` ma `bundle.targets: "all"` ale
  bez signing config, ikon (placeholdery)

### Otwarte tematy / dług techniczny

- **Bearer 401 z `api.udl.ai`** — Authorization header faktycznie leci
  (potwierdzone w Request tab). To realna odpowiedź serwera. Nie nasz bug.
- **JSON request body w przykładzie usera** — brakowało `}` przed `]` w
  trzecim obiekcie. Linter pokaże to natychmiast po fix-up.
- **CodeMirror "trójkącik" w gutterze** — przy obecnym setupie zhide-owany
  przez CSS `*` selektor. Jeśli wraca, kandydat na `markerFilter` w
  `lintGutter` config.
- **History panel** — backend gotowy, brak UI. Phase 9.
- **Multipart file size** — żadnego capa. Akceptowalne na MVP.
- **Per-body-field dotted-path overrides** (Phase 5 design) — odpuszczone,
  whole-body content override starcza na 95% case'ów.

### Tooling notes na następną sesję

- `source .envrc.local` w root przed czymkolwiek.
- `cd apiovnia-app && pnpm tauri:dev` — pełny dev z HMR.
- DB w `~/.local/share/tech.trurl.apiovnia/apiovnia.db`.
- Po zmianie migracji SQL: usunąć/zmigrować lokalną DB lub zacząć od świeżego pliku (`rm` po backup).
- Po zmianie pól `ExecutionResult`/`SentRequest` (Rust) — restart `tauri:dev` (frontend HMR samego nie wystarczy, Rust binary się zmienia).

