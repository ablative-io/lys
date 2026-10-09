import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { ADA, BEA, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { choose, serve, type } from './harness';

/** Adding a person is one act (POST /people/admit): their name, their email and their first grant, answered by one receipt naming each step's log leaf. */
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

async function fill(entry: HTMLFormElement) {
  await type(entry.querySelector('input[name="display_name"]'), 'Care helper');
  await type(entry.querySelector('input[name="email"]'), 'helper@example.test');
  await type(entry.querySelector('input[name="kind"]'), 'workspace');
  await type(entry.querySelector('input[name="resource"]'), 'ward-7');
  await choose(entry.querySelector('select[name="relation"]'), 'editor');
}

async function submit(entry: HTMLFormElement) {
  await act(async () => { entry.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
}

function admitted(body: unknown) {
  if (!body || typeof body !== 'object' || !('operation' in body) || typeof body.operation !== 'string') throw new Error('Missing operation');
  return {
    person: BEA, subject: 'subject-helper', grant: 'grant-' + 'b'.repeat(32),
    receipt: {
      operation: body.operation, person: BEA, failed: null,
      completed: ['register', 'account', 'bind', 'activate', 'grant'],
      logged: [
        { step: 'register', log: 'directory', index: 12 },
        { step: 'bind', log: 'directory', index: 13 },
        { step: 'activate', log: 'directory', index: 14 },
        { step: 'grant', log: 'grants', index: 7 },
      ],
    },
  };
}

describe('Add a person in one act', () => {
  it('asks for the name, the email and the first grant, and sends one admit', async () => {
    const { entry, posted } = await form({ 'POST /people/admit': (body) => ok(admitted(body)) });
    await fill(entry);
    await submit(entry);
    expect(posted.map((call) => call.path)).toEqual(['/people/admit']);
    const sent = posted[0]?.body as Record<string, unknown>;
    expect(sent.display_name).toBe('Care helper');
    expect(sent.email).toBe('helper@example.test');
    expect(sent.grant).toEqual({ route: 'browser', resource: { kind: 'workspace', id: 'ward-7' }, relation: 'editor' });
    expect(sent.operation).toMatch(/^op-[0-9a-f]{32}$/);
  });

  it('shows the receipt with each step and its log leaf', async () => {
    const { entry } = await form({ 'POST /people/admit': (body) => ok(admitted(body)) });
    await fill(entry);
    await submit(entry);
    const receipt = document.querySelector('[aria-label="Receipt"]');
    expect(receipt).not.toBeNull();
    const text = receipt?.textContent ?? '';
    for (const words of ['directory log leaf 12', 'directory log leaf 13', 'directory log leaf 14', 'grants log leaf 7']) {
      expect(text).toContain(words);
    }
    expect(text).toContain('issuer account');
    expect(document.querySelector('a[href="#/file/' + BEA + '"]')).not.toBeNull();
    expect(sessionStorage.getItem('lys.add-person.' + ADA)).toBeNull();
  });

  it('refuses an answer whose receipt is not for this operation, keeping the request', async () => {
    const { entry } = await form({ 'POST /people/admit': (body) => ok({ ...admitted(body), receipt: { ...admitted(body).receipt, operation: 'op-' + '0'.repeat(32) } }) });
    await fill(entry);
    await submit(entry);
    expect(document.querySelector('[aria-label="Receipt"]')).toBeNull();
    expect(entry.textContent).toContain('did not confirm');
    expect(sessionStorage.length).toBeGreaterThan(0);
  });

  it('names a refused step in the refusal and adds nothing on screen', async () => {
    const { entry } = await form({ 'POST /people/admit': refused(400, 'RelationUnknown', 'gamma is not a relation') });
    await fill(entry);
    await submit(entry);
    expect(entry.textContent).toContain('gamma is not a relation');
    expect(document.querySelector('[aria-label="Receipt"]')).toBeNull();
  });
});
