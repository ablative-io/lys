/** A person's computer controls send the same requests without an administrator. */
import { act, createRef } from 'react';
import { createRoot } from 'react-dom/client';
import type { ReactNode } from 'react';
import { afterEach, expect, it } from 'vitest';
import { $, click, serve, settle } from './harness';
import { ADA, OWN, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Connecting } from '../src/features/network/JoinCode';
import type { Route } from './fixtures';
import type { Machine, NameMachine } from '../src/features/network/contract';
import { AddComputerRow } from '../src/features/network/AddMachine';
import { MachineDetail } from '../src/features/network/MachineDetail';

let root: ReturnType<typeof createRoot> | null = null;
afterEach(() => { if (root) act(() => root?.unmount()); root = null; sessionStorage.clear(); });
const machine: Machine = { id: 'op-' + 'a'.repeat(32), name: 'Own laptop', kind: 'Computer',
  runtime: 'lys-runner', slots: 0, may_run: [], may_reach: [], named_by: ADA,
  named_at: 1, state: 'in_use', retired_at: null, last_report_at: null };
const computer = { machine, asked: 'answered' as const, runner: null, answers: null, reason: '', refusal: '', joining: false, running: [] };

async function draw(element: ReactNode, routes: Record<string, Route>) {
  const posted: { path: string; body: unknown }[] = [];
  serve({ ...SERVICE, '/people': ok(OWN), ...routes }, posted);
  const container = document.createElement('div'); document.body.append(container);
  root = createRoot(container);
  await act(async () => root?.render(element));
  await settle();
  return posted;
}

it('adds a self-service computer as a remote runner and gets its code without assigning the server runner', async () => {
  const routes: Record<string, Route> = {};
  routes['POST /network/machines'] = (body) => {
    const named = body as NameMachine;
    routes['POST /network/machines/' + named.operation + '/join-code'] = ok({
      machine: named.operation, server: 'https://lys.example/api',
      command: 'lys runner join --server https://lys.example/api --machine ' + named.operation,
      code: 'once-only-code',
    });
    return ok({ ...machine, ...named, id: named.operation });
  };
  const connected: unknown[] = [];
  const posted = await draw(<table><tfoot><AddComputerRow person={ADA} selfService changed={() => {}} name={createRef()} connect={(value) => connected.push(value)} /></tfoot></table>, routes);
  const field = $('input[name="name"]');
  if (!(field instanceof HTMLInputElement)) throw new Error('missing computer name');
  field.value = 'Own laptop';
  const form = $('form[aria-label="Add a computer"]');
  if (!form) throw new Error('missing addition');
  await act(async () => form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })));
  await settle();
  expect(posted.map((entry) => entry.path)).toEqual(['/network/machines', expect.stringMatching(/\/join-code$/)]);
  expect(connected).toHaveLength(1);
  expect(JSON.stringify([...Array(sessionStorage.length)].map((_, index) => sessionStorage.getItem(sessionStorage.key(index) ?? '')))).not.toContain('once-only-code');
});

it('offers an own computer connection code and names a refused code', async () => {
  const connections: Connecting[] = [];
  const posted = await draw(<MachineDetail computer={computer} admin={false} me={ADA} teams={[]} names={new Map()} changed={() => {}} connecting={null} connect={(value) => connections.push(value)} done={() => {}} />,
    { ['POST /network/machines/' + machine.id + '/join-code']: refused(403, 'not_your_computer', 'this computer belongs to someone else') });
  await click($('button[aria-label="Get a new connection code"]'));
  expect(posted.at(-1)?.path).toBe('/network/machines/' + machine.id + '/join-code');
  expect(connections.at(-1)?.failure).toBe('You do not manage this computer. Ask its owner or the administrator to make the change.');
  expect(connections.at(-1)?.detail).toBe('not_your_computer: this computer belongs to someone else');
});

it('offers only the person’s own agents on their computer and sends an allowance', async () => {
  const posted = await draw(<MachineDetail computer={computer} admin={false} me={ADA} teams={[]} names={new Map()} changed={() => {}} connecting={null} connect={() => {}} done={() => {}} />,
    { ['POST /network/machines/' + machine.id + '/agents']: (body) => ok({ machine: { ...machine, may_run: [{ id: SCRIBE, display_name: 'Scribe', state: 'active' }] }, recorded: body }) });
  const select = $('select[aria-label="Agent to allow on this computer"]');
  expect(select?.textContent).toContain('Scribe');
  expect(select?.textContent).not.toContain('Reviewer');
  await click($('button[aria-label="Allow Scribe"]'));
  expect(posted.at(-1)).toEqual({ path: '/network/machines/' + machine.id + '/agents',
    body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), agent: SCRIBE, allow: true } });
});

it('shows no self-service code or allowance controls on someone else’s computer', async () => {
  await draw(<MachineDetail computer={{ ...computer, machine: { ...machine, named_by: 'person-' + 'b'.repeat(32) } }} admin={false} me={ADA} teams={[]} names={new Map()} changed={() => {}} connecting={null} connect={() => {}} done={() => {}} />, {});
  expect($('button[aria-label="Get a new connection code"]')).toBeNull();
  expect($('select[aria-label="Agent to allow on this computer"]')).toBeNull();
});
