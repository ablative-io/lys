/** Getting started on an empty install: the steps it lists, declining this computer, and adding it for your own agents. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GettingStarted, readStanding } from '../src/features/dashboard/GettingStarted';
import type { Standing } from '../src/features/dashboard/GettingStarted';
import { $, serve } from './harness';
import { ADA, ARCHIVIST, COURIER, OWN, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Machine, NameMachine } from '../src/features/network/contract';

let root: Root | null = null;
beforeEach(() => { sessionStorage.clear(); localStorage.clear(); });
afterEach(() => { if (root) act(() => root?.unmount()); root = null; document.body.innerHTML = ''; });
const empty: Standing = { machines: 0, cambium: null, google: false, read: true, administrator: true };

function service(): Record<string, Route> {
  const routes: Record<string, Route> = { ...SERVICE, '/network': ok({ machines: [], reports_served: true, host_name: 'Ada’s MacBook Pro' }) };
  routes['POST /network/machines'] = (body) => {
    const request = body as NameMachine;
    const machine: Machine = { ...request, id: request.operation, may_run: request.may_run.map((id) => ({ id, display_name: id, state: 'active' })), named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: null };
    routes['POST /network/machines/' + request.operation + '/runner'] = ok({ machine: request.operation, runner: { kind: 'lys' } });
    return ok(machine);
  };
  return routes;
}

async function open(routes: Record<string, Route>, reread = () => undefined, standing = empty) {
  const posted: { path: string; body: unknown }[] = [];
  serve(routes, posted);
  const container = document.createElement('div'); document.body.appendChild(container); root = createRoot(container);
  await act(async () => { root?.render(<GettingStarted person={ADA} agents={0} standing={standing} reread={reread} />); });
  return { posted };
}

/** The page read again: what a reload shows. */
async function reopen(routes: Record<string, Route>, standing = empty) {
  if (root) act(() => root?.unmount());
  root = null; document.body.innerHTML = '';
  return open(routes, undefined, standing);
}

const titles = () => [...$('[aria-label="Getting started"]')?.querySelectorAll('li > b') ?? []].map((title) => title.textContent);
const notThisOne = async () => {
  const decline = [...document.querySelectorAll('button')].find((button) => (button.getAttribute('aria-label') ?? button.textContent) === 'Not this one');
  await act(async () => { decline?.click(); });
};

