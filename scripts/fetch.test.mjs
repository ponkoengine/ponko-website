import test from 'node:test';
import assert from 'node:assert/strict';
import { fetchBytes } from './fetch.mjs';
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
