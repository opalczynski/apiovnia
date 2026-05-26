import { defineConfig } from "astro/config";

// GitHub Pages config.
//
// Today we ship at https://opalczynski.github.io/apiovnia/ (project page → needs base path).
// Once apiovnia.opalczynski.com is wired (add public/CNAME + DNS), flip `site` to that origin
// and `base` to "/". Nothing else needs to change — internal links use `import.meta.env.BASE_URL`.
export default defineConfig({
  site: "https://opalczynski.github.io",
  base: "/apiovnia/",
  trailingSlash: "ignore",
  build: {
    assets: "_assets",
  },
});
