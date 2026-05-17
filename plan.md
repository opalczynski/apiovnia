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

## Faza 6 — DONE ✅ (2026-05-17)

**Cel zrealizowany:** mogę oznaczyć env jako encrypted, ustawić master password,
encryptować wszystkie variable values + override fields (kolumny secret-bearing).
Po restarcie app klucz znika; próba użycia env → unlock modal. Auto-lock po
10 min idle.

### Backend (`apiovnia-crypto`)

- `EnvKey([u8; 32])` z `Drop` + `zeroize::Zeroize` — bufor klucza zerowany
  przed dealokacją.
- `derive_key(password, salt)` — Argon2id, OWASP 2024 baseline (m=19_456 KiB,
  t=2, p=1, out=32). Salt 16 bajtów per env.
- `seal/open` — AES-256-GCM, format `nonce(12) ‖ ct ‖ tag(16)`.
  `seal_str/open_str` to ich base64-wrappery dla kolumn TEXT.
- `make_password_check` / `verify_password_check` — encryptujemy fixed
  marker `apiovnia/v1/password-check` przy seal envu; przy unlock derywujemy
  klucz z user-podanego hasła i próbujemy odszyfrować marker. Działa →
  hasło poprawne. AEAD failure → wrong password (jeden komunikat, nie
  rozróżniamy żeby nie wyciekać info).
- 13 unit testów: round-trip (short/long/string), unique nonces, wrong key,
  tampered tag, tampered nonce, too-short ciphertext, password check
  round-trip, password check wrong key, UTF-8 reject, random salt non-zero.

### Backend (`apiovnia-crypto::password_policy`)

- `zxcvbn` 3.1 — Dropbox port. Dla nas idealny bo łapie słownik + l33t +
  keyboard walks + daty + powtórzenia, czyli `password1234` ma score 0
  niezależnie od długości.
- `score_password(pw) -> PasswordStrength { score, label, crack_time_display,
  warning, suggestions, meets_policy, long_enough }`. `crack_time_display`
  jest human readable (`"3 days"`, `"centuries"`) — model offline slow hash
  (10⁴ guess/s), bo to nasz realny attacker przy Argon2id.
- `validate_policy(pw)` — enforce ≥ 8 znaków + score ≥ 3 ("safely
  unguessable"). Defense-in-depth: re-walidowane na backendzie nawet jak
  frontend gating zostanie pominięty (chyba że pro user opt-in z bypass).
- 8 unit testów: rejects short, rejects dictionary, accepts strong passphrase,
  score labels, validate policy paths, crack-time format.

### Backend (`apiovnia-storage`)

- Schema 0001_init.sql już miało `requires_unlock`, `is_encrypted`, `salt`,
  `password_check` na `environments` — wystarczyło nie ruszać i dopisać
  metody.
- `EnvironmentRepo::encryption_meta` / `set_encryption_in_tx` /
  `clear_encryption_in_tx` — getter + tx-aware settery. Specjalnie tx-aware,
  bo pierwotnie zrobiłem przez pool i dostałem `database is locked` —
  SQLite single-writer, otwarta transakcja na bulk-rewrite + osobny pool
  write na env-flip = deadlock z 5s busy timeout.
- `EnvVariableRepo::rewrite_values_in_tx` — bulk UPDATE wszystkich values
  dla env, używane przez enable/disable encryption (atomic migration).
- `OverrideRepo::get_raw` / `list_raw_for_env` / `rewrite_row_in_tx` /
  `upsert_raw` — raw-cols path obok typed path. Repos pozostają
  crypto-agnostic, encryption/decryption robimy w command layer.
- `RawOverrideCols::from_domain` / `into_domain` — typed ↔ raw bridge.

### Backend (`apiovnia-tauri`)

- `SessionKeyStore` w AppState: `Arc<RwLock<HashMap<EnvironmentId,
  StoredKey>>>`, gdzie `StoredKey { key: EnvKey, last_used: Instant }`.
  Klucze NIGDY nie przekraczają IPC. Auto-lock lazy: każdy `with_key`
  / `is_unlocked` / `unlocked_ids` sprawdza `elapsed > IDLE_AUTO_LOCK
  (10 min)` i evictuje stale entries (`Drop = Zeroize`).
- `commands/crypto.rs` z 7 IPC:
  - `enable_env_encryption(env_id, password, bypass_policy)` — generuje
    salt, derives key, encryptuje wszystkie istniejące vars + overrides
    w jednej transakcji, zapisuje meta. `bypass_policy` to escape hatch
    dla pro userów (skipuje zxcvbn score + length floor, ale non-empty
    nadal wymagane bo Argon2 musi z czego derywować).
  - `disable_env_encryption(env_id, password)` — verify hasła via
    `verify_password_check`, decrypt wszystkiego w tx, clear meta.
  - `unlock_env(env_id, password)` — derive + verify → load do store.
  - `lock_env(env_id)` — explicit drop, zeroize on Drop.
  - `is_env_unlocked(env_id) -> bool` — z lazy eviction.
  - `list_unlocked_envs() -> Vec<EnvironmentId>` — j.w., używane na
    startup do hydracji frontendowego setu.
  - `score_password(pw) -> PasswordStrength` — live scoring dla meterа.
    Stateless, nic nie loguje.
- `EnvLocked(env_id)` jako nowy wariant `StorageError` z formattem
  `"ENV_LOCKED:{env_id}"` — frontend pattern-matchuje na prefiksie żeby
  wystrzelić unlock modal z odpowiednim env.
- Modyfikacje istniejących commands:
  - `list_env_variables` / `upsert_env_variable` / `delete_env_variable`
    — gdy env encrypted, encrypt/decrypt na granicy. Bez session key →
    EnvLocked propaguje.
  - `get_override` / `upsert_override` / `delete_override` — analogicznie,
    przez raw cols path. URL/method/body_type zostają plaintext (low
    entropy + debuggable w sqlite3), reszta cipherem.
  - `execute_request(req, env)` — load env, jeśli encrypted to decrypt
    vars + override row przed resolverem. Brak klucza → EnvLocked
    bubbling up do frontu.

### Frontend

- `types/domain.ts` — `PasswordStrength` mirror.
- `api/ipc.ts` — 7 nowych wrapperów: `enableEnvEncryption(envId, pw,
  bypassPolicy=false)`, `disableEnvEncryption`, `unlockEnv`, `lockEnv`,
  `isEnvUnlocked`, `listUnlockedEnvs`, `scorePassword`.
- `stores/app.svelte.ts`:
  - `unlockedEnvIds: Set<string>` — session-only mirror SessionKeyStore.
  - `unlockPrompt: { envId, retry? } | null` — gdy backend zwróci
    EnvLocked, store stawia ten state, App.svelte renderuje UnlockEnvModal
    z opcjonalnym retry callback (typowo: powtórz Send po unlock).
  - `isEnvLockedError` / `envIdFromLockedError` helpery, używane wewnątrz
    `executeActive`, `refreshEnvVars`, `refreshActiveOverride`.
  - Actions: `enableEnvEncryption(envId, pw, bypass=false)`,
    `disableEnvEncryption`, `unlockEnv`, `lockEnv`, `promptUnlock`,
    `dismissUnlockPrompt`, `isEnvLocked(envId)`.
  - `loadAll` startuje od `refreshUnlockedSet()` żeby (w przyszłości,
    jakby ktoś implementował persistencję klucza) frontend wiedział
    co jest unlocked.
- `PromptModal` dostał `kind?: "text" | "password"` — input zamaskowany
  kropkami, autocomplete off, mono font. Używane przez disable-encryption
  prompt.
- `UnlockEnvModal.svelte` — port `artifact-unlock.jsx`: lock-icon header,
  env summary card (color-coded dot + locked pill), password field
  z amber border + ring + show/hide eye toggle, Cancel + Unlock buttons,
  retry callback po sukcesie. Pretty-fail messages (`wrong password →
  "Wrong password — try again."`).
- `SetEnvPasswordModal.svelte` — sealing flow:
  - Live strength meter (5 segmentów kolorowanych według score tier:
    red 0-1, amber 2, green 3-4, glow-green 4).
  - `Cracking time: ~3 days / centuries` line z `PasswordStrength.crackTimeDisplay`.
  - Warning + suggestion z zxcvbn pokazują się jako amber hint pod
    meterem.
  - "Apiovnia cannot recover this password" jako loud amber callout
    z bold opener — zamiast wcześniejszego timid grey hint.
  - Pro-user checkbox: "I'm a pro user — bypass password policy, I know
    what I'm doing." Domyślnie off, dashed border. Gdy on → border + tekst
    zmieniają się na amber. `canSubmit` ignoruje `meetsPolicy` gdy bypass
    true, ale empty-password nadal blokuje.
  - Confirm field + mismatch hint na czerwono.
- `EnvSelector.svelte` — pill pokazuje 🔒 (locked) / 🔓 (unlocked this
  session) gdy `env.isEncrypted`. Dropdown rzędy mają hover-revealed
  Lock/Unlock button po prawej.
- `EnvManageModal.svelte`:
  - "Lock with password…" CTA dla plaintext envów (otwiera
    SetEnvPasswordModal).
  - Dla encrypted+unlocked: "Lock" (drop session key) + "Disable
    encryption…" (dialog prompt z `kind:"password"` → verify + bulk
    decrypt).
  - Dla encrypted+locked: pełnopanelowy lock screen z "Unlock {env}"
    CTA zamiast variable editora.
  - Status pill "unlocked"/"locked" obok nazwy envu.
