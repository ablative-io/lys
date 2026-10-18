import { act, StrictMode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { App } from '../src/App';
import { api } from '../src/api';
import { Callback, signedInHere } from '../src/features/signin/Gate';
import { pendingGrantKey, restKey } from '../src/features/grants/pendingGrant';
import type { DelegateBody, Grant, GrantModel } from '../src/generated/grants';
import { ADA, GRANTS, ME, MODEL, SCRIBE, SCRIBE_G, SERVICE, ok, refused, type Route } from './fixtures';
import { serve } from './harness';

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
const link = (give = 'cambium.workspace:ablative:read', id = SCRIBE) => '#/file/' + id + '/access?give=' + give;
let root: Root | undefined;

beforeEach(() => {
  sessionStorage.clear();
  vi.spyOn(Date, 'now').mockReturnValue(1792100000000);
  vi.spyOn(crypto, 'getRandomValues').mockImplementation((array) => {
    if (array instanceof Uint8Array) array.fill(7);
    return array;
  });
});
afterEach(() => {
  if (root) act(() => root?.unmount());
  root = undefined;
});

async function open(hash = link(), held = [source], metadata = model, extra: Record<string, Route> = {}) {
  history.replaceState(null, '', '/' + hash);
  const requests = serve({
    ...SERVICE,
    '/grants': ok({ grants: held, revision: 7 }),
    '/grants/model': ok(metadata),
    '/roles': ok({ roles: [] }),
    ...Object.fromEntries(
      held.map((grant) => [
        '/grants/cannot-give?route=browser&source=' + grant.id + '&recipient=' + SCRIBE,
        ok({ source: grant.id, recipient: SCRIBE, items: [] }),
      ])
    ),
    'POST /grants': (body) => ok({ operation: (body as DelegateBody).operation, grant: SCRIBE_G, receipt: { caller: ADA } }),
    ...extra,
  });
  vi.stubGlobal('fetch', vi.fn(fetch));
  const page = document.createElement('div');
  document.body.appendChild(page);
  root = createRoot(page);
  await act(async () =>
    root?.render(
      <StrictMode>
        <App />
      </StrictMode>
    )
  );
  return { page, requests };
}

function drawer(page: HTMLElement): HTMLElement {
  const panel = page.querySelector<HTMLElement>('section[aria-label="Give"]');
  expect(panel, 'the link opens the Give drawer').not.toBeNull();
  if (!panel) throw new Error('The Give drawer did not open');
  return panel;
}

async function press(element: Element | null) {
  if (!(element instanceof HTMLElement)) throw new Error('The requested control was not shown');
  await act(async () => element.click());
}

async function select(element: Element | null, value: string) {
  if (!(element instanceof HTMLSelectElement)) throw new Error('The requested choice was not shown');
  await act(async () => {
    element.value = value;
    element.dispatchEvent(new Event('change', { bubbles: true }));
  });
}

function sent(): RequestInit[] {
  return vi
    .mocked(fetch)
    .mock.calls.filter(([path, init]) => path === '/api/grants' && init?.method === 'POST')
    .map(([, init]) => init as RequestInit);
}

async function manual(actions: string[], held = source, metadata = model): Promise<string[]> {
  if (root) act(() => root?.unmount());
  root = undefined;
  sessionStorage.clear();
  const { page } = await open('#/file/' + SCRIBE + '/access', [held], metadata);
  await press(page.querySelector('[data-act="grant"]'));
  const panel = drawer(page);
  await select(panel.querySelector('[aria-label="Grant to give"]'), held.id);
  for (const action of actions) await press(panel.querySelector('input[name="action"][value="' + action + '"]'));
  await press(panel.querySelector('[data-act="delegatedo"]'));
  return sent().map((init) => String(init.body));
}

it('ITEM 1 fills the same visible choices and sends byte-equal DelegateBody JSON after Give', async () => {
  const { page } = await open();
  const panel = drawer(page);
  expect(panel.querySelector<HTMLSelectElement>('[aria-label="Grant to give"]')?.value).toBe(source.id);
  expect(panel.querySelector<HTMLInputElement>('input[name="action"][value="read"]')?.checked).toBe(true);
  expect(panel.querySelector<HTMLSelectElement>('#dTo')?.value).toBe(SCRIBE);
  expect(panel.querySelector<HTMLSelectElement>('#dLease')?.value).toBe('7 days');
  expect(panel.querySelector<HTMLSelectElement>('#dPass')?.value).toBe('no');
  await press(panel.querySelector('[data-act="delegatedo"]'));
  const linked = sent().map((init) => String(init.body));
  expect(linked).toHaveLength(1);
  expect(linked).toEqual(await manual(['read']));
});

it('ITEM 2 never sends on load or remount; the person presses Give under their own session', async () => {
  const { page } = await open();
  drawer(page);
  expect(sent()).toEqual([]);
  await act(async () =>
    root?.render(
      <StrictMode>
        <App />
      </StrictMode>
    )
  );
  expect(sent()).toEqual([]);
  await press(drawer(page).querySelector('[data-act="delegatedo"]'));
  expect(sent()).toHaveLength(1);
  expect(sent()[0].credentials).toBe('same-origin');
  expect(JSON.parse(String(sent()[0].body))).toMatchObject({ recipient: SCRIBE, responsible: ADA, source: source.id, route: 'browser' });
});

it('ITEM 3 names what filled the form before the press', async () => {
  const { page } = await open();
  const words = drawer(page).textContent;
  for (const word of ['this link filled this form', 'Scribe', SCRIBE, 'cambium.workspace', 'ablative', 'read']) expect(words).toContain(word);
  expect(sent()).toEqual([]);
});

it.each([
  ['unknown resource kind', 'unknown.kind:ablative:read', 'Unknown resource kind unknown.kind'],
  ['missing action', 'cambium.workspace:ablative:', 'The give link is missing an action'],
  ['missing separators', 'cambium.workspace', 'Use resource:id:action'],
  ['bad percent encoding', 'cambium.workspace:bad%ZZ:read', 'invalid percent encoding'],
  ['unoffered action', 'cambium.workspace:ablative:admin', 'The Give form cannot offer action admin'],
  ['extra query member', 'cambium.workspace:ablative:read&pass_on=to', 'query member pass_on'],
  ['extra expiry member', 'cambium.workspace:ablative:read&expiry=never', 'query member expiry'],
  ['extra scope member', 'cambium.workspace:ablative:read&scope=all', 'query member scope'],
  ['duplicate give', 'cambium.workspace:ablative:read&give=cambium.workspace:ablative:query', 'exactly one give'],
])('ITEM 4 opens an empty drawer with words for %s', async (_name, give, reason) => {
  const { page } = await open(link(give));
  const panel = drawer(page);
  expect(panel.textContent).toContain(reason);
  expect(panel.querySelector<HTMLSelectElement>('[aria-label="Grant to give"]')?.value ?? '').toBe('');
  expect(panel.querySelector('[data-act="delegatedo"]')).toBeNull();
  expect(sent()).toEqual([]);
});

it.each([
  ['absent', []],
  ['use only', [{ ...source, pass_on: { kind: 'use_only' } }]],
  ['other person', [{ ...source, holder: 'person-' + '9'.repeat(32) }]],
  ['not standing', [{ ...source, standing: { stands: false, refusal: 'Revoked', grant: source.id, reason: 'Revoked' } }]],
  ['wrong id', [{ ...source, resource: { ...resource, id: 'elsewhere' } }]],
  ['action excluded', [{ ...source, pass_on: { kind: 'to', actions: ['query'], recipients: ['agent'] } }]],
] satisfies [string, Grant[]][])('ITEM 5 names the missing passable grant when the source is %s', async (_name, held) => {
  const { page } = await open(link(), held);
  const panel = drawer(page);
  for (const word of ['cambium.workspace', 'ablative', 'read', 'pass it on']) expect(panel.textContent).toContain(word);
  expect(panel.querySelector('[data-act="delegatedo"]')).toBeNull();
  expect(panel.querySelector<HTMLSelectElement>('[aria-label="Grant to give"]')?.value ?? '').toBe('');
  expect(sent()).toEqual([]);
});

it('ITEM 6 fills both read and query and sends the same bodies as two manual ticks', async () => {
  const { page } = await open(link('cambium.workspace:ablative:read,query'));
  const panel = drawer(page);
  for (const action of ['read', 'query']) expect(panel.querySelector<HTMLInputElement>('input[name="action"][value="' + action + '"]')?.checked).toBe(true);
  expect(sent()).toEqual([]);
  await press(panel.querySelector('[data-act="delegatedo"]'));
  const linked = sent().map((init) => String(init.body));
  expect(linked).toHaveLength(2);
  expect(linked).toEqual(await manual(['read', 'query']));
});

it('ITEM 7 percent-decodes every resource id character without treating it as a query member', async () => {
  const id = 'store:/ward?team=A&B=2+#%雪';
  const held = { ...source, resource: { ...resource, id } };
  const { page } = await open(link('cambium.workspace:' + encodeURIComponent(id) + ':read'), [held]);
  const panel = drawer(page);
  expect(panel.textContent).toContain(id);
  await press(panel.querySelector('[data-act="delegatedo"]'));
  expect(JSON.parse(String(sent()[0].body)).resource).toEqual(held.resource);
});

it('ITEM 8 opens an empty drawer when the link names an agent that does not exist', async () => {
  const missing = 'agent-' + 'f'.repeat(32);
  const { page } = await open(link(undefined, missing), [source], model, {
    ['/directory/agents/' + missing]: refused(404, 'AgentUnknown', 'This agent does not exist'),
  });
  const panel = drawer(page);
  expect(panel.textContent).toContain('This agent does not exist');
  expect(panel.textContent).toContain(missing);
  expect(panel.querySelector('[data-act="delegatedo"]')).toBeNull();
  expect(sent()).toEqual([]);
});

it('ITEM 9 keeps the exact give link through password sign-in on the page itself', async () => {
  const original = link('cambium.workspace:store%3Award:read,query');
  const reopened = vi.spyOn(signedInHere, 'open').mockImplementation(() => undefined);
  const { page } = await open(original, [source], model, {
    '/me': refused(401, 'NotSignedIn', 'Sign in first'),
    '/directory/people': refused(401, 'NotSignedIn', 'Sign in first'),
    'POST /sign-in': ok({ signed_in: ME.signed_in, authority: 'test' }),
  });
  expect(page.querySelector('a[href="/api/login"]')).toBeNull();
  const form = page.querySelector<HTMLFormElement>('form[aria-label="Sign in"]');
  if (!form) throw new Error('Sign in was not shown in place');
  const email = form.querySelector<HTMLInputElement>('[name="email"]');
  const password = form.querySelector<HTMLInputElement>('[name="password"]');
  if (!email || !password) throw new Error('The sign-in fields were not shown');
  email.value = 'person@example.test';
  password.value = 'test password';
  await act(async () => form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })));
  expect(reopened).toHaveBeenCalledOnce();
  expect(location.hash).toBe(original);
  expect(sent()).toEqual([]);
});

