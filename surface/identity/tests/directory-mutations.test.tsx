/** Browser mutations use the admitted routes and retain uncertain operations without resending. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, choose, click, mount, settle, text, unmountAll } from './harness';
import { ADA, RECEIPTS, SERVICE, ok, refused } from './fixtures';

beforeEach(() => sessionStorage.clear());

async function fill(form: Element, name: string, value: string) {
  const input = form.querySelector<HTMLInputElement>(`input[name="${name}"]`);
  if (!input) throw new Error(`No ${name} input`);
  input.value = value;
}

async function submit(form: Element) {
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

function form(title: string) {
  const value = $(`form[aria-label="${title}"]`);
  if (!value) throw new Error(`No ${title} form`);
  return value;
}

function recorded(body: unknown, grant = false) {
  if (!body || typeof body !== 'object' || !('operation' in body) || typeof body.operation !== 'string') throw new Error('Request has no operation');
  const receipt = { ...RECEIPTS[4].receipt, operation: body.operation, identity: ADA };
  return ok(grant
    ? { operation: body.operation, grant: 'grant-recorded', index: receipt.log.index, receipt: { ...receipt, caller: ADA, revision: 1 } }
    : { person: ADA, receipt });
}

describe('Directory mutations', () => {
  it('edits the selected identity name through the receipt-bearing profile route', async () => {
    const { posted } = await mount('#/directory/manage?action=profile&identity=' + ADA, { ...SERVICE, ['POST /identities/' + ADA + '/profile']: (body) => recorded(body) });
    const page = form('Save name');
    expect(page.querySelector<HTMLInputElement>('input[name="display_name"]')?.value).toBe('Ada (test person)');
    await fill(page, 'display_name', 'Ada Updated');
    await submit(page);
    expect(posted).toHaveLength(1);
    expect(posted[0]).toMatchObject({ path: '/identities/' + ADA + '/profile', body: { display_name: 'Ada Updated' } });
    expect(text()).toContain('Recorded receipt');
    expect(sessionStorage.getItem('lys.pending.change-profile')).toBeNull();
  });

  it('shows one action at a time and opens a linked action directly', async () => {
    await mount('#/directory/manage?action=login', SERVICE);
    expect(document.querySelectorAll('form')).toHaveLength(1);
    expect(form('Bind a sign-in identity')).toBeTruthy();
    expect($('nav[aria-label="Directory actions"] a[aria-current="page"]')?.textContent).toBe('Connect sign-in');
    expect(text()).not.toContain('Every change is admitted');
  });

  it('registers a person and displays the returned receipt without silently activating them', async () => {
    const { posted } = await mount('#/directory/manage', { ...SERVICE, 'POST /people': (body) => recorded(body) });
    const page = form('Register a person');
    await fill(page, 'display_name', 'New person');
    await submit(page);
    expect(posted).toHaveLength(1);
    expect(posted[0]).toEqual({ path: '/people', body: { display_name: 'New person', operation: expect.stringMatching(/^op-[0-9a-f]{32}$/) } });
    expect(text()).toContain('Recorded receipt');
    expect(text()).not.toContain('UnconfirmedReceipt');
    expect(sessionStorage.getItem('lys.pending.register-person')).toBeNull();
  });

  it('retains an uncertain change across unmount and refuses to submit it again', async () => {
    const routes = { ...SERVICE, 'POST /agents': refused(503, 'StorageUncertain', 'record outcome unknown') };
    const first = await mount('#/directory/manage?action=agent', routes);
    const page = form('Register an agent');
    await fill(page, 'display_name', 'New agent');
    await submit(page);
    await submit(page);
    expect(first.posted).toHaveLength(1);
    expect(text()).toContain('StorageUncertain');
    const retained = sessionStorage.getItem('lys.pending.register-agent');
    expect(retained).toContain('New agent');
    unmountAll();
    document.body.innerHTML = '';
    const second = await mount('#/directory/manage?action=agent', routes);
    await submit(form('Register an agent'));
    expect(second.posted).toHaveLength(0);
    expect(text()).toContain('Do not submit this change again');
  });

  it('retains a successful write whose receipt cannot be read without resending', async () => {
    const { posted } = await mount('#/directory/manage?action=agent', { ...SERVICE, 'POST /agents': { status: 200, body: 'not JSON' } });
    const page = form('Register an agent');
    await fill(page, 'display_name', 'New agent');
    await submit(page);
    await submit(page);
    expect(posted).toHaveLength(1);
    expect(text()).toContain('UnreadableResponse');
    expect(sessionStorage.getItem('lys.pending.register-agent')).toContain('New agent');
  });

  it.each([
    { name: 'empty result', body: {} },
    { name: 'another operation', body: { receipt: { ...RECEIPTS[4].receipt, operation: 'op-unrelated' } } },
    { name: 'incomplete receipt', body: { receipt: { operation: 'op-unrelated' } } },
  ])('retains $name as uncertain and prevents another submission', async ({ body }) => {
    const { posted } = await mount('#/directory/manage', { ...SERVICE, 'POST /people': ok(body) });
    const page = form('Register a person');
    await fill(page, 'display_name', 'New person');
    await submit(page);
    await submit(page);
    expect(posted).toHaveLength(1);
    expect(text()).toContain('UnconfirmedReceipt');
    expect(text()).not.toContain('Recorded receipt');
    expect(sessionStorage.getItem('lys.pending.register-person')).toContain('New person');
  });

  it('does not show administrator forms to a personal reader', async () => {
    await mount('#/directory/manage', { ...SERVICE, '/directory/people': refused(403, 'NotAdmitted', 'administrator only') });
    expect($('form')).toBeNull();
    expect(text()).toContain('Only the configured directory administrator');
  });

  it('binds the exact provider identity to the selected person', async () => {
    const { posted } = await mount('#/directory/manage?action=login', { ...SERVICE, ['POST /people/' + ADA + '/logins']: (body) => recorded(body) });
    const page = form('Bind a sign-in identity');
    await choose(page.querySelector('select[name="person"]'), ADA);
    await fill(page, 'issuer', 'https://issuer.example/');
    await fill(page, 'subject', 'subject-123');
    await submit(page);
    expect(posted[0]).toMatchObject({ path: '/people/' + ADA + '/logins', body: { issuer: 'https://issuer.example/', subject: 'subject-123' } });
  });

  it('records the selected transition with its reason', async () => {
    const { posted } = await mount('#/directory/manage?action=status', { ...SERVICE, ['POST /identities/' + ADA + '/transitions']: (body) => recorded(body) });
    const page = form('Record lifecycle change');
    await choose(page.querySelector('select[name="identity"]'), ADA);
    await choose(page.querySelector('select[name="transition"]'), 'suspend');
    await fill(page, 'reason', 'Owner requested suspension');
    await submit(page);
    expect(posted[0]).toMatchObject({ path: '/identities/' + ADA + '/transitions', body: { transition: 'suspend', reason: 'Owner requested suspension' } });
    expect(text()).toContain('Ada (test person) is now suspended.');
    expect(page.querySelector('details')?.hasAttribute('open')).toBe(false);
  });

  it('issues a root grant only with the explicitly selected holder, relation and lifetime', async () => {
    const { posted } = await mount('#/access/issue', { ...SERVICE, 'POST /grants/roots': (body) => recorded(body, true) });
    const page = form('Issue root grant');
    await choose(page.querySelector('select[name="holder"]'), ADA);
    await choose(page.querySelector('select[name="relation"]'), 'viewer');
    await fill(page, 'kind', 'project');
    await fill(page, 'resource', 'integration-check');
    const checks = page.querySelectorAll('input[type="checkbox"]');
    await click(checks[1]);
    await submit(page);
    expect(posted).toHaveLength(1);
    expect(posted[0]).toMatchObject({ path: '/grants/roots', body: { route: 'browser', holder: ADA, relation: 'viewer', resource: { kind: 'project', id: 'integration-check' }, pass_on: { kind: 'use_only' }, window: { starts_at: expect.any(Number), ends_at: null } } });
    expect(text()).toContain('grant-recorded');
  });
});


describe('Uncertain directory change recovery', () => {
  const operation = 'op-' + 'a'.repeat(32);
  const key = 'lys.pending.register-agent';
  const retained = JSON.stringify({ path: '/agents', body: { display_name: 'Original agent' }, operation });
  async function recover() {
    const button = [...document.querySelectorAll('button')].find((entry) => entry.textContent === 'Check whether Lys saved it');
    if (!button) throw new Error('No recovery button');
    await click(button);
    await settle();
  }
  it('requires an explicit check and reuses exact original operation and body after reload', async () => {
    sessionStorage.setItem(key, retained);
    const { posted } = await mount('#/directory/manage?action=agent', { ...SERVICE, 'POST /agents': (body) => recorded(body) });
    expect(posted).toHaveLength(0);
    await recover();
    expect(posted).toEqual([{ path: '/agents', body: { display_name: 'Original agent', operation } }]);
    expect(sessionStorage.getItem(key)).toBeNull();
    expect(text()).toContain('Recorded receipt');
  });
  it('keeps the original evidence after a recovery refusal', async () => {
    sessionStorage.setItem(key, retained);
    await mount('#/directory/manage?action=agent', { ...SERVICE, 'POST /agents': refused(403, 'NotAdmitted', 'permission changed') });
    await recover();
    expect(sessionStorage.getItem(key)).toBe(retained);
    expect(text()).toContain('NotAdmitted');
    expect(text()).toContain('Check whether Lys saved it');
  });
  it.each(['not JSON', JSON.stringify({ path: '/people', body: {}, operation })])('refuses damaged or wrong-form pending evidence without modifying it', async (saved) => {
    sessionStorage.setItem(key, saved);
    const { posted } = await mount('#/directory/manage?action=agent', SERVICE);
    await submit(form('Register an agent'));
    expect(posted).toHaveLength(0);
    expect(sessionStorage.getItem(key)).toBe(saved);
    expect(text()).not.toContain('Check whether Lys saved it');
  });
});
