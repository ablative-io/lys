/** Machine controls use server records and keep an uncertain registration under its original operation. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, click, mount, settle, text, unmountAll } from './harness';
import { ADA, OWN, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Machine, NameMachine } from '../src/features/network/contract';

beforeEach(() => sessionStorage.clear());
const machine: Machine = { id: 'op-' + 'a'.repeat(32), name: 'Workshop laptop', kind: 'laptop', runtime: null, slots: 0, may_run: [], may_run_roles: [], may_reach: [], named_by: ADA, named_at: 1790000000, state: 'in_use', retired_at: null, last_report_at: null };
const ROLE = 'op-' + 'b'.repeat(32);
const routes = { ...SERVICE, '/network': ok({ machines: [machine], reports_served: false }), '/roles': ok({ roles: [{ id: ROLE, name: 'Builder' }] }) };
const button = (label: string) => [...document.querySelectorAll('button')].find((value) => value.textContent === label) ?? null;
function input(name: string, value: string) {
  const element = $('[name="' + name + '"]');
  if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) throw new Error('Missing ' + name);
  element.value = value;
}
async function submit() {
  const form = $('form[aria-label="Register machine"]');
  if (!form) throw new Error('Missing registration form');
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}
const recorded = (body: NameMachine): Machine => ({ ...machine, ...body, id: body.operation, may_run: body.may_run.map((id) => ({ id, display_name: 'Scribe', state: 'active' })) });

describe('Network', () => {
  it('shows records without claiming live runtime status', async () => {
    await mount('#/network', routes);
    expect(text()).toContain('Workshop laptop');
    expect(text()).toContain('Runtime reporting is not connected');
    expect(text()).toContain('none received');
    expect(text()).not.toContain('not built yet');
  });
  it('registers a machine with explicit no-runtime placement and normalized hosts', async () => {
    const { posted } = await mount('#/network', { ...routes, 'POST /network/machines': (body) => ok(recorded(body as NameMachine)) });
    input('name', 'Lab'); input('kind', 'server'); input('hosts', 'API.EXAMPLE.TEST\napi.example.test');
    await submit();
    expect(posted).toHaveLength(1);
    expect(posted[0]).toEqual({ path: '/network/machines', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), name: 'Lab', kind: 'server', runtime: null, slots: 0, may_run: [], may_run_roles: [], may_reach: ['api.example.test'] } });
    expect(text()).toContain('Machine recorded.');
  });
  it('keeps an unknown registration across remount and sends only its original request', async () => {
    const first = await mount('#/network', { ...routes, 'POST /network/machines': refused(503, 'NetworkUnavailable', 'write outcome unknown') });
    input('name', 'Lab'); input('kind', 'server'); await submit();
    const original = first.posted[0].body;
    unmountAll(); document.body.innerHTML = '';
    const second = await mount('#/network', { ...routes, 'POST /network/machines': (body) => ok(recorded(body as NameMachine)) });
    await click(button('Check original registration'));
    expect(second.posted).toEqual([{ path: '/network/machines', body: original }]);
    expect(text()).toContain('Machine recorded.');
  });
  it('records chosen runtime capacity and agent placement', async () => {
    const { posted } = await mount('#/network', { ...routes, 'POST /network/machines': (body) => ok(recorded(body as NameMachine)) });
    input('name', 'Lab'); input('kind', 'server');
    await click($('form input[type="checkbox"]')); input('runtime', 'Norn'); input('slots', '2');
    await click($('input[name="agent"][value="' + SCRIBE + '"]')); await submit();
    expect(posted[0].body).toMatchObject({ runtime: 'Norn', slots: 2, may_run: [SCRIBE] });
  });
  it('records the roles whose holders may run on the machine and shows them by name', async () => {
    const { posted } = await mount('#/network', { ...routes, 'POST /network/machines': (body) => ok(recorded(body as NameMachine)) });
    input('name', 'Lab'); input('kind', 'server');
    await click($('form input[type="checkbox"]')); input('runtime', 'Norn'); input('slots', '1');
    await click($('input[name="role"][value="' + ROLE + '"]')); await submit();
    expect(posted[0].body).toMatchObject({ may_run: [], may_run_roles: [ROLE] });
    unmountAll(); document.body.innerHTML = '';
    await mount('#/network', { ...routes, '/network': ok({ machines: [{ ...machine, may_run_roles: [ROLE] }], reports_served: false }) });
    expect(text()).toContain('Builder');
  });
  it('requires confirmation for idempotent retirement and does not claim a process was stopped', async () => {
    const { posted } = await mount('#/network', { ...routes, ['POST /network/machines/' + machine.id + '/retire']: ok({ ...machine, state: 'retired' }) });
    await click(button('Retire machine')); expect(posted).toEqual([]);
    expect(text()).toContain('does not stop any running process');
    await click(button('Confirm retirement'));
    expect(posted).toEqual([{ path: '/network/machines/' + machine.id + '/retire', body: {} }]);
  });
  it('does not offer mutations to non-administrators or when the network is unavailable', async () => {
    await mount('#/network', { ...routes, '/directory/people': refused(403, 'NotAdmitted', 'not administrator'), '/people': ok(OWN) });
    expect(button('Register machine')).toBeNull(); expect(button('Retire machine')).toBeNull();
    unmountAll(); document.body.innerHTML = '';
    await mount('#/network', { ...routes, '/network': refused(503, 'NetworkUnavailable', 'not configured') });
    expect(text()).toContain('NetworkUnavailable'); expect(button('Register machine')).toBeNull();
  });
});
