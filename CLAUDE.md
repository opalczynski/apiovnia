# CLAUDE.md

Notes for Claude (and future-me) working in this repo. Keep it short and accurate — anything stale is worse than nothing.

## Documentation upkeep (rule of the house)

After **any meaningful change** — new feature, new dependency, new gotcha, removed feature, changed flow, finished phase — sweep the user-facing docs in the same change:

- [`README.md`](./README.md) — the "what works today" list, the roadmap table, the architecture diagram, the tech-stack bullets. Keep it accurate enough that a fresh reader trusts it.
- [`CLAUDE.md`](./CLAUDE.md) — this file. Update the "what's done" snapshot, gotchas, repo layout if dirs/files moved, conventions if any actually changed.
- [`plan.md`](./plan.md) — always update; that's the live progress log.

When a `docs/` directory lands at repo root (post-MVP, planned), add it to this list. Anything that drifts becomes worse than nothing — better to delete a stale paragraph than leave it lying.

## What this is

**Apiovnia** — local-first REST API client for solo devs. Tauri 2 desktop app (Rust backend + Svelte 5 frontend), SQLite as the single source of truth, optional master-password encryption per environment.

Authoritative product brief: `instruction.md`. Implementation plan + per-phase status: `plan.md`. Design canvas (React HTML mockups + tokens): `design_artifacts/`.

## Repo layout

```
.
├── instruction.md            # product brief (immutable spec)
├── plan.md                   # phased implementation plan + progress log
├── design_artifacts/         # static HTML/JSX design canvas + tokens.css
├── apiovnia-app/             # the actual app (Tauri 2 + Vite + Svelte 5)
│   ├── src/                  # Svelte frontend
│   │   ├── App.svelte
│   │   ├── app.css           # design-token CSS, ported from design_artifacts/tokens.css
│   │   └── lib/
│   │       ├── api/ipc.ts            # typed `invoke<T>` wrappers (one fn per Tauri command)
│   │       ├── stores/app.svelte.ts  # global $state — selection, save state, response
│   │       ├── stores/panels.svelte.ts
│   │       ├── stores/dialogs.svelte.ts
│   │       ├── types/domain.ts       # TS mirror of apiovnia-core models
│   │       └── components/{layout,panels,request,response,modals,ui}
│   └── src-tauri/            # Cargo workspace
│       ├── Cargo.toml        # workspace root (centralised dep versions)
│       ├── src/              # apiovnia-tauri binary (commands + setup)
│       └── crates/
│           ├── apiovnia-core/      # domain models (Project, Request, Env, …), no Tauri/SQL deps
│           ├── apiovnia-storage/   # sqlx + migrations + repos
│           ├── apiovnia-http/      # reqwest executor
│           ├── apiovnia-crypto/    # Argon2id + AES-256-GCM + zxcvbn (Phase 6)
│           └── apiovnia-openapi/   # oas3 import/export (Phase 7, scaffold only)
└── .envrc.local              # local PATH for rust + pnpm (not committed)
```

## Running it

```bash
source .envrc.local                     # puts cargo/pnpm on PATH (Linux dev box)
cd apiovnia-app
pnpm install                            # first time only
pnpm tauri:dev                          # full dev — Vite HMR + Rust rebuild on save
```

Frontend-only smoke (no native window):

```bash
cd apiovnia-app && pnpm dev
```

Local SQLite lives at `~/.local/share/tech.trurl.apiovnia/apiovnia.db` (Linux XDG). Delete it for a fresh state.

## Quality gates (run before declaring a phase done)

```bash
cd apiovnia-app
pnpm check                                                # svelte-check + tsc
cargo --manifest-path src-tauri/Cargo.toml clippy --workspace --all-targets -- -D warnings
cargo --manifest-path src-tauri/Cargo.toml test --workspace
pnpm build                                                # vite production bundle
```

For UI-affecting phases also do a manual `pnpm tauri:dev` run.

## Conventions

