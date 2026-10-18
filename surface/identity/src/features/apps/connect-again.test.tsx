// Domain 04 item 1: connecting an already connected app says, before it is done, that the running app's credentials are replaced.
// The second party is the service's own answer: the words follow the state /apps returns, never a value the page substituted.
import { afterEach, expect, it, vi } from 'vitest';
import { click, mount, settle } from '../../../tests/harness';
import { SERVICE, ok, refused } from '../../../tests/fixtures';

const ORIGIN = 'https://cambium.example.test';
const MANIFEST = ORIGIN + '/identity/lys/manifest';
const BACK = ORIGIN + '/identity/lys/connected';
const HASH = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(BACK) + '&state=s1';
const manifest = { id: 'cambium', name: 'Cambium', redirects: [ORIGIN + '/identity/oidc/callback'], profile: true, schema: { kinds: {} } };
const cambium = (state: string) => ({ id: 'cambium', name: 'Cambium', state, redirects: [], sign_in: null, schema: { kinds: {} }, version: 1, versions: [1], pending: null,
  client_id: 'cambium', service_account: null, client_credentials: [], registered_by: { kind: 'start' }, registered_at: 1 });
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.getAttribute('aria-label') === label) ?? null;
const warning = () => document.querySelector('.app-replace');

afterEach(() => { vi.unstubAllGlobals(); });

it('rests a connected Cambium with no primary act, and says what connecting again ends before the address is asked for', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [cambium('approved')] }) });
  const again = button('Connect Cambium');
  expect(again?.textContent).toBe('Connect again');
  expect(again?.classList.contains('primary')).toBe(false);
  expect(warning()).toBeNull();
  await click(again);
  expect(warning()?.textContent).toContain('ends the credentials the running Cambium uses');
  // One state, one word: the row says what the app's card below says.
  expect(document.querySelector('[aria-label="Known app cambium"] .app-state')?.textContent).toBe('Approved');
  expect(document.querySelector('input[aria-label="Cambium web address"]')).not.toBeNull();
});

it('keeps Connect as the primary act, with no warning, while Cambium is not connected', async () => {
  await mount('#/apps', { ...SERVICE, '/apps': () => ok({ apps: [] }) });
  const connect = button('Connect Cambium');
  expect(connect?.textContent).toBe('Connect');
  expect(connect?.classList.contains('primary')).toBe(true);
  await click(connect);
  expect(warning()).toBeNull();
});

it('names the replacement on the approve page and on its button when Lys says Cambium is already connected', async () => {
  await mount(HASH, { ...SERVICE, [MANIFEST]: () => ok(manifest), '/apps': () => ok({ apps: [cambium('approved')] }) });
  await settle();
  expect(warning()?.textContent).toContain('or to move to another Cambium');
  expect(button('Approve Cambium')?.textContent).toBe('Approve and replace its credentials');
});

it('approves plainly, with no replacement said, when Lys says Cambium is not yet connected', async () => {
  await mount(HASH, { ...SERVICE, [MANIFEST]: () => ok(manifest), '/apps': () => ok({ apps: [] }) });
  await settle();
  expect(warning()).toBeNull();
  expect(document.body.textContent).not.toContain('If Cambium is already connected');
  expect(button('Approve Cambium')?.textContent).toBe('Approve');
});

it('says the replacement as an "if" when Lys could not say whether Cambium is connected', async () => {
  await mount(HASH, { ...SERVICE, [MANIFEST]: () => ok(manifest), '/apps': () => refused(503, 'ServiceUnavailable', 'apps not read') });
  await settle();
  expect(warning()).toBeNull();
  expect(document.body.textContent).toContain('If Cambium is already connected, approving gives it new credentials');
});
