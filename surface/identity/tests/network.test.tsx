/** Machine controls use server records and keep an uncertain registration under its original operation. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, click, mount, settle, text, unmountAll } from './harness';
import { ADA, OWN, SCRIBE, SERVICE, ok, refused } from './fixtures';
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

describe('Network', () => {
  it('says how Lys starts agents on each computer, and that it cannot say when one was last heard from', async () => {
    await mount('#/network', routes);
    expect(text()).toContain('Workshop laptop'); expect(text()).toContain('Lys does not start agents here.');
    expect(text()).toContain('does not collect runner reports'); expect(button('Refresh network')).toBeNull();
  });
  it('adds a computer Lys does not start agents on, with its websites normalized', async () => {
    const { posted } = await mount('#/network', { ...routes, 'POST /network/machines': (body) => ok(recorded(body as NameMachine)) });
    input('name', 'Lab'); input('kind', 'Server'); await click($('input[name="starts"]:nth-of-type(1)'));
    const none = [...document.querySelectorAll<HTMLInputElement>('input[name="starts"]')][2]; await click(none);
    input('hosts', 'API.EXAMPLE.TEST\napi.example.test'); await submit();
    expect(posted).toEqual([{ path: '/network/machines', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), name: 'Lab', kind: 'Server', runtime: null, slots: 0, may_run: [], may_reach: ['api.example.test'] } }]);
    expect(text()).toContain('Lab was added. Lys will not start agents on it.');
  });
  it('adds the computer Lys runs on with its own runner and the agents that may start there', async () => {
    const { posted } = await mount('#/network', { ...routes, 'POST /network/machines': (body) => ok(recorded(body as NameMachine)), ['POST /network/machines/op-' + 'x'.repeat(32) + '/runner']: ok({}) });
    input('name', 'Lab'); await click($('input[name="agent"][value="' + SCRIBE + '"]')); await submit();
    expect(posted[0].body).toMatchObject({ runtime: 'lys-runner', slots: 0, may_run: [SCRIBE], may_run_roles: [] });
    const id = (posted[0].body as NameMachine).operation;
    expect(posted[1]).toEqual({ path: '/network/machines/' + id + '/runner', body: { runner: { kind: 'lys' } } });
  });
  it('refuses a runner key that is not 64 letters and digits', async () => {
    const { posted } = await mount('#/network', routes);
    input('name', 'Lab'); await click([...document.querySelectorAll<HTMLInputElement>('input[name="starts"]')][1]);
    input('runner-key', 'short'); await submit();
    expect(posted).toEqual([]); expect(text()).toContain('64 letters and digits');
  });
  it('keeps an unconfirmed addition across remount and sends only its original request', async () => {
    const first = await mount('#/network', { ...routes, 'POST /network/machines': refused(503, 'NetworkUnavailable', 'write outcome unknown') });
    input('name', 'Lab'); await submit();
    const original = first.posted[0].body;
    unmountAll(); document.body.innerHTML = '';
    const second = await mount('#/network', { ...routes, 'POST /network/machines': (body) => ok(recorded(body as NameMachine)) });
    await click(button('Check whether it was added'));
    expect(second.posted).toEqual([{ path: '/network/machines', body: original }]);
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
    expect(button('Add this computer')).toBeNull(); expect(button('Retire this computer')).toBeNull();
    unmountAll(); document.body.innerHTML = '';
    await mount('#/network', { ...routes, '/network': refused(503, 'NetworkUnavailable', 'not configured') });
    expect(text()).toContain('NetworkUnavailable'); expect(button('Add this computer')).toBeNull();
  });
  it('records selected roles separately from individual agents, and keeps the entry when the answer omits one', async () => {
    const role = { id: 'op-' + 'c'.repeat(32), name: 'Builder', holders: [], versions: [] };
    const { posted } = await mount('#/network', { ...routes, '/roles': ok({ roles: [role] }), 'POST /network/machines': (body) => ok({ ...recorded(body as NameMachine), may_run_roles: [] }) });
    input('name', 'Lab'); await click($('input[name="role"]')); await submit();
    expect(posted[0].body).toMatchObject({ may_run: [], may_run_roles: [role.id] });
    expect(text()).toContain('What you entered is kept'); expect(sessionStorage.length).toBe(1);
  });
});
