import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { $, $$, serve } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Machine, NameMachine } from '../src/features/network/contract';

let root: Root | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { if (root) act(() => root?.unmount()); root = null; });
const id = 'op-' + '1'.repeat(32);
const prefix = '/agents/' + SCRIBE;
const path = '/network/machines/' + id + '/agents';
const key = 'lys.pending.machine-agents.' + ADA + '.' + SCRIBE;
const machine: Machine = { id, name: 'Ward computer', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: ['mcp.example.test'], named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: null };
const capability = { openapi: '3.1.0', paths: { '/network/machines/{id}/agents': { post: {
  requestBody: { content: { 'application/json': { schema: { type: 'object', properties: { operation: { type: 'string' }, agent: { type: 'string' }, allow: { type: 'boolean' } }, required: ['operation', 'agent', 'allow'] } } } },
  responses: { '200': { content: { 'application/json': { schema: { type: 'object', properties: { machine: { type: 'object' }, recorded: { type: 'object' } }, required: ['machine', 'recorded'] } } } } },
} } } };
type Body = { operation: string; agent: string; allow: boolean };
const receipt = (body: Body, current = body.allow) => ({ machine: { ...machine, may_run: current ? [{ id: SCRIBE, display_name: 'Scribe', state: 'active' }] : [] }, recorded: { ...body, machine: id, by: ADA, at: 1 } });

function service(extra: Record<string, Route> = {}): Record<string, Route> {
  return { ...SERVICE, '/surface-contract': ok(capability), '/network': ok({ machines: [machine], reports_served: true }), '/harnesses': ok({ programs: [] }), '/skills': ok({ skills: [] }), '/secrets': ok({ secrets: [] }),
    [prefix + '/provisioning']: ok({ agent: SCRIBE, profile: null, versions: [], enforced: false }), ['POST ' + path]: (body) => ok(receipt(body as Body)), ...extra };
}

async function open(routes = service()) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve(routes, posted);
  history.replaceState(null, '', '/#/file/' + SCRIBE + '/provisioning');
  const container = document.createElement('div'); document.body.appendChild(container); root = createRoot(container);
  await act(async () => { root?.render(<App />); });
  return { posted, requests };
}

async function remount(routes: Record<string, Route>) {
  if (root) act(() => root?.unmount()); root = null; document.body.innerHTML = '';
  return open(routes);
}

function tick(computer = id) {
  const input = $('[aria-label="Where can Scribe run?"] input[value="' + computer + '"]');
  if (!(input instanceof HTMLInputElement)) throw new Error('The named computer tick is missing');
  return input;
}
async function change(computer = id) { await act(async () => { tick(computer).click(); }); }

function plain(code: string, words: string) {
  const details = $$('.refusal-name').find((entry) => entry.textContent?.includes(code));
  expect(details?.textContent).toContain(code);
  const face = document.body.cloneNode(true) as HTMLElement;
  for (const detail of face.querySelectorAll('.refusal-name')) detail.remove();
  expect(face.textContent).not.toContain(code);
  expect(face.textContent).toContain(words);
}

