// Domain 04 item 6 and DIRECTORY-102 R4: an app that connected (Lys answers its connect_origin) is given its credentials by
// connecting, so its card offers no hand-issued secret, only the way back to Connect again; any other app's issued secret is
// shown beside its client id, the pair an OpenID Connect client is configured with. Which is which is the service's answer.
import { expect, it } from 'vitest';
import { click, mount, settle } from '../../../tests/harness';
import { SERVICE, ok } from '../../../tests/fixtures';

const admin = { kind: 'operator', login: { provider: 'https://issuer.test', subject: 'administrator' } };
const live = { credential_id: '0a1b2c3d4e5f6a7b', issued_by: admin, issued_at: 1_700_000_000, revoked_by: null, revoked_at: null, revoked_reason: null, ended_by_retirement: false, live: true, ended_at_broker: false };
const app = (id: string, name: string, client_credentials: unknown[], connect_origin: string | null = null) => ({ id, name, state: 'approved', redirects: [], sign_in: null, schema: { kinds: {} }, version: 1, versions: [1], pending: null,
  client_id: id + '-client', service_account: null, client_credentials, registered_by: { kind: 'start' }, registered_at: 1, connect_origin });
const CONNECTED = 'https://cambium.example.test';
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.getAttribute('aria-label') === label) ?? null;
const section = (id: string) => document.querySelector('[aria-label="Client credentials of ' + id + '"]');

it('offers a connected Cambium no hand-issued secret, says connecting gives it its credentials, and its credentials can still be taken back', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [app('cambium', 'Cambium', [live], CONNECTED)] }) });
  await settle();
  expect(button('Issue a client credential for Cambium')).toBeNull();
  expect(section('cambium')?.textContent).toContain('Cambium is given its credentials by connecting');
  expect(section('cambium')?.textContent).toContain('use Connect again on this card');
  expect(button('Take back credential ' + live.credential_id)).not.toBeNull();
});

it('says connecting gives Cambium a credential when Lys lists none, never that one must be issued', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [app('cambium', 'Cambium', [], CONNECTED)] }) });
  await settle();
  expect(section('cambium')?.textContent).toContain('Lys lists no credential for Cambium yet: connecting gives it one.');
  expect(section('cambium')?.textContent).not.toContain('cannot sign anyone in yet');
});

it('shows another app’s issued secret beside its client id, each with its own copy', async () => {
  const value = 'lys-client.fixture_notes.' + 'c'.repeat(64);
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [app('fixture_notes', 'Notes fixture', [])] }), '/apps/fixture_notes': () => ok(app('fixture_notes', 'Notes fixture', [live])),
    'POST /apps/fixture_notes/credentials/issue': () => ok({ app: 'fixture_notes', credential_id: live.credential_id, credential: value }) });
  await settle();
  await click(button('Issue a client credential for Notes fixture'));
  expect(document.querySelector('[data-credential="client-id"]')?.textContent).toBe('fixture_notes-client');
  expect(document.querySelector('[data-credential="value"]')?.textContent).toBe(value);
  expect(section('fixture_notes')?.textContent).toContain('configured as an OpenID Connect client of Lys');
  expect(button('Copy the client id')).not.toBeNull();
  expect(button('Copy the credential')).not.toBeNull();
  // A second secret is not offered over the one being read; Done brings Issue back.
  expect(button('Issue a client credential for Notes fixture')).toBeNull();
  await click(button('Done with the credential'));
  expect(button('Issue a client credential for Notes fixture')).not.toBeNull();
});