- **Svelte 5 runes only** (`$state`, `$derived`, `$effect`). No `writable()` stores — global state lives in `lib/stores/*.svelte.ts` modules that export a proxy object with getters + actions. Components do `import { app } from "$lib/stores/app.svelte"`.
- **TS ↔ Rust models**: hand-maintained mirror in `lib/types/domain.ts`. Rust serialises `#[serde(rename_all = "camelCase")]`; the TS side matches. No `ts-rs` codegen on MVP — too much ceremony.
- **IPC**: one thin wrapper per Tauri command in `lib/api/ipc.ts`. Commands themselves stay under ~20 lines — they delegate to a repo or executor.
- **Rust crates are layered**: `core` has zero Tauri/SQL deps and is the only place pure domain logic (resolver, interpolation, validation) goes. `storage`/`http`/`crypto`/`openapi` depend on `core`. `apiovnia-tauri` (binary) depends on all of them and exposes commands.
- **Database migrations are additive** and never modified after release. New tables/columns go in a new `migrations/000N_*.sql` file.
- **Design tokens** live as CSS variables (`--bg`, `--accent`, `--m-get`, `--j-string`, …). Component styles reference them directly. Tailwind v4 `@theme` block exposes a subset as utility classes (`bg-bg`, `text-fg`, …) but most of the styling is plain CSS in scoped `<style>` blocks to keep parity with `design_artifacts/`.
- **Dark mode only** for MVP. Light mode is out of scope.
- **Comments**: only when the WHY is non-obvious. `app.css` and `CodeMirrorEditor.svelte` carry the longer notes because the CSS specificity / lint plumbing both have real gotchas.

## Save-cycle state machine (frontend)

Active request edits run through `app.updateActiveRequest(patch)` → debounced 250ms → `flushSave()`. The visible state machine:

- `idle` — clean, grey dot, no label
- `editing` — amber dot + "editing" (in the debounce window)
- `saving` — amber pulse + "saving…" (IPC in flight)
- `saved` — green dot + "saved" for 800ms, then fades to idle

`selectRequest()` always flushes pending saves before switching.

## What's done (snapshot — check `plan.md` for the live status)

- Phase 0 — bootstrap, three-panel layout, design tokens, fonts
- Phase 1 — SQLite storage + projects/collections/requests CRUD live in UI
- Phase 1.5 — custom dialogs (`prompt`/`confirm`) + context menus
- Phase 2 — request editor (URL bar, method, params/headers/body/auth tabs, CodeMirror, debounced save)
- Phase 3 — HTTP execution (`reqwest`), response viewer (Pretty/Headers/Request/Raw), JSON lint in body editor, history persistence
- Phase 4 — custom JSON viewer (`JsonView.svelte`) with collapsible nodes, ⌘F search + next/prev nav, hover-copy per value, expand/collapse all
- Phase 5 — Environments + per-`(request, env)` overrides + `{{var}}` interpolation. Pure `resolve_request` in `apiovnia-core` (33 tests). `EnvSelector` in UrlBar, `EnvManageModal`, `EnvOverridesTab`. Per-project active env persisted in localStorage. `execute_request(req, env)` routes through the resolver.
- Phase 5.5 — `BodyType::Multipart` (text + file parts via `tauri-plugin-dialog` picker). Request-tab snapshot synthesizes its own RFC-7578 preview because reqwest can't `try_clone()` streaming bodies.
- Phase 5.6 — filtres (text + method multi-select), cascade auto-pick first collection/request on switch, smart empty-state CTAs in DetailPanel, stacked Collections header, Send-elapsed timer
- **Phase 6 — Encrypted environments.** Master-password sealing per env: Argon2id (OWASP 2024 baseline) → AES-256-GCM. Frontend never sees the key — resolver decrypts in Rust before `execute_request`. zxcvbn-graded password meter with `~3 days` / `~centuries` crack-time line and a pro-user bypass checkbox. `EnvLocked` storage error (`ENV_LOCKED:{envId}` prefix) auto-opens the unlock modal with an optional retry callback. **10 min lazy idle auto-lock** — checked on every `with_key` access, evicted entries are `zeroize`d on drop. 21 crypto tests (13 AEAD + 8 policy).
- **Phase 8 — Command palette + Copy as… submenu.** ⌘K spotlight-style palette with custom fuzzy ranking, cross-project switch, per-env actions (`Enable/Disable encryption for {env}`, `Lock {env}`, `Manage envs`), one-key `⌘N` new request. Global `ToastHost` for transient feedback. **Copy as…** submenu in `RequestsPanel` context menu + per-format palette items, dispatching to `apiovnia-core::snippets::SnippetFormat::{Curl, PythonRequests, Httpie, JavaScriptFetch, PowerShell}` (43 unit tests). Shares `load_env_context` helper with `execute_request` so encrypted envs decrypt the same way; `EnvLocked` triggers unlock modal with auto-retry. `ContextMenu` supports nested `children` rendered as hover submenus (recursive self-import, Svelte 5 way). `EnvManageModal` + `SetEnvPasswordModal` are globally mounted in `App.svelte` (lifted from `DetailPanel`) so the palette can open them without prop-drilling.
- **Phase 7 — OpenAPI 3.x import + export.** `apiovnia-openapi` crate (56 unit tests + 1 integration against real petstore): `redact` (typed `<your-bearer-token>` / `<your-password>` placeholders per category), `export` (builds `oas3::Spec` with required fields + inferred per-request schema in `components.schemas` referenced by `$ref` from media type, format detection for `date-time`/`email`/`uuid`/`uri`, abort-on-collision for `(method, path)`), `import` (YAML/JSON via `oas3`, resolves `requestBody.content[json].schema.$ref` and **synthesises a dummy** from `properties`/`items`/`allOf` with depth guard, format hints, multi-server → `Environment`s). IPC: `import_openapi` / `export_collection_openapi` / `save_text_file`. **Persistent `OpLogHost` panel** (bottom-right, no auto-dismiss) shows tabular per-request rows + collapsible warnings + "Download log" button writing `.log` with `Utc::now()` timestamp. Export filename is `{project}_{collection}.yaml` — build IPC runs FIRST so the suggested name knows the project context.

