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
│           ├── apiovnia-crypto/    # Argon2id + AES-256-GCM (Phase 6, scaffold only)
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

Phases 6–9 are queued in `plan.md`. **Phase 6** (master password — Argon2id + AES-256-GCM, unlock modal) is the next one up. `apiovnia-crypto` crate is currently scaffold only.

## Gotchas worth knowing

- After changing a SQL migration, wipe the local DB (or write a new additive migration). The existing `0001_init.sql` / `0002_history_full.sql` are released.
- `pnpm tauri:dev` HMR only covers the frontend; Rust changes need the binary to rebuild, which happens automatically but is slower.
- CodeMirror lint inline marks are hidden via CSS — the gutter dot is the single visual indicator. See the long comment in `app.css` if anything regresses.
- `formatBytes`/`formatDuration` use thin spaces around values via `display: inline-flex` `gap` on `.metric` — HTML whitespace collapse will eat normal spaces.
- Never log decrypted env override values once Phase 6 lands. The decryption key never crosses the IPC boundary; resolver runs in Rust.
