/**
 * The Apps screen and its permission template builder against a stubbed
 * service that answers by method and path: a schema built from a template
 * with nothing typed saves the very body an upload sends; an uploaded schema
 * opens and edits in the builder; the bench asks the draft and shows the path;
 * a change that strands grants shows the count and cannot be saved; and an
 * approval shows the schema in words before its button and the secret once.
 */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Apps } from '../src/features/apps/Apps';
import { SchemaBuilder, fromSchema, toSchema } from '../src/features/apps/SchemaBuilder';
import type { SchemaJson } from '../src/features/apps/SchemaBuilder';
import { $$, choose, click, settle, text } from './harness';

type Answer = { status: number; body: unknown };
type Call = { method: string; path: string; body: unknown };
const APP = 'fixture_notes';
const roots: Root[] = [];

/** Stub the service: each `METHOD /path` answers from `routes`; anything else is a bare 404. */
function stub(routes: Record<string, (body: unknown) => Answer>): Call[] {
  const calls: Call[] = [];
  vi.stubGlobal('fetch', async (input: string, init?: RequestInit) => {
    const path = String(input).replace(/^\/api/, '');
    const method = init?.method ?? 'GET';
    const body: unknown = init?.body ? JSON.parse(String(init.body)) : undefined;
    calls.push({ method, path, body });
    const route = routes[method + ' ' + path];
    if (!route) return new Response('', { status: 404 });
    const answer = route(body);
    return new Response(JSON.stringify(answer.body), { status: answer.status, headers: { 'content-type': 'application/json' } });
  });
  return calls;
}

async function render(element: ReactNode): Promise<void> {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  roots.push(root);
  await act(async () => { root.render(element); });
  await settle();
}

async function type(el: Element | null, value: string): Promise<void> {
  if (!(el instanceof HTMLInputElement)) throw new Error('no input to type in');
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(el, value);
    el.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await settle();
}

const button = (label: string) => $$('button').find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === label) ?? null;
const labelled = (label: string) => document.querySelector('[aria-label="' + label + '"]');
const ok = (body: unknown): Answer => ({ status: 200, body });
const noDiff = { kinds_added: [], kinds_removed: [], relations_added: [], relations_removed: [], actions_added: [], actions_removed: [], parents_added: [], parents_removed: [] };
const applies = ok({ current: 1, next: 2, applies: true, stranded: [], diff: noDiff });

/** The schema an app uploads for the workspace template, written out by hand as the API takes it. */
const uploadedWorkspace: SchemaJson = { kinds: {
  'fixture_notes.workspace': { actions: ['read', 'write', 'admin'], relations: { owner: ['read', 'write', 'admin'], member: ['read'] }, parents: [] },
  'fixture_notes.channel': { actions: ['read', 'write'], relations: { poster: ['read', 'write'] }, parents: ['fixture_notes.workspace'] },
} };
const documents: SchemaJson = { kinds: { 'fixture_notes.doc': { actions: ['read', 'write'], relations: { reader: ['read'], editor: ['read', 'write'] }, parents: [] } } };

beforeEach(() => sessionStorage.clear());
afterEach(() => { for (const root of roots.splice(0)) act(() => root.unmount()); });

