import { act, type ReactNode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { renderToStaticMarkup } from 'react-dom/server';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { Apps } from '../src/features/apps/Apps';
import { SchemaBuilder, fromSchema, toSchema, type SchemaJson } from '../src/features/apps/SchemaBuilder';
import { SchemaMatrix } from '../src/features/apps/SchemaMatrix';
import { roleChanges } from '../src/features/apps/roleChanges';
import { ME } from './fixtures';

const app = 'fixture_notes';
const name = app + '.document';
const schema: SchemaJson = {
  kinds: {
    [name]: { actions: ['read', 'write', 'share'], relations: { editor: ['write', 'read'], viewer: ['read'] }, parents: [app + '.workspace'], hot: ['read'], agents: ['read', 'write'], roles: { auditor: ['read'] } },
    [app + '.workspace']: { actions: ['read'], relations: {}, parents: [], hot: [], agents: [], roles: {} },
  },
};
const roots: Root[] = [];

async function render(node: ReactNode) {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  roots.push(root);
  await act(async () => {
    root.render(node);
  });
  return container;
}

beforeEach(() => sessionStorage.clear());
afterEach(() => {
  for (const root of roots.splice(0)) act(() => root.unmount());
});

it('round-trips every server kind member including agents, hot and explicit empty values', () => {
  expect(toSchema(app, fromSchema(app, schema))).toEqual(schema);
});

it('preserves omitted optional members in an uploaded schema', () => {
  const minimal = { kinds: { [name]: { actions: ['read'] } } };
  expect(toSchema(app, fromSchema(app, minimal))).toEqual(minimal);
});

it('lets a person see and edit agent actions and keeps hot actions and roles when saving', async () => {
  const checked: unknown[] = [];
  vi.stubGlobal('fetch', async (_input: string, init?: RequestInit) => {
    checked.push(JSON.parse(String(init?.body)));
    return new Response(JSON.stringify({ current: 1, next: 2, applies: true, stranded: [], diff: { kinds_added: [], kinds_removed: [], relations_added: [], relations_removed: [], actions_added: [], actions_removed: [], parents_added: [], parents_removed: [] } }), { headers: { 'content-type': 'application/json' } });
  });
  const page = await render(<SchemaBuilder app={app} version={1} initial={schema} />);
  const read = page.querySelector<HTMLInputElement>('[aria-label="Agents may hold read on document"]');
  const write = page.querySelector<HTMLInputElement>('[aria-label="Agents may hold write on document"]');
  const share = page.querySelector<HTMLInputElement>('[aria-label="Agents may hold share on document"]');
  expect(read?.checked).toBe(true);
  expect(write?.checked).toBe(true);
  expect(share?.checked).toBe(false);
  if (!write || !share) throw new Error('Agent action choices were not shown');
  await act(async () => {
    write.click();
  });
  await act(async () => {
    share.click();
  });
  const check = page.querySelector<HTMLButtonElement>('[aria-label="Check the change"]');
  if (!check) throw new Error('Check the change was not shown');
  await act(async () => {
    check.click();
  });
  expect(checked).toEqual([{ replaces: 1, schema: { kinds: { ...schema.kinds, [name]: { ...schema.kinds[name], agents: ['read', 'share'] } } } }]);
});

it('removes a deleted action from agents without losing the remaining agent actions', () => {
  const draft = fromSchema(app, schema);
  const kind = draft.kinds.find((entry) => entry.name === 'document');
  if (!kind) throw new Error('Document kind was not opened');
  kind.actions = ['read', 'share'];
  expect(toSchema(app, draft).kinds[name]?.agents).toEqual(['read']);
});

it('carries agent permissions into a draft saved before the builder modeled them', async () => {
  sessionStorage.setItem('lys.schema-draft.' + app, JSON.stringify({ kinds: [{ name: 'document', actions: ['read', 'write', 'share'], relations: [], parents: [] }] }));
  const page = await render(<SchemaBuilder app={app} version={1} initial={schema} />);
  expect(page.querySelector<HTMLInputElement>('[aria-label="Agents may hold write on document"]')?.checked).toBe(true);
});

it('shows what agents may hold and which actions are hot in the permissions matrix', () => {
  const html = renderToStaticMarkup(<SchemaMatrix kinds={Object.entries(schema.kinds)} label="Its schema" />);
  const page = new DOMParser().parseFromString(html, 'text/html');
  const matrix = page.querySelector('[aria-label="Relations of ' + name + '"]');
  expect(matrix?.textContent).toContain('Agents may hold: read, write.');
  expect(matrix?.textContent).toContain('Hot actions (decided from the pass alone): read.');
});

it('names agent and hot action additions and removals before approval', () => {
  const before = { kinds: { [name]: { agents: ['read'], hot: ['write'] } } };
  const after = { kinds: { [name]: { agents: ['write'], hot: ['read'] } } };
  expect(roleChanges(before, after)).toEqual(['agents may now hold write on ' + name, 'agents may no longer hold read on ' + name, 'makes read hot on ' + name + ' (decided from the pass alone)', 'makes write deliberate on ' + name]);
  expect(roleChanges(after, after)).toEqual([]);
});

it('shows agent permissions and hot actions beside a pending schema approval', async () => {
  vi.stubGlobal('fetch', async (input: string) => {
    const answer =
      String(input) === '/api/apps'
        ? {
            apps: [
              {
                id: app,
                name: 'Notes',
                state: 'approved',
                redirects: [],
                sign_in: null,
                schema: { kinds: {} },
                version: 1,
                versions: [1],
                pending: { operation: 'op-change', replaces: 1, schema, by: { kind: 'service_account', id: app }, at: 2 },
                client_id: null,
                service_account: null,
                client_credentials: [],
                registered_by: { kind: 'start' },
                registered_at: 1,
              },
            ],
          }
        : ME;
    return new Response(JSON.stringify(answer), { headers: { 'content-type': 'application/json' } });
  });
  const page = await render(<Apps />);
  const approval = page.querySelector('[aria-label="Change waiting for ' + app + '"]');
  expect(approval?.textContent).toContain('agents may now hold write on ' + name);
  expect(approval?.textContent).toContain('makes read hot on ' + name);
  expect(approval?.textContent).toContain('Agents may hold: read, write.');
  expect(approval?.querySelector('[aria-label="Approve the change"]')).not.toBeNull();
});
