# Apiovnia

**A local-first REST API client for solo devs.** Postman without the cloud, the team features, or the launch screen.

Apiovnia is a Tauri 2 desktop app. SQLite is the single source of truth — no sync, no accounts, no telemetry. Environments (`dev` / `stage` / `prod`) live as per-request overrides instead of variable soup, and any environment can be locked behind a master password with everything inside it encrypted at rest.

> Status: **alpha**. MVP under active development. Phases 0–8 done (storage, request editor, HTTP execution, rich JSON viewer, environments + per-request overrides + `{{var}}` interpolation, multipart bodies with file parts, **master-password sealing for encrypted envs**, **command palette + Copy as… submenu with 5 formats**, **OpenAPI 3.x import + export**). Phase 9 (app icon + History panel UI + packaging) is the final MVP wrap-up. See [`plan.md`](./plan.md) for the per-phase tracker.

## Why another API client?

- **One SQLite file.** Back it up like any other file. Inspect it with `sqlite3`. Diff it. Move it between machines.
- **Environments as overrides, not variables.** Define the base request once, then patch fields per env. Switching from `dev` to `prod` swaps the URL, auth, headers — nothing else — and the diff is right there in the UI.
- **Secrets encrypted per env.** Mark an env as locked, set a password once; secrets get sealed with AES-256-GCM and an Argon2id-derived key. Unlock on demand, lock on quit.
- **Keyboard-first.** ⌘K palette, ⌘↵ send, ⌘N new request, ⌘1/2/3 panel focus.
- **No pre-request scripts.** No test assertions. No "workspaces". The roadmap is intentionally short.

## What works today

- Three-panel shell (Projects · Collections · Requests · Editor + Response), resizable, persisted.
- Projects → Collections → Requests CRUD against SQLite. Switching a project auto-cascades to the first collection and first request — no empty-flash.
- Filter inputs in both side panels: text filter over projects + collections (with ⌘P focus), text + HTTP-method-multi-select for requests.
- Request editor: method picker, URL, params, headers, body (None / JSON / **Form (urlencoded)** / **Multipart (text + files)** / Raw), auth (none / bearer / basic / apikey). Edits debounce-save in 250 ms.
- CodeMirror 6 body editor with JSON parse-lint (single gutter marker, banner above the editor on errors).
- HTTP execution (`reqwest`, rustls, gzip, redirects). 30 s timeout, 2 MiB body cap with truncation indicator.
- Response viewer with sub-tabs: **Pretty** (custom JSON viewer with collapsible nodes, ⌘F search + next/prev nav, hover-copy per value, expand/collapse-all; CodeMirror for HTML/XML/plain), **Headers**, **Request** (final URL + headers + body preview sent on the wire — multipart preview is reconstructed in RFC-7578 form with `[N bytes — file contents omitted]` placeholders), **Raw**.
- Request history persisted to SQLite; last successful response rehydrates on app restart and request switch.
- **Environments + per-request overrides + `{{var}}` interpolation.** Define `dev` / `stage` / `prod` envs per project, set per-(request, env) overrides for URL / method / headers / params / body / auth, and reference env variables anywhere via `{{name}}`. Resolution order: `request > env override > base`. Headers and params overrides are **full replacements**, not per-key merges. Pure resolver in `apiovnia-core` (33 unit tests).
- **Master-password sealing per env.** Mark an env as encrypted, set a master password (zxcvbn-graded with a live "cracking time" meter and a pro-user bypass), and every variable value + secret-bearing override field gets AES-256-GCM'd with an Argon2id-derived key. Frontend never sees the key — resolver runs in Rust. **Auto-locks after 10 min idle**; unlock modal pops automatically on any operation that needs the key, with a `retry` hook that re-runs the original Send.
- **Command palette (⌘K).** Spotlight-style switcher with fuzzy ranking across requests / collections / projects / envs of the active project, plus actions (`New X`, `Enable/Disable encryption for {env}`, `Lock {env}`, `Manage envs & variables`, `Copy as {format}: {active request}`, `Import/Export OpenAPI`). `⌘N` opens the new-request prompt.
- **Copy as… submenu (5 formats).** Right-click any request (or use the palette) → **curl** / **Python (`requests`)** / **HTTPie** / **JavaScript (`fetch`)** / **PowerShell (`Invoke-RestMethod`)**. Full env override + `{{var}}` resolution + decryption of encrypted env values. Locked envs trigger the unlock modal with an auto-retry. Per-format proper escaping, native idioms (`--user`/`auth=()`/`btoa()`/`[Convert]::ToBase64String`), multipart with `--form k=@/path` / `files=[(...)]` / `FormData` / `-Form @{... = Get-Item ...}`.
- **OpenAPI 3.x import + export.** Right-click project → "Import OpenAPI…" to ingest any YAML/JSON spec — `$ref`-resolved request bodies get dummy values inferred from the schema (with format hints for `date-time`/`email`/`uuid`/`uri`), `securitySchemes` map to `AuthConfig`, multi-server specs become `Environment`s with per-request URL overrides. Right-click collection → "Export OpenAPI…" to save a YAML with **aggressive secret scrubbing** (typed `<your-bearer-token>` / `<your-password>` / `<your-api-key>` placeholders), inferred per-request schema in `components.schemas` with `$ref` link, and **abort-on-collision** for `(method, path)` clashes. **Persistent OpLog panel** with tabular per-request rows + warnings + "Download log" button writes a `.log` audit trail.
- Smart empty-state CTAs (no project / no collection / no request) drive the create flow from any panel.
- Custom modal dialogs and context menus (no native `prompt`/`confirm`).
- Global toast notifications for transient feedback (Copy success, etc).
- Send-button elapsed timer so slow endpoints don't look frozen.

