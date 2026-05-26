# apiovnia-site

Landing + docs site for [Apiovnia](../README.md). Astro 5, static-only, deployed to GitHub Pages.

## Run locally

```bash
cd apiovnia-site
pnpm install
pnpm dev          # http://localhost:4321/apiovnia/
pnpm build        # → dist/
pnpm preview      # serve the production build locally
```

## Deploy

Static output in `dist/` is what Pages serves. GH Actions workflow not wired yet (Phase 14/15) — until then, build locally and push `dist/` to the `gh-pages` branch, or enable Pages → "Deploy from a branch" with a manual `gh-pages` push.

## Switching to a custom domain

Today we ship at `https://opalczynski.github.io/apiovnia/`. When `apiovnia.opalczynski.com` (or similar) lands:

1. Add `public/CNAME` containing the bare domain (no protocol, no trailing slash).
2. In `astro.config.mjs`, set `site: "https://apiovnia.opalczynski.com"` and `base: "/"`.
3. Configure DNS (CNAME → `opalczynski.github.io`) and enable HTTPS in repo Settings → Pages.

Internal links use `import.meta.env.BASE_URL`, so they don't need touching.

## Layout

```
apiovnia-site/
├── astro.config.mjs
├── package.json
├── public/                # static passthrough (favicon, og-image, icon copies)
└── src/
    ├── components/        # Astro components (Header, Footer, AppShell mock, …)
    ├── layouts/           # BaseLayout
    ├── pages/             # routes — file = url
    └── styles/            # tokens.css + global.css
```

`tokens.css` is the same set of CSS variables as the app — keep them in sync if any token gets added/renamed in `apiovnia-app/src/app.css`.
