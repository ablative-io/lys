/** Account registry acts retain operation ids and distinguish record retirement from provider access. */
import { act } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { $, click, choose, mount, settle, text, unmountAll } from './harness';
import { ADA, ME, SERVICE, ok, refused } from './fixtures';
const account = { id: 'op-' + 'a'.repeat(32), owner: ADA, name: 'Calendar account', description: 'Appointments', state: 'active', created_by: ME.signed_in, created_at: 1790000000, retired_by: null, retired_at: null };
const routes = { ...SERVICE, '/service-accounts': ok({ scope: 'personal', service_accounts: [account] }) };
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.textContent === label) ?? null;
async function fillName() {
  const input = $('form[aria-label="Register service account"] input'); if (!(input instanceof HTMLInputElement)) throw new Error('Account name missing');
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(input, 'Invoice account'); input.dispatchEvent(new Event('input', { bubbles: true })); }); await settle();
}
const created = (body: unknown) => { const asked = body as Record<string, unknown>; return ok({ ...account, id: asked.operation, name: asked.name, description: asked.description }); };
beforeEach(() => sessionStorage.clear());
describe('Service-account management', () => {
  it('creates an own account record without requesting credentials or making grants', async () => {
    const { posted } = await mount('#/service-accounts', { ...routes, 'POST /service-accounts': created });
    await fillName(); await click(button('Register account'));
    expect(posted).toEqual([{ path: '/service-accounts', body: { operation: expect.stringMatching(/^op-/), name: 'Invoice account', description: '' } }]); expect(sessionStorage.length).toBe(0);
  });
  it('retains the original creation after a lost reply', async () => {
    const first = await mount('#/service-accounts', { ...routes, 'POST /service-accounts': refused(503, 'ServiceAccountsUnavailable', 'Unknown outcome') });
    await fillName(); await click(button('Register account')); unmountAll(); document.body.innerHTML = '';
    const next = await mount('#/service-accounts', { ...routes, 'POST /service-accounts': created });
    await click(button('Check whether Lys saved it')); expect(next.posted).toEqual(first.posted);
  });
  it('requires confirmation to retire and does not imply the provider was closed', async () => {
    const path = '/service-accounts/' + account.id + '/retire';
    const { posted } = await mount('#/service-accounts', { ...routes, ['POST ' + path]: ok({ ...account, state: 'retired', retired_at: 1790000001, retired_by: ME.signed_in }) });
    await click(button('Retire this service account')); expect(posted).toEqual([]); expect(text()).toContain('does not close its provider account');
    await click(button('Yes, retire ' + account.name)); expect(posted).toEqual([{ path, body: { operation: expect.stringMatching(/^op-/) } }]);
  });
  it('shows an unavailable registry as a refusal instead of an empty list', async () => {
    await mount('#/service-accounts', { ...routes, '/service-accounts': refused(503, 'ServiceAccountsUnavailable', 'No registry configured') });
    expect(text()).toContain('ServiceAccountsUnavailable'); expect(text()).not.toContain('No service-account records were returned');
  });
  it('lets an administrator explicitly name a different owner and checks that owner in the receipt', async () => {
    const other = 'person-' + 'b'.repeat(32);
    const { posted } = await mount('#/service-accounts', { ...routes, '/service-accounts': ok({ scope: 'directory', service_accounts: [] }), '/directory/people': ok({ scope: 'directory', people: [{ ...ME.person, agents: [] }, { ...ME.person, id: other, display_name: 'Other owner', agents: [] }] }), 'POST /service-accounts': (body) => { const asked = body as Record<string, unknown>; return ok({ ...account, id: asked.operation, owner: asked.owner, name: asked.name, description: asked.description }); } });
    await choose($('select'), other); await fillName(); await click(button('Register account')); expect(posted[0].body).toMatchObject({ owner: other, name: 'Invoice account' }); expect(sessionStorage.length).toBe(0);
  });
  it('does not offer another-owner selection in a personal registry', async () => {
    await mount('#/service-accounts', routes); expect(text()).not.toContain('Choose a different owner');
  });

});