describe('Choosing where an agent can run', () => {
  it('lists every in-use computer by name, leaves retired computers out and explains one without a runtime', async () => {
    const { posted, requests } = await open(service({ '/network': ok({ machines: [machine, { ...machine, id: 'op-' + '2'.repeat(32), name: 'Second computer', may_run: [{ id: SCRIBE, display_name: 'Scribe', state: 'active' }] }, { ...machine, id: 'op-' + '3'.repeat(32), name: 'No runtime', runtime: null }, { ...machine, id: 'op-' + '4'.repeat(32), name: 'Retired computer', state: 'retired' }], reports_served: true }) }));
    expect($('[aria-label="Where can Scribe run?"]')?.textContent).toContain('Ward computer');
    expect($$('[aria-label="Where can Scribe run?"] input').map((input) => input.getAttribute('value'))).toEqual([id, 'op-' + '2'.repeat(32), 'op-' + '3'.repeat(32)]);
    expect(tick().checked).toBe(false);
    expect(tick('op-' + '2'.repeat(32)).checked).toBe(true);
    expect(tick('op-' + '3'.repeat(32)).disabled).toBe(true);
    expect($('[aria-label="Where can Scribe run?"]')?.textContent).not.toContain('Retired computer');
    expect(requests.filter((request) => request === '/network')).toHaveLength(1);
    expect(requests.filter((request) => request === '/surface-contract')).toHaveLength(1);
    expect(posted).toEqual([]);
  });

  it.each([
    { name: 'absent', schema: ok({ openapi: '3.1.0', paths: {} }) },
    { name: 'malformed', schema: ok({ paths: { '/network/machines/{id}/agents': { post: {} } } }) },
    { name: 'unavailable', schema: refused(503, 'SchemaUnavailable', 'The served schema could not be read') },
  ])('shows coming and sends no tick when the served route is $name', async ({ schema }) => {
    const { posted } = await open(service({ '/surface-contract': schema }));
    expect($('[aria-label="Where can Scribe run?"]')?.textContent).toContain('coming');
    expect(tick().disabled).toBe(true);
    await change();
    expect(posted).toEqual([]);
  });

  it('keeps computer permission changes administrator-only', async () => {
    const { posted } = await open(service({ '/directory/people': refused(403, 'NotAdmitted', 'not administrator') }));
    expect(tick().disabled).toBe(true);
    await change();
    expect(posted).toEqual([]);
  });

  it('keeps a role-admitted computer ticked and locked even without a direct allowance', async () => {
    const role = 'op-' + '5'.repeat(32);
    const { posted } = await open(service({ '/network': ok({ machines: [{ ...machine, may_run: [], may_run_roles: [role] }], reports_served: true }),
      '/roles': ok({ roles: [{ id: role, name: 'Care team', holders: [{ holder: SCRIBE, state: 'holding' }], versions: [] }] }) }));
    expect(tick().checked).toBe(true);
    expect(tick().disabled).toBe(true);
    expect($('[aria-label="Where can Scribe run?"]')?.textContent).toContain('allowed through role Care team');
    await change();
    expect(posted).toEqual([]);
  });

  it('uses a different retained operation for a tick and untick, confirms each receipt and shows current permissions', async () => {
    const { posted, requests } = await open(); await change();
    expect(posted).toHaveLength(1);
    expect(posted[0]).toEqual({ path, body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), agent: SCRIBE, allow: true } });
    expect(tick().checked).toBe(true);
    await change();
    expect(posted).toHaveLength(2);
    expect(posted[1].body).toMatchObject({ agent: SCRIBE, allow: false });
    expect((posted[1].body as Body).operation).not.toBe((posted[0].body as Body).operation);
    expect(tick().checked).toBe(false);
    expect(sessionStorage.getItem(key)).toBeNull();
    expect(requests.filter((request) => request === '/surface-contract')).toHaveLength(1);
  });

  it('replays the exact uncertain tick after remount and does not restore an allowance removed later', async () => {
    const routes = service({ ['POST ' + path]: refused(503, 'NetworkUnavailable', 'The change outcome is unknown') });
    const first = await open(routes); await change();
    expect(sessionStorage.getItem(key)).not.toBeNull();
    plain('NetworkUnavailable', 'Lys could not confirm this computer permission.');
    const original = first.posted[0]; routes['POST ' + path] = (body) => ok(receipt(body as Body, false));
    const next = await remount(routes);
    const retry = [...document.querySelectorAll('button')].find((button) => button.textContent === 'Check computer permission');
    if (!retry) throw new Error('The uncertain tick has no retry action');
    await act(async () => { retry.click(); });
    expect(next.posted).toEqual([original]);
    expect(tick().checked).toBe(false);
    expect(sessionStorage.getItem(key)).toBeNull();
  });

  it.each([
    { field: 'operation', value: 'op-' + '9'.repeat(32) },
    { field: 'machine', value: 'op-' + '8'.repeat(32) },
    { field: 'agent', value: 'agent-' + 'f'.repeat(32) },
    { field: 'allow', value: false },
    { field: 'by', value: 'person-' + 'f'.repeat(32) },
    { field: 'at', value: -1 },
  ])('retains the request when the receipt has a wrong $field', async ({ field, value }) => {
    const { posted } = await open(service({ ['POST ' + path]: (body) => {
      const answer = receipt(body as Body);
      return ok({ ...answer, recorded: { ...answer.recorded, [field]: value } });
    } }));
    await change();
    expect(posted).toHaveLength(1);
    expect(document.body.textContent).toContain('MachineAdmissionReceiptMismatch');
    expect(tick().checked).toBe(false);
    expect(sessionStorage.getItem(key)).not.toBeNull();
    plain('MachineAdmissionReceiptMismatch', 'Lys could not confirm this computer permission.');
  });

  it('names corrupt retained permission state and sends no replacement request', async () => {
    sessionStorage.setItem(key, '{');
    const { posted } = await open();
    expect(document.body.textContent).toContain('PendingMachineAdmissionUnreadable');
    expect(tick().disabled).toBe(true);
    await change();
    expect(posted).toEqual([]);
    plain('PendingMachineAdmissionUnreadable', 'Lys could not read the saved computer permission.');
  });

  it('blocks a second tick while its first change is being sent', async () => {
    const { posted } = await open();
    await act(async () => { tick().click(); tick().click(); });
    expect(posted).toHaveLength(1);
    expect(tick().checked).toBe(true);
  });

  it('adds this computer in place under one retained operation, admits only this agent and returns to its selected computer', async () => {
    const routes = service({ '/network': ok({ machines: [], reports_served: true }), '/surface-contract': ok({ openapi: '3.1.0', paths: {} }) });
    routes['POST /network/machines'] = (body) => {
      const draft = body as NameMachine;
      const named: Machine = { ...machine, ...draft, id: draft.operation, may_run: draft.may_run.map((agent) => ({ id: agent, display_name: 'Scribe', state: 'active' })) };
      routes['POST /network/machines/' + draft.operation + '/runner'] = (given) => ok({ machine: draft.operation, ...(given as object) });
      routes['/network'] = ok({ machines: [named], reports_served: true });
      return ok(named);
    };
    const { posted } = await open(routes);
    const add = [...document.querySelectorAll('button')].find((button) => button.textContent === 'Add this computer');
    if (!add) throw new Error('The settings page with no computer has no add-computer action');
    await act(async () => { add.click(); });
    const form = $('form[aria-label="Add a computer"]');
    const input = form?.querySelector<HTMLInputElement>('[name="name"]');
    if (!form || !input) throw new Error('The in-place naming form is missing');
    expect([...form.querySelectorAll('input')].map((field) => field.name)).toEqual(['name']);
    input.value = 'Ward computer';
    await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
    expect(posted).toHaveLength(2);
    expect(posted[0].body).toEqual({ operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), name: 'Ward computer', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [SCRIBE], may_run_roles: [], may_reach: [] });
    const computer = (posted[0].body as NameMachine).operation;
    expect(posted[1]).toEqual({ path: '/network/machines/' + computer + '/runner', body: { runner: { kind: 'lys' } } });
    expect(tick(computer).checked).toBe(true);
    expect($('select[name="machine"]')).toBeNull();
    expect($('form[aria-label="Add a computer"]')).toBeNull();
    expect(location.hash).toBe('#/file/' + SCRIBE + '/provisioning');
  });
});
