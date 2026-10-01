/** Machine controls use server records and keep an uncertain registration under its original operation. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, $$, click, mount, settle, text, unmountAll } from './harness';
import { ADA, ME, OWN, REVIEWER, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Machine, NameMachine } from '../src/features/network/contract';

beforeEach(() => sessionStorage.clear());
const machine: Machine = { id: 'op-' + 'a'.repeat(32), name: 'Workshop laptop', kind: 'laptop', runtime: null, slots: 0, may_run: [], may_reach: [], named_by: ADA, named_at: 1790000000, state: 'in_use', retired_at: null, last_report_at: null };
const routes = { ...SERVICE, '/network': ok({ machines: [machine], reports_served: false }), ['/network/machines/' + machine.id + '/runner']: ok({ machine: machine.id, runner: null }) };
const button = (label: string) => [...document.querySelectorAll('button')].find((value) => value.textContent === label) ?? null;
function input(name: string, value: string) {
  const element = $('[name="' + name + '"]');
  if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement || element instanceof HTMLSelectElement)) throw new Error('Missing ' + name);
  element.value = value;
}
async function submit() {
  const form = $('form[aria-label="Add a computer"]');
  if (!form) throw new Error('Missing registration form');
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}
const recorded = (body: NameMachine): Machine => ({ ...machine, ...body, id: body.operation, may_run: body.may_run.map((id) => ({ id, display_name: 'Scribe', state: 'active' })) });

async function adding(extra: Record<string, Route> = {}) {
  for (const [path, route] of Object.entries(routes)) if (!(path in extra)) extra[path] = route;
  const mounted = await mount('#/network', extra);
  await click(button('+ Add a computer'));
  return mounted;
}
function naming(runner: (id: string) => Route = (id) => ok({ machine: id, runner: { kind: 'lys' } })): Record<string, Route> {
  const answers: Record<string, Route> = {};
  answers['POST /network/machines'] = (body) => {
    const draft = body as NameMachine;
    answers['POST /network/machines/' + draft.operation + '/runner'] = runner(draft.operation);
    answers['/network/machines/' + draft.operation + '/runner'] = ok({ machine: draft.operation, runner: { kind: 'lys' } });
    return ok(recorded(draft));
  };
  return answers;
}

describe('Network', () => {
  it('lists each computer with its status and who may start there, and opens it beside the list', async () => {
    await mount('#/network', routes);
    expect($('#screen .page.fill .pane table')).not.toBeNull();
    const row = $$('#screen tbody tr[data-href]')[0];
    expect(row.textContent).toContain('Workshop laptop'); expect(row.textContent).toContain('Does not run agents'); expect(row.textContent).toContain('Lys does not start agents here.');
    expect($('.detail h2')?.textContent).toBe('Workshop laptop');
    expect(button('Refresh network')).toBeNull();
  });
  it('adds this computer with a name and no website permissions', async () => {
    const { posted } = await adding(naming());
    expect($('input[name="kind"]')).toBeNull(); expect($('input[type="radio"]')).toBeNull();
    expect($('[name="hosts"]')).toBeNull();
    input('name', 'Lab'); await submit();
    expect(posted[0]).toEqual({ path: '/network/machines', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), name: 'Lab', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: [] } });
  });
  it('adds this computer with its own runner and grants no agent or role from the global page', async () => {
    const { posted } = await adding(naming());
    input('name', 'Lab'); await submit();
    expect($('input[aria-label="Add an agent"]')).toBeNull();
    expect(posted[0].body).toMatchObject({ runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [] });
    const id = (posted[0].body as NameMachine).operation;
    expect(posted[1]).toEqual({ path: '/network/machines/' + id + '/runner', body: { runner: { kind: 'lys' } } });
  });
  it('keeps the runner phase unresolved when its answer is refused', async () => {
    const { posted } = await adding(naming(() => refused(503, 'RunnerUnavailable', 'The runner was not confirmed')));
    input('name', 'Lab'); await submit();
    expect(posted).toHaveLength(2); expect(text()).toContain('RunnerUnavailable');
    expect(text()).not.toContain('Lab was added.'); expect(sessionStorage.length).toBe(1);
    expect(button('Check whether it was added')).not.toBeNull();
  });
  it('keeps an unconfirmed addition across remount and sends only its original request', async () => {
    const first = await adding({ 'POST /network/machines': refused(503, 'NetworkUnavailable', 'write outcome unknown') });
    input('name', 'Lab'); await submit();
    const original = first.posted[0].body;
    unmountAll(); document.body.innerHTML = '';
    const second = await adding(naming());
    await click(button('Check whether it was added'));
    expect(second.posted[0]).toEqual({ path: '/network/machines', body: original });
    expect(text()).toContain('Lab was added.');
  });
  it('asks before retiring and says running agents keep running', async () => {
    const { posted } = await mount('#/network', { ...routes, ['POST /network/machines/' + machine.id + '/retire']: ok({ ...machine, state: 'retired' }) });
    await click(button('Retire this computer')); expect(posted).toEqual([]);
    expect(text()).toContain('Agents already running on it keep running');
    await click(button('Confirm retirement'));
    expect(posted).toEqual([{ path: '/network/machines/' + machine.id + '/retire', body: {} }]);
  });
  it('does not offer changes to non-administrators or when the network is unavailable', async () => {
    await mount('#/network', { ...routes, '/directory/people': refused(403, 'NotAdmitted', 'not administrator'), '/people': ok(OWN) });
    expect(button('+ Add a computer')).toBeNull(); expect(button('Retire this computer')).toBeNull();
    unmountAll(); document.body.innerHTML = '';
    await mount('#/network', { ...routes, '/network': refused(503, 'NetworkUnavailable', 'not configured') });
    expect(text()).toContain('NetworkUnavailable'); expect(button('+ Add a computer')).toBeNull();
  });
  it('replays a legacy role declaration exactly and keeps the entry when the answer omits that role', async () => {
    const role = { id: 'op-' + 'c'.repeat(32), name: 'Builder', holders: [], versions: [] };
    const legacy: NameMachine = { operation: 'op-' + 'd'.repeat(32), name: 'Lab', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [role.id], may_reach: [] };
    sessionStorage.setItem('lys.pending.machine.' + ADA, JSON.stringify(legacy));
    const { posted } = await adding({ '/roles': ok({ roles: [role] }), 'POST /network/machines': (body) => ok({ ...recorded(body as NameMachine), may_run_roles: [] }) });
    await click(button('Check whether it was added'));
    expect(posted[0].body).toMatchObject({ may_run: [], may_run_roles: [role.id] });
    expect(posted[0].body).toEqual(legacy);
    expect(text()).toContain('What you entered is kept'); expect(sessionStorage.length).toBe(1);
  });
});

describe('Computers at the size of a business', () => {
  const team = (n: number, members: string[]) => ({ id: 'op-' + String(n).padStart(32, '0'), name: 'Team ' + n, owner: ADA, parent: null, lead: null, members, held: [], description: '', state: 'active', created_by: ME.signed_in, created_at: 1, retired_at: null });
  const now = Math.floor(Date.now() / 1000);
  const fleet: Machine[] = Array.from({ length: 60 }, (_, n) => ({ ...machine, id: 'op-' + String(n).padStart(32, 'a'), name: 'Builder ' + String(n).padStart(2, '0'), runtime: 'lys-runner', may_run: [{ id: n % 2 ? SCRIBE : REVIEWER, display_name: n % 2 ? 'Scribe' : 'Reviewer', state: 'active' }], last_report_at: n < 55 ? now : now - 86400 }));
  const big = { ...routes, '/teams': ok({ teams: [team(1, [SCRIBE]), team(2, [REVIEWER])] }), '/network': ok({ machines: fleet, reports_served: true }),
    ...Object.fromEntries(fleet.map((each) => ['/network/machines/' + each.id + '/runner', ok({ machine: each.id, runner: { kind: 'dialled', key: 'a'.repeat(64) } })])) };
  it('groups computers by the teams whose agents start there, and counts what is up', async () => {
    await mount('#/network', big);
    expect($$('#screen tr.group .group-name').map((name) => name.textContent)).toEqual(['Team 1', 'Team 2']);
    expect($$('.stat .n').map((n) => n.textContent)).toEqual(['60', '55', '5', '0']);
    expect($('.tools .count')?.textContent).toBe('60 computers in 2 groups');
  });
  it('shows only the computers not heard from, in one click', async () => {
    await mount('#/network', big);
    await click(button('Not heard from'));
    expect($$('#screen tbody tr[data-href]').map((row) => row.querySelector('td')?.textContent)).toEqual(['Builder 55', 'Builder 57', 'Builder 59', 'Builder 56', 'Builder 58']);
  });
});