describe('the permission template builder', () => {
  it('names an operator registration as the install operator, never a sign-in or service start', async () => {
    const app = { id: APP, name: 'Notes', state: 'pending', redirects: [], schema: documents, version: 0, versions: [], pending: null, client_id: null, service_account: null, client_credentials: [], registered_by: { kind: 'operator', login: { provider: 'https://issuer.test', subject: 'administrator' } }, registered_at: 1 };
    stub({ 'GET /apps': () => ok({ apps: [app] }) });
    await render(<Apps />);
    expect(text()).toContain('Registered by the install operator for administrator');
    expect(text()).not.toContain('Lys at start');
  });

  it('builds a two-kind schema with a parent from a template, nothing typed, and saves the body an upload sends', async () => {
    const calls = stub({ ['POST /apps/' + APP + '/schema/check']: () => applies, ['PUT /apps/' + APP + '/schema']: () => ok({ applied: true }) });
    let saved = '';
    await render(<SchemaBuilder app={APP} version={1} initial={documents} onSaved={(message) => { saved = message; }} />);
    await click(button('A workspace whose members reach its channels'));
    const workspace = labelled('Kind workspace');
    expect(workspace?.closest('.sb-node')?.querySelector('.sb-children [aria-label="Kind channel"]')).not.toBeNull();
    expect(button('Save version 2')?.hasAttribute('disabled')).toBe(true);
    await click(button('Check the change'));
    await click(button('Save version 2'));
    const put = calls.filter((call) => call.method === 'PUT');
    expect(put).toHaveLength(1);
    expect(put[0]?.body).toEqual({ operation: expect.stringMatching(/^op-/), replaces: 1, schema: uploadedWorkspace });
    expect(calls.find((call) => call.method === 'POST')?.body).toEqual({ replaces: 1, schema: uploadedWorkspace });
    expect(saved).toBe('The schema was saved.');
    expect(toSchema(APP, fromSchema(APP, uploadedWorkspace))).toEqual(uploadedWorkspace);
  });

  it('opens an uploaded schema and edits it there', async () => {
    const calls = stub({ ['POST /apps/' + APP + '/schema/check']: () => applies, ['PUT /apps/' + APP + '/schema']: () => ok({ applied: true }) });
    await render(<SchemaBuilder app={APP} version={1} initial={documents} />);
    const writes = labelled('reader carries write');
    if (!(writes instanceof HTMLInputElement)) throw new Error('the uploaded relation did not open');
    expect(writes.checked).toBe(false);
    expect((labelled('editor carries write') as HTMLInputElement | null)?.checked).toBe(true);
    await click(writes);
    await type(labelled('Add a relation to doc'), 'auditor');
    await click(button('Add a relation to doc'));
    await click(labelled('auditor carries read'));
    await click(button('Check the change'));
    await click(button('Save version 2'));
    const put = calls.find((call) => call.method === 'PUT');
    expect((put?.body as { schema: SchemaJson }).schema.kinds['fixture_notes.doc']?.relations).toEqual({ reader: ['read', 'write'], editor: ['read', 'write'], auditor: ['read'] });
  });

  it('asks the draft on the bench and shows the path, and closes the bench', async () => {
    const calls = stub({
      'POST /apps/bench': () => ok({ bench: 'b1', app: APP }),
      'POST /apps/bench/b1/ask': () => ok({ allowed: true, path: ['fixture_notes.channel:general is placed in fixture_notes.workspace:team', 'X holds member on fixture_notes.workspace:team', 'member carries read'] }),
      'POST /apps/bench/b1/close': () => ok({ closed: 'b1' }),
    });
    await render(<SchemaBuilder app={APP} version={1} initial={uploadedWorkspace} />);
    await type(labelled('Example 1 person or agent'), 'X');
    await choose(labelled('Example 1 kind'), 'workspace');
    await choose(labelled('Example 1 relation'), 'member');
    await type(labelled('Example 1 resource id'), 'team');
    await click(button('Add an example placement'));
    await choose(labelled('Placement 1 child kind'), 'channel');
    await type(labelled('Placement 1 child id'), 'general');
    await choose(labelled('Placement 1 parent kind'), 'workspace');
    await type(labelled('Placement 1 parent id'), 'team');
    await type(labelled('Who asks'), 'X');
    await choose(labelled('Kind asked about'), 'channel');
    await choose(labelled('Action'), 'read');
    await type(labelled('Resource id asked about'), 'general');
    await click(button('Ask the draft'));
    expect(calls.find((call) => call.path === '/apps/bench')?.body).toEqual({ app: APP, schema: uploadedWorkspace });
    expect(calls.find((call) => call.path === '/apps/bench/b1/ask')?.body).toEqual({
      holdings: [{ subject: 'X', relation: 'member', resource: { kind: 'fixture_notes.workspace', id: 'team' } }],
      placements: [{ child: { kind: 'fixture_notes.channel', id: 'general' }, parent: { kind: 'fixture_notes.workspace', id: 'team' } }],
      question: { subject: 'X', action: 'read', resource: { kind: 'fixture_notes.channel', id: 'general' } },
    });
    expect(text()).toContain('Allowed');
    expect(text()).toContain('X holds member on fixture_notes.workspace:team');
    for (const root of roots.splice(0)) act(() => root.unmount());
    await settle();
    expect(calls.filter((call) => call.path === '/apps/bench/b1/close')).toHaveLength(1);
  });

  it('shows the stranded count of a change and cannot save it', async () => {
    const stranding = ok({ current: 1, next: 2, applies: false, stranded: [{ kind: 'fixture_notes.doc', relation: 'reader', count: 2 }], diff: { ...noDiff, relations_removed: [{ kind: 'fixture_notes.doc', name: 'reader' }] } });
    const calls = stub({ ['POST /apps/' + APP + '/schema/check']: () => stranding });
    await render(<SchemaBuilder app={APP} version={1} initial={documents} />);
    await click(button('Check the change'));
    expect(text()).toContain('2 standing grants of reader on fixture_notes.doc');
    expect(text()).toContain('Relations removed: reader on fixture_notes.doc');
    expect(button('Save version 2')?.hasAttribute('disabled')).toBe(true);
    await click(button('Save version 2'));
    expect(calls.filter((call) => call.method === 'PUT')).toHaveLength(0);
  });
});

