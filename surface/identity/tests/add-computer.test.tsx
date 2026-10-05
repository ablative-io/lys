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

/** Whether the add row takes nothing: its name box and its button are both disabled, or neither is. */
function locked(): boolean {
  const name = $('form[aria-label="Add a computer"] [name="name"]');
  const button = $('button[type="submit"][form="' + $('form[aria-label="Add a computer"]')?.id + '"]');
  expect(name?.hasAttribute('disabled')).toBe(button?.hasAttribute('disabled'));
  return Boolean(name?.hasAttribute('disabled'));
}

async function retry() {
  const button = [...document.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === 'Check whether it was added');
  if (!button) throw new Error('The retained addition has no retry action');
  await act(async () => { button.click(); });
}

function plain(code: string) {
  const details = [...document.querySelectorAll('.refusal-name')].find((entry) => entry.textContent?.includes(code));
  expect(details?.textContent).toContain(code);
  const face = document.body.cloneNode(true) as HTMLElement;
  for (const detail of face.querySelectorAll('.refusal-name')) detail.remove();
  expect(face.textContent).not.toContain(code);
  expect(face.textContent).toContain('Lys could not confirm this computer addition.');
}

const legacy: NameMachine = { operation: 'op-' + 'c'.repeat(32), name: 'Earlier computer', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [SCRIBE], may_run_roles: ['role-one'], may_reach: ['mcp.example.test'] };

