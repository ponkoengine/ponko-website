import { readFile, writeFile, mkdir, rm } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { fetchBytes } from "./fetch.mjs";
export const engine = JSON.parse(
  await readFile(new URL("../games/engine.json", import.meta.url)),
);
export async function prepareEngine() {
  if (
    engine.repository !== "ponkoengine/ponko" ||
    !/^[a-f0-9]{40}$/.test(engine.revision)
  )
    throw new Error("Invalid public engine pin");
  const source = resolve(".cache/engine");
  if (
    (await readFile(resolve(source, ".ponko-revision"), "utf8").catch(
      () => "",
    )) === engine.revision
  )
    return source;
  await mkdir(".cache", { recursive: true });
  const archive = resolve(".cache/game-engine.tar.gz");
  await writeFile(
    archive,
    await fetchBytes(
      `https://codeload.github.com/${engine.repository}/tar.gz/${engine.revision}`,
    ),
  );
  await rm(source, { recursive: true, force: true });
  await mkdir(source, { recursive: true });
  execFileSync("tar", ["-xzf", archive, "--strip-components=1", "-C", source]);
  await writeFile(resolve(source, ".ponko-revision"), engine.revision);
  return source;
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
)
  await prepareEngine();
