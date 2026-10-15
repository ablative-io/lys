/** Getting started on an empty install: the steps it lists, declining this computer, and adding it for your own agents. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GettingStarted } from '../src/features/dashboard/GettingStarted';
import { $, serve } from './harness';
import { ADA, ARCHIVIST, COURIER, SCRIBE, SERVICE, ok } from './fixtures';
import type { Route } from './fixtures';
import type { Machine, NameMachine } from '../src/features/network/contract';

let root: Root | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { if (root) act(() => root?.unmount()); root = null; document.body.innerHTML = ''; });
const empty = { machines: 0, cambium: null, read: true };

function service(): Record<string, Route> {
  const routes: Record<string, Route> = { ...SERVICE, '/network': ok({ machines: [], reports_served: true, host_name: 'Ada’s MacBook Pro' }) };
  routes['POST /network/machines'] = (body) => {
    const request = body as NameMachine;
    const machine: Machine = { ...request, id: request.operation, may_run: request.may_run.map((id) => ({ id, display_name: id, state: 'active' })), named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: null };
    routes['POST /network/machines/' + request.operation + '/runner'] = ok({ machine: request.operation, runner: { kind: 'lys' } });
    return ok(machine);
  };
  return routes;
}

async function open(routes: Record<string, Route>, reread = () => undefined) {
  const posted: { path: string; body: unknown }[] = [];
  serve(routes, posted);
  const container = document.createElement('div'); document.body.appendChild(container); root = createRoot(container);
  await act(async () => { root?.render(<GettingStarted person={ADA} agents={0} standing={empty} reread={reread} />); });
  return { posted };
}

describe('Getting started on an empty install', () => {
  it('lists every step a fresh Lys still needs, in order, with this computer’s own name offered', async () => {
    const { posted } = await open(service());
    const panel = $('[aria-label="Getting started"]');
    expect([...panel?.querySelectorAll('li > b') ?? []].map((title) => title.textContent)).toEqual(['Choose where agents run', 'Let people sign in with Google', 'Connect Cambium', 'Add your first agent']);
    expect(panel?.textContent).toContain('0 of 4 done');
    expect(($('[aria-label="Run agents on this computer"] [name="name"]') as HTMLInputElement | null)?.value).toBe('Ada’s MacBook Pro');
    expect(posted).toEqual([]);
  });

  it('declines this computer with Not this one and sends nothing', async () => {
    const { posted } = await open(service());
    const decline = [...document.querySelectorAll('button')].find((button) => (button.getAttribute('aria-label') ?? button.textContent) === 'Not this one');
    await act(async () => { decline?.click(); });
    expect(document.body.textContent).toContain('Agents won’t run on this computer.');
    expect($('[aria-label="Run agents on this computer"]')).toBeNull();
    expect(posted).toEqual([]);
  });

  it('adds this computer for the person’s own agents with Yes, checks its runner and reads the list again', async () => {
    const reread = vi.fn();
    const { posted } = await open(service(), reread);
    const form = $('[aria-label="Run agents on this computer"]');
    await act(async () => { form?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
    expect(posted).toHaveLength(2);
    expect(posted[0]).toEqual({ path: '/network/machines', body: { operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), name: 'Ada’s MacBook Pro', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [SCRIBE, COURIER, ARCHIVIST], may_run_roles: [], may_reach: [] } });
    expect(posted[1]).toEqual({ path: '/network/machines/' + (posted[0].body as NameMachine).operation + '/runner', body: { runner: { kind: 'lys' } } });
    expect(reread).toHaveBeenCalledTimes(1);
  });
});
