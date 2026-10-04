import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { ADA, BEA, RECEIPTS, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { serve, type } from './harness';

let root: Root | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { const mounted = root; if (mounted) act(() => mounted.unmount()); root = null; });

async function form(routes: Record<string, Route>) {
  const posted: { path: string; body: unknown }[] = [];
  serve({ ...SERVICE, ...routes }, posted);
  history.replaceState(null, '', '/#/people/new');
  const container = document.createElement('div'); document.body.appendChild(container);
  const mounted = createRoot(container); root = mounted;
  await act(async () => { mounted.render(<App />); });
  const entry = document.querySelector('form');
  if (!entry) throw new Error('Missing add-person form');
  return { entry, posted };
}
async function submit(entry: HTMLFormElement) {
  await act(async () => { entry.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
}
function receipt(body: unknown) {
  if (!body || typeof body !== 'object' || !('operation' in body) || typeof body.operation !== 'string') throw new Error('Missing operation');
  return { ...RECEIPTS[4].receipt, identity: BEA, operation: body.operation, change_kind: 1 };
}

describe('Add a person', () => {
  it('is one row: the name field and its button beside it', async () => {
    const { entry } = await form({});
    const row = entry.querySelector('.add-person-row');
    expect(row?.querySelector('input[name="display_name"]')).not.toBeNull();
    expect(row?.querySelector('button[type="submit"]')?.textContent).toBe('Add person');
  });
});

describe('Add-person retry safety', () => {
  it('retains the exact registration across reload after an uncertain response', async () => {
    const first = await form({ 'POST /people': refused(503, 'StorageUncertain', 'The outcome is not known') });
    await type(first.entry.querySelector('input[name="display_name"]'), 'Care helper');
    await submit(first.entry);
    expect(first.posted).toHaveLength(1);
    expect(sessionStorage.length).toBeGreaterThan(0);
    expect(first.entry.textContent).not.toContain('Nothing was added');
    const mounted = root;
    if (!mounted) throw new Error('Missing mounted form');
    act(() => mounted.unmount()); root = null;
    const second = await form({ 'POST /people': refused(503, 'StorageUncertain', 'The outcome is not known') });
    expect(second.posted).toHaveLength(0);
    await submit(second.entry);
    expect(second.posted).toEqual(first.posted);
  });

  it('refuses an unconfirmed receipt without navigating', async () => {
    const { entry, posted } = await form({ 'POST /people': ok({ person: BEA }) });
    await type(entry.querySelector('input[name="display_name"]'), 'Care helper');
    await submit(entry);
    expect(posted.map((call) => call.path)).toEqual(['/people']);
    expect(location.hash).toBe('#/people/new');
    expect(sessionStorage.length).toBeGreaterThan(0);
    expect(entry.textContent).toContain('The answer did not confirm this change');
  });

  it('sends one registration for two simultaneous submits', async () => {
    const { entry, posted } = await form({
      'POST /people': (body) => ok({ person: BEA, receipt: receipt(body) }),
    });
    await type(entry.querySelector('input[name="display_name"]'), 'Care helper');
    await act(async () => {
      entry.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
      entry.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    });
    expect(posted.map((call) => call.path)).toEqual(['/people']);
    expect(location.hash).toBe('#/file/' + BEA);
    expect(sessionStorage.getItem('lys.add-person.' + ADA)).toBeNull();
  });
});
