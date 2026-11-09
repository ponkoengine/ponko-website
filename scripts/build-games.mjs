import {
  readFile,
  writeFile,
  mkdir,
  cp,
  readdir,
} from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { resolve } from "node:path";
import { prepareEngine, engine } from "./prepare-engine.mjs";
import { fetchBytes } from "./fetch.mjs";
await prepareEngine();
const run = (command, args, options = {}) =>
  execFileSync(command, args, { stdio: "inherit", ...options });
// Pages builds and local builds use the same pinned Rust/wasm-bindgen toolchain.
// Bootstrap rustup locally only on hosts where the build image has none.
let rustup = "rustup";
try {
  execFileSync(rustup, ["--version"], { stdio: "ignore" });
} catch {
  const bin = resolve(".cache/rust/bin");
  const script = resolve(".cache/rustup-init.sh");
  await writeFile(script, await fetchBytes("https://sh.rustup.rs"));
  process.env.CARGO_HOME = resolve(".cache/rust");
  process.env.RUSTUP_HOME = resolve(".cache/rustup");
  run("sh", [
    script,
    "-y",
    "--no-modify-path",
    "--profile",
    "minimal",
    "--default-toolchain",
    engine.rust,
  ]);
  rustup = resolve(bin, "rustup");
  process.env.PATH = `${bin}:${process.env.PATH}`;
}
run(rustup, [
  "toolchain",
  "install",
  engine.rust,
  "--profile",
  "minimal",
  "--target",
  "wasm32-unknown-unknown",
]);
const cargo = (args) => run(rustup, ["run", engine.rust, "cargo", ...args]);
const bindgen = resolve(".cache/tooling/wasm-bindgen/bin/wasm-bindgen");
let installed;
try {
  installed = execFileSync(bindgen, ["--version"], { encoding: "utf8" }).trim();
} catch {}
if (installed !== `wasm-bindgen ${engine.wasmBindgen}`)
  cargo([
    "install",
    "wasm-bindgen-cli",
    "--version",
    engine.wasmBindgen,
    "--locked",
    "--root",
    resolve(".cache/tooling/wasm-bindgen"),
  ]);
cargo([
  "build",
  "--manifest-path",
  "games/Cargo.toml",
  "--locked",
  "--release",
  "--target",
  "wasm32-unknown-unknown",
  "--features",
  "runtime",
  "--bins",
]);
const games = ["asteroids", "breakout", "arena"];
const hash = (data) => createHash("sha256").update(data).digest("hex");
// Fingerprint exactly the Rust inputs exposed in the source viewer.
async function sources(dir) {
  const files = [];
  for (const entry of (await readdir(dir, { withFileTypes: true })).sort(
    (a, b) => a.name.localeCompare(b.name),
  )) {
    const path = `${dir}/${entry.name}`;
    if (entry.isDirectory()) files.push(...(await sources(path)));
    else files.push(path);
  }
  return files;
}
const inputFiles = [
  "games/Cargo.toml",
  "games/Cargo.lock",
  "games/engine.json",
  ...(await sources("games/src")),
];
const sourceHash = createHash("sha256");
for (const path of inputFiles)
  sourceHash.update(path).update(await readFile(path));
const manifest = {
  engineRevision: engine.revision,
  sourceHash: sourceHash.digest("hex"),
  games: {},
};
for (const game of games) {
  const out = resolve("public/play", game);
  await mkdir(`${out}/assets`, { recursive: true });
  run(bindgen, [
    "--target",
    "web",
    "--no-typescript",
    "--remove-name-section",
    "--remove-producers-section",
    "--out-dir",
    out,
    "--out-name",
    "game",
    resolve(`games/target/wasm32-unknown-unknown/release/${game}.wasm`),
  ]);
  await cp("games/shell.html", `${out}/index.html`);
  await writeFile(`${out}/assets/index.json`, "[]\n");
  const wasm = await readFile(`${out}/game_bg.wasm`);
  if (wasm.length > 25 * 1024 * 1024)
    throw new Error(`${game} exceeds the hosting asset limit`);
  manifest.games[game] = { bytes: wasm.length, sha256: hash(wasm) };
  console.log(
    `${game}: ${(wasm.length / 1024 / 1024).toFixed(2)} MiB of compiled WASM`,
  );
}
await writeFile(
  "public/play/build-info.json",
  JSON.stringify(manifest, null, 2) + "\n",
);
