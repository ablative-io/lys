/** Getting started on an empty install: the steps it lists, declining this computer, and adding it for your own agents. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GettingStarted, readStanding } from '../src/features/dashboard/GettingStarted';
import type { Standing } from '../src/features/dashboard/GettingStarted';
import { $, serve } from './harness';
import { ADA, ARCHIVIST, COURIER, ME, OWN, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Machine, NameMachine } from '../src/features/network/contract';

let root: Root | null = null;
beforeEach(() => { sessionStorage.clear(); localStorage.clear(); });
afterEach(() => { if (root) act(() => root?.unmount()); root = null; document.body.innerHTML = ''; });
const empty: Standing = { machines: 0, app: null, asking: null, apps: true, google: false, read: true, administrator: true, declined: false };
const KEY = 'lys.getting-started.not-this-computer.' + ADA;
const me = (declined: boolean) => ok({ ...ME, getting_started: { host_declined: declined } });

function service(): Record<string, Route> {
  const routes: Record<string, Route> = { ...SERVICE, '/network': ok({ machines: [], reports_served: true, host_name: 'Ada’s MacBook Pro' }),
    'PUT /me/getting-started': (body) => ok(body) };
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
    expect([...panel?.querySelectorAll('li > b') ?? []].map((title) => title.textContent)).toEqual(['Choose where agents run', 'Let people sign in with Google', 'Connect an app', 'Add your first agent']);
    expect(panel?.textContent).toContain('0 of 4 done');
    expect(($('[aria-label="Run agents on the Lys computer"] [name="name"]') as HTMLInputElement | null)?.value).toBe('Ada’s MacBook Pro');
    expect(posted).toEqual([]);
  });

  it('declines this computer with Not this one, kept by Lys for the person and not by this browser', async () => {
    const { posted } = await open(service());
    await notThisOne();
    expect(document.body.textContent).toContain('Agents won’t run on the computer Lys runs on. Add another computer under Computers when you’re ready.');
    expect($('[aria-label="Getting started"] [role="status"] a')?.getAttribute('href')).toBe('#/network');
    expect($('[aria-label="Run agents on the Lys computer"]')).toBeNull();
    expect(posted).toEqual([{ path: 'PUT /me/getting-started', body: { host_declined: true } }]);
    expect(localStorage.getItem(KEY)).toBeNull();
  });

  it('hides the step when /me says it was declined, from any browser, and counts it as answered', async () => {
    await open(service(), undefined, { ...empty, declined: true });
    expect(titles()).toEqual(['Let people sign in with Google', 'Connect an app', 'Add your first agent']);
    expect($('[aria-label="Getting started"]')?.textContent).toContain('1 of 4 done');
    expect($('[aria-label="Run agents on the Lys computer"]')).toBeNull();
  });

  it('reads the answer from /me, the person’s own, and shows the step when it was not declined', async () => {
    serve({ ...service(), '/me': me(true) }, []);
    expect((await readStanding()).declined).toBe(true);
    serve({ ...service(), '/me': me(false) }, []);
    expect((await readStanding()).declined).toBe(false);
    await open(service());
    expect(titles()[0]).toBe('Choose where agents run');
  });

  it('moves an answer this browser kept before to Lys once, then removes it', async () => {
    localStorage.setItem(KEY, 'yes');
    const posted: { path: string; body: unknown }[] = [];
    serve({ ...service(), '/me': me(false) }, posted);
    expect((await readStanding()).declined).toBe(true);
    expect(posted).toEqual([{ path: 'PUT /me/getting-started', body: { host_declined: true } }]);
    expect(localStorage.getItem(KEY)).toBeNull();
    // Once Lys has it, there is nothing left to move.
    serve({ ...service(), '/me': me(true) }, posted);
    expect((await readStanding()).declined).toBe(true);
    expect(posted).toHaveLength(1);
  });

  it('keeps the browser’s answer, and still honours it, when Lys will not take it yet', async () => {
    localStorage.setItem(KEY, 'yes');
    serve({ ...service(), '/me': me(false), 'PUT /me/getting-started': refused(503, 'DirectoryUnavailable', 'closed') }, []);
    expect((await readStanding()).declined).toBe(true);
    expect(localStorage.getItem(KEY)).toBe('yes');
  });

  it('honours the browser’s answer, sending nothing, when Lys could not read its own', async () => {
    localStorage.setItem(KEY, 'yes');
    const posted: { path: string; body: unknown }[] = [];
    const unread = ok({ ...ME, getting_started: { refusal: 'DirectoryUnavailable', reason: 'closed' } });
    serve({ ...service(), '/me': unread }, posted);
    expect((await readStanding()).declined).toBe(true);
    expect(posted).toHaveLength(0);
    expect(localStorage.getItem(KEY)).toBe('yes');
    localStorage.clear();
    serve({ ...service(), '/me': unread }, posted);
    expect((await readStanding()).declined).toBe(false);
  });

  it('says so when Lys will not keep the answer', async () => {
    await open({ ...service(), 'PUT /me/getting-started': refused(503, 'DirectoryUnavailable', 'the directory is closed') });
    await notThisOne();
    expect($('[role="status"]')?.textContent).toContain('Lys could not keep the answer, so it will ask again next time.');
  });

  it('finishes once every required step is done or answered, leaving Google optional', async () => {
    await open(service(), undefined, { ...empty, app: 'approved', declined: true });
    expect(titles()).toEqual(['Let people sign in with Google', 'Add your first agent']);
    await reopen(service(), { ...empty, app: 'approved', google: true, declined: true });
    expect(titles()).toEqual(['Add your first agent']);
    expect($('[aria-label="Getting started"]')?.textContent).toContain('3 of 4 done');
  });

  it('adds this computer for the person’s own agents with Yes, checks its runner and reads the list again', async () => {
    const reread = vi.fn();
    const { posted } = await open(service(), reread);
    const form = $('[aria-label="Run agents on the Lys computer"]');
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
    const { posted } = await open(service(), undefined, { ...empty, administrator: false, declined: true });
    expect(titles()).toEqual(['Add your own computer', 'Connect an app', 'Add your first agent']);
    expect(document.body.textContent).toContain('Another computer, then download its installer');
    expect($('[aria-label="Getting started"] a')?.getAttribute('href')).toBe('#/network');
    expect($('[aria-label="Run agents on the Lys computer"]')).toBeNull();
    expect(document.body.textContent).not.toContain('Lys runs on');
    expect($('[aria-label="Getting started"] a')?.textContent).toBe('Computers');
    // A member cannot connect an app: the step says who does, with no link to a page they cannot use.
    expect([...document.querySelectorAll('[aria-label="Getting started"] li')].find((li) => li.querySelector('b')?.textContent === 'Connect an app')?.textContent).toBe('Connect an appNo app is connected to Lys yet. An administrator connects one on Apps; once it is, this step is done.');
    expect(document.body.textContent).not.toContain('Let people sign in with Google');
    expect(posted).toEqual([]);
  });

  it('reads any approved app as the step done, whatever its id', async () => {
    serve({ ...service(), '/apps': ok({ apps: [{ id: 'pending_fixture', name: 'Pending Fixture', state: 'pending' }, { id: 'fixture_product', name: 'Fixture Product', state: 'approved' }] }) }, []);
    const standing = await readStanding();
    expect([standing.app, standing.asking]).toEqual(['approved', null]);
  });

  it('names the first app that has asked to connect, and sends the administrator to Apps to review it', async () => {
    serve({ ...service(), '/apps': ok({ apps: [{ id: 'fixture_product', name: 'Fixture Product', state: 'pending' }, { id: 'other_fixture', state: 'pending' }] }) }, []);
    const standing = await readStanding();
    expect([standing.app, standing.asking]).toEqual(['pending', 'Fixture Product']);
    await reopen(service(), standing);
    const step = [...document.querySelectorAll('[aria-label="Getting started"] li')].find((li) => li.querySelector('b')?.textContent === 'Connect an app');
    expect(step?.textContent).toBe('Connect an appFixture Product has asked to connect. Review what it asks for and approve it on Apps.');
    expect(step?.querySelector('a')?.getAttribute('href')).toBe('#/apps');
  });

  it('asks the administrator to connect an app when none has asked', async () => {
    await open(service());
    const step = [...document.querySelectorAll('[aria-label="Getting started"] li')].find((li) => li.querySelector('b')?.textContent === 'Connect an app');
    expect(step?.textContent).toBe('Connect an appConnect an app to Lys on Apps.');
  });

  it('never reads provider settings for a member', async () => {
    const requests = serve({ ...service(), '/directory/people': refused(403, 'NotAdmitted', 'Only the administrator.'),
      '/people': ok(OWN), '/apps': ok({ apps: [] }) });
    expect(await readStanding()).toEqual({ ...empty, administrator: false });
    expect(requests).not.toContain('/sign-in-providers');
  });

  it('names the computer Lys runs on by its name, never as this computer, which is the one the browser is on', async () => {
    await open(service());
    const form = $('[aria-label="Run agents on the Lys computer"]');
    expect(form?.querySelector('p')?.textContent).toBe('Lys runs on Ada’s MacBook Pro. Should your agents run there too? Only say yes if that is a computer you want agents to work on.');
    expect(form?.querySelector('label')?.textContent).toBe('That computer’s name');
    expect(document.body.textContent).not.toContain('this computer');
    await act(async () => { (form?.querySelector('[name="name"]') as HTMLInputElement).value = ''; form?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
    expect(form?.querySelector('[role="alert"]')?.textContent).toBe('Give that computer a name, for example "My MacBook".');
    // A host that names none is still never "this computer".
    await reopen({ ...service(), '/network': ok({ machines: [], reports_served: true }) });
    expect($('[aria-label="Run agents on the Lys computer"] p')?.textContent).toContain('Lys runs on the computer it was installed on. Should your agents run there too?');
  });

  it('offers Connect an app only when Apps can be read, and still lists the other steps when it cannot', async () => {
    serve({ ...service(), '/apps': refused(403, 'NotAdmitted', 'Only the administrator.') }, []);
    const standing = await readStanding();
    expect([standing.apps, standing.read]).toEqual([false, true]);
    await reopen(service(), standing);
    expect(titles()).toEqual(['Choose where agents run', 'Let people sign in with Google', 'Add your first agent']);
    expect($('[aria-label="Getting started"] a[href="#/apps"]')).toBeNull();
    expect($('[aria-label="Getting started"]')?.textContent).toContain('0 of 3 done');
  });
});
