/**
 * A link left open on a shared screen never fills in an act for a different person. Bea was the last person
 * signed in on this browser; Ada signs in on the page Bea left at a give link to Ada's own agent. The link must
 * not fill the Give drawer: Lys opens the page without it and says why. The same person signing back in keeps it.
 */
import { act, StrictMode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { App } from '../src/App';
import { api } from '../src/api';
import { Callback, signedInHere } from '../src/features/signin/Gate';
import { signedOutPage } from '../src/features/me/SignOut';
import type { Grant, GrantModel } from '../src/generated/grants';
import { ADA, BEA, GRANTS, ME, MODEL, SCRIBE, SERVICE, ok, refused, type Route } from './fixtures';
import { $$, click, mount, serve } from './harness';

const resource = { kind: 'cambium.workspace', id: 'ablative' };
const source: Grant = {
  ...GRANTS[0],
  resource,
  relation: 'member',
  actions: ['read', 'query'],
  pass_on: { kind: 'to', actions: ['read', 'query'], recipients: ['agent'] },
};
const model: GrantModel = {
  ...MODEL,
  kinds: { ...MODEL.kinds, [resource.kind]: { agents: ['read', 'query'], relations: { reader: ['read'], querier: ['query'] } } },
};
const page = '#/file/' + SCRIBE + '/access';
const link = page + '?give=cambium.workspace:ablative:read';
const PERSON = 'lys.sign-in.person';
const DROPPED = 'Someone else was signed in on this browser before you';
let root: Root | undefined;

beforeEach(() => {
  sessionStorage.clear();
  localStorage.clear();
});
afterEach(() => {
  if (root) act(() => root?.unmount());
  root = undefined;
  vi.restoreAllMocks();
});

/** The service: signed out until a password sign-in is posted, Ada afterwards (or from the start, when `signedIn`). */
function service(signedIn: boolean, extra: Record<string, Route> = {}): Record<string, Route> {
  let session = signedIn;
  const gated = (answer: Route): Route => () => (session ? (typeof answer === 'function' ? answer(undefined) : answer) : refused(401, 'NotSignedIn', 'Sign in first'));
  return {
    ...SERVICE,
    '/me': gated(ok(ME)),
    '/directory/people': gated(SERVICE['/directory/people']),
    '/grants': gated(ok({ grants: [source], revision: 7 })),
    '/grants/model': gated(ok(model)),
    '/roles': ok({ roles: [] }),
    ['/grants/cannot-give?route=browser&source=' + source.id + '&recipient=' + SCRIBE]: ok({ source: source.id, recipient: SCRIBE, items: [] }),
    'POST /sign-in': () => {
      session = true;
      return ok({ signed_in: ME.signed_in, authority: 'test' });
    },
    '/sign-in/providers': ok({ providers: [{ id: 'google-1', name: 'Google', provider: 'google' }] }),
    ...extra,
  };
}

async function open(hash: string, routes: Record<string, Route>): Promise<HTMLElement> {
  history.replaceState(null, '', '/' + hash);
  serve(routes);
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => root?.render(<StrictMode><App /></StrictMode>));
  return container;
}

const filled = (container: HTMLElement) => container.querySelector('section[aria-label="Give"]');

async function signInInPlace(container: HTMLElement) {
  const form = container.querySelector<HTMLFormElement>('form[aria-label="Sign in"]');
  if (!form) throw new Error('Sign in was not shown in place');
  const email = form.querySelector<HTMLInputElement>('[name="email"]');
  const password = form.querySelector<HTMLInputElement>('[name="password"]');
  if (!email || !password) throw new Error('The sign-in fields were not shown');
  email.value = 'ada@example.test';
  password.value = 'test password';
  await act(async () => form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })));
}

async function pressGoogle(container: HTMLElement) {
  const login = container.querySelector<HTMLAnchorElement>('a[href^="/api/sign-in/providers/google-1"]');
  if (!login) throw new Error('Sign in with Google was not offered');
  login.addEventListener('click', (event) => event.preventDefault());
  await act(async () => login.click());
  act(() => root?.unmount());
  root = undefined;
}

