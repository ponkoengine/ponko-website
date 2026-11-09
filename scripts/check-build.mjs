import { readFile, readdir, stat } from "node:fs/promises";
import { resolve, dirname } from "node:path";
import { createHash } from "node:crypto";
const root = resolve("dist");
let count = 0;
async function walk(directory) {
  for (const name of await readdir(directory, { withFileTypes: true })) {
    const path = resolve(directory, name.name);
    if (name.isDirectory()) {
      await walk(path);
      continue;
    }
    if (!name.name.endsWith(".html")) continue;
    count++;
    const html = await readFile(path, "utf8");
    for (const match of html.matchAll(
      /(?:href|src)="([^"#?]+)(?:[?#][^"]*)?"/g,
    )) {
      const url = match[1];
      if (/^(?:[a-z]+:|\/\/)/i.test(url)) continue;
      const target = url.startsWith("/")
        ? resolve(root, "." + decodeURI(url))
        : resolve(dirname(path), decodeURI(url));
      const info = await stat(target).catch(() => null);
      if (!info) throw new Error(`Broken local link in ${path}: ${url}`);
      if (info.isDirectory()) await stat(resolve(target, "index.html"));
    }
  }
}
await walk(root);
console.log(`Checked local links and assets in ${count} HTML pages.`);

const builds = JSON.parse(await readFile(resolve(root, "play/build-info.json"), "utf8"));
for (const game of ["asteroids", "breakout", "arena"]) {
  const wasm = await readFile(resolve(root, `play/${game}/game_bg.wasm`));
  if (!wasm.subarray(0, 4).equals(Buffer.from([0, 97, 115, 109]))) throw new Error(`${game}: invalid WASM`);
  const checksum = createHash("sha256").update(wasm).digest("hex");
  if (checksum !== builds.games[game].sha256 || wasm.length !== builds.games[game].bytes) throw new Error(`${game}: WASM does not match build manifest`);
}
console.log("Verified all three WASM signatures, sizes, and checksums.");
