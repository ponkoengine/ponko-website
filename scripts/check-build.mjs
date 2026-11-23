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
// The engine draws into the page's <canvas id="ponko">; without one it adds
// its own canvas below the page, where the demo can't be seen.
for (const demo of ["tide", "hole"]) {
  const page = await readFile(resolve(root, `play/${demo}/index.html`), "utf8");
  if (!page.includes('<canvas id="ponko"')) {
    throw new Error(`play/${demo}/index.html has no <canvas id="ponko"> for the engine to draw into`);
  }
}
console.log(`Checked local links and assets in ${count} HTML pages.`);
