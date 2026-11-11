import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defineConfig } from "astro/config";
import mdx from "@astrojs/mdx";
import { codeFrame, ponkoNight } from "./src/lib/shiki-ponko.mjs";

// The dev server doesn't serve public/<dir>/index.html for a URL that ends in
// a slash, the way the deployed site does. Without this, /play/tide/ is a 404
// in development.
const publicIndexes = {
  name: "public-directory-indexes",
  configureServer(server) {
    const root = fileURLToPath(new URL("./public", import.meta.url));
    server.middlewares.use((req, _res, next) => {
      const [path, query] = (req.url ?? "").split("?");
      if (path.endsWith("/") && !path.includes("..") && existsSync(`${root}${path}index.html`)) {
        req.url = `${path}index.html${query ? `?${query}` : ""}`;
      }
      next();
    });
  },
};

export default defineConfig({
  site: "https://ponko.dev",
  output: "static",
  trailingSlash: "ignore",
  integrations: [mdx()],
  markdown: {
    shikiConfig: { theme: ponkoNight, wrap: false, transformers: [codeFrame] },
  },
  vite: { plugins: [publicIndexes] },
});
