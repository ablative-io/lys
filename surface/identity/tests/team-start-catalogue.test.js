import { afterEach, describe, expect, test } from 'bun:test';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { JSDOM } from 'jsdom';
import * as start from '../src/features/run/Start';
import { ADA, SCRIBE, SCRIBE_VIEW } from './fixtures';

const machine = 'op-' + '1'.repeat(32);
const program = '/opt/agents/codex';
const catalogue = (builds, not_found) => ({ machine, programs: [{ name: 'Codex', builds, not_found }] });
const installed = { program, package: 'codex fixture', from: 'installed' };
const pendingKey = 'lys.pending.agent-start.' + ADA + '.' + SCRIBE;
const originalFetch = globalThis.fetch;
let root;
let dom;
afterEach(() => {
  if (root) act(() => root.unmount());
  root = undefined;
  globalThis.fetch = originalFetch;
  dom?.window.close();
  dom = undefined;
});

describe('the chosen computer determines whether its program is available', () => {
  test('reads only the chosen computer and accepts its installed executable', async () => {
    const asked = [];
    const answer = await start.readMachineHarness(machine, program, async (path) => {
      asked.push(path);
      return catalogue([installed]);
    });
    expect(asked).toEqual(['/harnesses?machine=' + encodeURIComponent(machine)]);
    expect(answer).toBeNull();
  });

  test('a profile declaration does not prove a computer has the program', async () => {
    const answer = await start.readMachineHarness(machine, program, async () => catalogue(
      [{ ...installed, from: 'profile' }], 'CommandNotFound: codex is not installed',
    ));
    expect(answer.refusal).toBe('harness_absent_on_machine');
    expect(answer.reason).toContain(machine);
    expect(answer.reason).toContain(program);
    expect(answer.reason).toContain('CommandNotFound');
  });

  test('an installed copy at another path does not prove the profile path exists', async () => {
    const answer = await start.readMachineHarness(machine, program, async () => catalogue(
      [{ ...installed, program: '/usr/bin/codex' }],
    ));
    expect(answer.refusal).toBe('harness_absent_on_machine');
    expect(answer.reason).toContain(program);
  });

  test('rejects an unbound catalogue and a catalogue for another computer', async () => {
    for (const wrong of [undefined, 'op-' + '2'.repeat(32)]) {
      const answer = await start.readMachineHarness(machine, program, async () => ({
        ...catalogue([installed]), machine: wrong,
      }));
      expect(answer.refusal).toBe('harness_catalogue_wrong_machine');
      expect(answer.reason).toContain(machine);
    }
  });

  test('propagates a refused runner read instead of offering a local copy', async () => {
    const refusal = new Error('runner_unreachable: the chosen computer did not answer');
    expect(start.readMachineHarness(machine, program, async () => { throw refusal; })).rejects.toBe(refusal);
  });

  async function showUnavailable(pending = null) {
    dom = new JSDOM('<!doctype html><div id="root"></div>', { url: 'http://identity.test/' });
    for (const name of ['window', 'document', 'location', 'sessionStorage', 'localStorage', 'HTMLElement']) {
      Object.defineProperty(globalThis, name, { configurable: true, writable: true, value: dom.window[name] });
    }
    globalThis.IS_REACT_ACT_ENVIRONMENT = true;
    if (pending) sessionStorage.setItem(pendingKey, JSON.stringify(pending));
    const asked = [];
    const routes = {
      ['/directory/agents/' + SCRIBE]: SCRIBE_VIEW,
      '/roles': { roles: [] },
      '/network': { machines: [{ id: machine, name: 'Ward computer', state: 'in_use', runtime: 'lys-runner', may_run: [{ id: SCRIBE }], may_run_roles: [] }], reports_served: true },
      ['/agents/' + SCRIBE + '/provisioning']: { profile: { version: 1, reviewed_by: ADA, runs_on: machine, working_folder: '/work', harness: { name: 'Codex', program } } },
      ['/harnesses?machine=' + machine]: catalogue([], 'CommandNotFound: codex is not installed'),
    };
    globalThis.fetch = async (path, options) => {
      const route = String(path).replace(/^\/api/, '');
      asked.push((options?.method ?? 'GET') + ' ' + route);
      if (!(route in routes)) throw new Error('unexpected request: ' + route);
      return new Response(JSON.stringify(routes[route]), { status: 200, headers: { 'content-type': 'application/json' } });
    };
    root = createRoot(document.getElementById('root'));
    await act(async () => root.render(createElement(start.Start, {
      entry: { id: SCRIBE, display_name: 'Scribe', person: { id: ADA } },
      me: ADA, admin: false, changed: () => {}, settings: () => {}, straightAway: true,
    })));
    return asked;
  }

  test('shows an unavailable program with its reason and sends no start or review', async () => {
    const asked = await showUnavailable();
    expect(asked).toContain('GET /harnesses?machine=' + machine);
    expect(document.querySelector('button').disabled).toBe(true);
    expect(document.querySelector('[role="alert"]').textContent).toContain('Ward computer');
    expect(document.querySelector('[role="alert"]').textContent).toContain(program);
    expect(document.querySelector('[role="alert"]').textContent).toContain('CommandNotFound');
    expect(asked.filter((path) => path.startsWith('POST '))).toEqual([]);
  });

  test('keeps an earlier uncertain start available for replay when its program disappears', async () => {
    const pending = {
      stage: 'start', path: '/agents/' + SCRIBE + '/start-command',
      body: { machine, operation: 'op-' + '2'.repeat(32) }, machine, version: 1,
    };
    const asked = await showUnavailable(pending);
    expect(asked.filter((path) => path.startsWith('POST '))).toEqual(['POST ' + pending.path]);
    expect(document.querySelector('button').disabled).toBe(false);
    expect(sessionStorage.getItem(pendingKey)).toBe(JSON.stringify(pending));
  });
});