- `EnvOverridesTab.svelte` — gdy active env encrypted+locked, renderuje
  lock screen z unlock CTA zamiast field rows.
- `App.svelte` mountuje `UnlockEnvModal` na końcu, kontrolowany przez
  `app.unlockPrompt`.

### Auto-lock (10 min idle, lazy)

- `IDLE_AUTO_LOCK = 10 min`. Każda enkrypcja/dekrypcja przez `with_key`
  refreshuje `last_used`; każdy peek (`is_unlocked`, `unlocked_ids`)
  evictuje stale entries ale nie odświeża. Czyli zostawienie modala
  otwartego ≠ session extension; aktywna praca tak.
- User dostanie EnvLocked przy następnej akcji po idle → frontend pokaże
  unlock modal → identyczny flow co restart. Zero nowej UI potrzeba.
- Bez background threadu — wszystko lazy on access.

### Quality gates

- `pnpm check` ✓ (275 plików, 0/0/0)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test --workspace` ✓ (**61/61**: 33 core + 21 crypto (13 cipher
  + 8 policy) + 2 http + 5 storage)
- `pnpm build` ✓

---

## Faza 8 — DONE ✅ (2026-05-17)

**Cel zrealizowany:** ⌘K spotlight-style palette + ⌘N skrót. Z palety
nawiguję do dowolnego requestu / kolekcji / projektu / envu w aktywnym
projekcie, odpalam akcje (New X, env management, encryption flow,
Copy as curl). Cross-platform mod-key auto-detect.

### Frontend

- `lib/keymap.ts` — single window-level keydown listener z platform detect
  (`navigator.platform` → Cmd na macOS, Ctrl reszta). Skróty:
  - `⌘K` toggle palette (działa wszędzie, też w inputach — meta nav)
  - `⌘N` new request prompt (suppressed w inputach żeby nie kradło typowania)
  - Context-scoped już istniejące: `⌘P` filter, `⌘Enter` Send, `⌘F` JSON search
- `lib/components/modals/CommandPalette.svelte` — custom Svelte 5
  (nie cmdk, spójność z resztą modali). Native `<dialog>` + focus trap,
  Spotlight-style position (14vh od góry, 640×60vh, radial dim).
- Catalog load na każde otwarcie: `listCollections(project) + Promise.all(
  listRequests(c))`. Typowo <100ms. Bez cache — palette krótkożyjące.
- Items: requesty (z method badge + breadcrumb), kolekcje, projekty
  (cross-project switch), envy aktywnego projektu, dynamiczne akcje per env
  (Enable/Disable encryption, Lock), kontekstowa akcja "Copy as curl: {name}",
  "Manage envs & variables", "New X…".
- Custom fuzzy ranking: case-insensitive substring + boosty (start label +1000,
  word boundary +600 malejące z pozycją, hint-only weak +50, krótsze labele
  wygrywają tie). Tie-break per kind: request > collection > env > project > action.
- Keyboard: ↑↓ nav, Enter run, Esc close, mouse hover też zmienia cursor;
  auto-scrollIntoView dla długich list.

### Quality gates

- `pnpm check` ✓ (278 plików, 0/0/0)
- `cargo test --workspace` ✓ (61/61 — bez zmian)

**Decyzja:** `⌘1/2/3` (focus panel) odłożone do Phase 9 — wymaga zdefiniowania
co znaczy "focus" per panel (filter input vs first row vs URL bar) plus
visual emphasis story.

---

## Faza 8.5 — Palette polish + Copy as curl — DONE ✅ (2026-05-17)

Drobne rozszerzenia palette + jeden killer ficzer.

### Lifted modals do globalnego state

- `EnvManageModal` + `SetEnvPasswordModal` zamontowane raz w `App.svelte`,
  kontrolowane przez store: `envManageOpen`, `envPasswordSetupId`. Wcześniej
  były lokalne w `DetailPanel` / `EnvManageModal` — przekierowanie wymagało
  prop-drillingu, niepraktyczne dla palette.
- `disableEncryptionWithPrompt(envId, name)` wyciągnięte ze `EnvManageModal`
  do store action — jeden flow używany z modal + palette.

### Rename "Lock with password…" → "Enable encryption…"

W EnvManageModal button dla plaintext envu. Spójne nazewnictwo z bratnim
"Disable encryption…", lepsza para "enable/disable" niż "lock with
password/disable".

### Nowe akcje w palette

- **Manage envs & variables** — otwiera EnvManageModal (disabled bez projektu)
- Per encrypted env: **Disable encryption for {env}** + **Lock {env}** (gdy unlocked)
- Per plaintext env: **Enable encryption for {env}**
- **Copy as curl: {active request}** — kontekstowa akcja na aktywny request
  (disabled gdy nic nie wybrane). Pojedyncza akcja zamiast per-request, żeby
  duże kolekcje nie zaśmiecały palety.

### Copy as curl (killer feature)

- `apiovnia-core::curl::to_curl(&Request) -> String` — pure builder, **17
  unit testów**. Pokrywa:
  - `-X METHOD` (omijany dla GET — curl default)
  - Query params (enabled-only) folded into URL, RFC 3986 percent-encoded
  - `--user 'u:p'` dla Basic auth (canonical curl idiom)
  - `-H 'Authorization: Bearer …'` dla Bearer
  - ApiKey-in-header → header; ApiKey-in-query → folded do URL
  - JSON body → `--data-raw` + auto Content-Type (chyba że user już ustawił)
  - Raw body → `--data-binary`
  - Form body → `--data-urlencode 'k=v'` per row
  - Multipart → `--form 'key=value'` / `--form 'key=@/path;type=mime'`
  - POSIX-safe single-quote escape z `'\''` dance dla apostrofów w wartościach
  - Auth emit **przed** user headers (user wygrywa — parity z executorem)
  - `\` continuations dla czytelnego paste do terminala
- IPC `build_curl(request_id, env_id)` — dzieli nowy helper `load_env_context`
  z `execute_request` (DRY refactor — wcześniej resolution+decryption była
  inline 35-linijkową ścianą). EnvLocked propaguje normalnie → frontend
  pokazuje unlock modal z retry callback który ponawia copy po unlocku.
- Frontend `app.copyRequestAsCurl(id)`: flush save → IPC → `navigator.clipboard.writeText` → toast.
- UI entry points:
  - `RequestsPanel` context menu (right-click + ⋯ button) → nowy item **Copy as curl**
  - Command palette → **Copy as curl: {active request name}**

### Global toast host

- `ToastHost.svelte` — mini bottom-right notification, slide-in animation,
  auto-dismiss 2.2s, klik zamyka. `ok`/`err` warianty (zielony check / czerwony x).
- `app.showToast(text, kind)` — generic dispatch. Newest replaces older
  (zero queue żeby user widział latest action, nie backlog).
- Używany przez Copy-as-curl, podstawa pod kolejne "did the thing" feedbacki.

### Quality gates

- `pnpm check` ✓ (278 plików, 0/0/0)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test --workspace` ✓ (**78/78**: 50 core (33 + 17 curl) + 21 crypto
  (13 cipher + 8 policy) + 2 http + 5 storage)
