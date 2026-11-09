import {
  mkdir,
  writeFile,
  readFile,
  chmod,
  rm,
  readdir,
} from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { createHash } from "node:crypto";
import { fetchBytes } from "./fetch.mjs";

// Only fetch the public engine. Never read sibling checkouts during a deployment.
const cache = resolve(".cache");
await mkdir(cache, { recursive: true });
async function json(url) {
  return JSON.parse((await fetchBytes(url)).toString('utf8'));
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
const revision =
  requested ??
  (await json("https://api.github.com/repos/ponkoengine/ponko/commits/main"))
    .sha;
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
const targets = {
  "linux-x64": "x86_64-unknown-linux-musl",
  "linux-arm64": "aarch64-unknown-linux-musl",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
};
const target = targets[`${process.platform}-${process.arch}`];
if (!target) throw new Error("Unsupported build host: use Linux or macOS");
const binDir = resolve(cache, `mdbook-${version}-${target}`);
const bin = resolve(binDir, "mdbook");
try {
  await readFile(bin);
} catch {
  const release = await json(
    `https://api.github.com/repos/rust-lang/mdBook/releases/tags/v${version}`,
  );
  const asset = release.assets.find(
    (a) => a.name === `mdbook-v${version}-${target}.tar.gz`,
  );
  if (!asset?.digest?.startsWith("sha256:"))
    throw new Error("Missing mdBook release checksum");
  const archive = resolve(cache, "mdbook.tar.gz");
  await download(asset.url, archive, asset.digest, {
    headers: { Accept: "application/octet-stream" },
  });
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
      env: { ...process.env, MDBOOK_OUTPUT__HTML__SITE_URL: "/guide/" },
    },
  );
} finally {
  await writeFile(webChapter, originalWebChapter);
}
// The engine book also supports local rustdoc; this deployment uses docs.rs.
for (const name of await readdir("dist/guide")) {
  if (!name.endsWith(".html")) continue;
  const path = resolve("dist/guide", name);
  const html = await readFile(path, "utf8");
  await writeFile(
    path,
    html.replace(
      /href="api\/(ponko(?:_ecs|_ffi)?)\/([^"]*)"/g,
      (_, crate, page) =>
        `href="https://docs.rs/${crate}/0.0.1/${crate}/${page}"`,
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