it('GUARD 1 a give link Bea left open does not fill the Give drawer when Ada signs in on that page', async () => {
  localStorage.setItem(PERSON, BEA);
  const reopened = vi.spyOn(signedInHere, 'open').mockImplementation(() => undefined);
  const routes = service(false);
  const container = await open(link, routes);
  await signInInPlace(container);
  expect(reopened).toHaveBeenCalledOnce();
  expect(location.hash).toBe(page);
  expect(localStorage.getItem(PERSON)).toBe(ADA);
  // The page reads again, as location.reload() would.
  act(() => root?.unmount());
  root = undefined;
  const again = await open(location.hash, routes);
  expect(filled(again)).toBeNull();
  expect(again.textContent).toContain(DROPPED);
});

it('GUARD 2 the same person signing back in on the page keeps the give link', async () => {
  localStorage.setItem(PERSON, ADA);
  const reopened = vi.spyOn(signedInHere, 'open').mockImplementation(() => undefined);
  const container = await open(link, service(false));
  await signInInPlace(container);
  expect(reopened).toHaveBeenCalledOnce();
  expect(location.hash).toBe(link);
  expect(container.textContent).not.toContain(DROPPED);
});

it('GUARD 3 a give link kept through Sign in with Google is not restored for a different person', async () => {
  localStorage.setItem(PERSON, BEA);
  await pressGoogle(await open(link, service(false)));
  const returned = await open('#/me', service(true));
  expect(location.hash).toBe(page);
  expect(filled(returned)).toBeNull();
  expect(returned.textContent).toContain(DROPPED);
});

it('GUARD 4 a give link kept through the issuer callback is not restored for a different person', async () => {
  localStorage.setItem(PERSON, BEA);
  await pressGoogle(await open(link, service(false)));
  serve(service(true));
  const replace = vi.fn();
  vi.stubGlobal('location', { pathname: '/callback', search: '?code=once&state=bound', hash: '', replace });
  vi.spyOn(api, 'callback').mockResolvedValue({ signed_in: { issuer: 'https://issuer.test', subject: 'ada' }, authority: 'test' });
  root = createRoot(document.createElement('div'));
  await act(async () => root?.render(<StrictMode><Callback /></StrictMode>));
  vi.unstubAllGlobals();
  expect(replace).toHaveBeenCalledExactlyOnceWith('/#/');
});

it('GUARD 5 when Lys cannot say who signed in, a link left by someone else is not filled', async () => {
  localStorage.setItem(PERSON, BEA);
  vi.spyOn(signedInHere, 'open').mockImplementation(() => undefined);
  const routes = service(false);
  const container = await open(link, routes);
  routes['/me'] = refused(503, 'DirectoryUnavailable', 'the directory is not answering');
  await signInInPlace(container);
  expect(location.hash).toBe(page);
});

it('GUARD 6 Sign out forgets every link kept for a sign-in, and a signed-in page notes who is signed in', async () => {
  vi.spyOn(signedOutPage, 'open').mockImplementation(() => undefined);
  const sessions = { person: ADA, sessions: [{ id: 'session-current', login: { issuer: 'https://login.test', subject: 'ada' }, started_at: 1792100000, ends_at: 1794700000, current: true }] };
  await mount('#/me', { ...SERVICE, '/sessions': ok(sessions), 'POST /sessions/session-current/end': ok({ ended: 'session-current' }) });
  expect(localStorage.getItem(PERSON)).toBe(ADA);
  // Kept by a sign-in begun in this tab and never finished.
  sessionStorage.setItem('lys.sign-in.file-return', link);
  sessionStorage.setItem('lys.sign-in.landing', link);
  await click($$('.page-header button').find((each) => each.getAttribute('aria-label') === 'Sign out') ?? null);
  expect(sessionStorage.getItem('lys.sign-in.file-return')).toBeNull();
  expect(sessionStorage.getItem('lys.sign-in.landing')).toBeNull();
});
