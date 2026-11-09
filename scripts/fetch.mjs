import { setTimeout as delay } from 'node:timers/promises';

// Retry temporary upstream failures, including errors while reading the body.
export async function fetchBytes(url, { fetchImpl = fetch, pause = delay, headers = {} } = {}) {
  const attempts = 4;
  for (let attempt = 1; attempt <= attempts; attempt++) {
    let response;
    try {
      response = await fetchImpl(url, {
        headers: { 'User-Agent': 'ponko-website-build', ...headers },
        signal: AbortSignal.timeout(60_000),
      });
      if (response.ok) return Buffer.from(await response.arrayBuffer());
    } catch (error) {
      if (attempt === attempts) throw new Error(`Download failed after ${attempts} attempts: ${url}`, { cause: error });
    }
    if (response && !response.ok) {
      const retryable = [408, 429, 500, 502, 503, 504].includes(response.status);
      await response.body?.cancel();
      if (!retryable || attempt === attempts) throw new Error(`${url}: HTTP ${response.status}`);
    }
    console.warn(`Temporary download failure; retry ${attempt + 1}/${attempts}: ${url}`);
    await pause(1000 * 2 ** (attempt - 1));
  }
}