- `pnpm build` ✓

---

## Faza 7 — OpenAPI import + export — DONE ✅ (2026-05-17)

**Cel zrealizowany:** import dowolnego `OpenAPI 3.x` YAML/JSON do nowej
kolekcji (z mapingiem servers→environments, security→auth, request body
z `$ref` schema resolution + dummy synth z heurystyką typów + formatów).
Export kolekcji do `{project}_{collection}.yaml` z **agresywnym secret
scrubbing** (typed placeholdery per-typ) + **inferred per-request schema
w components.schemas z `$ref` link** z media type. **Last-win + abort**
na (method, path) collision z konkretnym error msg żeby user wiedział
co poprawić. Persistent **OpLog panel** z tabelaryczną stats + warnings
+ "Download log" button (zapisuje `.log` z pełnym audit trailem).

### Backend (`apiovnia-openapi`)

- **`redact.rs`** — secret scrubbing z typed placeholderami:
  - Headers (exact match case-insensitive): `Authorization`, `X-API-Key`,
    `Cookie`, `X-CSRF-Token`, etc. → `<your-bearer-token>` / `<your-api-key>` /
    `<your-cookie>` / `<your-token>`
  - Query params + body fields (substring): `password` / `secret` /
    `token` / `api_key` / `bearer` / `jwt` / `ssn` / `cvv` / `pin` / `otp` /
    `private_key` / `credentials` etc. → typed placeholder per category
  - Auth: Bearer/ApiKey value → placeholder; Basic password → placeholder,
    username preserved
  - Multipart file paths → `./placeholder.bin`
  - `Policy.extra_*` lists pod Phase 11 user-customizable list
  - 14 unit testów

- **`export.rs`** — buduje `oas3::Spec`:
  - Required fields: `openapi: "3.0.3"`, `info.title/version`, paths,
    `responses: '200': Success` per op
  - `servers[]` z unique URL bases (BTreeSet dedup), `securitySchemes`
    w components z stable nazwami (`bearerAuth`/`basicAuth`/`apiKeyAuth`)
  - Path param detection: `{name}` → `parameters: in: path, required: true`,
    `{{var}}` (Apiovnia template) skipuje
  - **Schema inference z JSON example** (`schema_from_example` recursive):
    - string → `type: string` + format detection (date-time / date / email / uuid / uri)
    - integer → `type: integer, format: int64`
    - number → `type: number, format: double`
    - boolean / null → odpowiednio
    - array → `type: array, items: {recursive z pierwszego elementu}`
    - object → `type: object, properties: {recursive}, required: [non-null keys]`
  - Per-request schema emitowana jako `{PascalCase(opId)}Request` w
    `components.schemas`, media type referuje przez `$ref`
  - **Last-win + abort** na (method, path) collision → `ExportError::Collision { method, path, requests: [Vec<String>] }` z user-friendly message
  - 21 unit testów (basic + format hints + schema inference + collision + nulls + unparseable body)

