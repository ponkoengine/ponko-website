import { readFile, readdir, stat } from "node:fs/promises";
import { resolve, dirname } from "node:path";
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
