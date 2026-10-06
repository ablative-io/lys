// An approved app's client credentials on the Apps screen (DIRECTORY-081): issued and shown once with a way to copy
// it, gone once the administrator is done or leaves; listed with who issued and revoked each; revoked only after the
// screen asks; every refusal in Lys's words. Not opened in a browser: these run under jsdom.
import { afterEach, expect, it, vi } from 'vitest';
import { click, mount, text, type, unmountAll } from './harness';
import { SERVICE, ok, refused } from './fixtures';

const APP = 'fixture_notes';
const BACK = 'https://notes.example.test/signed-in';
const VALUE = 'lys-client.' + APP + '.' + 'c'.repeat(64);
const ID = '0a1b2c3d4e5f6a7b';
const admin = { kind: 'operator', login: { provider: 'https://issuer.test', subject: 'administrator' } };
const live = { credential_id: ID, issued_by: admin, issued_at: 1_700_000_000, revoked_by: null, revoked_at: null, revoked_reason: null, ended_by_retirement: false, live: true, ended_at_broker: false };
const revoked = { ...live, revoked_by: admin, revoked_at: 1_700_000_100, revoked_reason: 'rotated', live: false, ended_at_broker: true };
const app = (client_credentials: unknown[]) => ({ id: APP, name: 'Notes fixture', state: 'approved', redirects: [BACK], sign_in: { redirects: [BACK], profile: false, operation: 'op-approve', by: { kind: 'start' }, at: 2 }, schema: { kinds: {} }, version: 1, versions: [1], pending: null, client_id: APP, service_account: null, client_credentials, registered_by: { kind: 'start' }, registered_at: 1 });

const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === label) ?? null;
const alert = () => document.querySelector('[role="alert"]')?.textContent ?? '';

/** A service holding the app with `held` credentials, issuing one more on asking. */
function routes(held: unknown[] = []) {
  let current = held;
  return {
    ...SERVICE,
    '/apps': () => ok({ apps: [app(current)] }),
    ['/apps/' + APP]: () => ok(app(current)),
    ['POST /apps/' + APP + '/credentials/issue']: () => { current = [...current, live]; return ok({ app: APP, credential_id: ID, credential: VALUE }); },
    ['POST /apps/' + APP + '/credentials/' + ID + '/revoke']: () => { current = [revoked]; return ok(app(current)); },
  };
}

afterEach(() => { vi.unstubAllGlobals(); });

it('an issued credential is shown once with a copy control, listed without its value, and gone when done', async () => {
  const writeText = vi.fn(async () => undefined);
  vi.stubGlobal('navigator', { ...navigator, clipboard: { writeText } });
  const { posted } = await mount('#/apps', routes());
  expect(text()).toContain('cannot sign anyone in yet');
  await click(button('Issue a client credential for Notes fixture'));
  expect(posted).toEqual([{ path: '/apps/' + APP + '/credentials/issue', body: { operation: expect.stringMatching(/^op-/) } }]);
  expect(text()).toContain(VALUE);
  expect(text()).toContain('It is shown once, here, and Lys keeps no copy');
  await click(button('Copy the credential'));
  expect(writeText).toHaveBeenCalledWith(VALUE);
  expect(text()).toContain('The credential is copied.');
  const row = document.querySelector('[data-credential="' + ID + '"]')?.textContent ?? '';
  expect(row).toContain('In use');
  expect(row).not.toContain(VALUE);
  await click(button('Done with the credential'));
  expect(document.body.innerHTML).not.toContain(VALUE);
  expect(JSON.stringify(Object.entries(sessionStorage))).not.toContain(VALUE);
  expect(JSON.stringify(Object.entries(localStorage))).not.toContain(VALUE);
});

it('leaving the screen leaves no credential value behind', async () => {
  const served = routes();
  await mount('#/apps', served);
  await click(button('Issue a client credential for Notes fixture'));
  expect(text()).toContain(VALUE);
  unmountAll();
  await mount('#/apps', served);
  expect(document.body.innerHTML).not.toContain(VALUE);
  expect(document.querySelector('[data-credential="' + ID + '"]')).not.toBeNull();
});

it('revoking asks first, sends the reason, and lists who revoked it and why', async () => {
  const { posted } = await mount('#/apps', routes([live]));
  await click(button('Revoke credential ' + ID));
  expect(posted).toHaveLength(0);
  expect(text()).toContain('This cannot be undone');
  await click(button('Keep the credential'));
  expect(posted).toHaveLength(0);
  expect(button('Revoke credential ' + ID + ' now')).toBeNull();
  await click(button('Revoke credential ' + ID));
  await type(document.querySelector('[aria-label="Revoke credential ' + ID + '?"] input'), 'rotated');
  await click(button('Revoke credential ' + ID + ' now'));
  expect(posted).toEqual([{ path: '/apps/' + APP + '/credentials/' + ID + '/revoke', body: { operation: expect.stringMatching(/^op-/), reason: 'rotated' } }]);
  const row = document.querySelector('[data-credential="' + ID + '"]')?.textContent ?? '';
  expect(row).toContain('Revoked by');
  expect(row).toContain('rotated');
  expect(button('Revoke credential ' + ID)).toBeNull();
});

it('a refused issue and a refused revocation are said in Lys’s words, and nothing is shown as given', async () => {
  await mount('#/apps', { ...routes([live]),
    ['POST /apps/' + APP + '/credentials/issue']: refused(502, 'SecretsUnavailable', 'the secrets broker could not be reached'),
    ['POST /apps/' + APP + '/credentials/' + ID + '/revoke']: refused(401, 'credential_refused', 'no live client credential of this app has that id') });
  await click(button('Issue a client credential for Notes fixture'));
  expect(alert()).toContain('SecretsUnavailable');
  expect(alert()).toContain('could not be reached');
  expect(text()).not.toContain('It is shown once, here');
  await click(button('Revoke credential ' + ID));
  await click(button('Revoke credential ' + ID + ' now'));
  expect(alert()).toContain('credential_refused');
  expect(alert()).toContain('no live client credential');
});

it('an issue answer for another app is refused in words and shows no value', async () => {
  await mount('#/apps', { ...routes(), ['POST /apps/' + APP + '/credentials/issue']: ok({ app: 'another_app', credential_id: ID, credential: VALUE }) });
  await click(button('Issue a client credential for Notes fixture'));
  expect(alert()).toContain('did not give a credential');
  expect(document.body.innerHTML).not.toContain(VALUE);
});

it('a retired app lists the credentials its retirement ended, offering neither issue nor revoke', async () => {
  const ended = { ...live, live: false, ended_by_retirement: true };
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [{ ...app([ended]), state: 'retired' }] }) });
  const row = document.querySelector('[data-credential="' + ID + '"]')?.textContent ?? '';
  expect(row).toContain('Ended when the app was retired.');
  expect(row).toContain('Lys secrets is told to forget it');
  expect(button('Revoke credential ' + ID)).toBeNull();
  expect(button('Issue a client credential for Notes fixture')).toBeNull();
});