- **`import.rs`** — parsuje YAML lub JSON (try YAML first → JSON fallback):
  - `info.title` → Collection name (fallback "Imported")
  - `paths × methods` → Request per op, name = `summary || operationId || "{method} {path}"`
  - `parameters` query/header → `params`/`headers`, path → preserved jako `{name}` w URL + warning
  - `requestBody.content["application/json"]`: jeśli `example` → użyj wprost; jeśli `$ref` → **resolve via `mt.schema(spec)`** → **synthesize dummy** (`synthesize_from_schema` recursive z depth guard, allOf merge, anyOf/oneOf pick-first, format hints dla date-time/email/uuid/uri/ipv4/ipv6)
  - `securitySchemes` (operation-level wins nad global): Bearer/Basic/ApiKey (empty values do uzupełnienia) → `AuthConfig`; OAuth2/OpenIdConnect/mTLS → warning + None
  - `servers[]` 2+ → `Environment` per server (name z `description` parse: prod/stage/dev/qa/test/sandbox/local) + per-request `url_override` w `EnvOverride`
  - 16 unit testów + **1 integration test na real petstore z `tests/petstore_sample.yaml`**

### Backend (storage + IPC)

- `CollectionRepo::insert_full` / `RequestRepo::insert_full` — bulk insert
  for OpenAPI import (pre-allocated IDs, complete rows)
- `commands::openapi::import_openapi(project_id, file_path) -> ImportResultDto` —
  reads file, parses, persists collection + requests + envs + overrides
  (with env-id remap z parsed do real ID), returns DTO z rows/warnings/log
- `commands::openapi::export_collection_openapi(collection_id) -> ExportResultDto` —
  loads collection + project (for filename), all requests, redacts each,
  builds spec, returns YAML + suggested filename + rows + warnings + log
- `commands::openapi::save_text_file(path, contents)` — generic helper
  for OpLog "Download log" + YAML save (frontend picks path via dialog)
- Suggested filenames: `{project}_{collection}.yaml` (export) /
  `import_{file_stem}_{date}.log` / `export_{project}_{collection}_{date}.log`
  with `chrono::Utc` timestamp

### Frontend

- `types/domain.ts` — `ImportResult` / `ExportResult` / `ImportRow` / `ExportRow` mirrors
- `api/ipc.ts` — `importOpenapi` / `exportCollectionOpenapi` / `saveTextFile` wrappers
- Store:
  - `OpLog` type + `state.opLog` field + `showOpLog` / `dismissOpLog` actions
  - `importOpenapiForProject(projectId, filePath)` — IPC + cascade-refresh
    (cache projects → collections → envs → switch to new collection +
    new request body) + show OpLog
  - **`exportCollectionInteractive(collectionId)`** — build IPC first (gives
    `yamlFilename` with `{project}_{collection}.yaml`), then `tauri-plugin-dialog::save`
    with that defaultPath, then `saveTextFile` write. Building BEFORE
    dialog is what lets the suggested filename include the project name —
    frontend doesn't always have project context.
- `OpLogHost.svelte` — persistent bottom-right (460×65vh, slide-in 180ms,
  z-index 900), tabular rows (method badge + path + name + redaction count
  for export), collapsible warnings section, **"Download log"** button →
  native save dialog → `saveTextFile`. Closed via × or "Done" (no auto-dismiss).
- `ProjectsPanel` context menu: project → "Import OpenAPI…" (`open` file
  picker → `importOpenapiForProject`); collection → "Export OpenAPI…"
  (`exportCollectionInteractive`)
- `CommandPalette`: `Import OpenAPI into {project}` + `Export {collection}
  as OpenAPI` (gated on active project / collection)
- Tauri capability: `dialog:allow-save`

### Quality gates

- `pnpm check` ✓ (279 plików)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test --workspace` ✓ (**135/135**: 50 core + 21 crypto + 2 http +
  **56 openapi** (14 redact + 21 export + 16 import + petstore integration) +
  **5 import schema synth tests** + 5 storage)
- `pnpm build` ✓

### Bugfixes po user testach

- **Petstore `addPet` body** — bez `$ref` resolution na schemę emitowaliśmy
  pusty `{}`. Fix: `synthesize_from_schema` walking `properties`/`items`
  z format hints. Real petstore integration test żeby nie regresnęło.
- **Filename `{project}_{collection}.yaml`** — frontend liczył lokalny
  `defaultPath` ZANIM wołał IPC, więc backend filename był ignorowany.
  Fix: odwrócony flow — build → dialog z backend's filename → save.

---

## Faza 8.6 — Copy as… submenu (curl/python/httpie/javascript/powershell) — DONE ✅ (2026-05-17)

### Backend

- **Refaktor `apiovnia-core::curl` → `apiovnia-core::snippets`** module:
  - `SnippetFormat` enum (`Curl | PythonRequests | Httpie | JavaScriptFetch | PowerShell`) z `.render(req) -> String` dispatchem
  - Shared helpers w `mod.rs`: `effective_query_params`, `url_with_params`, `percent_encode`, `parse_kv_list`, `parse_multipart_list`, `MultipartRow`, `user_has_content_type`
  - 5 plików per format, każdy z własnymi unit testami

- Per-format design:
  - **curl** (zachowany behavior z Phase 8.5): `\` continuations, `--user 'u:p'` Basic, `--form key=@/path[;type=mime]`
  - **Python `requests`**: `import requests` + `requests.<verb>(url, headers=, json=/data=/files=, auth=)`. Multipart przez `files=[(name, (filename, open(path,'rb'), mime))]`. Basic natywne `auth=("u","p")`.
  - **HTTPie**: `Name:value` headers, `name==value` query, `--auth u:p`, `--raw '{...}'` dla JSON, `--form k=v` form, `--multipart k@/path` files.
  - **JavaScript `fetch`**: top-level await, `JSON.stringify({...})` z pretty object literal, `URLSearchParams` form, `FormData` multipart z TODO comment dla files, `btoa()` dance dla Basic.
  - **PowerShell**: `$headers = @{...}` + `Invoke-RestMethod -Uri ... -Method ... -Headers $headers -Body $body`. Basic przez `[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(...))`. Multipart przez `-Form @{key = Get-Item path}` (PS 6+). Single-quote literal dla JSON body żeby nie było `$var` interpolacji.

- **IPC `build_request_snippet(request_id, env_id, format)`** zastąpił `build_curl`. Dzieli `load_env_context` z `execute_request` (env resolve + decryption). EnvLocked propaguje normalnie.

### Frontend

- `SnippetFormat` TS type + `snippetFormatLabel()` + `SNIPPET_FORMATS` exported ze store
- `app.copyRequestAsSnippet(requestId, format)` — flush save → IPC → clipboard → format-aware toast (`Copied Python (requests) to clipboard`); EnvLocked → unlock modal z retry
- **`ContextMenu` rozszerzony o `children?: MenuItem[]`** — hover/focus na rzędzie z dziećmi otwiera **submenu** anchored przy prawej krawędzi (recursive self-import per Svelte 5, nie deprecated `<svelte:self>`). Rząd dostaje `›` caret indicator + `active` state.
- **`RequestsPanel`** context menu: stary "Copy as curl" → **"Copy as…"** parent z 5 dziećmi (curl / Python (requests) / HTTPie / JavaScript (fetch) / PowerShell)
- **`CommandPalette`**: zamiast jednej akcji 5 osobnych (`Copy as curl: {name}`, `Copy as Python (requests): {name}`, etc.) — searchable per format ("python" → Python, "fetch" → JavaScript)

### Bugfix po user teście

- **Submenu click nie odpalał akcji** — outside-click handler robił
  `document.querySelector('.menu')` (zwraca tylko PIERWSZY match = root menu),
  więc klik w submenu rzędzie był traktowany jako outside → `onClose()` na
  root → cała struktura unmountowała się na `mousedown` zanim `click`
  dotarł do submenu. Fix: `querySelectorAll('.menu')` + iteracja przez
  wszystkie otwarte menu w drzewie.

### Quality gates

- `pnpm check` ✓ (279 plików)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test --workspace` ✓ (**163/163**: **78 core** (50 base + 17 curl + 6 python + 8 httpie + 6 javascript + 6 powershell = 43 snippet tests) + 21 crypto + 2 http + 56 openapi + 1 integration + 5 storage)
- `pnpm build` ✓

