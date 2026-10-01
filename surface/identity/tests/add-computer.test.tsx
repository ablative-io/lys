import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { $, serve } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { Machine, NameMachine } from '../src/features/network/contract';

let root: Root | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { if (root) act(() => root?.unmount()); root = null; });
const pendingKey = 'lys.pending.machine.' + ADA;
const recorded = (body: NameMachine): Machine => ({ ...body, id: body.operation, may_run: body.may_run.map((id) => ({ id, display_name: 'Scribe', state: 'active' })), named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: null });

function service(runner: Route = (body) => ok({ machine: 'op-' + 'a'.repeat(32), ...(body as object) })) {
  const routes: Record<string, Route> = { ...SERVICE, '/network': ok({ machines: [], reports_served: true }) };
  routes['POST /network/machines'] = (body) => {
    const request = body as NameMachine;
    routes['POST /network/machines/' + request.operation + '/runner'] = typeof runner === 'function' ? (given) => {
      const answer = runner(given);
      return answer.status === 200 ? { ...answer, body: { ...(answer.body as object), machine: request.operation } } : answer;
    } : runner;
    return ok(recorded(request));
  };
  return routes;
}

async function open(routes: Record<string, Route>) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve(routes, posted);
  history.replaceState(null, '', '/#/network?add=computer');
  const container = document.createElement('div'); document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => { root?.render(<App />); });
  return { posted, requests };
}

async function remount(routes: Record<string, Route>) {
  if (root) act(() => root?.unmount()); root = null;
  document.body.innerHTML = '';
  return open(routes);
}

async function submit(name = 'Ward computer', twice = false) {
  const form = $('form[aria-label="Add a computer"]');
  const input = form?.querySelector<HTMLInputElement>('[name="name"]');
  if (!form || !input) throw new Error('The computer naming form is missing');
  input.value = name;
  await act(async () => {
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    if (twice) form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  });
}

async function retry() {
  const button = [...document.querySelectorAll('button')].find((entry) => entry.textContent === 'Check whether it was added');
  if (!button) throw new Error('The retained addition has no retry action');
  await act(async () => { button.click(); });
}

const legacy: NameMachine = { operation: 'op-' + 'c'.repeat(32), name: 'Earlier computer', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [SCRIBE], may_run_roles: ['role-one'], may_reach: ['mcp.example.test'] };

describe('Naming the computer Lys runs on', () => {
  it('asks only its name and explains why it starts empty', async () => {
    const { posted } = await open(service());
    const form = $('form[aria-label="Add a computer"]');
    expect([...form?.querySelectorAll('input') ?? []].map((input) => input.name)).toEqual(['name']);
    expect(form?.querySelector('[name="name"]')?.getAttribute('value')).not.toBeTruthy();
    expect(form?.querySelectorAll('textarea, select, input[type="checkbox"], input[type="search"]').length).toBe(0);
    expect(form?.textContent).toContain('Type a name for this computer.');
    expect(form?.textContent).not.toContain('Another computer');
    expect(posted).toEqual([]);
  });

  it('names a computer without granting any agent, role or host and checks its local runner', async () => {
    const { posted } = await open(service());
    await submit();
    expect(posted).toHaveLength(2);
    expect(posted[0].body).toEqual({ operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), name: 'Ward computer', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: [] });
    const id = (posted[0].body as NameMachine).operation;
    expect(posted[1]).toEqual({ path: '/network/machines/' + id + '/runner', body: { runner: { kind: 'lys' } } });
    expect(sessionStorage.getItem(pendingKey)).toBeNull();
    expect(document.body.textContent).toContain('Ward computer was added.');
  });

  it('keeps an unconfirmed local-runner phase across remount and does not name the machine twice', async () => {
    const routes = service(refused(503, 'RunnerUnavailable', 'The runner recording outcome is unknown'));
    const first = await open(routes); await submit();
    expect(first.posted).toHaveLength(2);
    expect(sessionStorage.getItem(pendingKey)).not.toBeNull();
    const original = first.posted[1];
    routes['POST ' + original.path] = (body) => ok({ machine: (first.posted[0].body as NameMachine).operation, ...(body as object) });
    const next = await remount(routes); await retry();
    expect(next.posted).toEqual([original]);
    expect(sessionStorage.getItem(pendingKey)).toBeNull();
    expect(document.body.textContent).toContain('Ward computer was added.');
  });

  it('migrates an old pending body without changing its agent, roles, hosts or operation and reads its runner', async () => {
    sessionStorage.setItem(pendingKey, JSON.stringify(legacy));
    const routes = service();
    routes['/network/machines/' + legacy.operation + '/runner'] = ok({ machine: legacy.operation, runner: { kind: 'dialled', key: 'd'.repeat(64) } });
    const { posted, requests } = await open(routes); await retry();
    expect(posted).toEqual([{ path: '/network/machines', body: legacy }]);
    expect(requests).toContain('/network/machines/' + legacy.operation + '/runner');
    expect(sessionStorage.getItem(pendingKey)).toBeNull();
    expect(document.body.textContent).toContain('Earlier computer was added.');
  });

  it.each([
    { name: 'missing', answer: ok({ machine: legacy.operation, runner: null }) },
    { name: 'unreadable', answer: ok({ machine: legacy.operation, runner: { kind: 'invented' } }) },
  ])('keeps a legacy addition unresolved when its runner is $name', async ({ answer }) => {
    sessionStorage.setItem(pendingKey, JSON.stringify(legacy));
    const routes = service(); routes['/network/machines/' + legacy.operation + '/runner'] = answer;
    const { posted } = await open(routes); await retry();
    expect(posted).toEqual([{ path: '/network/machines', body: legacy }]);
    expect(document.body.textContent).toContain('runner');
    expect(document.body.textContent).not.toContain('Lys will start agents on it');
    expect(sessionStorage.getItem(pendingKey)).not.toBeNull();
  });

  it('does not clear the local-runner phase when its answer names another machine', async () => {
    const { posted } = await open(service(ok({ machine: 'another-machine', runner: { kind: 'lys' } })));
    await submit();
    expect(posted).toHaveLength(2);
    expect(document.body.textContent).toContain('RunnerReceiptMismatch');
    expect(sessionStorage.getItem(pendingKey)).not.toBeNull();
    expect(document.body.textContent).not.toContain('Lys will start agents on it');
  });

  it('names corrupt pending state and sends no replacement request', async () => {
    sessionStorage.setItem(pendingKey, '{');
    const { posted } = await open(service());
    expect(document.body.textContent).toContain('PendingMachineUnreadable');
    expect($('fieldset')?.hasAttribute('disabled')).toBe(true);
    expect(posted).toEqual([]);
  });

  it('blocks a second submit while naming and runner recording are in progress', async () => {
    const { posted } = await open(service()); await submit('Ward computer', true);
    expect(posted.filter((entry) => entry.path === '/network/machines')).toHaveLength(1);
    expect(posted).toHaveLength(2);
    expect(sessionStorage.getItem(pendingKey)).toBeNull();
  });
});
