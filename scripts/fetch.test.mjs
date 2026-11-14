import test from 'node:test';
import assert from 'node:assert/strict';
import { fetchBytes, parseMainRef } from './fetch.mjs';
const pause = async () => {};
test('recovers from the observed GitHub 504 timeout', async () => {
  let calls = 0;
  const result = await fetchBytes('https://example.com/tool', { pause,
    fetchImpl: async () => ++calls === 1 ? new Response('', { status: 504 }) : new Response('archive'),
  });
  assert.equal(result.toString(), 'archive');
  assert.equal(calls, 2);
});
test('does not retry a permanent missing asset', async () => {
  let calls = 0;
  await assert.rejects(fetchBytes('https://example.com/missing', { pause,
    fetchImpl: async () => { calls++; return new Response('', { status: 404 }); },
  }), /HTTP 404/);
  assert.equal(calls, 1);
});
test('bounds network failure retries', async () => {
  let calls = 0;
  await assert.rejects(fetchBytes('https://example.com/offline', { pause,
    fetchImpl: async () => { calls++; throw new TypeError('network failure'); },
  }), /after 4 attempts/);
  assert.equal(calls, 4);
});
test('retries a connection lost while reading an archive', async () => {
  let calls = 0;
  const result = await fetchBytes('https://example.com/interrupted', { pause,
    fetchImpl: async () => ++calls === 1
      ? { ok: true, arrayBuffer: async () => { throw new TypeError('terminated'); } }
      : new Response('complete'),
  });
  assert.equal(result.toString(), 'complete');
  assert.equal(calls, 2);
});

test('forwards binary asset negotiation headers', async () => {
  await fetchBytes('https://example.com/asset', { pause,
    headers: { Accept: 'application/octet-stream' },
    fetchImpl: async (_url, options) => {
      assert.equal(options.headers.Accept, 'application/octet-stream');
      assert.equal(options.headers['User-Agent'], 'ponko-website-build');
      return new Response('archive');
    },
  });
});

test('reads the main commit from a Git ref advertisement', () => {
  const head = 'a'.repeat(40), main = 'b'.repeat(40), other = 'c'.repeat(40);
  const refs = `001e# service=git-upload-pack\n0000` +
    `0159${head} HEAD\0multi_ack symref=HEAD:refs/heads/main\n` +
    `003f${other} refs/heads/feature\n003f${main} refs/heads/main\n0000`;
  assert.equal(parseMainRef(refs), main);
  // main as the first ref, carrying the capabilities after a NUL.
  assert.equal(parseMainRef(`0000015c${main} refs/heads/main\0multi_ack\n0000`), main);
  assert.equal(parseMainRef(`003f${other} refs/heads/mainline\n0000`), undefined);
});
