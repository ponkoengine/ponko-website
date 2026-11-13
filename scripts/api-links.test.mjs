import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { packageVersion, engineVersions, rewriteApiLinks } from './api-links.mjs';

const manifest = (name, version) =>
  `[package]\nname = "${name}"\nversion = "${version}"\nedition = "2024"\n\n[dependencies]\nfoo = { version = "9.9.9" }\n`;

test('reads the package version, not a dependency version', () => {
  assert.equal(packageVersion(manifest('ponko', '0.1.0')), '0.1.0');
  assert.equal(packageVersion('[dependencies]\nx = { version = "2.0.0" }\n[package]\nversion = "3.4.5-rc.1"\n'), '3.4.5-rc.1');
});

test('rejects a manifest without a package version', () => {
  assert.throws(() => packageVersion('[workspace]\nmembers = []\n'), /No \[package\] version/);
  assert.throws(() => packageVersion('[package]\nversion.workspace = true\n'), /No \[package\] version/);
});

test('rewrites API links to the version of each crate', () => {
  const versions = { ponko: '0.1.0', ponko_ecs: '0.1.1', ponko_ffi: '0.2.0' };
  const html = '<a href="api/ponko/game/struct.Game.html">a</a> <a href="api/ponko_ecs/index.html">b</a> <a href="api/ponko_ffi/x.html#y">c</a> <a href="api/other/x.html">d</a>';
  assert.equal(
    rewriteApiLinks(html, versions),
    '<a href="https://docs.rs/ponko/0.1.0/ponko/game/struct.Game.html">a</a> <a href="https://docs.rs/ponko_ecs/0.1.1/ponko_ecs/index.html">b</a> <a href="https://docs.rs/ponko_ffi/0.2.0/ponko_ffi/x.html#y">c</a> <a href="api/other/x.html">d</a>',
  );
});

test('takes versions from the engine revision being built', async () => {
  const dir = await mkdtemp(resolve(tmpdir(), 'engine-'));
  try {
    await mkdir(resolve(dir, 'crates/ponko_ecs'), { recursive: true });
    await mkdir(resolve(dir, 'crates/ponko_ffi'), { recursive: true });
    await writeFile(resolve(dir, 'Cargo.toml'), manifest('ponko', '1.2.3'));
    await writeFile(resolve(dir, 'crates/ponko_ecs/Cargo.toml'), manifest('ponko_ecs', '1.2.4'));
    await writeFile(resolve(dir, 'crates/ponko_ffi/Cargo.toml'), manifest('ponko_ffi', '1.2.5'));
    assert.deepEqual(await engineVersions(dir), { ponko: '1.2.3', ponko_ecs: '1.2.4', ponko_ffi: '1.2.5' });
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
