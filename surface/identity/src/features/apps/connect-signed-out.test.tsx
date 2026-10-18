// Domain 04 item 3: when Lys's sign-in ends before Approve reaches it, the approve page says so, says nothing was approved,
// and offers Sign in again, rather than a generic line. The 401 is the service's own answer, never one the page assumed.
import { afterEach, expect, it, vi } from 'vitest';
import { click, mount, settle } from '../../../tests/harness';
import { SERVICE, ok, refused } from '../../../tests/fixtures';
import { takeFileReturn } from '../signin/fileReturn';

const ORIGIN = 'https://cambium.example.test';
const MANIFEST = ORIGIN + '/identity/lys/manifest';
const BACK = ORIGIN + '/identity/lys/connected';
const HASH = '#/apps/connect?manifest=' + encodeURIComponent(MANIFEST) + '&return=' + encodeURIComponent(BACK) + '&state=s1';
const manifest = { id: 'cambium', name: 'Cambium', redirects: [ORIGIN + '/identity/oidc/callback'], profile: true, schema: { kinds: {} } };
const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => entry.getAttribute('aria-label') === label) ?? null;
const signIn = () => [...document.querySelectorAll('a')].find((entry) => entry.textContent === 'Sign in again') ?? null;

afterEach(() => { vi.unstubAllGlobals(); });

async function approveAnswering(answer: ReturnType<typeof refused>) {
  const assign = vi.fn();
  const { posted } = await mount(HASH, { ...SERVICE, [MANIFEST]: () => ok(manifest), '/apps': () => ok({ apps: [] }), 'POST /apps/connect': () => answer });
  vi.stubGlobal('location', { ...window.location, assign, origin: window.location.origin });
  await settle();
  await click(button('Approve Cambium'));
  return { posted, assign };
}

it('says a lapsed sign-in as itself, that nothing was approved, and offers Sign in again', async () => {
  const { posted, assign } = await approveAnswering(refused(401, 'NotSignedIn', 'no session'));
  expect(posted).toHaveLength(1);
  expect(assign).not.toHaveBeenCalled();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('Your sign-in to Lys ended before Approve reached it, so nothing was approved and Cambium is still waiting. Sign in again, and Lys brings you back to this page.');
  expect(signIn()?.getAttribute('href')).toBe('/api/login');
  // Approve would only meet the same answer again, so Sign in takes its place; Decline needs no sign-in and stays.
  expect(button('Approve Cambium')).toBeNull();
  expect(button('Decline Cambium')).not.toBeNull();
});

it('offers no sign-in for an answer that is not about signing in', async () => {
  await approveAnswering(refused(403, 'NotAdmitted', 'not administrator'));
  expect(document.querySelector('[role="alert"]')?.textContent).not.toContain('Your sign-in to Lys ended');
  expect(signIn()).toBeNull();
  expect(button('Approve Cambium')).not.toBeNull();
});

it('keeps this page’s link when Sign in again is pressed, so the sign-in comes back to it', async () => {
  await approveAnswering(refused(401, 'NotSignedIn', 'no session'));
  expect(takeFileReturn()).toBeNull();
  // The browser would leave for the sign-in; the page's own handler runs first and keeps the link.
  const stay = (event: Event) => event.preventDefault();
  document.addEventListener('click', stay);
  try { await click(signIn()); } finally { document.removeEventListener('click', stay); }
  expect(takeFileReturn()).toBe(HASH);
  expect(document.querySelector('[role="alert"]')?.textContent).not.toContain('could not keep this link');
});
