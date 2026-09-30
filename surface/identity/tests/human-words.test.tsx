/** Screens use the confirmed write response and explain failures without internal codes. */
import { act } from 'react';
import type { ReactNode } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { MemoryRouter } from 'react-router';
import { Refused } from '../src/api';
import { SignInProviders } from '../src/features/connections/SignInProviders';
import { Secrets } from '../src/features/secrets/Secrets';
import { ADA, DIRECTORY, ME, SERVICE } from './fixtures';
import { Requests } from '../src/features/requests/Requests';
import { Apps } from '../src/features/apps/Apps';
import { SessionList } from '../src/features/sessions/Sessions';
import { Reviews } from '../src/features/reviews/Reviews';

const roots: Root[] = [];
afterEach(() => { for (const root of roots.splice(0)) act(() => root.unmount()); });
async function show(node: ReactNode) {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  roots.push(root);
  await act(async () => { root.render(node); });
  return container;
}
async function enter(input: HTMLInputElement, value: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(input, value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

it('shows the provider write answer even when a later read would be stale', async () => {
  const empty = { providers: [], offered: ['google'], redirect_address: 'https://example.test/callback' };
  const saved = { ...empty, providers: [{ id: 'provider-1', provider: 'google', name: 'Google', enabled: true, client_id: 'client-confirmed' }] };
  let reads = 0;
  const sent: unknown[] = [];
  vi.stubGlobal('fetch', async (_url: string, init?: RequestInit) => {
    if (init?.method === 'POST') {
      sent.push(JSON.parse(String(init.body)));
      return Response.json(saved);
    }
    reads += 1;
    return Response.json(empty);
  });
  const view = await show(<SignInProviders />);
  const inputs = view.querySelectorAll<HTMLInputElement>('form input');
  if (inputs.length !== 2) throw new Error('The provider form must have two inputs');
  await enter(inputs[0], 'client-confirmed');
  await enter(inputs[1], 'never-display-this-secret');
  await act(async () => { view.querySelector<HTMLFormElement>('form')?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  expect(sent).toEqual([{ provider: 'google', client_id: 'client-confirmed', client_secret: 'never-display-this-secret' }]);
  expect(reads).toBe(1);
  expect(view.textContent).toContain('Enabled');
  expect(view.textContent).not.toContain('No sign-in provider is set yet');
  expect(view.innerHTML).not.toContain('never-display-this-secret');
});

it('explains a secrets read failure and the next step without its refusal code', async () => {
  vi.stubGlobal('fetch', async () => Response.json(DIRECTORY));
  const view = await show(<Secrets read={async () => { throw new Refused(503, { refusal: 'SecretsUnavailable', reason: 'no secrets broker is configured for this service' }); }} />);
  expect(view.textContent).toContain('Lys has no secrets store set up');
  expect(view.textContent).toContain('service configuration');
  expect(view.textContent).toContain('restart Lys');
  expect(view.textContent).not.toContain('Ask your administrator');
  expect(view.textContent).not.toContain('SecretsUnavailable');
  expect(view.textContent).not.toContain('scope');
});

it('puts a secret owner identifier behind a details toggle', async () => {
  const owner = ADA;
  vi.stubGlobal('fetch', async () => Response.json(DIRECTORY));
  const view = await show(<Secrets read={async () => ({ secrets: [{ name: 'Calendar', class: 'credential', owner, sequence: 2, upstream: null, header: null }] })} />);
  const details = view.querySelectorAll('details');
  const account = [...details].find((detail) => detail.textContent?.includes(owner));
  expect(account).toBeDefined();
  expect(view.textContent).toContain(DIRECTORY.people[0].display_name);
  expect(account?.open).toBe(false);
  for (const detail of details) detail.remove();
  expect(view.textContent).not.toContain(owner);
});

it('keeps the recorded request on screen without replacing it with an older list', async () => {
  const sent: Record<string, unknown>[] = [];
  vi.stubGlobal('fetch', async (url: string, init?: RequestInit) => {
    const path = String(url).replace(/^\/api/, '');
    if (path === '/requests' && init?.method === 'POST') {
      const body = JSON.parse(String(init.body)) as Record<string, unknown>;
      sent.push(body);
      return Response.json({ id: body.operation, asked_by: ADA, asked_by_name: ME.person.display_name,
        responsible: ME.person, resource: body.resource, relation: body.relation, actions: ['view'],
        ends_at: body.ends_at, why: body.why, asked_at: 1, state: 'waiting', approvers: [], sources: [], decision: null });
    }
    if (path === '/requests') return Response.json({ requests: [] });
    const route = SERVICE[path];
    if (!route || typeof route === 'function') throw new Error('Unexpected fixture read: ' + path);
    return Response.json(route.body, { status: route.status });
  });
  const view = await show(<MemoryRouter><Requests /></MemoryRouter>);
  const form = view.querySelector<HTMLFormElement>('form[aria-label="Ask for access"]');
  if (!form) throw new Error('The request form is missing');
  await act(async () => {
    const selects = form.querySelectorAll('select');
    if (selects.length !== 2) throw new Error('The request must offer a resource and access');
    selects[0].value = '0';
    selects[0].dispatchEvent(new Event('change', { bubbles: true }));
    selects[1].value = 'viewer';
    selects[1].dispatchEvent(new Event('change', { bubbles: true }));
    form.querySelector<HTMLInputElement>('input[type="checkbox"]')?.click();
  });
  await act(async () => {
    const why = form.querySelector('textarea');
    if (!why) throw new Error('The reason input is missing');
    why.value = 'A newly recorded request';
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  });
  expect(sent).toHaveLength(1);
  expect(sent[0]).toMatchObject({ relation: 'viewer', ends_at: null, why: 'A newly recorded request' });
  expect(view.querySelector('.listing tr[data-href]')?.textContent).toContain('viewer on');
});

async function clickNamed(view: HTMLElement, name: string) {
  const button = [...view.querySelectorAll('button')].find((entry) => entry.textContent === name);
  if (!button) throw new Error('Missing button: ' + name);
  await act(async () => button.click());
}

it('shows app approval from its response without relying on another list read', async () => {
  const pending = { id: 'fixture_notes', name: 'Notes', state: 'pending', redirects: [],
    schema: { kinds: {} }, version: 0, versions: [], pending: null, client_id: null,
    service_account: null, registered_by: { kind: 'start' }, registered_at: 1 };
  let reads = 0;
  const sent: unknown[] = [];
  vi.stubGlobal('fetch', async (url: string, init?: RequestInit) => {
    const path = String(url).replace(/^\/api/, '');
    if (path === '/apps/fixture_notes/approve' && init?.method === 'POST') {
      sent.push(JSON.parse(String(init.body)));
      return Response.json({ app: { ...pending, state: 'approved', version: 1, versions: [1] }, client: null, credentials: null });
    }
    if (path === '/apps') { reads += 1; return Response.json({ apps: [pending] }); }
    if (path === '/directory/people') return Response.json(DIRECTORY);
    throw new Error('Unexpected fixture read: ' + path);
  });
  const view = await show(<Apps />);
  await clickNamed(view, 'Approve Notes');
  expect(sent).toHaveLength(1);
  expect(reads).toBe(1);
  expect(view.querySelector('article')?.textContent).toContain('approved');
  expect([...view.querySelectorAll('button')].some((entry) => entry.textContent === 'Approve Notes')).toBe(false);
});

it('removes only the sign-in session confirmed ended without another list read', async () => {
  const session = { id: 'session-other', current: false, login: { issuer: 'https://example.test', subject: 'account' }, started_at: 1, ends_at: 2 };
  let reads = 0;
  const sent: unknown[] = [];
  vi.stubGlobal('fetch', async (_url: string, init?: RequestInit) => {
    if (init?.method === 'POST') { sent.push(JSON.parse(String(init.body))); return Response.json({ ended: session.id }); }
    reads += 1;
    return Response.json({ person: ADA, sessions: [session] });
  });
  const view = await show(<SessionList person="" />);
  await clickNamed(view, 'End session');
  await clickNamed(view, 'Confirm end session');
  expect(sent).toEqual([{}]);
  expect(reads).toBe(1);
  expect(view.textContent).not.toContain('Another signed-in session');
});

it('shows the kept review and its person from the confirmed answer without rereading', async () => {
  const review = { scope: 'personal', revision: 4, judged_at: 1, decisions_recorded: true, unanswered: [],
    due: [{ agent: { id: 'agent-example', display_name: 'Builder', state: 'active' }, reviewer: ME.person,
      last_kept: null, grant: { id: 'grant-review', relation: 'viewer', resource: { kind: 'project', id: 'Lys' }, actions: ['view'] } }] };
  let reads = 0;
  vi.stubGlobal('fetch', async (url: string, init?: RequestInit) => {
    const path = String(url).replace(/^\/api/, '');
    if (path === '/reviews/grant-review/keep' && init?.method === 'POST') {
      return Response.json({ ...JSON.parse(String(init.body)), grant: 'grant-review', kept_by: ADA, at: 2, revision: 4 });
    }
    if (path === '/reviews') { reads += 1; return Response.json(review); }
    if (path === '/me') return Response.json(ME);
    if (path === '/directory/people') return Response.json(DIRECTORY);
    throw new Error('Unexpected fixture read: ' + path);
  });
  const view = await show(<MemoryRouter><Reviews /></MemoryRouter>);
  await clickNamed(view, 'Keep access');
  await clickNamed(view, 'Confirm keep');
  expect(reads).toBe(1);
  expect(view.textContent).toContain('Last kept by ' + ME.person.display_name);
});


it('refuses an approval answer for a different app without displaying success or reading again', async () => {
  const pending = { id: 'fixture_notes', name: 'Notes', state: 'pending', redirects: [], schema: { kinds: {} }, version: 0, versions: [], pending: null, client_id: null, service_account: null, registered_by: { kind: 'start' }, registered_at: 1 };
  let reads = 0;
  vi.stubGlobal('fetch', async (_url: string, init?: RequestInit) => {
    if (init?.method === 'POST') return Response.json({ app: { ...pending, id: 'another_app', state: 'approved', version: 1, versions: [1] }, client: null, credentials: null });
    reads += 1;
    return Response.json({ apps: [pending] });
  });
  const view = await show(<Apps />);
  await clickNamed(view, 'Approve Notes');
  expect(reads).toBe(1);
  expect(view.querySelector('[role="alert"]')?.textContent).toContain('did not confirm this app’s approval');
  expect(view.querySelector('article')?.textContent).toContain('pending');
  expect(view.querySelector('article')?.textContent).not.toContain('is approved');
});

it('does not let the saved approval hide a later retirement answer', async () => {
  const pending = { id: 'fixture_notes', name: 'Notes', state: 'pending', redirects: [], schema: { kinds: {} }, version: 0, versions: [], pending: null, client_id: null, service_account: null, registered_by: { kind: 'start' }, registered_at: 1 };
  const approved = { ...pending, state: 'approved', version: 1, versions: [1] };
  let retired = false;
  let reads = 0;
  vi.stubGlobal('fetch', async (url: string, init?: RequestInit) => {
    if (init?.method === 'POST' && String(url).endsWith('/approve')) return Response.json({ app: approved, client: null, credentials: null });
    if (init?.method === 'POST') { retired = true; return Response.json({ ...approved, state: 'retired' }); }
    reads += 1;
    return Response.json({ apps: [retired ? { ...approved, state: 'retired' } : pending] });
  });
  const view = await show(<Apps />);
  await clickNamed(view, 'Approve Notes');
  expect(reads).toBe(1);
  await clickNamed(view, 'Retire Notes');
  expect(view.querySelector('article .app-state')?.textContent).toBe('retired');
  expect([...view.querySelectorAll('button')].some((entry) => entry.textContent === 'Retire Notes')).toBe(false);
});
