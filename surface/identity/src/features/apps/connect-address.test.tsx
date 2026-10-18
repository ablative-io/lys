// DIRECTORY-102 R2: the address an app is told to call Lys back at is Lys's own public address as GET /apps answers it, sent
// as lys= and shown before the browser leaves Lys; when Lys has none, the page says Lys's reason, that the app will be handed
// this browser's address instead, and what to change. Said, never enforced: Continue stays available.
import { afterEach, expect, it, vi } from 'vitest';
import { click, mount, type } from '../../../tests/harness';
import { SERVICE, ok } from '../../../tests/fixtures';
import { CONNECT_PATH } from './ConnectApp';

const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.getAttribute('aria-label') === label) ?? null;
const PUBLIC = 'https://lys.example.test/identity';
const REASON = 'Lys is served only at http://localhost:8490, an address only the computer it runs on answers, so an app on another computer cannot call Lys back there; Lys needs an https address other computers reach.';

afterEach(() => { vi.unstubAllGlobals(); });

async function open(answer: unknown) {
  const assign = vi.fn();
  await mount('#/apps', { ...SERVICE, '/apps': () => ok(answer) });
  vi.stubGlobal('location', { ...window.location, assign, origin: window.location.origin });
  await click(button('Connect another app'));
  await type(document.querySelector('input[aria-label="The app web address"]'), 'https://notes.example.test');
  return { assign, row: document.querySelector('[aria-label="Another app"]') };
}

it('sends and shows Lys’s own public address, not the browser’s', async () => {
  const { assign, row } = await open({ apps: [], public_address: PUBLIC, public_address_reason: null });
  expect(row?.textContent).toContain('The app will call Lys back at ' + PUBLIC + '.');
  expect(row?.textContent).not.toContain(window.location.origin);
  await click(button('Continue to the app'));
  expect(assign).toHaveBeenCalledWith('https://notes.example.test' + CONNECT_PATH + '?lys=' + encodeURIComponent(PUBLIC));
});

it('says Lys’s reason, hands the browser’s address instead, and names what to change when Lys has no public address', async () => {
  const { assign, row } = await open({ apps: [], public_address: null, public_address_reason: REASON });
  const said = row?.querySelector('[role="note"]')?.textContent ?? '';
  expect(said).toContain(REASON);
  expect(said).toContain('The app will be handed this browser’s address, ' + window.location.origin + ', instead.');
  expect(said).toContain('change Lys’s registered address');
  expect(row?.textContent).not.toContain('will call Lys back at');
  expect(button('Continue to the app')?.hasAttribute('disabled')).toBe(false);
  await click(button('Continue to the app'));
  expect(assign).toHaveBeenCalledWith('https://notes.example.test' + CONNECT_PATH + '?lys=' + encodeURIComponent(window.location.origin));
});