it('ITEM 10 restores the give link through the existing callback path after Sign in with Google', async () => {
  const original = link();
  const { page } = await open(original, [source], model, {
    '/me': refused(401, 'NotSignedIn', 'Sign in first'),
    '/directory/people': refused(401, 'NotSignedIn', 'Sign in first'),
    '/sign-in/providers': ok({ providers: [{ id: 'google-1', name: 'Google', provider: 'google' }] }),
  });
  const login = page.querySelector<HTMLAnchorElement>('a[href^="/api/sign-in/providers/google-1"]');
  expect(login).not.toBeNull();
  login?.addEventListener('click', (event) => event.preventDefault());
  await press(login);
  act(() => root?.unmount());
  root = undefined;
  const replace = vi.fn();
  vi.stubGlobal('location', { pathname: '/callback', search: '?code=once&state=bound', replace });
  vi.spyOn(api, 'callback').mockResolvedValue({ signed_in: { issuer: 'https://issuer.test', subject: 'test' }, authority: 'test' });
  root = createRoot(document.createElement('div'));
  await act(async () =>
    root?.render(
      <StrictMode>
        <Callback />
      </StrictMode>
    )
  );
  expect(replace).toHaveBeenCalledExactlyOnceWith('/' + original);
});

it('ITEM 11 restores the give link after Sign in with Google returns to /me', async () => {
  const original = link();
  const { page } = await open(original, [source], model, {
    '/me': refused(401, 'NotSignedIn', 'Sign in first'),
    '/directory/people': refused(401, 'NotSignedIn', 'Sign in first'),
    '/sign-in/providers': ok({ providers: [{ id: 'google-1', name: 'Google', provider: 'google' }] }),
  });
  const login = page.querySelector<HTMLAnchorElement>('a[href^="/api/sign-in/providers/google-1"]');
  expect(login).not.toBeNull();
  login?.addEventListener('click', (event) => event.preventDefault());
  await press(login);
  act(() => root?.unmount());
  root = undefined;
  const returned = await open('#/me');
  expect(location.hash).toBe(original);
  drawer(returned.page);
  expect(sent()).toEqual([]);
});