Phases 9-11 are queued in `plan.md`. **Phase 9** (app icon, History panel UI, `⌘1/2/3` focus, onboarding, packaging) is the MVP wrap-up. Phase 10 collects security/UX hardening ideas (configurable lock timeout, change-password flow, per-field secrets, hardware keychain wrap, …). Phase 11 is a Settings panel + 3-4 themes (monokai, tokyo-night, light, default).

## Gotchas worth knowing

- After changing a SQL migration, wipe the local DB (or write a new additive migration). The existing `0001_init.sql` / `0002_history_full.sql` are released.
- `pnpm tauri:dev` HMR only covers the frontend; Rust changes need the binary to rebuild, which happens automatically but is slower.
- CodeMirror lint inline marks are hidden via CSS — the gutter dot is the single visual indicator. See the long comment in `app.css` if anything regresses.
- `formatBytes`/`formatDuration` use thin spaces around values via `display: inline-flex` `gap` on `.metric` — HTML whitespace collapse will eat normal spaces.
- Never log decrypted env override values. The decryption key never crosses the IPC boundary; resolver runs in Rust.
- **Bulk env crypto migration runs in one tx.** `enable_env_encryption` / `disable_env_encryption` use `EnvironmentRepo::set_encryption_in_tx` / `clear_encryption_in_tx` + `EnvVariableRepo::rewrite_values_in_tx` + `OverrideRepo::rewrite_row_in_tx` — all in a single `pool.begin().await?`. If you ever add a pool-based variant alongside an open tx, SQLite returns `database is locked` (single-writer + 5s busy timeout).
- **Auto-lock semantics**: `with_key` refreshes `last_used`, `is_unlocked` / `unlocked_ids` only peek + evict. So leaving the unlock modal open doesn't extend a session — active encrypt/decrypt does.
- **Shared env resolution path**: `execute_request` and `build_request_snippet` both call `load_env_context(state, request_id, env_id)` to load + decrypt the override + vars. If you add a third command that needs resolution, reuse this helper — duplicating the encrypted-env branch is the #1 way to make a bug that only fires when an env is sealed.
- **Modal mounts live in App.svelte**: `EnvManageModal`, `SetEnvPasswordModal`, `UnlockEnvModal`, `CommandPalette`, `ToastHost`, `OpLogHost` — all controlled by store flags (`envManageOpen`, `envPasswordSetupId`, `unlockPrompt`, `commandPaletteOpen`, `toast`, `opLog`). Don't mount them locally inside panels; the palette + other meta entry points need to be able to open them from anywhere.
- **ContextMenu outside-click uses `querySelectorAll('.menu')`**, NOT `querySelector`. With submenus we have multiple `.menu` elements in the DOM; checking only the first treats a click on a submenu row as outside-click → `onClose` on `mousedown` → tree unmounts before `click` reaches the submenu handler. We learned that one the hard way.
- **OpenAPI export filename needs project context**, which the frontend doesn't always have. The flow is **build-then-dialog** in `app.exportCollectionInteractive`: IPC first (returns `yamlFilename` = `{project}_{collection}.yaml`), then `tauri-plugin-dialog::save` with that default, then `saveTextFile`. Don't compute the filename frontend-side before the IPC; you'll regress to just-collection-name.
- **Petstore-style `$ref` bodies must be resolved** at import. `oas3::MediaType::schema(spec)` follows the ref; from there `synthesize_from_schema` walks properties/items/allOf with a depth guard. Without this the body imports as `{}` — confusing for users testing with real specs.
