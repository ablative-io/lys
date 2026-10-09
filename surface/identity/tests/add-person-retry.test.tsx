import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { ADA, BEA, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { choose, serve, type } from './harness';

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
async function fill(entry: HTMLFormElement) {
  await type(entry.querySelector('input[name="display_name"]'), 'Care helper');
  await type(entry.querySelector('input[name="email"]'), 'helper@example.test');
  await type(entry.querySelector('input[name="kind"]'), 'workspace');
  await type(entry.querySelector('input[name="resource"]'), 'ward-7');
  await choose(entry.querySelector('select[name="relation"]'), 'editor');
}
function admitted(body: unknown) {
  if (!body || typeof body !== 'object' || !('operation' in body) || typeof body.operation !== 'string') throw new Error('Missing operation');
  return { person: BEA, subject: 'subject-helper', grant: 'grant-' + 'b'.repeat(32), receipt: {
    operation: body.operation, person: BEA, failed: null, completed: ['register', 'account', 'bind', 'activate', 'grant'],
    logged: [{ step: 'register', log: 'directory', index: 12 }, { step: 'bind', log: 'directory', index: 13 },
      { step: 'activate', log: 'directory', index: 14 }, { step: 'grant', log: 'grants', index: 7 }],
  } };
}

describe('Add a person', () => {
  it('has the name, the email and the first grant, and one Add button', async () => {
    const { entry } = await form({});
    for (const name of ['display_name', 'email', 'kind', 'resource']) expect(entry.querySelector(`input[name="${name}"]`)).not.toBeNull();
    expect(entry.querySelector('select[name="relation"]')).not.toBeNull();
    const add = entry.querySelector('button[type="submit"]');
    expect([add?.getAttribute('aria-label'), add?.textContent]).toEqual(['Add person', 'Add']);
  });
});

describe('Add-person retry safety', () => {
  it('retains the exact registration across reload after an uncertain response', async () => {
    const first = await form({ 'POST /people/admit': refused(503, 'StorageUncertain', 'The outcome is not known') });
    await fill(first.entry);
    await submit(first.entry);
    expect(first.posted).toHaveLength(1);
    expect(sessionStorage.length).toBeGreaterThan(0);
    expect(first.entry.textContent).not.toContain('Nothing was added');
    const mounted = root;
    if (!mounted) throw new Error('Missing mounted form');
    act(() => mounted.unmount()); root = null;
    const second = await form({ 'POST /people/admit': refused(503, 'StorageUncertain', 'The outcome is not known') });
    expect(second.posted).toHaveLength(0);
    await submit(second.entry);
    expect(second.posted).toEqual(first.posted);
  });

  it('refuses an unconfirmed receipt without navigating', async () => {
    const { entry, posted } = await form({ 'POST /people/admit': ok({ person: BEA }) });
    await fill(entry);
    await submit(entry);
    expect(posted.map((call) => call.path)).toEqual(['/people/admit']);
    expect(location.hash).toBe('#/people/new');
    expect(sessionStorage.length).toBeGreaterThan(0);
    expect(entry.textContent).toContain('did not confirm');
  });

  it('sends one registration for two simultaneous submits', async () => {
    const { entry, posted } = await form({
      'POST /people/admit': (body) => ok(admitted(body)),
    });
    await fill(entry);
    await act(async () => {
      entry.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
      entry.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    });
    expect(posted.map((call) => call.path)).toEqual(['/people/admit']);
    expect(document.querySelector('[aria-label="Receipt"]')).not.toBeNull();
    expect(sessionStorage.getItem('lys.add-person.' + ADA)).toBeNull();
  });
});
