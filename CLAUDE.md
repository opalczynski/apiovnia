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
│   │       ├── stores/settings.svelte.ts  # theme + prefs (Phase 11)
│   │       ├── types/domain.ts       # TS mirror of apiovnia-core models
│   │       └── components/{layout,panels,request,response,modals,ui}
│           # Notables added in Phase 9/9.5: panels/HistoryPanel.svelte,
│           # OnboardingOverlay.svelte, TitleBar logo SVG inline.
│           # Phase 11: modals/SettingsModal.svelte (theme picker).
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
- **Phase 9 (partial) — History panel + `⌘1/2/3` + onboarding overlay.** New IPCs `list_history(limit=200) -> Vec<HistoryRowDto>` (per-call cache for request/collection/project/env name lookups, so it's O(distinct ids) not O(rows)) and `get_history_response(id) -> Option<ExecutionResult>` (shares `rehydrate()` with `get_last_response`). `HistoryPanel.svelte` is a 460 px slide-in from the left, toggled by the icon in `ProjectsPanel.svelte`'s footer (previously a no-op). Row click → `app.openHistoryEntry()` navigates top-down via `navigateToRequest` then restores the saved response. Keymap adds `⌘1/2/3` → `document.querySelector('[data-focus-target="left|mid|right"]')` → `focus()` + `select()`; the three inputs (ProjectsPanel filter, RequestsPanel filter, UrlBar URL input) carry that `data-focus-target` attribute. `OnboardingOverlay.svelte` is a full-shell welcome card rendered only when `!app.loading && app.projects.length === 0` — primary "Create your first project" CTA plus a secondary "Start from OpenAPI spec…" path, plus a 3-step panel tour and a keyboard cheat-sheet. **169 tests total.**
- **Phase 9.5 — App icon, TitleBar logo, shortcut swap, UI polish.** App icon: master SVG (`src-tauri/icons/apiovnia.svg`) per `design_artifacts/App Icon - Honeycomb Set.html` spec (squircle bg gradient + outer faint hex + amber inner hex + filled centre dot + glint), rendered to 1024² PNG via `magick`, then `pnpm tauri icon` generates the full desktop + MSIX + iOS + Android set. TitleBar gets a small honeycomb logo (`<svg>` inline, 18 px) plus an "Apiovnia" label in the top-left, on the same row as the breadcrumbs. **Shortcuts swapped**: `⌘P` is now the command palette (matches Postman/Insomnia), `⌘K` focuses the left-panel filter (matches Slack/Linear sidebar search). The TitleBar Search button is now a non-actionable placeholder (no `onclick`, `tabindex="-1"`, `aria-disabled`) — its function is reserved for a future global search. Footer in `DetailPanel` shows `v0.1.0` (from `package.json` via Vite `define: __APP_VERSION__`) instead of the request id hash. Beautify button removed from `BodyTab`, Tests tab removed from `DetailPanel`, both confirmed unused. `ParamsTab`/`HeadersTab` got `padding-top: 8px` so the first table row breathes from the tab divider.

- **Phase 11 (partial) — Settings panel + themes.** `lib/stores/settings.svelte.ts` mirrors `panels.svelte.ts`: a persisted `$state` blob in `localStorage["apiovnia.settings.v1"]` (`theme` + `historyLimit`) plus a transient, non-persisted `open` flag. **Five themes** as CSS-variable bundles in `app.css`: `apiovnia` (default, lives on bare `:root`), `atomic-dark`, `tokyo-night`, `monokai`, `light`. The store sets `<html data-theme="…">`; `applyTheme()` runs at module-init so a non-default theme doesn't flash the amber default. New shared tokens `--on-accent` / `--scrollbar` / `--scrollbar-hover`; hardcoded amber/white-rgba leaks (CTA gradients, JSON match-highlight, scrollbar, CM active-line, palette ring, lock-icon) converted to `var()` / `color-mix` so themes actually re-skin everything. `SettingsModal.svelte` — left-rail nav (Appearance / History / About), live theme picker, History retention segmented control. Entry points: gear in `ProjectsPanel` footer, `⌘,`, palette "Open settings" action plus one `Theme: {name}` action per theme (last in the list). `refreshHistory` reads `settings.historyLimit`.

What's still left in Phase 9: **packaging/signing** (`tauri.conf.json` has `bundle.targets: "all"` but no signing config). Phase 10 collects security/UX hardening ideas (configurable lock timeout, change-password flow, per-field secrets, hardware keychain wrap, …). Phase 12 is the Settings expansion — send timeout, proxy/TLS, UI density, clear-history — see `plan.md` for the audit grouped by cost. Phase 13 sketches multi-protocol support (GraphQL / WebSocket / SSE / gRPC), a core refactor not yet fully spec'd. Phase 14 automates CI + multi-OS release builds (GitHub Actions + `tauri-action` → GitHub Releases); Phase 15 is a docs/landing site on GitHub Pages. This is a product, not an MVP — the roadmap stays open-ended.

## Gotchas worth knowing

- After changing a SQL migration, wipe the local DB (or write a new additive migration). The existing `0001_init.sql` / `0002_history_full.sql` are released.
- `pnpm tauri:dev` HMR only covers the frontend; Rust changes need the binary to rebuild, which happens automatically but is slower.
- CodeMirror lint inline marks are hidden via CSS — the gutter dot is the single visual indicator. See the long comment in `app.css` if anything regresses.
- **Themes are CSS-variable bundles, nothing more.** `settings.svelte.ts` sets `<html data-theme="…">`; `app.css` has a `:root[data-theme="x"]` block per non-default theme that redefines the ~35 tokens. The `apiovnia` default lives on bare `:root` (no block) — an unknown/missing `data-theme` still renders correctly. Adding or removing a token means touching ALL theme blocks: a token absent from one block silently falls back to the amber default. Components must never hardcode a colour that should theme — use `var(--token)` or `color-mix(in srgb, var(--accent) …%, …)`. `--on-accent` is the text colour for accent-filled surfaces (CTA buttons, active marks). The two honeycomb logo SVGs (`TitleBar`, `ProjectsPanel` footer) are the one deliberate exception — brand amber `#F59E0B`, hardcoded, NOT themed.
- `formatBytes`/`formatDuration` use thin spaces around values via `display: inline-flex` `gap` on `.metric` — HTML whitespace collapse will eat normal spaces.
- Never log decrypted env override values. The decryption key never crosses the IPC boundary; resolver runs in Rust.
- **Bulk env crypto migration runs in one tx.** `enable_env_encryption` / `disable_env_encryption` use `EnvironmentRepo::set_encryption_in_tx` / `clear_encryption_in_tx` + `EnvVariableRepo::rewrite_values_in_tx` + `OverrideRepo::rewrite_row_in_tx` — all in a single `pool.begin().await?`. If you ever add a pool-based variant alongside an open tx, SQLite returns `database is locked` (single-writer + 5s busy timeout).
- **Auto-lock semantics**: `with_key` refreshes `last_used`, `is_unlocked` / `unlocked_ids` only peek + evict. So leaving the unlock modal open doesn't extend a session — active encrypt/decrypt does.
- **Shared env resolution path**: `execute_request` and `build_request_snippet` both call `load_env_context(state, request_id, env_id)` to load + decrypt the override + vars. If you add a third command that needs resolution, reuse this helper — duplicating the encrypted-env branch is the #1 way to make a bug that only fires when an env is sealed.
- **Modal mounts live in App.svelte**: `EnvManageModal`, `SetEnvPasswordModal`, `UnlockEnvModal`, `CommandPalette`, `ToastHost`, `OpLogHost` — all controlled by store flags (`envManageOpen`, `envPasswordSetupId`, `unlockPrompt`, `commandPaletteOpen`, `toast`, `opLog`). Don't mount them locally inside panels; the palette + other meta entry points need to be able to open them from anywhere.
- **ContextMenu outside-click uses `querySelectorAll('.menu')`**, NOT `querySelector`. With submenus we have multiple `.menu` elements in the DOM; checking only the first treats a click on a submenu row as outside-click → `onClose` on `mousedown` → tree unmounts before `click` reaches the submenu handler. We learned that one the hard way.
- **OpenAPI export filename needs project context**, which the frontend doesn't always have. The flow is **build-then-dialog** in `app.exportCollectionInteractive`: IPC first (returns `yamlFilename` = `{project}_{collection}.yaml`), then `tauri-plugin-dialog::save` with that default, then `saveTextFile`. Don't compute the filename frontend-side before the IPC; you'll regress to just-collection-name.
- **Petstore-style `$ref` bodies must be resolved** at import. `oas3::MediaType::schema(spec)` follows the ref; from there `synthesize_from_schema` walks properties/items/allOf with a depth guard. Without this the body imports as `{}` — confusing for users testing with real specs.
- **`⌘1/2/3` panel-focus targets are DOM-attribute driven**, not store-mediated. Each focusable input carries `data-focus-target="left|mid|right"`; `keymap.ts` does a `document.querySelector(...)` and focuses. Adding a new "focusable panel root" → just slap the attribute, no store wiring. Conversely, removing/renaming a target attribute silently breaks the shortcut.
- **`HistoryRowDto` carries `projectId` + `collectionId` even though the names alone would render**. That's because `openHistoryEntry` uses them to `navigateToRequest(projectId, collectionId, requestId)` cross-project — without the ids it'd need a backend lookup per-click. Don't strip them.
- **OnboardingOverlay condition is `!app.loading && app.projects.length === 0`**. If you remove the `loading` guard, the overlay flashes during the boot cascade (projects load → overlay shows for 50 ms → hides again).
- **`__APP_VERSION__` is a Vite build-time constant**, injected via `define` in `vite.config.ts` from `package.json#version`. Don't `import` from `package.json` directly (Vite would bundle the whole JSON). The TS declaration lives in `src/vite-env.d.ts`. To bump the version: update `package.json` AND `tauri.conf.json` (they're separate sources of truth).
- **Icon master lives in `design_artifacts/icons/master/apiovnia-1024.png`** (+ `apiovnia.svg`). To regenerate the runtime icon set: `pnpm tauri icon design_artifacts/icons/master/apiovnia-1024.png`. Tauri also drops `ios/`, `android/`, `Square*Logo.png` and `StoreLogo.png` into `src-tauri/icons/` — we don't ship MSIX/iOS/Android, so delete those after regen. **Keep only:** `apiovnia.svg`, `32x32.png`, `64x64.png`, `128x128.png`, `128x128@2x.png`, `icon.png`, `icon.icns`, `icon.ico`. Don't try to render the SVG locally via ImageMagick+Inkscape — Inkscape-on-snap can't read most paths and the output renders as "amber dot on black square" instead of the honeycomb. The PNG master is the source of truth.
- **TitleBar Search button is an alias for the palette**, not a no-op. Its `onclick` calls `app.openPalette()` and it shows the `⌘P` kbd hint. Don't strip either — they make the shortcut discoverable. The brand logo (top-left) and the ProjectsPanel footer avatar both use the same inline honeycomb SVG (mid variant: amber outline hex + filled centre dot). If you change one, change the other.
- **Linux dev runs show no dock icon** for `pnpm tauri:dev` even with a valid `bundle.icon`. That's a GNOME/KDE thing — the WM associates icons via the installed `.desktop` file's `StartupWMClass`, which only exists after `tauri build` + install. The window icon itself (Wayland client-side decoration / X11 `WM_HINTS`) is fine, but the dock entry stays generic until install. Not a bug to chase.
