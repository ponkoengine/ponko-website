import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

// The guide's API links point at docs.rs, at the version of each crate in the
// engine revision being built, so they follow the engine's releases.
export const apiCrates = {
  ponko: "Cargo.toml",
  ponko_ecs: "crates/ponko_ecs/Cargo.toml",
  ponko_ffi: "crates/ponko_ffi/Cargo.toml",
};

// The `version` of the `[package]` table of a Cargo.toml.
export function packageVersion(toml) {
  let inPackage = false;
  for (const line of toml.split(/\r?\n/)) {
    const table = line.match(/^\s*\[([^\]]*)\]\s*(?:#.*)?$/);
    if (table) {
      inPackage = table[1].trim() === "package";
      continue;
    }
    const version = inPackage && line.match(/^\s*version\s*=\s*"([^"]+)"/);
    if (version) return version[1];
  }
  throw new Error("No [package] version in Cargo.toml");
}

// Reads each crate's version from an unpacked engine revision.
export async function engineVersions(source) {
  const versions = {};
  for (const [crate, manifest] of Object.entries(apiCrates))
    versions[crate] = packageVersion(await readFile(resolve(source, manifest), "utf8"));
  return versions;
}

// Replaces the book's local `api/<crate>/...` links with docs.rs links.
export function rewriteApiLinks(html, versions) {
  return html.replace(
    /href="api\/(ponko(?:_ecs|_ffi)?)\/([^"]*)"/g,
    (_, crate, page) =>
      `href="https://docs.rs/${crate}/${versions[crate]}/${crate}/${page}"`,
  );
}
