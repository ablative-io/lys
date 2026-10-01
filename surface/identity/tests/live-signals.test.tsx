import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { useLive } from '../src/api';
import { refreshLive, subscribeChanges } from '../src/live';
import { readBounded, readTogether } from '../src/reads';

function deferred<T>() {
  let resolve: (value: T) => void = () => { throw new Error('promise not constructed'); };
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
const generation = (digit: string) => ({ generation: 'op-' + digit.repeat(32) });
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); });

it('shares one signal wait, never starts a timer, and aborts when the final view leaves', async () => {
  const requests: { path: string; signal?: AbortSignal | null; result: ReturnType<typeof deferred<Response>> }[] = [];
  vi.stubGlobal('fetch', vi.fn((path: string, init?: RequestInit) => {
    const result = deferred<Response>();
    requests.push({ path, signal: init?.signal, result });
    return result.promise;
  }));
  const timer = vi.spyOn(globalThis, 'setInterval');
  const ready = deferred<void>();
  const changed = vi.fn(() => ready.resolve());
  const refused = vi.fn();
  const first = subscribeChanges(changed, refused);
  const second = subscribeChanges(changed, refused);
  expect(requests).toHaveLength(1);
  requests[0].result.resolve(new Response(JSON.stringify(generation('1'))));
  await ready.promise;
  expect(changed).toHaveBeenCalledTimes(2);
  expect(requests).toHaveLength(2);
  expect(requests[1].path).toContain('/changes?after=');
  expect(timer).not.toHaveBeenCalled();
  first();
  expect(requests[1].signal?.aborted).toBe(false);
  second();
  expect(requests[1].signal?.aborted).toBe(true);
  requests[1].result.resolve(new Response(JSON.stringify(generation('2'))));
  expect(refused).not.toHaveBeenCalled();
});

it('coalesces refreshes while a view read is active and exposes feed refusals', async () => {
  const waiting = deferred<Response>();
  vi.stubGlobal('fetch', vi.fn((path: string) => path.includes('?') ? waiting.promise : Promise.resolve(new Response(JSON.stringify(generation('1'))))));
  const reads = [deferred<string>(), deferred<string>()];
  const begun = [deferred<void>(), deferred<void>()];
  let calls = 0;
  const read = () => { const at = calls++; begun[at].resolve(); return reads[at].promise; };
  function View() { const load = useLive(read, 'fixture'); return <p>{load.status === 'ok' ? load.data : load.status === 'refused' ? load.refused.refusal.refusal : 'loading'}</p>; }
  const container = document.createElement('div');
  const root = createRoot(container);
  try {
    await act(async () => { root.render(<View />); });
    await begun[0].promise;
    refreshLive(); refreshLive();
    expect(calls).toBe(1);
    await act(async () => { reads[0].resolve('first'); await begun[1].promise; reads[1].resolve('second'); });
    expect(calls).toBe(2);
    expect(container.textContent).toBe('second');
    await act(async () => { waiting.resolve(new Response(JSON.stringify({ refusal: 'RuntimeUnavailable', reason: 'feed unavailable' }), { status: 503 })); });
    expect(container.textContent).toBe('RuntimeUnavailable');
  } finally { act(() => root.unmount()); }
});

it('starts independent reads together and limits collection fan-out to four', async () => {
  const a = deferred<number>(); const b = deferred<string>();
  const all = readTogether({ a: a.promise, b: b.promise });
  b.resolve('ready'); a.resolve(7);
  expect(await all).toEqual({ a: 7, b: 'ready' });
  const slots = Array.from({ length: 9 }, () => deferred<number>());
  const started: number[] = [];
  const result = readBounded(slots.map((_, i) => i), (i) => { started.push(i); return slots[i].promise; });
  expect(started).toEqual([0, 1, 2, 3]);
  for (const [i, slot] of slots.entries()) slot.resolve(i);
  expect(await result).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8]);
});
