import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { ADA, BEA, COURIER, DIRECTORY, ME, RECEIPTS, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { serve, type } from './harness';

let root: Root | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { const mounted = root; if (mounted) act(() => mounted.unmount()); root = null; });

async function form(routes: Record<string, Route>, query = '') {
  const posted: { path: string; body: unknown }[] = [];
  serve({ ...SERVICE, ...routes }, posted);
  history.replaceState(null, '', '/#/agents/new' + query);
  const container = document.createElement('div'); document.body.appendChild(container);
  const mounted = createRoot(container); root = mounted;
  await act(async () => { mounted.render(<App />); });
  const entry = document.querySelector('form');
  if (!entry) throw new Error('Missing add-agent form');
  return { entry, posted };
}
async function submit(entry: HTMLFormElement) {
  await act(async () => { entry.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
}
function receipt(body: unknown) {
  if (!body || typeof body !== 'object' || !('operation' in body) || typeof body.operation !== 'string') throw new Error('Missing operation');
  return { ...RECEIPTS[4].receipt, identity: COURIER, operation: body.operation };
}

describe('Add-agent retry safety', () => {
  it('retains the exact registration across reload after an uncertain response', async () => {
    const first = await form({ 'POST /agents': refused(503, 'StorageUncertain', 'The outcome is not known') });
    await type(first.entry.querySelector('input[name="display_name"]'), 'Care helper');
    await submit(first.entry);
    expect(first.posted).toHaveLength(1);
    expect(sessionStorage.length).toBeGreaterThan(0);
    expect(first.entry.textContent).not.toContain('Nothing was added');
    const mounted = root;
    if (!mounted) throw new Error('Missing mounted form');
    act(() => mounted.unmount()); root = null;
    const second = await form({ 'POST /agents': refused(503, 'StorageUncertain', 'The outcome is not known') });
    expect(second.posted).toHaveLength(0);
    await submit(second.entry);
    expect(second.posted).toEqual(first.posted);
  });

  it('refuses an unconfirmed receipt without activating or navigating', async () => {
    const { entry, posted } = await form({ 'POST /agents': ok({ agent: COURIER }) });
    await type(entry.querySelector('input[name="display_name"]'), 'Care helper');
    await submit(entry);
    expect(posted.map((call) => call.path)).toEqual(['/agents']);
    expect(location.hash).toBe('#/agents/new');
    expect(sessionStorage.length).toBeGreaterThan(0);
    expect(entry.textContent).toContain('UnconfirmedReceipt');
  });

  it('sends one registration and one activation for two simultaneous submits', async () => {
    const { entry, posted } = await form({
      'POST /agents': (body) => ok({ agent: COURIER, responsible: ADA, receipt: receipt(body) }),
      ['POST /identities/' + COURIER + '/transitions']: (body) => ok({ receipt: { ...receipt(body), change_kind: 5 } }),
    });
    await type(entry.querySelector('input[name="display_name"]'), 'Care helper');
    await act(async () => {
      entry.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
      entry.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    });
    expect(posted.map((call) => call.path)).toEqual(['/agents', '/identities/' + COURIER + '/transitions']);
    expect(location.hash).toBe('#/file/' + COURIER);
    expect(sessionStorage.getItem('lys.add-agent.' + ADA)).toBeNull();
  });
  it('retains the chosen person and team through an uncertain membership and reload', async () => {
    const team = 'op-' + 'd'.repeat(32);
    const membership = '/teams/' + team + '/members';
    const choices = {
      '/directory/people': ok({ ...DIRECTORY, people: DIRECTORY.people.map((person) => ({ ...person, state: 'active' })) }),
      '/teams': ok({ teams: [{ id: team, owner: ADA, name: 'Care team', members: [], state: 'active' }] }),
      'POST /agents': (body: unknown) => ok({ agent: COURIER, responsible: BEA, receipt: receipt(body) }),
      ['POST /identities/' + COURIER + '/transitions']: (body: unknown) => ok({ receipt: { ...receipt(body), change_kind: 5 } }),
      ['POST ' + membership]: refused(503, 'StorageUncertain', 'The outcome is not known'),
    };
    const first = await form(choices, '?answers_to=' + BEA + '&team=' + team);
    await type(first.entry.querySelector('input[name="display_name"]'), 'Care helper');
    await submit(first.entry);
    expect(first.posted.map((call) => call.path)).toEqual(['/agents', '/identities/' + COURIER + '/transitions', membership]);
    expect(first.posted[0].body).toMatchObject({ answers_to: BEA });
    expect(first.posted[2].body).toMatchObject({ member: COURIER });
    expect(location.hash).toContain('/agents/new');
    const mounted = root;
    if (!mounted) throw new Error('Missing mounted form');
    act(() => mounted.unmount()); root = null;
    const second = await form({ ...choices, ['POST ' + membership]: (body) => {
      const operation = receipt(body).operation;
      return ok({ id: team, members: [COURIER], recorded: { operation, act: 'added', member: COURIER, by: ME.signed_in, at: 1 } });
    } });
    expect(second.posted).toEqual([]);
    await submit(second.entry);
    expect(second.posted).toEqual([first.posted[2]]);
    expect(location.hash).toBe('#/file/' + COURIER);
    expect(sessionStorage.getItem('lys.add-agent.' + ADA)).toBeNull();
  });

  it('replays a request saved by the earlier form without changing its operation or owner', async () => {
    const register = 'op-' + '1'.repeat(32);
    sessionStorage.setItem('lys.add-agent.' + ADA, JSON.stringify({ name: 'Earlier helper', register, activate: 'op-' + '2'.repeat(32), agent: null }));
    const { entry, posted } = await form({
      'POST /agents': (body) => ok({ agent: COURIER, responsible: ADA, receipt: receipt(body) }),
      ['POST /identities/' + COURIER + '/transitions']: (body) => ok({ receipt: { ...receipt(body), change_kind: 5 } }),
    }, '?answers_to=' + BEA);
    await submit(entry);
    expect(posted[0]).toEqual({ path: '/agents', body: { operation: register, display_name: 'Earlier helper' } });
    expect(posted).toHaveLength(2);
    expect(location.hash).toBe('#/file/' + COURIER);
    expect(sessionStorage.getItem('lys.add-agent.' + ADA)).toBeNull();
  });

});
