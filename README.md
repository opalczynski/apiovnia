<div align="center">

<img src="apiovnia-app/src-tauri/icons/apiovnia.svg" width="96" height="96" alt="Apiovnia logo" />

# Apiovnia

**A local-first REST API client that minds its own business.**

One SQLite file on your machine. Environments as per-request overrides, not variable soup. Any environment can be sealed behind a master password. No cloud, no accounts, no roadmap toward either.

[![CI](https://github.com/opalczynski/apiovnia/actions/workflows/ci.yml/badge.svg)](https://github.com/opalczynski/apiovnia/actions/workflows/ci.yml)
[![License: PolyForm NC 1.0.0](https://img.shields.io/badge/license-PolyForm--NC--1.0.0-F59E0B.svg)](./LICENSE)
[![Status: alpha](https://img.shields.io/badge/status-alpha-FBBF24.svg)](#status)

[**Website**](https://opalczynski.github.io/apiovnia/) · [**Features**](https://opalczynski.github.io/apiovnia/features) · [**Roadmap**](https://opalczynski.github.io/apiovnia/roadmap) · [**Docs**](https://opalczynski.github.io/apiovnia/docs)

</div>

---

## Why this exists

Every tool in this category starts the same way. Small, sharp, written by someone who needed it themselves. Then comes the second act — an investor, a strategic partner, a pivot toward "the enterprise." The roadmap fills with features nobody asked for and empties of the ones that mattered. Five years in, the tool that solved your problem is now a problem of its own.

Apiovnia is built next to that structure, not against it. There's no investor to satisfy. No growth curve to bend. The whole thing is one SQLite file on your machine — yours to back up, inspect, copy, or delete. The price is honest. Free, forever. The full version of this argument lives [on the website](https://opalczynski.github.io/apiovnia/#manifesto).

## Status

**Alpha, under active development.** Tested daily by the author. Public binaries land with Phase 14 (CI release automation). Until then, build from source — it's a couple of minutes if Rust + Node are already in your PATH.

182 tests behind the resolver, the crypto, the snippets, and the GraphQL body. Phases 0–11 + 13.1 + 15 done; see [`plan.md`](./plan.md) for the live per-phase tracker.

## What it does (today)

- **Projects → Collections → Requests** in one SQLite file, with full CRUD
- **Request editor** — method, URL, params, headers, body (JSON / GraphQL / Form / Multipart / Raw), auth (Bearer / Basic / API key / none); CodeMirror 6 body editor with parse-lint
- **HTTP execution** via `reqwest` + rustls, gzip, redirects, 30 s timeout, 2 MiB body cap
- **Response viewer** with a custom JSON tree (collapsible nodes, ⌘F search, hover-copy per value) and Pretty / Headers / Request / Raw tabs
- **Environments + per-(request, env) overrides + `{{var}}` interpolation**, resolved in Rust before the wire request is built
- **Master-password encrypted environments** — Argon2id + AES-256-GCM, 10 min idle auto-lock, zxcvbn meter with explicit pro-user bypass
- **Command palette (⌘P)** — fuzzy ranking across requests, collections, projects, envs, and actions
- **Copy as…** — curl / Python (`requests`) / HTTPie / JavaScript (`fetch`) / PowerShell, env-resolved and secrets-decrypted
- **OpenAPI 3.x** import + export — `$ref` resolution, schema inference, secret scrubbing, persistent OpLog audit panel
- **GraphQL** — split query + variables editor, POST or GET per the GraphQL-over-HTTP spec
- **History panel** — last ~200 executions, filterable, click to navigate and rehydrate the saved response
- **Five themes** — apiovnia (amber default) / atomic-dark / tokyo-night / monokai / light, applied live

Full feature deep-dive is on the [website](https://opalczynski.github.io/apiovnia/features).

## What it deliberately won't become

No pre-request scripts. No response test assertions. No team workspaces. No sync. No browser version. No mobile. No auto-updater. The longer this list grows, the more useful Apiovnia stays.

## Install

### Pre-built binaries

Grab the latest from [Releases](https://github.com/opalczynski/apiovnia/releases). Three platforms, three different stories about signing:

| Platform | Format | Signing | First-launch UX |
|---|---|---|---|
| **Linux** | `.deb` / `.rpm` / `.AppImage` | n/a | Just works |
| **macOS** | `.dmg` (universal: Apple Silicon + Intel) | **Developer ID + notarized** | Just works |
| **Windows** | `.msi` | **Unsigned** — no plans to sign (EV cert cost doesn't make sense for a tool this size) | SmartScreen will block: click **More info → Run anyway** |

### Build from source

Requires Rust stable, Node ≥ 20, pnpm, plus the usual Tauri 2 system deps. On Linux that's `webkit2gtk-4.1`, `libayatana-appindicator3`, `librsvg2-dev`, `libgtk-3-dev`, `libssl-dev` — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform.

```bash
git clone https://github.com/opalczynski/apiovnia.git
cd apiovnia/apiovnia-app
pnpm install
pnpm tauri:dev          # opens the native window with Vite HMR + Rust hot rebuild
```

For a release bundle (`.deb` / `.AppImage` / `.dmg` / `.msi` depending on your platform):

```bash
pnpm tauri:build
```

Output lands in `apiovnia-app/src-tauri/target/release/bundle/`.

### Where your data lives

Linux: `~/.local/share/tech.trurl.apiovnia/apiovnia.db`
macOS: `~/Library/Application Support/tech.trurl.apiovnia/apiovnia.db`
Windows: `%APPDATA%\tech.trurl.apiovnia\apiovnia.db`

Delete the file for a fresh state. There is nothing else to clean.

### Cutting a release (for me / future me)

Releases are tag-driven. Merge whatever needs merging to `main`, then locally:

```bash
# Bump version in BOTH places (they're separate sources of truth):
#   apiovnia-app/package.json#version
#   apiovnia-app/src-tauri/tauri.conf.json#version
# Then:
git commit -am "release: v0.1.0"
git tag v0.1.0
git push && git push --tags
```

The `Release` workflow (`.github/workflows/release.yml`) picks up the tag, builds on three runners in parallel (Ubuntu / macOS / Windows), and creates a **draft GitHub Release** with the artifacts attached. Review, edit notes, publish.

For alpha / beta / rc cycles, use `v0.1.0-alpha.1` style tags — they get marked as pre-release automatically.

If one platform's build fails after a tag is already published, re-run via **Actions → Release → Run workflow** and pass the existing tag — the workflow will (re-)upload to the same draft release.

## Repo layout

```
.
├── apiovnia-app/             # the desktop app — Tauri 2 + Vite + Svelte 5
│   ├── src/                  # Svelte 5 frontend (runes only)
│   └── src-tauri/            # Cargo workspace
│       ├── src/              # apiovnia-tauri binary — thin command layer
│       └── crates/
│           ├── apiovnia-core/     # domain models, resolver, interpolation, snippets — no Tauri/SQL deps
│           ├── apiovnia-storage/  # sqlx pool, migrations, repos
│           ├── apiovnia-http/     # reqwest executor, content-type classification
│           ├── apiovnia-crypto/   # Argon2id + AES-256-GCM, zxcvbn password policy
│           └── apiovnia-openapi/  # oas3 import/export, secret scrubbing, schema inference
├── apiovnia-site/            # Astro 5 landing + docs (deploys to GH Pages)
├── instruction.md            # immutable product brief
├── plan.md                   # phased implementation plan + live progress log
├── CLAUDE.md                 # notes for Claude / future-me working in this repo
└── LICENSE                   # PolyForm Noncommercial 1.0.0
```

`apiovnia-core` has zero Tauri / SQL / HTTP dependencies — it's where unit-testable logic lives (resolver, variable interpolation, validation, snippets, GraphQL body). Everything else depends on it.

## Tests + quality gates

These are the same gates CI runs. Everything has to pass before a phase is called done.

```bash
cd apiovnia-app

# frontend
pnpm check                                                # svelte-check + tsc
pnpm build                                                # Vite production bundle

# backend
cargo --manifest-path src-tauri/Cargo.toml clippy --workspace --all-targets -- -D warnings
cargo --manifest-path src-tauri/Cargo.toml test --workspace
```

Coverage: 5 storage cases, 2 HTTP content-type cases, 33 resolver + interpolation cases, 45 snippet cases (curl / Python / HTTPie / JS / PowerShell), 11 GraphQL body cases + 2 snippet folds, 21 crypto cases (13 AEAD round-trip / tamper + 8 zxcvbn policy), 56 OpenAPI cases + 1 integration against the real petstore.

## Tech stack

- **Tauri 2.x** — desktop shell, IPC, packaging
- **Rust** — `reqwest` (rustls), `sqlx` (SQLite, async), `argon2` + `aes-gcm` + `zxcvbn`, `oas3`
- **Svelte 5** — runes API only, no legacy stores
- **Vite 6** — frontend bundler (no SvelteKit; this is a desktop app, SSR is irrelevant)
- **Tailwind v4** — CSS-first `@theme` config, plus plain CSS in scoped `<style>` blocks for design-token parity
- **CodeMirror 6** — body editor (JSON / HTML / XML / GraphQL) with a custom dark theme keyed to the design tokens
- **Astro 5** — static site generator for the landing page

## Contributing

Not really, no. Apiovnia is a single-developer product with a deliberately small surface area and an ending roadmap. Random feature PRs ("add WebSocket support", "add a Postman importer", "what if it ran in the browser") will be politely closed. The list of things Apiovnia [will not become](https://opalczynski.github.io/apiovnia/roadmap#oos) is the feature.

**But if you are determined enough**, the door isn't locked:

- **Bug reports** — yes, always. Open an [issue](https://github.com/opalczynski/apiovnia/issues) with reproduction steps and your platform. The smaller and weirder the repro, the more I'll like you.
- **Documentation fixes** — yes. Typo, outdated path, broken link in the site, dead instruction in `docs/` — PR welcome, no ceremony.
- **Bugfix PRs** — yes, if the bug is real and the fix is surgical. Open an issue first so we can agree the bug exists before you write code.
- **Feature PRs** — almost never. If you have an idea you genuinely think fits the project's posture, open an issue describing the use case (not the implementation) and wait for a "go" before writing code. PRs without prior discussion will be closed.
- **Refactor / cleanup PRs** — no. The code is the way it is on purpose; please don't.

The repo follows the conventions in [`CLAUDE.md`](./CLAUDE.md) (runes only, layered crates, additive migrations, design tokens, save-cycle state machine). Read it before opening a PR.

## License

[**PolyForm Noncommercial 1.0.0**](./LICENSE).

Use, copy, modify, distribute — fine for personal use, education, private research, and non-profit organisations. Commercial use requires a separate licence from the author. If you're not sure which side of that line you sit on, [open a discussion](https://github.com/opalczynski/apiovnia/discussions) and we'll figure it out.

---

<div align="center">
<sub>Built by <a href="https://github.com/opalczynski">Sebastian Opałczyński</a>, somewhere with good coffee and bad Rust.</sub>
</div>