## On the roadmap (in order)

| Phase | What | Status |
|---|---|---|
| 4 | Rich JSON viewer — collapsible nodes, ⌘F search, hover-copy | ✅ done |
| 5 | Environments + per-request overrides + resolver | ✅ done |
| 5.5/5.6 | Multipart bodies + polish (filtres, cascade auto-pick, empty states) | ✅ done |
| 6 | Master password / per-env encryption (Argon2id + AES-256-GCM, zxcvbn meter, 10 min idle auto-lock) | ✅ done |
| 8 | Command palette + ⌘K/⌘N, palette env actions, **Copy as… submenu (5 formats)** | ✅ done |
| 7 | OpenAPI 3.x import + export (schema inference, secret scrubbing, OpLog panel) | ✅ done |
| 9 | App icon, History panel UI, `⌘1/2/3` focus, onboarding, packaging (.dmg/.deb/.AppImage) | **next** |
| 10 | Security & UX hardening (configurable lock timeout, change-password flow, per-field secrets, hardware keychain wrap, …) | |
| 11 | Settings panel + themes (monokai, tokyo-night, light, default) | |

Out of scope for the MVP: WebSocket / SSE / gRPC / GraphQL, pre-request scripts, response test assertions, sync / sharing / team features, mobile, auto-updater, light theme, drag-to-reorder.

## Architecture at a glance

```
apiovnia-app/
├── src/                      # Svelte 5 frontend (runes, Vite, Tailwind v4)
│   ├── App.svelte
│   └── lib/
│       ├── api/ipc.ts        # typed wrappers around `invoke<T>`
│       ├── stores/           # $state modules: app, panels, dialogs
│       └── components/{layout,panels,request,response,modals,ui}
└── src-tauri/                # Cargo workspace
    ├── src/                  # apiovnia-tauri binary — thin command layer
    └── crates/
        ├── apiovnia-core     # domain models, resolver, interpolation, snippet generators
        ├── apiovnia-storage  # sqlx pool, migrations, repos
        ├── apiovnia-http     # reqwest executor, content-type classification
        ├── apiovnia-crypto   # Argon2id + AES-256-GCM, zxcvbn password policy
        └── apiovnia-openapi  # oas3 import/export, secret scrubbing, schema inference
```

The `core` crate has zero Tauri / SQL / HTTP dependencies — it's where unit-testable logic lives (resolver, variable interpolation, validation). Everything else depends on it.

## Running it locally

Requires Rust stable, Node ≥ 20, pnpm, plus the usual Tauri 2 system deps (`webkit2gtk-4.1`, `libayatana-appindicator3` on Linux — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)).

```bash
# Linux dev box only — adjusts PATH for cargo + pnpm
source .envrc.local

cd apiovnia-app
pnpm install
pnpm tauri:dev          # opens the native window with Vite HMR + Rust hot rebuild
```

Frontend-only smoke (no native window):

```bash
cd apiovnia-app && pnpm dev
```

Local data lives at `~/.local/share/tech.trurl.apiovnia/apiovnia.db` (Linux XDG) / `~/Library/Application Support/tech.trurl.apiovnia/apiovnia.db` (macOS). Wipe the file for a fresh state.

## Quality gates

```bash
cd apiovnia-app
pnpm check                                                                    # svelte-check + tsc
cargo --manifest-path src-tauri/Cargo.toml clippy --workspace --all-targets -- -D warnings
cargo --manifest-path src-tauri/Cargo.toml test --workspace
pnpm build                                                                    # Vite production bundle
```

Rust unit tests cover the storage layer (in-memory SQLite, 5 cases), HTTP content-type classification (2 cases), the env resolver + `{{var}}` interpolation (33 cases), the snippet generators (43 cases across curl/Python/HTTPie/JavaScript/PowerShell — methods, query encoding, auth flavours, all body types, multipart, language-specific escaping), crypto (21 cases — 13 AEAD round-trip / wrong-key / tamper / password check, 8 zxcvbn policy paths), and OpenAPI (56 cases + 1 integration test against the real petstore — redact, export schema inference, import `$ref` synthesis). **163 tests total today.**

## Tech stack

- **Tauri 2.x** — desktop shell, IPC, packaging
- **Rust** — `reqwest` (rustls), `sqlx` (SQLite, async), `argon2` + `aes-gcm` + `zxcvbn`, `oas3` (Phase 7)
- **Svelte 5** — runes API only, no legacy stores
- **Vite 6** — frontend bundler (no SvelteKit — this is a desktop app, SSR is irrelevant)
- **Tailwind v4** — CSS-first `@theme` config, plus plain CSS in scoped `<style>` blocks for design-token parity
- **CodeMirror 6** — body editor (JSON / HTML / XML) with our own dark theme keyed on the design tokens
- **`tauri-plugin-dialog`** — native file picker for multipart file parts

## Repo conventions

- Phased plan in `plan.md` — every phase ships something runnable, no internal-only phases.
- Product brief in `instruction.md` is the immutable contract; deviations get noted in `plan.md`.
- Design canvas (`design_artifacts/`) is the visual reference. Component CSS mirrors `tokens.css` 1:1.
- Notes for Claude / future-me: [`CLAUDE.md`](./CLAUDE.md).
- Commits live on a single branch; no `main` vs `develop` gymnastics.

## License

MIT (provisional — to be confirmed at first release).
