import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { App } from '../src/App';
import { ADA, BEA, COURIER, DIRECTORY, GRANTS, MODEL, RECEIPTS, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { serve } from './harness';

let root: Root | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { if (root) act(() => root?.unmount()); root = null; });
const key = 'lys.add-agent.' + ADA;
const grantId = 'grant-' + 'f'.repeat(32);
const source = GRANTS[0];
const model = { ...MODEL, withheld_from_agents: ['grant'], relations: { ...MODEL.relations, 'only.edit': ['edit'] } };
const editChoice = source.id + ':only.edit';
const viewChoice = source.id + ':viewer';
const ledgerChoice = GRANTS[1].id + ':viewer';

function routes() {
  let given: Record<string, unknown> | null = null;
  const receipt = (body: unknown, change_kind = 2) => ({ ...RECEIPTS[4].receipt, operation: (body as { operation: string }).operation, identity: COURIER, change_kind });
  return { ...SERVICE,
    '/surface-contract': ok({ paths: { '/agents': { post: { requestBody: { content: { 'application/json': { schema: { properties: { answers_to: {} } } } } } } } } }),
    '/grants': ok({ grants: GRANTS, revision: 7 }), '/grants/model': ok(model),
    'POST /agents': (body: unknown) => ok({ agent: COURIER, responsible: ADA, reports_to: { id: (body as { answers_to?: string }).answers_to ?? ADA, kind: (body as { answers_to?: string }).answers_to?.startsWith('agent-') ? 'agent' : 'person' }, receipt: receipt(body) }),
    ['POST /identities/' + COURIER + '/transitions']: (body: unknown) => ok({ receipt: receipt(body, 5) }),
    'POST /grants': (body: unknown) => { given = body as Record<string, unknown>; return ok({ operation: given.operation, grant: grantId, receipt: { caller: ADA } }); },
    ['/grants/' + grantId]: () => ok({ ...source, ...given, id: grantId, issuer: ADA, holder: COURIER, actions: ['edit'] }),
  } satisfies Record<string, Route>;
}
async function open(extra: Record<string, Route> = {}, query = '') {
  const posted: { path: string; body: unknown }[] = [];
  serve({ ...routes(), ...extra }, posted);
  history.replaceState(null, '', '/#/agents/new' + query);
  const container = document.createElement('div'); document.body.appendChild(container); root = createRoot(container);
  await act(async () => { root?.render(<App />); });
  const form = document.querySelector<HTMLFormElement>('form[aria-label="Add an agent"]');
  if (!form) throw new Error('Missing add-agent form');
  return { form, posted };
}
async function choose(value: string) {
  const select = document.querySelector<HTMLSelectElement>('[name="answers_to"]');
  if (!select) throw new Error('Missing reporting choice');
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')?.set?.call(select, value); select.dispatchEvent(new Event('change', { bubbles: true })); });
}
async function tick() {
  const checkbox = document.querySelector<HTMLInputElement>('input[type="checkbox"][value="' + editChoice + '"]');
  if (!checkbox) throw new Error('Missing grant checkbox');
  await act(async () => { checkbox.click(); });
}
async function submit(form: HTMLFormElement) {
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
}

