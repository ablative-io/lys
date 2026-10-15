// Connect an app: the Apps page's known-app card, the collapsed developer section, and #/apps/connect.
// Not opened in a browser: these run under jsdom.
import { afterEach, expect, it, vi } from 'vitest';
import { click, mount, settle, text, type } from './harness';
import { SERVICE, ok } from './fixtures';
import { declinedAddress } from '../src/features/apps/ConnectApp';

const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === label) ?? null;
const ORIGIN = 'https://cambium.example.test';
const MANIFEST = ORIGIN + '/identity/lys/manifest';
const BACK = ORIGIN + '/identity/lys/connected';
const manifest = { id: 'cambium', name: 'Cambium', redirects: [ORIGIN + '/identity/oidc/callback'], post_logout_redirects: [ORIGIN + '/'], profile: true,
  schema: { kinds: { 'cambium.place': { actions: ['read'], relations: { member: ['read'] }, parents: [] } } } };

afterEach(() => { vi.unstubAllGlobals(); });

it('shows Cambium as a known app with its state and a Connect button that asks for its address once', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [] }) });
  const card = document.querySelector('[aria-label="Known apps"]');
  expect(card?.textContent).toContain('Cambium');
  expect(card?.textContent).toContain('Not connected');
  await click(button('Connect Cambium'));
  const field = document.querySelector<HTMLInputElement>('input[aria-label="Cambium web address"]');
  expect(field?.placeholder).toBe('https://cambium.example.com');
  expect(document.querySelectorAll('[aria-label="Known app cambium"] input')).toHaveLength(1);
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
  const routes = { ...SERVICE, ['POST /apps/connect']: () => ok({ redirect: BACK + '?code=c0de&state=s1' }), [MANIFEST]: () => ok(manifest) };
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
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [] }) });
  vi.stubGlobal('location', { ...window.location, assign, origin: window.location.origin });
  await click(button('Connect Cambium'));
  const row = document.querySelector('[aria-label="Known app cambium"]');
  expect(row?.textContent).toContain('Lys sends your browser to Cambium, which asks to connect and brings you back here to approve what it asks for.');
  expect(button('Continue to Cambium')?.hasAttribute('disabled')).toBe(true);
  expect(row?.textContent).toContain('Type Cambium’s web address to continue.');
  const field = document.querySelector<HTMLInputElement>('input[aria-label="Cambium web address"]');
  await type(field, 'cambium.example');
  expect(button('Continue to Cambium')?.hasAttribute('disabled')).toBe(true);
  expect(row?.textContent).toContain('That is not a whole web address. Type it as it appears in the browser, starting with https://.');
  await type(field, ORIGIN + '/some/page');
  expect(button('Continue to Cambium')?.hasAttribute('disabled')).toBe(false);
  expect(row?.textContent).not.toContain('to continue.');
  await click(button('Continue to Cambium'));
  expect(assign).toHaveBeenCalledWith(ORIGIN + '/identity/lys/connect?lys=' + encodeURIComponent(window.location.origin));
  await click(button('Cancel connecting Cambium'));
  expect(document.querySelector('input[aria-label="Cambium web address"]')).toBeNull();
  expect(button('Connect Cambium')).not.toBeNull();
});

it('declines by sending the browser back to the app with error=access_denied and its state, so the app is told', async () => {
  const assign = vi.fn();
  const hash = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(BACK) + '&state=s1';
  const { posted } = await mount(hash, { ...SERVICE, [MANIFEST]: () => ok(manifest) });
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
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('This link is incomplete. Start again from Cambium’s Connect to Lys.');
});

it('names the address Lys could not reach and how to start again', async () => {
  const hash = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(BACK) + '&state=s1';
  // The browser's fetch rejects when nothing answers at the address.
  await mount(hash, { ...SERVICE, [MANIFEST]: () => { throw new TypeError('Failed to fetch'); } });
  await settle();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('Lys could not reach Cambium at ' + ORIGIN + '. Check that Cambium is running and the address is right, then start again from Cambium.');
});

it('says a refused manifest by its status, not as an unreachable app', async () => {
  const hash = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(BACK) + '&state=s1';
  await mount(hash, { ...SERVICE });
  await settle();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('Lys reached ' + ORIGIN + ', but it answered 404 for the manifest, so it is not Cambium as Lys knows it. Check that the address is Cambium’s, then start again from Cambium.');
});