it.each(['pending', 'remaining'])('ITEM 12 refuses a link over a retained %s request instead of hiding earlier choices', async (state) => {
  const key = pendingGrantKey(ADA, source.id);
  const body: DelegateBody = {
    operation: 'op-' + '4'.repeat(32),
    source: source.id,
    route: 'browser',
    recipient: SCRIBE,
    responsible: ADA,
    resource,
    relation: 'querier',
    pass_on: { kind: 'use_only' },
    window: { starts_at: 1792100000, ends_at: null },
  };
  sessionStorage.setItem(state === 'pending' ? key : restKey(key), JSON.stringify(state === 'pending' ? body : [body]));
  const { page } = await open();
  const panel = drawer(page);
  expect(panel.textContent).toContain('earlier grant request');
  expect(panel.querySelector('[data-act="delegatedo"]')).toBeNull();
  expect(sent()).toEqual([]);
});

it('ITEM 13 refuses an action that is declared but only offered in a broader relation', async () => {
  const combined = { ...model, kinds: { [resource.kind]: { agents: ['read', 'query'], relations: { member: ['read', 'query'] } } } };
  const { page } = await open(link(), [source], combined);
  const panel = drawer(page);
  expect(panel.textContent).toContain('The Give form cannot offer action read');
  expect(panel.querySelector('[data-act="delegatedo"]')).toBeNull();
  expect(sent()).toEqual([]);
});

it('ITEM 14 fills a combined choice only when all of its visible actions were requested', async () => {
  const combined = { ...model, kinds: { [resource.kind]: { agents: ['read', 'query'], relations: { member: ['read', 'query'] } } } };
  const { page } = await open(link('cambium.workspace:ablative:read,query'), [source], combined);
  const panel = drawer(page);
  expect(panel.querySelector<HTMLInputElement>('input[name="action"][value="relation:member"]')?.checked).toBe(true);
  await press(panel.querySelector('[data-act="delegatedo"]'));
  const linked = sent().map((init) => String(init.body));
  expect(linked).toEqual(await manual(['relation:member'], source, combined));
});

it('ITEM 16 never treats an encoded resource-id colon as the missing action separator', async () => {
  const held = { ...source, resource: { ...resource, id: 'store' } };
  const { page } = await open(link('cambium.workspace:store%3Aread'), [held]);
  const panel = drawer(page);
  expect(panel.textContent).toContain('missing an action');
  expect(panel.querySelector('[data-act="delegatedo"]')).toBeNull();
  expect(sent()).toEqual([]);
});
