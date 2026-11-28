import {
  mkdir,
  writeFile,
  readFile,
  chmod,
  rm,
  readdir,
  copyFile,
} from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { createHash } from "node:crypto";
import { fetchBytes, parseMainRef } from "./fetch.mjs";
import { engineVersions, rewriteApiLinks } from "./api-links.mjs";

// Only fetch the public engine. Never read sibling checkouts during a deployment.
const cache = resolve(".cache");
await mkdir(cache, { recursive: true });
// The engine's `main` commit, from Git's own ref advertisement: unlike the
// GitHub API, it has no anonymous rate limit that a shared build host can
// use up.
async function engineMain() {
  const refs = (
    await fetchBytes(
      "https://github.com/ponkoengine/ponko.git/info/refs?service=git-upload-pack",
    )
  ).toString("utf8");
  const sha = parseMainRef(refs);
  if (!sha) throw new Error("No refs/heads/main in the engine's ref list");
  return sha;
}
async function download(url, destination, digest, options) {
  const data = await fetchBytes(url, options);
  if (
    digest &&
    `sha256:${createHash("sha256").update(data).digest("hex")}` !== digest
  ) {
    throw new Error("Download checksum mismatch");
  }
  await writeFile(destination, data);
}
const requested = process.env.PONKO_REVISION;
if (requested && !/^[a-f0-9]{40}$/.test(requested))
  throw new Error("PONKO_REVISION must be a full commit SHA");
const revision = requested ?? (await engineMain());
const source = resolve(cache, `engine-${revision}`);
try {
  await readFile(resolve(source, "book/book.toml"));
} catch {
  const archive = resolve(cache, "engine.tar.gz");
  await download(
    `https://codeload.github.com/ponkoengine/ponko/tar.gz/${revision}`,
    archive,
  );
  await rm(source, { recursive: true, force: true });
  await mkdir(source, { recursive: true });
  execFileSync("tar", ["-xzf", archive, "--strip-components=1", "-C", source]);
}
const version = "0.5.4";
// The release's SHA-256 digests, pinned with the version: reading them from
// the GitHub API at build time fails whenever the build host's anonymous API
// limit is used up.
const targets = {
  "linux-x64": ["x86_64-unknown-linux-musl", "sha256:5222beabd3e37dc5be0d18ff99b79058469354db5c220153a1b92db5ba12be89"],
  "linux-arm64": ["aarch64-unknown-linux-musl", "sha256:753e5c5c363ee8a56972344dcf91466f005a51db84a7aeffe427ae3ef83d6d44"],
  "darwin-x64": ["x86_64-apple-darwin", "sha256:a47d7bf0d5d670cff9ee6cce95537cbeb62dc10704d9e7131ffbd13e2b59a5de"],
  "darwin-arm64": ["aarch64-apple-darwin", "sha256:03e8a6d8b13a2971e0b3280affd03b388373c1485e26f73407c3a76b0b1838df"],
};
const [target, digest] = targets[`${process.platform}-${process.arch}`] ?? [];
if (!target) throw new Error("Unsupported build host: use Linux or macOS");
const binDir = resolve(cache, `mdbook-${version}-${target}`);
const bin = resolve(binDir, "mdbook");
try {
  await readFile(bin);
} catch {
  const archive = resolve(cache, "mdbook.tar.gz");
  await download(
    `https://github.com/rust-lang/mdBook/releases/download/v${version}/mdbook-v${version}-${target}.tar.gz`,
    archive,
    digest,
  );
  await mkdir(binDir, { recursive: true });
  execFileSync("tar", ["-xzf", archive, "-C", binDir]);
  await chmod(bin, 0o755);
}
// Legacy Pages demos were retired. Keep this deployment honest about availability.
const webChapter = resolve(source, "book/src/web.md");
const originalWebChapter = await readFile(webChapter, "utf8");
await writeFile(
  webChapter,
  originalWebChapter.replace(
    /The same game code[\s\S]*?(?=## Building your game)/,
    "The engine includes browser-build tooling. Hosted playable demos are not yet deployed on ponko.dev. Build the public [battle](https://github.com/ponkoengine/ponko/blob/main/examples/battle.rs), [flow](https://github.com/ponkoengine/ponko/blob/main/examples/flow.rs), and [ants](https://github.com/ponkoengine/ponko/blob/main/examples/ants.rs) examples locally using the instructions below.\n\n",
  ),
);
try {
  execFileSync(
    bin,
    ["build", resolve(source, "book"), "-d", resolve("dist/guide")],
    {
      stdio: "inherit",
      env: {
        ...process.env,
        MDBOOK_OUTPUT__HTML__SITE_URL: "/guide/",
        // Ink by default, like the site, whatever the system prefers. The
        // guide's own theme button still offers the light ones.
        MDBOOK_OUTPUT__HTML__DEFAULT_THEME: "coal",
        MDBOOK_OUTPUT__HTML__PREFERRED_DARK_THEME: "coal",
      },
    },
  );
} finally {
  await writeFile(webChapter, originalWebChapter);
}
// The site's look: the panda favicon and a warm theme over mdBook's own.
// mdBook names its favicons favicon[-hash].svg/png; replace whichever it wrote.
for (const name of await readdir("dist/guide")) {
  const icon = name.match(/^favicon(?:-[0-9a-f]+)?\.(svg|png)$/);
  if (icon)
    await copyFile(resolve("public", `favicon.${icon[1]}`), resolve("dist/guide", name));
}
await copyFile("scripts/guide-brand.css", "dist/guide/ponko-brand.css");
// The engine book also supports local rustdoc; this deployment uses docs.rs,
// at the crate versions of the engine revision being built.
const apiVersions = await engineVersions(source);
for (const name of await readdir("dist/guide")) {
  if (!name.endsWith(".html")) continue;
  const path = resolve("dist/guide", name);
  const html = await readFile(path, "utf8");
  await writeFile(
    path,
    rewriteApiLinks(html, apiVersions)
      .replace(
        "</head>",
        '<link rel="stylesheet" href="/guide/ponko-brand.css">\n</head>',
      ),
  );
}
await writeFile(
  "dist/build-info.json",
  JSON.stringify({ engineRevision: revision, mdbook: version }, null, 2) + "\n",
);
// Cloudflare Pages free plan limits each asset to 25 MiB.
async function check(directory) {
  for (const item of await readdir(directory, { withFileTypes: true })) {
    const path = resolve(directory, item.name);
    if (item.isDirectory()) await check(path);
    else if ((await readFile(path)).length > 25 * 1024 * 1024)
      throw new Error(`Asset exceeds Pages limit: ${path}`);
  }
}
await check("dist");
console.log(`Guide built from public ponko@${revision}`);