---

## Faza 9 — częściowo DONE ✅ (2026-05-17)

**Status:** trzy z pięciu punktów MVP wrap-up zamknięte: History panel UI,
`⌘1/2/3` focus shortcuts, fresh-DB onboarding overlay. Zostały **app icon**
i **packaging/signing** — wymagają osobnej decyzji designu / setupu CI.

### History panel (backend + UI)

- **Backend (`apiovnia-storage`)**: `HistoryRepo::get(id)` dorzucone (do
  rehydracji jednego wpisu). `list_recent` było już z Phase 3.
- **Backend (IPC w `commands/execution.rs`)**:
  - `list_history(limit = 200) -> Vec<HistoryRowDto>` — DTO z metadanymi
    (request name, project/collection/env names, status code, duration,
    method, url, error_message do 140 znaków). Per-call cache (req/coll/
    proj/env) trzyma lookupy w O(distinct ids) zamiast O(rows).
  - `get_history_response(history_id) -> Option<ExecutionResult>` — zwraca
    pełny ExecutionResult zrekonstruowany z zapisanego row'a. Współdzieli
    funkcję `rehydrate()` z `get_last_response` (refaktor — wcześniej
    inline ściana w `get_last_response`).
- **Frontend (`HistoryPanel.svelte`)**: 460 px slide-in overlay od lewej
  (slide-in animation 180 ms), nad title-barem. Filter input (substring
  match po name + url + method + project + collection + env), licznik w
  nagłówku, refresh button, Esc/X/scrim → close.
  - Per-row: MethodBadge + request name + collection crumb + URL (mono,
    truncate) + status pill (zielony 2xx / amber 3-4xx / czerwony 5xx lub
    error) + relative time (HH:MM same-day / "12 May · HH:MM" older) +
    env name (jeśli był).
  - Click row → `app.openHistoryEntry(entry)` → navigate top-down do
    project/collection/request → rehydrate response do prawego pane'a.
  - Deleted request → row pokazuje `(deleted request)`, klik nadal
    pokazuje zapisany response (nawigacja non-fatal).
- **Wiring**: ProjectsPanel footer's history icon (był no-op) → toggle.
  App.svelte mount conditional na `app.historyPanelOpen`. Store:
  `historyPanelOpen`, `historyEntries`, `historyLoading`,
  `openHistoryPanel`/`closeHistoryPanel`/`toggleHistoryPanel`/`refreshHistory`/
  `openHistoryEntry`.

### `⌘1/2/3` focus shortcuts

- Każdy z trzech inputów dostaje `data-focus-target="left|mid|right"`:
  - left → ProjectsPanel filter input
  - mid → RequestsPanel filter input
  - right → UrlBar's URL input
- `keymap.ts` rozszerzony: nowy handler na keys `1`/`2`/`3` z mod-key
  robi `document.querySelector('[data-focus-target="..."]')` → `focus()`
  + `select()` jeśli to input. Działa również wewnątrz innych inputów —
  zmiana focusu JEST intencją.

### Fresh-DB onboarding overlay

- `OnboardingOverlay.svelte` — full-shell card (560 px max) wyrenderowany
  tylko gdy `!app.loading && app.projects.length === 0`. Replaces the
  three-empty-panels first impression.
- Sekcje: brand header (logo + name + tagline), big "Welcome." headline +
  lede, dwa CTA buttony ("Create your first project" primary +
  "Start from OpenAPI spec…" secondary), 3-step tour panelu (lewy/środek/
  prawy), footer z keyboard shortcuts cheat-sheet.
- "Start from OpenAPI" path: prompt na project name → file picker →
  `createProject` → `importOpenapiForProject`. Po stworzeniu pierwszego
  projektu overlay znika (cascade auto-pick + DetailPanel's per-state
  CTAs przejmują dalszy funnel).

### Quality gates

- `pnpm check` ✓ (281 plików, 0/0/0)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test --workspace` ✓ (**169/169**: 84 core + 21 crypto + 2 http +
  56 openapi + 1 petstore integration + 5 storage)
- `pnpm build` ✓ (699 kB JS / 236 kB gzip)

### Co zostało z Phase 9

- **Packaging** — `tauri.conf.json` ma `bundle.targets: "all"` ale brak
  signing config. Build dla macOS (.dmg + .app), Linux (.deb +
  .AppImage), Windows (.msi) nice-to-have. Auto-update **skip** dla MVP.

---

## Faza 9.5 — UI polish & app icon — DONE ✅ (2026-05-17)

Drobne zmiany z user-testów po Phase 9, plus dodanie ikony aplikacji.

### Keyboard shortcuts swap (⌘K ↔ ⌘P)

- **Było:** ⌘K → palette, ⌘P → focus left filter
- **Jest:** ⌘P → palette (matches Postman/Insomnia), ⌘K → focus left filter
  (matches Slack/Linear sidebar search convention)
- Zmiany: `keymap.ts` (global handler), `ProjectsPanel.svelte` (lokalny
  filter handler + kbd hint), `OnboardingOverlay.svelte` (cheat-sheet —
  teraz pokazuje obie pary), komenarze w `CommandPalette.svelte` i
  `app.svelte.ts` zaktualizowane na ⌘P.

### TitleBar — Search button + Apiovnia logo

- **Search button** w prawej części titlebara: visible alias dla palety —
  `⌘P` kbd hint po prawej, `onclick={() => app.openPalette()}`. Działa
  identycznie jak skrót klawiszowy. (Wcześniejsza iteracja "placeholder
  bez akcji" była błędem — user chciał click→palette + kbd hint na miejscu.)
- **Logo** w lewym górnym rogu: 18×18 SVG honeycomb (mid variant z designu
  — outline hex + filled center dot, amber `#F59E0B`) + bold "Apiovnia"
  napis obok. Pierwszy element titlebara, na tym samym pasku co breadcrumbs.

