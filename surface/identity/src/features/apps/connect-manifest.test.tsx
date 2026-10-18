// DIRECTORY-102 R3: the approve page reads the app's manifest through Lys's own server (GET /apps/connect/manifest), so no
// browser rule stands between them, and each reason Lys names is said as its own line with the next act: not answering,
// answered with a status, not a manifest. The page never reads the app itself and never says it cannot tell which.
import { afterEach, expect, it, vi } from 'vitest';
import { mount, settle } from '../../../tests/harness';
import { SERVICE, ok, refused } from '../../../tests/fixtures';

const ORIGIN = 'https://notes.example.test';
const MANIFEST = ORIGIN + '/identity/lys/manifest';
const HASH = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(ORIGIN + '/back') + '&state=s1';
const READ = '/apps/connect/manifest?address=' + encodeURIComponent(MANIFEST);
const alert = () => document.querySelector('[role="alert"]')?.textContent ?? '';
const refusedWith = (status: number, refusal: string, reason: string, more: Record<string, unknown>) => ({ ...refused(status, refusal, reason), body: { refusal, reason, ...more } });

afterEach(() => { vi.unstubAllGlobals(); });

async function opened(answer: ReturnType<typeof ok>) {
  const direct = vi.fn(() => ok({}));
  await mount(HASH, { ...SERVICE, '/apps': () => ok({ apps: [] }), [READ]: () => answer, [MANIFEST]: direct });
  await settle();
  expect(direct).not.toHaveBeenCalled();
}

it('shows the manifest Lys read, under the app’s own name', async () => {
  await opened(ok({ manifest: { id: 'notes', name: 'Notes', redirects: [], profile: false, schema: { kinds: {} } }, origin: ORIGIN }));
  expect(document.querySelector('[aria-label="Connect an app"] h3')?.textContent).toBe('Notes');
  expect(alert()).toBe('');
});

it('says an app that is not answering as itself, with the connection’s words and the next act', async () => {
  await opened(refusedWith(502, 'manifest_not_answering', 'Lys could not reach ' + ORIGIN + ': Connection refused', { error: 'Connection refused' }));
  expect(alert()).toBe('Lys could not reach the app at ' + ORIGIN + ': Connection refused. Check that the app is running and answers at that address, then start again from the app.');
});

it('says an app that answered with an error by its status, with the next act', async () => {
  await opened(refusedWith(502, 'manifest_answered_status', 'the app answered 503', { status: 503 }));
  expect(alert()).toBe('The app at ' + ORIGIN + ' answered 503 for its manifest, so Lys could not read it. Check that the address is the app’s and that the app is running fully, then start again from the app.');
});

it('says an answer that is not a manifest by what was found, with the next act', async () => {
  await opened(refusedWith(502, 'manifest_not_a_manifest', 'the app answered a page', { found: 'a text/html answer of 52 bytes' }));
  expect(alert()).toBe('The app at ' + ORIGIN + ' answered with a text/html answer of 52 bytes, not a manifest Lys reads. Check that the address is the app’s own, then start again from the app.');
});

it('never says the two-cause sentence or offers the manifest to open', async () => {
  await opened(refusedWith(502, 'manifest_not_answering', 'Lys could not reach ' + ORIGIN, { error: 'Connection refused' }));
  expect(document.body.textContent).not.toContain('cannot tell');
  expect([...document.querySelectorAll('a')].some((link) => link.getAttribute('href') === MANIFEST)).toBe(false);
  expect(document.querySelector('[aria-label="Find out which"]')).toBeNull();
});
