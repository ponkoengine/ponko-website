# Deploy ponko.dev

This is an Astro static site. The homepage is built here; the mdBook guide is
built from the **public** `ponkoengine/ponko` repository into `/guide/`.
API documentation stays at https://docs.rs/ponko/.

## Cloudflare Pages (free)

1. In Cloudflare, open **Workers & Pages → Create application → Pages → Connect to Git**.
2. Connect GitHub and allow access to `ponkoengine/ponko-website`.
3. Create a Pages project named `ponko`, with production branch `main`.
4. Select Astro, build command `npm run build`, and output directory `dist`.
   Root directory is the repository root. Set `NODE_VERSION` to `26.13.0`.
5. Save and deploy. In **Custom domains**, add `ponko.dev`. Because its zone is
   already in Cloudflare, accept the generated DNS record. Add `www.ponko.dev`
   only if you want that hostname too. Do not change the domain's nameservers.

Every push to website `main` then builds and deploys production. Other branches
and pull requests get preview deployments. No runtime, database, paid service,
or Cloudflare API token is required in this repository.

There is no CI: run `npm ci`, `npm test`, `npm run build` and `npm run check`
locally before opening a pull request.

## Guide updates from the engine

Each full build resolves public engine `main` once and records the exact SHA in
`/build-info.json`. For a reproducible rebuild, set `PONKO_REVISION` to a full
40-character engine commit SHA in the build environment. mdBook is pinned to
0.5.4; its downloaded release binary is verified against GitHub's SHA-256 digest.

A push to the engine repository alone does **not** trigger the website's Git
integration. To deploy guide changes immediately, create a Cloudflare Pages
**deploy hook** for `main`, keep its URL out of the repository, and call it by
hand after the engine change has merged:

```sh
curl --fail --silent --show-error --request POST "$DEPLOY_HOOK"
```

The hook URL is a secret. Never put it in source or build output. This hook is
optional for launching the website; until connected, guide changes arrive on
the next website deployment or a manual retry in Cloudflare.

## The playable demos

The home page runs `/play/hole/`, a black hole whose disk is up to 600,000
particles simulated every frame (source in `demo/hole`). `/play/tide/`, a
small tower defence (source in `demo/tide`), is no longer on the home page but
stays for the blog posts that link to it. Each demo is pinned to an exact
public engine commit in its `Cargo.toml`, and its compiled WebAssembly is
committed under `public/play/<name>/`, so deployments need no Rust toolchain.
After changing a demo or bumping its engine revision, rebuild it with
`scripts/build-demo.sh hole` (or `tide`; it needs the `wasm32-unknown-unknown`
target and `wasm-bindgen-cli` 0.2.130) and commit the output. It prints the
module's size raw, gzipped and, where `brotli` is installed, as Brotli.
It optimizes Tide's module with Binaryen's `wasm-opt` when available
(`brew install binaryen`). Use the printed Brotli size for the home page's
download figure.

Both need WebGPU and say so on browsers without it. The black hole traces
every pixel's light in one full-screen shader (`demo/hole/src/trace.wgsl`);
its simulated particles are added into a small texture of the disk each frame,
which the shader reads where a ray crosses the disk. It draws its traced
picture below the window's resolution on large or slow screens. On the home
page it is a poster (`public/play/hole/poster.webp`, a screenshot of the demo)
and a button that opens the demo in its own tab. `build-demo.sh` skips
`wasm-opt` for it: the optimized module compresses worse (Brotli 2.75 MB
against 2.59 MB).

## Local development

Use Node 26.13.0 and `npm ci`. `npm run dev` runs the Astro homepage. Run
`npm run build && npm run check` for the complete site, including the guide;
`npm run preview` serves that production build. The full build needs internet
access and `tar`, downloads only public engine content, and caches downloads
in ignored `.cache/`. It never reads private sibling repositories.

`npm run build:site` builds only Astro (the home page and the blog) for rapid
layout work. Blog posts are written as described in [WRITING.md](WRITING.md);
the RSS feed and `sitemap.xml` are generated from them on each build. Guide links need
the full build to work. On localhost, clipboard access requires a secure
context; the copy button also supplies a readable fallback if access fails.
