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

GitHub Actions also builds and checks local links on each PR. Configure the
`build` check as required in the repository rules before merging changes.

## Guide updates from the engine

Each full build resolves public engine `main` once and records the exact SHA in
`/build-info.json`. For a reproducible rebuild, set `PONKO_REVISION` to a full
40-character engine commit SHA in the build environment. mdBook is pinned to
0.5.4; its downloaded release binary is verified against GitHub's SHA-256 digest.

A push to the engine repository alone does **not** trigger the website's Git
integration. To deploy guide changes immediately, create a Cloudflare Pages
**deploy hook** for `main`, save its URL as an engine Actions secret named
`PONKO_WEBSITE_DEPLOY_HOOK`, and add this step to an engine workflow triggered
by pushes to `main` that change `book/**`, `examples/**`, or `tests/**`:

```yaml
- name: Rebuild ponko.dev guide
  env:
    DEPLOY_HOOK: ${{ secrets.PONKO_WEBSITE_DEPLOY_HOOK }}
  run: curl --fail --silent --show-error --request POST "$DEPLOY_HOOK"
```

The hook URL is a secret. Never put it in source or build output. This hook is
optional for launching the website; until connected, guide changes arrive on
the next website deployment or a manual retry in Cloudflare.

## Local development

Use Node 26.13.0 and `npm ci`. `npm run dev` runs the Astro homepage. Run
`npm run build && npm run check` for the complete site, including the guide;
`npm run preview` serves that production build. The full build needs internet
access and `tar`, downloads only public engine content, and caches downloads
in ignored `.cache/`. It never reads private sibling repositories.

`npm run build:site` builds only Astro for rapid layout work. Guide links need
the full build to work. On localhost, clipboard access requires a secure
context; the copy button also supplies a readable fallback if access fails.

## Playable games

The full build now compiles the games in `games/` to WASM before building Astro.
It fetches only the pinned public Ponko revision in `games/engine.json`, uses
locked Rust dependencies, and installs the pinned Rust and wasm-bindgen tooling
if the build host needs them. Allow network access to GitHub, crates.io,
static.rust-lang.org, and sh.rustup.rs. The first build takes longer; `.cache/`
and `games/target/` are safe build caches. No binaries are checked into Git.

`public/play/build-info.json` (served at `/play/build-info.json`) records engine
provenance, the game source fingerprint and each WASM checksum. Same-origin
frames are allowed by `_headers` so the homepage can embed these games. Games
remain available directly at `/play/asteroids/`, `/play/breakout/` and `/play/arena/`.
See `games/README.md` for development, controls and verification.