describe('the Apps screen', () => {
  it('shows a pending app in words before approval and never its secret after', async () => {
    const pending = { id: APP, name: 'Notes fixture', state: 'pending', redirects: ['https://app.example.test/signed-in'], sign_in: null, schema: uploadedWorkspace, version: 0, versions: [], pending: null, client_id: null, service_account: null, client_credentials: [], registered_by: { kind: 'service_account', id: 'op-1' }, registered_at: 1 };
    // Approval keeps the registration's address as the sign-in settings, the name withheld.
    const signIn = { redirects: ['https://app.example.test/signed-in'], profile: false, operation: 'op-approve', by: { kind: 'start' }, at: 2 };
    let approved = false;
    const calls = stub({
      'GET /apps': () => ok({ apps: [approved ? { ...pending, state: 'approved', version: 1, versions: [1], client_id: APP, sign_in: signIn } : pending] }),
      ['POST /apps/' + APP + '/approve']: () => { approved = true; return ok({ app: { ...pending, state: 'approved', sign_in: signIn }, client: { client_id: APP, client_secret: 'f'.repeat(64), credential: 'lys-app.' + APP + '.' + 'f'.repeat(64) } }); },
    });
    await render(<Apps />);
    expect(text()).toContain('https://app.example.test/signed-in');
    const channel = document.querySelector('table[aria-label="Relations of fixture_notes.channel"]');
    // The one permissions matrix: a column for each relation, a row for each action.
    expect([...channel?.querySelectorAll('thead th') ?? []].map((cell) => cell.textContent)).toEqual(['fixture_notes.channel', 'poster']);
    expect([...channel?.querySelectorAll('tbody th[scope="row"]') ?? []].map((cell) => cell.textContent)).toEqual(['read', 'write']);
    expect([...channel?.querySelectorAll('tbody td') ?? []].map((cell) => cell.getAttribute('aria-label'))).toEqual(['poster may read', 'poster may write']);
    expect(channel?.querySelector('tfoot')?.textContent).toBe('What is held on fixture_notes.workspace reaches it.');
    await click(button('Approve Notes fixture'));
    // The approval sends the sign-in settings it offered: the registration's address, the name withheld.
    expect(calls.find((call) => call.method === 'POST')?.body).toEqual({ operation: expect.stringMatching(/^op-/), redirects: ['https://app.example.test/signed-in'], profile: false });
    // Credentials are issued below; nothing is offered to save.
    expect(labelled('Client credentials of ' + APP)).not.toBeNull();
    expect(document.body.innerHTML).not.toContain('f'.repeat(64));
    expect(text()).toContain('approved');
    expect(sessionStorage.getItem('lys.schema-draft.' + APP)).toBeNull();
    expect(JSON.stringify(Object.entries(localStorage))).not.toContain('f'.repeat(64));
  });
});