describe('Naming the computer Lys runs on', () => {
  it('asks only its name and explains why it starts empty', async () => {
    const { posted } = await open(service());
    const form = $('form[aria-label="Add a computer"]');
    expect([...form?.querySelectorAll('input') ?? []].map((input) => input.name)).toEqual(['name']);
    expect(form?.querySelector('[name="name"]')?.getAttribute('value')).not.toBeTruthy();
    expect(form?.querySelectorAll('textarea, select, input[type="checkbox"], input[type="search"]').length).toBe(0);
    expect(form?.textContent).not.toContain('Type a name for this computer.');
    expect(form?.textContent).not.toContain('Give this computer a name');
    expect(form?.textContent).not.toContain('Another computer');
    expect(posted).toEqual([]);
  });

  it('says a name is needed only after a submit without one, and sends nothing', async () => {
    const { posted } = await open(service());
    expect(document.body.textContent).not.toContain('Give this computer a name');
    await submit('   ');
    expect(document.body.textContent).toContain('Give this computer a name');
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

  it.each([
    { status: 400, code: 'RequestMalformed' },
    { status: 403, code: 'NotAdmitted' },
    { status: 409, code: 'MachineReused' },
  ])('releases a definite first machine rejection $code and permits a corrected request', async ({ status, code }) => {
    const routes = service();
    const nameMachine = routes['POST /network/machines'];
    routes['POST /network/machines'] = refused(status, code, 'The computer request was rejected');
    const { posted } = await open(routes); await submit();
    expect(posted).toHaveLength(1);
    expect(sessionStorage.getItem(pendingKey)).toBeNull();
    expect(locked()).toBe(false);
    expect(document.body.textContent).not.toContain('Ward computer was added.');
    plain(code);
    routes['POST /network/machines'] = nameMachine;
    await submit('Corrected computer');
    expect(posted).toHaveLength(3);
    expect(posted[1].body).toMatchObject({ name: 'Corrected computer' });
    expect((posted[1].body as NameMachine).operation).not.toBe((posted[0].body as NameMachine).operation);
    expect(sessionStorage.getItem(pendingKey)).toBeNull();
    expect(document.body.textContent).toContain('Corrected computer was added.');
  });

  it('retains a runner-phase rejection and retries only the same runner request', async () => {
    const routes = service(refused(403, 'NotAdmitted', 'The runner request was rejected'));
    const { posted } = await open(routes); await submit();
    expect(posted).toHaveLength(2);
    expect(sessionStorage.getItem(pendingKey)).not.toBeNull();
    expect(locked()).toBe(true);
    expect(document.body.textContent).not.toContain('Ward computer was added.');
    plain('NotAdmitted');
    const original = posted[1];
    const computer = (posted[0].body as NameMachine).operation;
    routes['POST ' + original.path] = (body) => ok({ machine: computer, ...(body as object) });
    await retry();
    expect(posted).toEqual([posted[0], original, original]);
    expect(sessionStorage.getItem(pendingKey)).toBeNull();
    expect(document.body.textContent).toContain('Ward computer was added.');
  });

  it('retains an unknown original when a later manual machine retry is rejected', async () => {
    const routes = service(); routes['POST /network/machines'] = refused(503, 'NetworkUnavailable', 'The original outcome is unknown');
    const { posted } = await open(routes); await submit();
    expect(posted).toHaveLength(1);
    const original = posted[0]; const pending = sessionStorage.getItem(pendingKey);
    routes['POST /network/machines'] = refused(403, 'NotAdmitted', 'The retry is not admitted');
    await retry();
    expect(posted).toEqual([original, original]);
    expect(sessionStorage.getItem(pendingKey)).toBe(pending);
    expect(locked()).toBe(true);
    expect(document.body.textContent).not.toContain('Ward computer was added.');
    plain('NotAdmitted');
  });

  it('retains a machine request when a 4xx answer names no definite refusal', async () => {
    const routes = service(); routes['POST /network/machines'] = { status: 403, body: 'Unreadable refusal' };
    const { posted } = await open(routes); await submit();
    expect(posted).toHaveLength(1);
    expect(sessionStorage.getItem(pendingKey)).not.toBeNull();
    expect(locked()).toBe(true);
    plain('Unanswered');
  });

  it('keeps an unconfirmed local-runner phase across remount and does not name the machine twice', async () => {
    const routes = service(refused(503, 'RunnerUnavailable', 'The runner recording outcome is unknown'));
    const first = await open(routes); await submit();
    expect(first.posted).toHaveLength(2);
    expect(sessionStorage.getItem(pendingKey)).not.toBeNull();
    plain('RunnerUnavailable');
    const original = first.posted[1];
    routes['POST ' + original.path] = (body) => ok({ machine: (first.posted[0].body as NameMachine).operation, ...(body as object) });
    const next = await remount(routes); await retry();
    expect(next.posted).toEqual([original]);
    expect(sessionStorage.getItem(pendingKey)).toBeNull();
    expect(document.body.textContent).toContain('Ward computer was added.');
  });

  it('reads one saved format: an earlier saved addition is said to be unreadable, is kept as it was, and nothing is sent', async () => {
    const raw = JSON.stringify(legacy); sessionStorage.setItem(pendingKey, raw);
    const { posted } = await open(service());
    expect(document.body.textContent).toContain('The retained computer addition cannot be read');
    expect(posted).toEqual([]);
    expect(sessionStorage.getItem(pendingKey)).toBe(raw);
  });

  it('does not clear the local-runner phase when its answer names another machine', async () => {
    const { posted } = await open(service(ok({ machine: 'another-machine', runner: { kind: 'lys' } })));
    await submit();
    expect(posted).toHaveLength(2);
    expect(document.body.textContent).toContain('RunnerReceiptMismatch');
    expect(sessionStorage.getItem(pendingKey)).not.toBeNull();
    expect(document.body.textContent).not.toContain('Lys will start agents on it');
    plain('RunnerReceiptMismatch');
  });

  it('names corrupt pending state and sends no replacement request', async () => {
    sessionStorage.setItem(pendingKey, '{');
    const { posted } = await open(service());
    expect(document.body.textContent).toContain('PendingMachineUnreadable');
    expect(locked()).toBe(true);
    expect(posted).toEqual([]);
    plain('PendingMachineUnreadable');
  });

  it('blocks a second submit while naming and runner recording are in progress', async () => {
    const { posted } = await open(service()); await submit('Ward computer', true);
    expect(posted.filter((entry) => entry.path === '/network/machines')).toHaveLength(1);
    expect(posted).toHaveLength(2);
    expect(sessionStorage.getItem(pendingKey)).toBeNull();
  });
});

describe('Adding another computer', () => {
  const CODE = '7'.repeat(64);
  const command = (id: string) => 'lys runner join --server https://lys.example.test --machine ' + id;
  /** The service as `service()` serves it, with a join-code route for every computer named that answers `code`. */
  function remote(code: (id: string) => Route = (id) => ok({ machine: id, server: 'https://lys.example.test', command: command(id), code: CODE })) {
    const routes = service();
    const name = routes['POST /network/machines'];
    routes['POST /network/machines'] = (body) => {
      const id = (body as NameMachine).operation;
      routes['POST /network/machines/' + id + '/join-code'] = code(id);
      return typeof name === 'function' ? name(body) : name;
    };
    return routes;
  }
  const choose = async (label: string) => {
    const choice = [...document.querySelectorAll('[aria-label="Which computer"] button')].find((each) => (each.getAttribute('aria-label') ?? each.textContent) === label);
    if (!(choice instanceof HTMLElement)) throw new Error('The choice ' + label + ' is missing');
    await act(async () => { choice.click(); });
  };
  const kept = () => Object.keys(sessionStorage).map((key) => sessionStorage.getItem(key) ?? '').join('\n');
  const button = (label: string) => [...document.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent) === label) ?? null;

  it('offers this computer or another in a cell of its own, this one chosen first', async () => {
    await open(service());
    const cells = [...($('tr[data-add="computer"]')?.children ?? [])];
    const choice = cells[2]?.querySelector('[aria-label="Which computer"]');
    expect([...choice?.querySelectorAll('button') ?? []].map((each) => each.getAttribute('aria-pressed'))).toEqual(['true', 'false']);
    await choose('Another computer');
    expect([...choice?.querySelectorAll('button') ?? []].map((each) => each.getAttribute('aria-pressed'))).toEqual(['false', 'true']);
    expect([cells[3]?.querySelector('button')?.getAttribute('aria-label'), cells[3]?.textContent]).toEqual(['Add and get its code', 'Add']);
  });

  it('names it, asks for its code, and shows the command and the code once, each with Copy, keeping neither', async () => {
    const writes: string[] = [];
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async (value: string) => { writes.push(value); } } });
    const { posted } = await open(remote());
    await choose('Another computer');
    await submit();
    const id = (posted[0].body as NameMachine).operation;
    expect(posted[0].body).toMatchObject({ name: 'Ward computer', runtime: 'lys-runner' });
    expect(posted[1]).toEqual({ path: '/network/machines/' + id + '/join-code', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/) } });
    expect(posted).toHaveLength(2);
    expect(posted.some((entry) => entry.path.endsWith('/runner'))).toBe(false);
    const row = $('tr[data-add="connect"]');
    expect(row?.textContent).toContain('Run this on that computer. The code works once and is not shown again.');
    expect(row?.querySelector('[data-join="command"]')?.textContent).toBe(command(id));
    expect(row?.querySelector('[data-join="code"]')?.textContent).toBe(CODE);
    const copies = [...row?.querySelectorAll<HTMLButtonElement>('button') ?? []].filter((each) => each.dataset.symbol === 'copy');
    expect(copies).toHaveLength(2);
    expect(copies.every((each) => each.getAttribute('aria-label')?.startsWith('Copy '))).toBe(true);
    await act(async () => { $('button[aria-label="Copy the command"]')?.click(); });
    await act(async () => { $('button[aria-label="Copy the code"]')?.click(); });
    expect(writes).toEqual([command(id), CODE]);
    expect(kept()).not.toContain(CODE);
    expect(sessionStorage.getItem(pendingKey)).toBeNull();
    await act(async () => { button('Done')?.click(); });
    expect(document.body.textContent).not.toContain(CODE);
  });

  it('keeps an unanswered naming by its operation and asks for the code only once it is confirmed', async () => {
    const routes = remote();
    const name = routes['POST /network/machines'];
    routes['POST /network/machines'] = refused(503, 'NetworkUnavailable', 'The outcome is unknown');
    const { posted } = await open(routes);
    await choose('Another computer');
    await submit();
    expect(posted).toHaveLength(1);
    expect(JSON.parse(sessionStorage.getItem(pendingKey) ?? '{}')).toMatchObject({ phase: 'machine', remote: true });
    routes['POST /network/machines'] = name;
    await retry();
    const id = (posted[0].body as NameMachine).operation;
    expect(posted.map((entry) => entry.path)).toEqual(['/network/machines', '/network/machines', '/network/machines/' + id + '/join-code']);
    expect(posted[1].body).toEqual(posted[0].body);
    expect(sessionStorage.getItem(pendingKey)).toBeNull();
    expect($('tr[data-add="connect"] [data-join="code"]')?.textContent).toBe(CODE);
  });

  it('offers a new code when the answer with the code is lost, and the new one replaces it', async () => {
    let answer: Route = refused(503, 'NetworkUnavailable', 'The answer was lost');
    const { posted } = await open(remote(() => (body) => typeof answer === 'function' ? answer(body) : answer));
    await choose('Another computer');
    await submit();
    expect(posted).toHaveLength(2);
    const row = $('tr[data-add="connect"]');
    expect(row?.textContent).toContain('If one was given, it is not shown again: a new code replaces it.');
    expect(row?.querySelector('[data-join="code"]')).toBeNull();
    const id = (posted[0].body as NameMachine).operation;
    answer = ok({ machine: id, server: 'https://lys.example.test', command: command(id), code: CODE });
    await act(async () => { button('Get a new connection code')?.click(); });
    expect(posted).toHaveLength(3);
    expect(posted[2].path).toBe('/network/machines/' + id + '/join-code');
    expect((posted[2].body as { operation: string }).operation).not.toBe((posted[1].body as { operation: string }).operation);
    expect($('tr[data-add="connect"] [data-join="code"]')?.textContent).toBe(CODE);
    expect(kept()).not.toContain(CODE);
  });

  it('reads a kept addition of another computer in one format only', async () => {
    const raw = JSON.stringify({ version: 1, body: legacy, phase: 'runner', machine: recorded(legacy), remote: true });
    sessionStorage.setItem(pendingKey, raw);
    const { posted } = await open(service());
    expect(document.body.textContent).toContain('PendingMachineUnreadable');
    expect(posted).toEqual([]);
  });
});