describe('Add-agent reporting and permissions', () => {
  it('reads a grant held by a service account and never offers it', async () => {
    const loader = { ...GRANTS[0], id: 'grant-' + 'e'.repeat(32), holder: 'op-' + 'd'.repeat(32) };
    const { form } = await open({ '/grants': ok({ grants: [...GRANTS, loader], revision: 7 }) });
    expect(form.textContent).not.toContain('GrantsUnreadable');
    expect([...form.querySelectorAll<HTMLInputElement>('input[name="action"]')].map((entry) => entry.value)).toEqual([editChoice, viewChoice, ledgerChoice]);
  });
  it('disables a passable fixture.doc grant and never posts that access for an agent', async () => {
    const app = { ...source, resource: { kind: 'fixture.doc', id: 'doc7' }, relation: 'reader', actions: ['read'],
      pass_on: { kind: 'to', actions: ['read'], recipients: ['agent'] } };
    const { form, posted } = await open({ '/grants': ok({ grants: [app], revision: 7 }) });
    const checkbox = form.querySelector<HTMLInputElement>('input[name="action"]');
    expect(checkbox?.disabled).toBe(true);
    expect(form.textContent).toContain("Lys can't give an agent this app's actions until the app allows it.");
    await act(async () => { checkbox?.click(); });
    expect(posted).toEqual([]);
    await submit(form);
    expect(posted.map((entry) => entry.path)).toEqual(['/agents', '/identities/' + COURIER + '/transitions']);
  });
  it('offers a live named default and only the chosen boss grants, with nothing checked', async () => {
    const { form, posted } = await open();
    expect(form.querySelector<HTMLButtonElement>('button[type="submit"]')?.disabled).toBe(false);
    expect(form.querySelector<HTMLInputElement>('[name="display_name"]')?.value).toBe('New agent');
    expect([...form.querySelectorAll<HTMLInputElement>('input[name="action"]')].map((entry) => entry.value)).toEqual([editChoice, viewChoice, ledgerChoice]);
    expect(form.querySelectorAll('input[type="checkbox"]:checked')).toHaveLength(0);
    expect(form.textContent).toContain('Edit this resource');
    expect(posted).toEqual([]);
  });
  it('uses a served identity name and links an unnamed project without inventing its name', async () => {
    const { form } = await open({ '/grants': ok({ grants: [{ ...source, resource: { kind: 'identity', id: ADA } }, GRANTS[1]], revision: 7 }) });
    expect(form.textContent).toContain('Ada (test person)’s identity');
    expect(form.textContent).toContain('a project, ledger');
    expect(form.querySelector('a[href="#/resources"]')).not.toBeNull();
    expect(form.textContent).not.toContain('identity:' + ADA);
  });
  it('preselects the agent whose page supplied the reporting target', async () => {
    const { form, posted } = await open({}, '?answers_to=' + SCRIBE);
    expect(form.querySelector<HTMLSelectElement>('[name="answers_to"]')?.value).toBe(SCRIBE);
    expect(form.querySelectorAll('input[type="checkbox"]')).toHaveLength(0);
    expect(form.querySelector('a[href="#/file/' + SCRIBE + '/access"]')).not.toBeNull();
    expect(form.textContent).toContain('This form cannot pass on Scribe’s access.');
    await submit(form);
    expect(posted[0].body).toMatchObject({ answers_to: SCRIBE });
    expect(posted.map((entry) => entry.path)).toEqual(['/agents', '/identities/' + COURIER + '/transitions']);
  });
  it('clears checked grants when the reporting target changes', async () => {
    const { form } = await open(); await tick();
    expect(form.querySelectorAll('input[type="checkbox"]:checked')).toHaveLength(1);
    await choose(SCRIBE); await choose(ADA);
    expect(form.querySelectorAll('input[type="checkbox"]:checked')).toHaveLength(0);
    expect(form.querySelector<HTMLInputElement>('input[value="' + ledgerChoice + '"]')?.disabled).toBe(true);
    expect(form.textContent).toContain('This access cannot be passed on.');
  });
  it('delegates only a checked grant after activation and confirms its recorded scope', async () => {
    const { form, posted } = await open(); await tick(); await submit(form);
    expect(posted.map((entry) => entry.path)).toEqual(['/agents', '/identities/' + COURIER + '/transitions', '/grants']);
    expect(posted[2].body).toMatchObject({ route: 'browser', source: source.id, recipient: COURIER, responsible: ADA, resource: source.resource, relation: 'only.edit', pass_on: { kind: 'use_only' } });
    expect((posted[2].body as { window: { ends_at: number } }).window.ends_at).toBe(source.effective_ends_at);
    expect(sessionStorage.getItem(key)).toBeNull();
    expect(location.hash).toBe('#/file/' + COURIER);
  });
  it('keeps and replays the exact delegation after an unknown outcome and remount', async () => {
    const extra = { 'POST /grants': refused(503, 'StorageUncertain', 'The outcome is unknown') };
    const first = await open(extra); await tick(); await submit(first.form);
    expect(first.posted).toHaveLength(3);
    const saved = sessionStorage.getItem(key); expect(saved).not.toBeNull();
    act(() => root?.unmount()); root = null; document.body.innerHTML = '';
    const second = await open(extra); expect(second.posted).toEqual([]);
    expect(second.form.querySelector<HTMLInputElement>('input[value="' + editChoice + '"]')?.checked).toBe(true);
    await submit(second.form);
    expect(second.posted).toEqual([first.posted[2]]);
    expect(sessionStorage.getItem(key)).toBe(saved);
    expect(location.hash).toBe('#/agents/new');
    expect(second.form.textContent).toContain('StorageUncertain');
    const details = second.form.querySelector('details');
    expect(details).not.toBeNull();
    expect(details?.open).toBe(false);
  });
  it('refuses a mismatched grant receipt before claiming completion', async () => {
    const { form, posted } = await open({ 'POST /grants': ok({ operation: 'op-' + '0'.repeat(32), grant: grantId, receipt: { caller: ADA } }) });
    await tick(); await submit(form);
    expect(posted).toHaveLength(3);
    expect(form.textContent).toContain('GrantReceiptMismatch');
    expect(sessionStorage.getItem(key)).not.toBeNull();
    expect(location.hash).toBe('#/agents/new');
  });
  it('refuses a recorded grant that names a different recipient', async () => {
    const { form } = await open({ ['/grants/' + grantId]: ok({ ...source, id: grantId, holder: SCRIBE }) });
    await tick(); await submit(form);
    expect(form.textContent).toContain('GrantReceiptMismatch');
    expect(sessionStorage.getItem(key)).not.toBeNull();
    expect(location.hash).toBe('#/agents/new');
  });
  it.each(['issuer', 'responsible', 'source', 'resource', 'relation', 'window', 'pass_on', 'actions'] as const)('refuses a recorded grant with a different %s', async (field) => {
    let body: Record<string, unknown> = {};
    const changed: Record<string, unknown> = { issuer: BEA, responsible: BEA, source: GRANTS[1].id, resource: { kind: 'project', id: 'other' }, relation: 'owner',
      window: { starts_at: 0, ends_at: null }, pass_on: { kind: 'to', actions: ['view'], recipients: ['agent'] }, actions: ['edit', 'grant', 'view'] };
    const { form, posted } = await open({
      'POST /grants': (given) => { body = given as Record<string, unknown>; return ok({ operation: body.operation, grant: grantId, receipt: { caller: ADA } }); },
      ['/grants/' + grantId]: () => ok({ ...source, ...body, id: grantId, issuer: ADA, holder: COURIER, actions: ['edit'], [field]: changed[field] }),
    });
    await tick(); await submit(form);
    expect(posted).toHaveLength(3);
    expect(form.textContent).toContain('GrantReceiptMismatch');
    expect(sessionStorage.getItem(key)).not.toBeNull();
    expect(location.hash).toBe('#/agents/new');
  });
  it('refuses a damaged saved delegation without issuing another registration', async () => {
    const first = await open({ 'POST /grants': refused(503, 'StorageUncertain', 'The outcome is unknown') }); await tick(); await submit(first.form);
    const saved = JSON.parse(sessionStorage.getItem(key) ?? 'null');
    saved.grants[0].body.recipient = SCRIBE;
    const raw = JSON.stringify(saved); sessionStorage.setItem(key, raw);
    act(() => root?.unmount()); root = null; document.body.innerHTML = '';
    const second = await open(); await submit(second.form);
    expect(second.posted).toEqual([]);
    expect(second.form.textContent).toContain('PendingAgentGrantsUnreadable');
    expect(sessionStorage.getItem(key)).toBe(raw);
    expect(location.hash).toBe('#/agents/new');
  });
  it('confirms the exact immediate reporting edge rather than accepting the accountable person alone', async () => {
    const { form, posted } = await open({ 'POST /agents': (body) => ok({ agent: COURIER, responsible: ADA, reports_to: { id: ADA, kind: 'person' }, receipt: { ...RECEIPTS[4].receipt, identity: COURIER, operation: (body as { operation: string }).operation } }) }, '?answers_to=' + SCRIBE);
    await submit(form);
    expect(posted).toHaveLength(1);
    expect(form.textContent).toContain('UnconfirmedReceipt');
    expect(sessionStorage.getItem(key)).not.toBeNull();
    expect(location.hash).toBe('#/agents/new?answers_to=' + SCRIBE);
  });
  it('keeps adding available when grants cannot be read and grants nothing', async () => {
    const { form, posted } = await open({ '/grants': refused(503, 'GrantStoreUnavailable', 'Access could not be read') });
    expect(form.textContent).toContain('GrantStoreUnavailable');
    expect(form.querySelectorAll('input[type="checkbox"]')).toHaveLength(0);
    await submit(form);
    expect(posted.map((entry) => entry.path)).toEqual(['/agents', '/identities/' + COURIER + '/transitions']);
  });
  it('sends nothing when this browser cannot retain the registration and grant plan', async () => {
    const { form, posted } = await open(); await tick();
    const write = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('Storage is unavailable'); });
    try {
      await submit(form);
      expect(posted).toEqual([]);
      expect(form.textContent).toContain('PendingAgentRetentionFailed');
      expect(sessionStorage.getItem(key)).toBeNull();
      expect(location.hash).toBe('#/agents/new');
    } finally { write.mockRestore(); }
  });
  it('lists inactive identities without offering them as reporting targets', async () => {
    const { form } = await open();
    expect(form.querySelector<HTMLOptionElement>('[name="answers_to"] option[value="' + BEA + '"]')?.disabled).toBe(true);
    expect(form.querySelector<HTMLOptionElement>('[name="answers_to"] option[value="' + COURIER + '"]')?.disabled).toBe(true);
  });
  it('chooses a fresh default name when that reporting target already has New agent', async () => {
    const { form } = await open({ '/directory/people': ok({ ...DIRECTORY, people: [{ ...DIRECTORY.people[0], agents: [...DIRECTORY.people[0].agents, { id: 'agent-' + 'a'.repeat(32), display_name: 'New agent', state: 'active' }] }] }) });
    expect(form.querySelector<HTMLInputElement>('[name="display_name"]')?.value).toBe('New agent 2');
    expect(form.querySelector<HTMLButtonElement>('button[type="submit"]')?.disabled).toBe(false);
  });
});
