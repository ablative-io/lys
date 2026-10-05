/** Browser mutations use the admitted routes and retain uncertain operations without resending. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, choose, click, mount, pick, settle, text, unmountAll } from './harness';
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

const profile = '/identities/' + ADA + '/profile';
/** The form an act in the head of Ada's file opens: pressed in the head itself, or in the head's one menu once that is opened. */
async function head(act: string, title: string, routes: Record<string, unknown> = SERVICE, at = '#/file/' + ADA) {
  const world = await mount(at, routes as typeof SERVICE);
  if (!$('.file .head [data-act="' + act + '"]')) await click($('.file .head [aria-label="More actions"]'));
  await click($('.file .head [data-act="' + act + '"]'));
  return { ...world, page: form(title) };
}

describe('Directory mutations', () => {
  it('edits the name of the identity whose file it is through the receipt-bearing profile route', async () => {
    const { posted, page } = await head('rename', 'Save name', { ...SERVICE, ['POST ' + profile]: (body: unknown) => recorded(body) });
    expect(page.querySelector<HTMLInputElement>('input[name="display_name"]')?.value).toBe('Ada (test person)');
    expect(page.querySelector('[name="identity"]')).toBeNull();
    await fill(page, 'display_name', 'Ada Updated');
    await submit(page);
    expect(posted).toHaveLength(1);
    expect(posted[0]).toMatchObject({ path: profile, body: { display_name: 'Ada Updated' } });
    expect(text()).toContain('Name saved: Ada Updated.');
    expect(sessionStorage.getItem('lys.pending.change-profile')).toBeNull();
  });

  it('has no separate page of directory controls', async () => {
    await mount('#/directory/manage?action=login', SERVICE);
    expect($('form[aria-label="Bind a sign-in identity"]')).toBeNull();
    expect($('nav[aria-label="Directory actions"]')).toBeNull();
  });

  it('retains an uncertain change across unmount and refuses to submit it again', async () => {
    const routes = { ...SERVICE, ['POST ' + profile]: refused(503, 'StorageUncertain', 'record outcome unknown') };
    const first = await head('rename', 'Save name', routes);
    await fill(first.page, 'display_name', 'New name');
    await submit(first.page);
    await submit(first.page);
    expect(first.posted).toHaveLength(1);
    expect(text()).toContain('StorageUncertain');
    const retained = sessionStorage.getItem('lys.pending.change-profile');
    expect(retained).toContain('New name');
    unmountAll();
    document.body.innerHTML = '';
    const second = await head('rename', 'Save name', routes);
    await submit(second.page);
    expect(second.posted).toHaveLength(0);
    expect(text()).toContain('Do not submit this change again');
  });

  it('retains a successful write whose receipt cannot be read without resending', async () => {
    const { posted, page } = await head('rename', 'Save name', { ...SERVICE, ['POST ' + profile]: { status: 200, body: 'not JSON' } });
    await fill(page, 'display_name', 'New name');
    await submit(page);
    await submit(page);
    expect(posted).toHaveLength(1);
    expect(text()).toContain('UnreadableResponse');
    expect(sessionStorage.getItem('lys.pending.change-profile')).toContain('New name');
  });

  it.each([
    { name: 'empty result', body: {} },
    { name: 'another operation', body: { receipt: { ...RECEIPTS[4].receipt, operation: 'op-unrelated' } } },
    { name: 'incomplete receipt', body: { receipt: { operation: 'op-unrelated' } } },
  ])('retains $name as uncertain and prevents another submission', async ({ body }) => {
    const { posted, page } = await head('rename', 'Save name', { ...SERVICE, ['POST ' + profile]: ok(body) });
    await fill(page, 'display_name', 'New name');
    await submit(page);
    await submit(page);
    expect(posted).toHaveLength(1);
    expect(text()).toContain('UnconfirmedReceipt');
    expect(text()).not.toContain('Name saved');
    expect(sessionStorage.getItem('lys.pending.change-profile')).toContain('New name');
  });

  it('binds the exact provider identity to the person whose Credentials tab it is', async () => {
    const { posted } = await mount('#/file/' + ADA + '/credentials', { ...SERVICE, ['POST /people/' + ADA + '/logins']: (body) => recorded(body) });
    const page = form('Bind a sign-in identity');
    expect(page.querySelector('[name="person"]')).toBeNull();
    await fill(page, 'issuer', 'https://issuer.example/');
    await fill(page, 'subject', 'subject-123');
    await submit(page);
    expect(posted[0]).toMatchObject({ path: '/people/' + ADA + '/logins', body: { issuer: 'https://issuer.example/', subject: 'subject-123' } });
  });

  it('records the transition of the pressed button with its reason', async () => {
    const { posted, page } = await head('suspend', 'Record lifecycle change', { ...SERVICE, ['POST /identities/' + ADA + '/transitions']: (body: unknown) => recorded(body) });
    expect(page.querySelector('select[name="transition"]')).toBeNull();
    await fill(page, 'reason', 'Owner requested suspension');
    await submit(page);
    expect(posted[0]).toMatchObject({ path: '/identities/' + ADA + '/transitions', body: { transition: 'suspend', reason: 'Owner requested suspension' } });
    expect(text()).toContain('Ada (test person) is now suspended.');
    expect(document.querySelector('.file details')).toBeNull();
  });

  it('issues a root grant only with the explicitly selected holder, relation and lifetime', async () => {
    const { posted } = await mount('#/access/issue', { ...SERVICE, 'POST /grants/roots': (body) => recorded(body, true) });
    const page = form('Issue root grant');
    await pick(page, 'Find a person', 'Ada', 'Ada (test person)');
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
  const key = 'lys.pending.change-profile';
  const retained = JSON.stringify({ path: profile, body: { display_name: 'Original name' }, operation });
  async function recover() {
    const button = [...document.querySelectorAll('button')].find((entry) => entry.textContent === 'Check whether Lys saved it');
    if (!button) throw new Error('No recovery button');
    await click(button);
    await settle();
  }
  it('requires an explicit check and reuses exact original operation and body after reload', async () => {
    sessionStorage.setItem(key, retained);
    const { posted } = await head('rename', 'Save name', { ...SERVICE, ['POST ' + profile]: (body: unknown) => recorded(body) });
    expect(posted).toHaveLength(0);
    await recover();
    expect(posted).toEqual([{ path: profile, body: { display_name: 'Original name', operation } }]);
    expect(sessionStorage.getItem(key)).toBeNull();
    expect(text()).toContain('Name saved: Original name.');
  });
  it('keeps the original evidence after a recovery refusal', async () => {
    sessionStorage.setItem(key, retained);
    await head('rename', 'Save name', { ...SERVICE, ['POST ' + profile]: refused(403, 'NotAdmitted', 'permission changed') });
    await recover();
    expect(sessionStorage.getItem(key)).toBe(retained);
    expect(text()).toContain('NotAdmitted');
    expect(text()).toContain('Check whether Lys saved it');
  });
  it.each(['not JSON', JSON.stringify({ path: '/people', body: {}, operation })])('refuses damaged or wrong-form pending evidence without modifying it', async (saved) => {
    sessionStorage.setItem(key, saved);
    const { posted } = await head('rename', 'Save name', SERVICE);
    await submit(form('Save name'));
    expect(posted).toHaveLength(0);
    expect(sessionStorage.getItem(key)).toBe(saved);
    expect(text()).not.toContain('Check whether Lys saved it');
  });
});
