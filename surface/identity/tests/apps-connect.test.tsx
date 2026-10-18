// Connect an app: the Apps page's Connect card, the collapsed developer section, and #/apps/connect, whose manifest Lys's
// server reads (DIRECTORY-102 R3).
// Not opened in a browser: these run under jsdom.
import { afterEach, expect, it, vi } from 'vitest';
import { click, mount, settle, text, type } from './harness';
import { SERVICE, ok } from './fixtures';
import { declinedAddress } from '../src/features/apps/ConnectApp';

const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === label) ?? null;
const ORIGIN = 'https://cambium.example.test';
const MANIFEST = ORIGIN + '/identity/lys/manifest';
const BACK = ORIGIN + '/identity/lys/connected';
const READ = '/apps/connect/manifest?address=' + encodeURIComponent(MANIFEST);
const PUBLIC = 'https://lys.example.test';
const manifest = { id: 'cambium', name: 'Cambium', redirects: [ORIGIN + '/identity/oidc/callback'], post_logout_redirects: [ORIGIN + '/'], profile: true,
  schema: { kinds: { 'cambium.place': { actions: ['read'], relations: { member: ['read'] }, parents: [] } } } };

afterEach(() => { vi.unstubAllGlobals(); });

it('offers Another app on the Connect card, asking for its address once', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [], public_address: PUBLIC, public_address_reason: null }) });
  const card = document.querySelector('[aria-label="Connect an app"]');
  expect(card?.textContent).toContain('Another app');
  await click(button('Connect another app'));
  const field = document.querySelector<HTMLInputElement>('input[aria-label="The app web address"]');
  expect(field?.placeholder).toBe('https://app.example.com');
  expect(document.querySelectorAll('[aria-label="Another app"] input')).toHaveLength(1);
});

it('keeps the free-text registration in a collapsed developer section, with single-line return addresses', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [] }) });
  const section = document.querySelector<HTMLDetailsElement>('details[aria-label="Add your own app (for developers)"]');
  expect(section).not.toBeNull();
  expect(section?.open).toBe(false);
  expect(section?.querySelector('[aria-label="Register an app"]')).not.toBeNull();
  expect(section?.querySelector('textarea')).toBeNull();
  expect(section?.querySelectorAll('input[aria-label^="Return address"]')).toHaveLength(1);
  await click(button('Add another address'));
  expect(section?.querySelectorAll('input[aria-label^="Return address"]')).toHaveLength(2);
});

it('approves a fetched manifest in one act and sends the browser back with the code', async () => {
  const assign = vi.fn();
  const routes = { ...SERVICE, ['POST /apps/connect']: () => ok({ redirect: BACK + '?code=c0de&state=s1' }), [READ]: () => ok({ manifest, origin: ORIGIN }) };
  const hash = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(BACK) + '&state=s1';
  const { posted } = await mount(hash, routes);
  vi.stubGlobal('location', { ...window.location, assign, origin: window.location.origin });
  await settle();
  expect(text()).toContain('Cambium');
  expect(text()).toContain(ORIGIN);
  expect(document.querySelector('[aria-label="Permissions of cambium"]')).not.toBeNull();
  await click(button('Approve Cambium'));
  expect(posted).toEqual([{ path: '/apps/connect', body: { operation: expect.stringMatching(/^op-/), manifest, return: BACK, state: 's1' } }]);
  expect(assign).toHaveBeenCalledWith(BACK + '?code=c0de&state=s1');
  expect(button('Decline Cambium')).not.toBeNull();
});

it('asks for the whole address with a reason while Continue cannot go, says where the browser goes, and cancels back to Connect', async () => {
  const assign = vi.fn();
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [], public_address: PUBLIC, public_address_reason: null }) });
  vi.stubGlobal('location', { ...window.location, assign, origin: window.location.origin });
  await click(button('Connect another app'));
  const row = document.querySelector('[aria-label="Another app"]');
  expect(row?.textContent).toContain('Lys sends your browser to the app, which asks to connect and brings you back here to approve what it asks for.');
  expect(button('Continue to the app')?.hasAttribute('disabled')).toBe(true);
  expect(row?.textContent).toContain('Type the app’s web address to continue.');
  const field = document.querySelector<HTMLInputElement>('input[aria-label="The app web address"]');
  await type(field, 'cambium.example');
  expect(button('Continue to the app')?.hasAttribute('disabled')).toBe(true);
  expect(row?.textContent).toContain('That is not a whole web address. Type it as it appears in the browser, starting with https://.');
  await type(field, ORIGIN + '/some/page');
  expect(button('Continue to the app')?.hasAttribute('disabled')).toBe(false);
  expect(row?.textContent).not.toContain('to continue.');
  await click(button('Continue to the app'));
  expect(assign).toHaveBeenCalledWith(ORIGIN + '/identity/lys/connect?lys=' + encodeURIComponent(PUBLIC));
  await click(button('Cancel connecting the app'));
  expect(document.querySelector('input[aria-label="The app web address"]')).toBeNull();
  expect(button('Connect another app')).not.toBeNull();
});

it('declines by sending the browser back to the app with error=access_denied and its state, so the app is told', async () => {
  const assign = vi.fn();
  const hash = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(BACK) + '&state=s1';
  const { posted } = await mount(hash, { ...SERVICE, [READ]: () => ok({ manifest, origin: ORIGIN }) });
  vi.stubGlobal('location', { ...window.location, assign, origin: window.location.origin });
  await settle();
  await click(button('Decline Cambium'));
  expect(assign).toHaveBeenCalledWith(BACK + '?error=access_denied&state=s1');
  expect(posted).toEqual([]);
});

it('never follows a decline to a return address off the manifest’s own origin', () => {
  expect(declinedAddress('https://elsewhere.example.test/x', MANIFEST, 's1')).toBeNull();
  expect(declinedAddress('not an address', MANIFEST, 's1')).toBeNull();
  expect(declinedAddress(BACK + '?keep=1', MANIFEST, 's 2')).toBe(BACK + '?keep=1&error=access_denied&state=s+2');
});

it('says what to do next when the link is incomplete or Cambium cannot be reached', async () => {
  await mount('#/apps/connect?manifest=' + encodeURIComponent(MANIFEST), { ...SERVICE });
  await settle();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('This link is incomplete. Start again from the app’s Connect to Lys.');
});

it('names the address Lys could not reach and how to start again', async () => {
  const hash = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(BACK) + '&state=s1';
  // Lys's server read the manifest, so nothing answering is said as itself, never as a browser rule.
  await mount(hash, { ...SERVICE, [READ]: () => ({ status: 502, body: { refusal: 'manifest_not_answering', reason: 'Lys could not reach ' + ORIGIN, error: 'dns error: no such host' } }) });
  await settle();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('Lys could not reach the app at ' + ORIGIN + ': dns error: no such host. Check that the app is running and answers at that address, then start again from the app.');
});

it('says a refused manifest by its status, not as an unreachable app', async () => {
  const hash = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(BACK) + '&state=s1';
  await mount(hash, { ...SERVICE, [READ]: () => ({ status: 502, body: { refusal: 'manifest_answered_status', reason: 'the app answered 404', status: 404 } }) });
  await settle();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('The app at ' + ORIGIN + ' answered 404 for its manifest, so Lys could not read it. Check that the address is the app’s and that the app is running fully, then start again from the app.');
});
