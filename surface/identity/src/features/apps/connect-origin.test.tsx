// DIRECTORY-102 R4: Lys knows which apps connected by themselves and from where (connect_origin), so the surface holds no
// app's name: an app that registered itself is listed waiting under the name it gave, and "given its credentials by
// connecting" is keyed on connect_origin, never on a name.
import { expect, it } from 'vitest';
import { mount, settle } from '../../../tests/harness';
import { SERVICE, ok } from '../../../tests/fixtures';

const record = (id: string, name: string, state: string, connect_origin: string | null, registered_by: unknown) => ({ id, name, state, redirects: ['https://' + id + '.example.test/callback'], sign_in: null,
  schema: { kinds: {} }, version: state === 'pending' ? 0 : 1, versions: state === 'pending' ? [] : [1], pending: null, client_id: state === 'pending' ? null : id, service_account: null,
  client_credentials: [], registered_by, registered_at: 1, connect_origin });
const SELF = { kind: 'service_account', id: '0b5e7a1c-0000-4000-8000-000000000001' };

it('lists an app that registered itself as waiting, under its own name', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [record('quillwork', 'Quillwork', 'pending', null, SELF)], public_address: null, public_address_reason: 'no address' }) });
  await settle();
  expect([...document.querySelectorAll('nav[aria-label="Apps"] button')].map((entry) => entry.textContent)).toEqual(['Quillwork']);
  const card = document.querySelector('[aria-label="App quillwork"]');
  expect(card?.querySelector('h3')?.textContent).toBe('Quillwork Waiting for your approval');
  expect(card?.textContent).toContain('Registered by a service account.');
});

it('says an app that connected is given its credentials by connecting, whatever its name', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [record('quillwork', 'Quillwork', 'approved', 'https://quillwork.example.test', SELF)], public_address: null, public_address_reason: 'no address' }) });
  await settle();
  const section = document.querySelector('[aria-label="Client credentials of quillwork"]');
  expect(section?.textContent).toContain('Quillwork is given its credentials by connecting');
  expect(section?.textContent).toContain('use Connect again on this card');
  expect(document.querySelector('button[aria-label="Issue a client credential for Quillwork"]')).toBeNull();
});

it('offers a hand-issued credential to an app registered by hand, even one named Cambium', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [record('cambium', 'Cambium', 'approved', null, { kind: 'start' })], public_address: null, public_address_reason: 'no address' }) });
  await settle();
  expect(document.querySelector('[aria-label="Client credentials of cambium"]')?.textContent).not.toContain('given its credentials by connecting');
  expect(document.querySelector('button[aria-label="Issue a client credential for Cambium"]')).not.toBeNull();
});