describe('Getting started on an empty install', () => {
  it('lists every step a fresh Lys still needs, in order, with this computer’s own name offered', async () => {
    const { posted } = await open(service());
    const panel = $('[aria-label="Getting started"]');
    expect([...panel?.querySelectorAll('li > b') ?? []].map((title) => title.textContent)).toEqual(['Choose where agents run', 'Let people sign in with Google', 'Connect Cambium', 'Add your first agent']);
    expect(panel?.textContent).toContain('0 of 4 done');
    expect(($('[aria-label="Run agents on this computer"] [name="name"]') as HTMLInputElement | null)?.value).toBe('Ada’s MacBook Pro');
    expect(posted).toEqual([]);
  });

  it('declines this computer with Not this one and sends nothing', async () => {
    const { posted } = await open(service());
    await notThisOne();
    expect(document.body.textContent).toContain('Agents won’t run on this computer.');
    expect($('[aria-label="Run agents on this computer"]')).toBeNull();
    expect(posted).toEqual([]);
  });

  it('keeps Not this one for the person across a reload and counts the step as answered', async () => {
    await open(service());
    await notThisOne();
    expect($('[aria-label="Getting started"]')?.textContent).toContain('1 of 4 done');
    expect(localStorage.getItem('lys.getting-started.not-this-computer.' + ADA)).toBe('yes');
    await reopen(service());
    expect(titles()).toEqual(['Let people sign in with Google', 'Connect Cambium', 'Add your first agent']);
    expect($('[aria-label="Getting started"]')?.textContent).toContain('1 of 4 done');
    expect($('[aria-label="Run agents on this computer"]')).toBeNull();
  });

  it('keeps one person’s answer to that person', async () => {
    localStorage.setItem('lys.getting-started.not-this-computer.' + SCRIBE, 'yes');
    await open(service());
    expect(titles()[0]).toBe('Choose where agents run');
  });

  it('says so when this browser will not keep the answer', async () => {
    const refuse = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new DOMException('full', 'QuotaExceededError'); });
    try {
      await open(service());
      await notThisOne();
      expect($('[role="status"]')?.textContent).toContain('This browser would not keep the answer, so Lys will ask again next time.');
    } finally { refuse.mockRestore(); }
  });

  it('finishes once every required step is done or answered, leaving Google optional', async () => {
    localStorage.setItem('lys.getting-started.not-this-computer.' + ADA, 'yes');
    await open(service(), undefined, { ...empty, cambium: 'approved' });
    expect(titles()).toEqual(['Let people sign in with Google', 'Add your first agent']);
    await reopen(service(), { ...empty, cambium: 'approved', google: true });
    expect(titles()).toEqual(['Add your first agent']);
    expect($('[aria-label="Getting started"]')?.textContent).toContain('3 of 4 done');
  });

  it('adds this computer for the person’s own agents with Yes, checks its runner and reads the list again', async () => {
    const reread = vi.fn();
    const { posted } = await open(service(), reread);
    const form = $('[aria-label="Run agents on this computer"]');
    await act(async () => { form?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
    expect(posted).toHaveLength(2);
    expect(posted[0]).toEqual({ path: '/network/machines', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), name: 'Ada’s MacBook Pro', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [SCRIBE, COURIER, ARCHIVIST], may_run_roles: [], may_reach: [] } });
    expect(posted[1]).toEqual({ path: '/network/machines/' + (posted[0].body as NameMachine).operation + '/runner', body: { runner: { kind: 'lys' } } });
    expect(reread).toHaveBeenCalledTimes(1);
  });

  it('reads Google as done only when Google is set up and switched on, as sign-in providers shows it', async () => {
    const provider = (enabled: boolean) => ({ id: 'google', provider: 'google', name: 'Google', enabled, client_id: 'c' });
    const providers = (list: unknown[]) => ok({ providers: list, offered: ['google'], redirect_address: 'http://localhost/callback' });
    const base = { ...service(), '/apps': ok({ apps: [] }) };
    serve({ ...base, '/sign-in-providers': providers([provider(true)]) }, []);
    expect((await readStanding()).google).toBe(true);
    serve({ ...base, '/sign-in-providers': providers([provider(false)]) }, []);
    expect((await readStanding()).google).toBe(false);
    serve({ ...base, '/sign-in-providers': providers([]) }, []);
    expect(await readStanding()).toEqual({ ...empty });
    serve(base, []);
    expect((await readStanding()).google).toBe(false);
  });

  it('takes a member to their own installer without offering the server or Google setup', async () => {
    localStorage.setItem('lys.getting-started.not-this-computer.' + ADA, 'yes');
    const { posted } = await open(service(), undefined, { ...empty, administrator: false });
    expect(titles()).toEqual(['Add your own computer', 'Connect Cambium', 'Add your first agent']);
    expect(document.body.textContent).toContain('Another computer, then download its installer');
    expect($('[aria-label="Getting started"] a')?.getAttribute('href')).toBe('#/network');
    expect($('[aria-label="Run agents on this computer"]')).toBeNull();
    expect(document.body.textContent).not.toContain('Lys is running on this computer');
    expect(document.body.textContent).not.toContain('Let people sign in with Google');
    expect(posted).toEqual([]);
  });

  it('never reads provider settings for a member', async () => {
    const requests = serve({ ...service(), '/directory/people': refused(403, 'NotAdmitted', 'Only the administrator.'),
      '/people': ok(OWN), '/apps': ok({ apps: [] }) });
    expect(await readStanding()).toEqual({ ...empty, administrator: false });
    expect(requests).not.toContain('/sign-in-providers');
  });
});