### ProjectsPanel footer — honeycomb avatar

- **Było:** orange-square `<div>A</div>` (temporary placeholder z startera).
- **Jest:** ten sam 20×20 SVG honeycomb co w TitleBar. Spójność wizualna —
  jedna marka, dwa miejsca.

### App icon (honeycomb)

- **Master source**: `design_artifacts/icons/master/apiovnia-1024.png`
  (z towarzyszącym `apiovnia.svg`) — przygotowany ręcznie wg designu z
  `App Icon - Honeycomb Set.html`. Wcześniej próbowaliśmy renderować
  z SVG przez ImageMagick + Inkscape — wyglądało źle (pomarańczowa
  kropka w czarnym kwadracie zamiast hex'a), bo Inkscape w snapie ma
  problemy z dostępem do plików. Użycie przygotowanego masteru z
  designu jest jedyną wiarygodną ścieżką.
- Cała generacja: `pnpm tauri icon design_artifacts/icons/master/apiovnia-1024.png`.
  Tauri pluje ios/ + android/ + MSIX Square*Logo.png + StoreLogo.png — nie
  używamy ich (MVP target = macOS + Linux + Windows .ico bez Microsoft
  Store), więc kasujemy. Zostaje **minimalny zestaw**:
  - `apiovnia.svg` (kopia z designu, vector reference)
  - `32x32.png`, `64x64.png`, `128x128.png`, `128x128@2x.png`
  - `icon.png` (512²), `icon.icns` (macOS), `icon.ico` (Windows)
- `tauri.conf.json` był już prawidłowy, plain regen.
- **Logo TitleBar + ProjectsPanel footer**: oba mountują inline SVG
  honeycomb (mid variant — amber outline hex + filled center dot)
  zamiast image-tag z plikiem. Lekkie (~200 B markup), nie potrzebują
  dodatkowego fetch'a, idealnie się skalują.

### Footer (DetailPanel)

- **Było:** `req_5a1c…` (16-znaków hash request id) w prawym dolnym rogu.
  Debug-grade artifact, nikomu niepotrzebny w prod.
- **Jest:** `v0.1.0` (app version) — czytane z `package.json` przez
  Vite `define: { __APP_VERSION__: JSON.stringify(pkg.version) }`.
  Globalna deklaracja w `src/vite-env.d.ts`. Zero runtime cost — string
  literal podstawiany przy buildzie. Klasy `.version` (dim color +
  letter-spacing) żeby nie odciągał uwagi.

### Body editor — Beautify removed

- Usunięty button "Beautify" z `BodyTab.svelte` (był visible tylko dla
  JSON body type). Pure UI cleanup — funkcja `beautify()` też usunięta
  (faktycznie działała: `JSON.stringify(JSON.parse(content), null, 2)`,
  ale w praktyce CodeMirror + JSON syntax highlight + parse-lint banner
  pokazują od razu czy JSON jest prawidłowy, a 99% wartości w API klientcie
  to już pretty JSON. Re-format on-demand nie wnosił wartości).
- Orphan `.grow` style usunięty.

### Tests tab removed

- Tab "Tests" w `DetailPanel.svelte` (był `soon: true, disabled: true`
  placeholder) usunięty z `TabId` union + spec. Out-of-scope dla MVP
  (i nadal — pre-request scripts / test assertions to różne narzędzia,
  nie API client).

### Tab spacing — Params / Headers

- `ParamsTab` + `HeadersTab` dostały `padding-top: 8px` na `.wrap`.
  Wcześniej `KeyValueTable` zaczynała się 1px od dolnej krawędzi taba —
  wyglądało jakby skleiło się do `Tabs` headera. Z 8px breath room
  czytelne że "tu zaczyna się treść".

### Theme set documentation (Phase 11 spec lock-in)

- Phase 11 sekcja w plan.md zaktualizowana: **5 themes** zamiast "3-4
  motywy + opcjonalnie":
  1. `apiovnia` (default) — current amber/dark
  2. `atomic-dark` — minimalistyczny czarny, zero color noise
  3. `tokyo-night` — granatowo-fioletowy (Zed/Neovim port)
  4. `monokai` — klasyczny Sublime look, różowe akcenty
  5. `light` — biała baza, wymaga audytu wszystkich rgba/opacity overlayów
- Inspiracja: Zed editor's palette.
- Light theme caveat: audyt `rgba(0,0,0,…)` shadows, `color-mix` overlay
  na erroach/ok states, scrim alpha — w dark wszystko ginie w tle, w light
  wybielają się.

### Quality gates

- `pnpm check` ✓ (284 plików, 0/0/0)
- `cargo clippy --workspace --all-targets -- -D warnings` ✓
- `cargo test --workspace` ✓ (**169/169** — bez zmian, frontend-only polish)
- `pnpm build` ✓ (699 kB JS / 236 kB gzip)

---

## Fazy 9 (reszta) + 10-11 — PENDING

### Punch-list co realnie zostało

**Faza 10 — Security & UX hardening** _(post-MVP, opcjonalne — patrz niżej)_
- Configurable auto-lock timeout, UI countdown, lock-on-blur/system-lock,
  change-master-password, per-field secrets, hardware keychain wrap,
  brute-force throttling, audit log, full-DB encryption (SQLCipher),
  crypto rotation/versioning. Pełna lista w sekcji "Faza 10 — pomysły".

**Faza 11 — Settings panel + themes** _(post-MVP, ~2-3 dni)_

Settings ikonka w lewym dolnym rogu już jest w designie ale nic nie robi.
Domknąć: otwiera modal/panel z konfiguracją per-user.

- `SettingsModal.svelte` (lub side-panel sliding from right) z sekcjami:
  - **Appearance** — theme picker (**5 motywów**, locked-in w Phase 9.5):
    1. `apiovnia` (default) — current amber/dark, source of truth dla tokens
    2. `atomic-dark` — minimalistyczny czarny z subtelnymi neutralnymi
       akcentami; przeciwwaga dla amber, dla osób które chcą "zero color
       noise"
    3. `tokyo-night` — granatowo-fioletowy, popularny Zed/Neovim port,
       blue/cyan akcenty zamiast amber
    4. `monokai` — klasyczny czarno-zielony Sublime/TextMate look, różowe
       akcenty
    5. `light` — biała baza, ciemne akcenty (no nareszcie); jedyny non-dark
       motyw, wymaga oddzielnego review wszystkich `color-mix(...)` i opacity
       overlayów żeby nie wybielały się do invisible
  - Inspiracja zestawem: Zed editor's theme palette. Wszystkie 5 podzielają
    ten sam layout, tylko CSS variables się zmieniają.
  - Theme = zestaw CSS variables w `app.css`; przełącznik nadpisuje
    `:root` w klasie body. Tokeny już są wycentralizowane, więc dodanie
    motywu = nowa CSS var bundle (~30 zmiennych). Persisted w localStorage.
  - Per-theme CodeMirror palette: każdy motyw potrzebuje swojego mapowania
    `--j-key/-string/-number/-bool/-null/-punct` (JSON viewer +
    CodeMirror highlight). Patrz `app.css` jak `apiovnia` to definiuje.
  - **Light theme caveat** — wymaga audytu wszystkich miejsc gdzie zakładamy
    dark: rgba overlays (np. `rgba(0,0,0,0.45)` shadows), color-mix
    z `var(--err)`/`var(--ok)`, scrim opacity. Lista wystąpień do zebrania
    przed implementacją.
  - Theme = zestaw CSS variables w `app.css`; przełącznik nadpisuje
    `:root` w klasie body. Tokeny już są wycentralizowane, więc dodanie
    motywu = nowa CSS var bundle (~30 zmiennych). Persisted w localStorage.
  - **Security** (z Phase 10 hooków, jak będą):
    - Auto-lock timeout (5 / 10 / 15 / 30 min / never)
    - Lock-on-blur toggle
    - Lock-on-system-lock toggle
  - **Editor**:
    - JSON body editor: tab size (2/4), trailing newline on save
    - Send timeout (5/15/30/60s)
    - Max response body size (1/2/5/10 MiB)
  - **Network**:
    - Proxy URL (HTTP/HTTPS/SOCKS5)
    - Custom CA cert path (file picker)
    - Verify TLS toggle
  - **History**:
    - Retention (last N executions, default 200; bumpable do 1000)
    - "Clear all history" destructive button
  - **About** — version, repo link, license, "Built with Tauri/Svelte/Rust"
- Storage: `app_settings` SQLite table (single row, JSON blob) + cached
  w nowym `lib/stores/settings.svelte.ts`. Loaded once on boot.
- Theme apply: `$effect` w App.svelte ustawia `document.documentElement.dataset.theme`
  na zmianę → CSS `:root[data-theme="monokai"] { --bg: ... }`.

**Audit do zrobienia przed implementacją:** prześledzić appkę i zebrać
listę hardcoded'ów które mają sens jako settings:
- timeouty (executor: 30s — hardcoded)
- debounce save (250ms — hardcoded)
- body cap (2MiB — hardcoded)
- history cap (200 — hardcoded w design intent)
- panel sizes default (już persisted ale bez UI do resetu)
- font size (UI 12-14px, mono 12-13px — może `compact|cozy|comfortable`?)

---

### Otwarte feature-flagi do dokończenia (mapowanie 1:1 do faz)

| Co | Status | Phase |
|---|---|---|
| History panel UI | ✅ done | 9 |
| `⌘1/2/3` focus panel | ✅ done | 9 |
| Onboarding empty state | ✅ done | 9 |
| App icon (honeycomb) | ✅ done | 9.5 |
| TitleBar logo | ✅ done | 9.5 |
| ⌘P palette / ⌘K filter swap | ✅ done | 9.5 |
| Footer shows version | ✅ done | 9.5 |
| Beautify / Tests / search btn cleanup | ✅ done | 9.5 |
| **Packaging (signing, bundle targets)** | placeholdery | **9 (next)** |
| Hardening features | brak | 10 |
| Settings modal + themes (5 themes spec'd) | brak (ikona już jest, no-op) | 11 |

### Faza 10 — pomysły / dług bezpieczeństwa

Zebrane po Phase 6, do rozważenia gdy MVP wyląduje:

- **Configurable auto-lock timeout** — settings: `5 / 10 / 15 / 30 min / never`.
  10 min hardcoded jako default; pro userzy z napompowanym wątkiem chcą
  dłuższe okno, paranoidalni krótsze. Trzymać per-user, nie per-env.
- **UI countdown** — `unlocked · 7m 23s left` w `EnvSelectorze` przy
  encrypted+unlocked env. Wymaga frontend polla `is_env_unlocked` (co 30s?)
  + nowego IPC `idle_seconds_remaining(env_id) -> u64`. Albo backend emituje
  `tauri::Event` na 60s przed auto-lockem, frontend pokazuje subtle toast
  "prod will lock in 1 minute — keep working to stay unlocked".
- **Lock-on-app-blur opcja** — gdy okno Apiovni straci focus na > N sekund,
  evictuj wszystkie klucze. Niektórzy security folks tego oczekują.
  Wymaga `tauri::WindowEvent::Focused(false)` listenera.
- **Lock-on-system-lock** — gdy OS robi screen lock, my robimy nasz lock.
  Linux: `org.freedesktop.ScreenSaver.ActiveChanged` D-Bus signal. macOS:
  `NSWorkspaceDidWakeNotification` / `NSWorkspaceSessionDidBecomeActiveNotification`.
  Windows: `WTSSessionChange`. Realnie tylko jeśli ktoś z testerów o to
  poprosi.
- **Change master password flow** — obecnie nie da się zmienić hasła do
  encrypted env bez disable+enable cyklu. Nowy IPC `change_env_password(
  env_id, current, new)`: verify current → derive new key → re-encrypt
  wszystkie rows → zaktualizować salt + password_check. Atomic w jednej tx.
- **Per-field encryption flag** — zamiast whole-env on/off, per-variable
  `is_secret` faktycznie respektować (jest już w schemacie). Wtedy
  `base_url=https://api.dev.example` zostaje plaintext (debuggable), a
  `api_key` zaszyfrowane. Trochę więcej UI work bo każdy row potrzebuje
  toggla.
- **Hardware-backed key wrap** — opcjonalnie zawijać derived key w OS
  keychain (macOS Keychain, libsecret/GNOME Keyring na Linuxie, Windows
  Hello/DPAPI). User wpisuje OS password raz, my zapamiętujemy klucz tam
  zamiast w RAM-ie. Trade-off: szybsze życie codzienne, większe blast
  radius (jak ktoś przejmie OS keychain, ma wszystko).
- **Brute-force throttling** — design canvas mentions "Three wrong
  attempts will throttle prod requests for 60s." Łatwy do dodania:
  counter wrong attempts per env w SessionKeyStore + cooldown timer.
  Niski threat na local-only app, ale UX hint że "ktoś się dobiera"
  użyteczny.
- **Audit log opcjonalny** — `unlock_log` table z (env_id, timestamp,
  result). User w settings widzi historię "prod unlocked 7 razy
  w tym tygodniu, ostatni raz 2 godziny temu". Privacy-positive bo
  wszystko lokalne.
- **Encrypted-at-rest cały DB** — alternatywa do per-env: cały
  `apiovnia.db` zaszyfrowany przez SQLCipher. Wymaga unlock app-level
  zamiast env-level. Plus: jednorazowe wpisanie hasła. Minus: pasuje
  do MVP "envs sealed, ale projekty publiczne" gorzej. Może opcja w
  settings: "wymagaj master password przy starcie app".
- **Crypto rotation / versioning** — nasz format ciphertext nie ma
  version byte. Jakbyśmy chcieli kiedyś zmienić KDF (Argon2 → scrypt?
  albo bumpnąć m_cost), nie ma czystej ścieżki migracji. Format v2 z
  jawnym prefiksem `apv2|nonce|ct|tag` + path `re_encrypt_v1_to_v2`.

### Co konkretnie zostało (snapshot stanu)

**Działa end-to-end:**
- Pełny CRUD na projektach/kolekcjach/requestach z persisted active selection
- Request editor (URL/method/params/headers/body/auth/env-overrides) z debounced save
- Body types: None/JSON/Form/Multipart(text+file)/Raw — wszystkie wysyłane poprawnie
- HTTP execute z reqwest, full response viewer (Pretty JSON / Headers / Request / Raw)
- Env CRUD + variables + per-(req,env) overrides + resolver + `{{var}}` interpolacja
- **Encrypted envs** z master password (Argon2id + AES-256-GCM), zxcvbn
  strength meter, pro-bypass, idle auto-lock 10 min
- **Command palette** ⌘K + ⌘N skrót, fuzzy ranking, cross-project switch,
  per-env akcje (enable/disable encryption, lock, manage)
- **Copy as…** submenu z 5 formatami (curl / Python requests / HTTPie /
  JavaScript fetch / PowerShell), pełna env resolution + decryption,
  EnvLocked auto-unlock retry; dostępne z context-menu + palette per format
- **OpenAPI import** (oas3, `$ref` resolution + dummy synthesis z typed
  defaults + format hints) + **export** (z secret-scrubbing per-typ
  placeholderami, inferred per-request schema w components z `$ref`,
  collision abort, `{project}_{collection}.yaml` filename)
- **OpLog panel** — persistent bottom-right tabelka z "Download log" button
- Global toast host (transient bottom-right feedback)
- Filtry (text + method), cascade auto-pick, smart empty states z CTA

**Działa od Phase 9 (2026-05-17):**
- **History panel** — slide-in od lewej (toggle: ikona w left footer),
  lista 200 ostatnich wykonań z filterem, status pill, time, env badge.
  Klik → navigate do origin request + rehydrate stored response.
- **`⌘1/2/3`** — focus left filter / mid filter / URL bar via
  `data-focus-target` selectors.
- **Onboarding overlay** — fresh-DB full-shell welcome z primary CTA
  ("Create your first project") + secondary ("Start from OpenAPI spec…")
  + 3-step tour + keyboard shortcuts cheat-sheet.

**Działa od Phase 9.5 (2026-05-17):**
- **App icon (honeycomb)** — pełny set: 32/64/128/128@2x PNG +
  icon.icns/icon.ico + MSIX tiles. Master SVG + rendered PNG w
  `src-tauri/icons/`.
- **TitleBar logo** — 18px honeycomb mark + bold "Apiovnia" napis lewy
  górny róg, na tym samym pasku co breadcrumbs.
- **Shortcut swap**: ⌘P → palette, ⌘K → focus left filter (Postman/Slack
  conventions).
- **Footer version**: `v0.1.0` zamiast `req_xxx` hash.
- **UI cleanup**: Beautify button (BodyTab) + Tests tab (DetailPanel) +
  Search button kbd hint usunięte; TitleBar Search teraz placeholder bez
  onclick. Params/Headers tabs dostały 8px top padding (no longer touch
  border).

**Brak feature'u w ogóle:**
- Packaging dla release — `tauri.conf.json` ma `bundle.targets: "all"` ale
  bez signing config (Phase 9)
- Settings panel + themes — ikona jest, no-op; **5 themes spec'd**
  w Phase 11 (apiovnia / atomic-dark / tokyo-night / monokai / light)
- Phase 10 hardening — patrz lista pomysłów wyżej

### Otwarte tematy / dług techniczny

- **CodeMirror "trójkącik" w gutterze** — przy obecnym setupie zhide-owany
  przez CSS `*` selektor. Jeśli wraca, kandydat na `markerFilter` w
  `lintGutter` config.
- **Multipart file size** — żadnego capa. 500 MB ZIP ląduje w RAM podczas
  Send. Akceptowalne na MVP.
- **Per-body-field dotted-path overrides** (Phase 5 design) — odpuszczone,
  whole-body content override starcza na 95% case'ów.
- **Phase 10 hardening** — pełna lista wyżej; nic z tego nie blokuje MVP.

### Tooling notes na następną sesję

- `source .envrc.local` w root przed czymkolwiek.
- `cd apiovnia-app && pnpm tauri:dev` — pełny dev z HMR.
- DB w `~/.local/share/tech.trurl.apiovnia/apiovnia.db`.
- Po zmianie migracji SQL: usunąć/zmigrować lokalną DB lub zacząć od świeżego pliku (`rm` po backup).
- Po zmianie pól `ExecutionResult`/`SentRequest` (Rust) — restart `tauri:dev` (frontend HMR samego nie wystarczy, Rust binary się zmienia).

