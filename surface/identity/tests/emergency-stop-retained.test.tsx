import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { EmergencyStop } from '../src/features/file/EmergencyStop';

const agent = 'agent-' + 'a'.repeat(32);
const operation = 'op-' + 'b'.repeat(32);
const path = '/agents/' + agent + '/stop';
const key = 'lys.pending.stop.' + agent;
const body = { operation, reason: 'Key exposed' };
const saved = JSON.stringify({ path, body });
let root: Root | undefined;

afterEach(async () => {
  if (root) await act(async () => { root?.unmount(); });
  root = undefined;
  sessionStorage.clear();
});

async function mount(stopped: (answer: unknown) => void) {
  const host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
  await act(async () => { root?.render(<EmergencyStop id={agent} active={false} stopped={stopped} />); });
}

async function retry() {
  const button = [...document.querySelectorAll('button')].find((entry) => entry.textContent === 'Check whether Lys saved it');
  if (!button) throw new Error('Retained stop retry missing');
  await act(async () => { button.click(); });
}

function answer(overrides: Record<string, unknown> = {}) {
  return Response.json({ agent, operation, state: 'suspended', reason: body.reason, by: 'person-' + 'c'.repeat(32), at: 1,
    certificates_withdrawn: [], credentials_ended: [], credentials_refused: null, sessions_asked: [], ...overrides });
}

it('keeps the supplied stop key through an unanswered retry and a remount', async () => {
  sessionStorage.setItem(key, saved);
  const fetch = vi.fn().mockResolvedValueOnce(Response.json({ refusal: 'StopsUnavailable', reason: 'Outcome unknown' }, { status: 503 })).mockResolvedValueOnce(answer());
  vi.stubGlobal('fetch', fetch);
  const stopped = vi.fn();
  await mount(stopped);
  await retry();
  expect(sessionStorage.getItem(key)).toBe(saved);
  expect(stopped).not.toHaveBeenCalled();
  await act(async () => { root?.unmount(); });
  root = undefined;
  await mount(stopped);
  await retry();
  expect(fetch).toHaveBeenCalledTimes(2);
  for (const [url, init] of fetch.mock.calls) {
    expect(url).toBe('/api' + path);
    expect(init.body).toBe(JSON.stringify(body));
  }
  expect(stopped).toHaveBeenCalledOnce();
  expect(sessionStorage.getItem(key)).toBeNull();
});

it('retains the original stop when an answer names another operation', async () => {
  sessionStorage.setItem(key, saved);
  const fetch = vi.fn().mockResolvedValueOnce(answer({ operation: 'op-' + 'd'.repeat(32) })).mockResolvedValueOnce(answer());
  vi.stubGlobal('fetch', fetch);
  const stopped = vi.fn();
  await mount(stopped);
  await retry();
  expect(sessionStorage.getItem(key)).toBe(saved);
  expect(stopped).not.toHaveBeenCalled();
  expect(document.body.textContent).toContain('The answer did not confirm this recorded change');
  await retry();
  expect(fetch.mock.calls.map((call) => call[1].body)).toEqual([JSON.stringify(body), JSON.stringify(body)]);
  expect(stopped).toHaveBeenCalledOnce();
  expect(sessionStorage.getItem(key)).toBeNull();
});
