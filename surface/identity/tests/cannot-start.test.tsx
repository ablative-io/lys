import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { CannotStart, whyNot } from '../src/features/runtime/CannotStart';
import type { StartRefusal } from '../src/features/runtime/CannotStart';
import type { Machine } from '../src/features/network/contract';
import { click, serve, settle } from './harness';
import { ADA, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

let root: Root | null = null;
let host: HTMLElement | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { if (root) act(() => root?.unmount()); root = null; host?.remove(); host = null; });

const computer = 'op-' + '1'.repeat(32);
const machine = (id: string, name: string): Machine => ({
  id, name, kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: [],
  named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: null,
});
const notActive = (state: string): StartRefusal => ({ refusal: 'AgentNotActive', reason: 'the agent is ' + state + ' and only an active agent is started' });

/** The component alone, over a stubbed service; `again` counts the starts it asked for. */
async function shown(refusal: StartRefusal, routes: Record<string, Route> = {}) {
  const posted: { path: string; body: unknown }[] = [];
  serve({ ...SERVICE, ...routes }, posted);
  const calls = { again: 0 };
  const shownIn = document.createElement('div');
  document.body.append(shownIn);
  host = shownIn;
  root = createRoot(shownIn);
  await act(async () => { root?.render(<CannotStart agent={SCRIBE} name="Scribe" refusal={refusal} again={() => { calls.again += 1; }} />); });
  await settle();
  return { host: shownIn, posted, calls };
}
const buttons = (scope: HTMLElement) => [...scope.querySelectorAll('button, a.btn')].map((entry) => entry.textContent?.trim());

describe('why an agent cannot start', () => {
  it('says each reason in one sentence and names the one act that fixes it', () => {
    expect(whyNot('Scribe', notActive('registered'))).toEqual({ sentence: 'Scribe is not turned on yet.', fix: { kind: 'turn-on', label: 'Turn on', transition: 'activate' } });
    expect(whyNot('Scribe', notActive('suspended'))).toEqual({ sentence: 'Scribe is paused.', fix: { kind: 'turn-on', label: 'Turn back on', transition: 'reinstate' } });
    expect(whyNot('Scribe', notActive('retired'))).toEqual({ sentence: 'Scribe is retired and cannot be started again.', fix: { kind: 'nothing' } });
    expect(whyNot('Scribe', { refusal: 'WorkingFolderUnnamed', reason: 'the launch names no working folder' })).toEqual({ sentence: 'Scribe has no folder to work in.', fix: { kind: 'choose-folder' } });
    for (const refusal of ['MachineNotForAgent', 'MachineUnavailable']) {
      expect(whyNot('Scribe', { refusal, reason: 'no computer admits this agent' })).toEqual({ sentence: 'No computer is allowed to run Scribe.', fix: { kind: 'allow-computer' } });
    }
  });

  it('gives any other reason as a sentence with its name off the front, and offers the same start again', () => {
    expect(whyNot('Scribe', { refusal: 'ProfileNotReviewed', reason: 'ProfileNotReviewed: version 3 of the agent\'s profile is not reviewed; review it before the agent is started' }))
      .toEqual({ sentence: 'Version 3 of the agent\'s profile is not reviewed; review it before the agent is started.', fix: { kind: 'try-again' } });
    expect(whyNot('Scribe', { refusal: 'Unanswered', reason: '' }).sentence).toBe('This agent could not be started.');
  });

  it('shows the sentence open, never folded, with the refusal name small at its end', async () => {
    const { host } = await shown(notActive('suspended'));
    expect(host.querySelector('details')).toBeNull();
    expect(host.querySelector('[role="alert"] p')?.textContent).toBe('Scribe is paused. AgentNotActive');
    expect(host.querySelector('small.refusal-name')?.textContent).toBe('AgentNotActive');
    expect(buttons(host)).toEqual(['Turn back on']);
  });

  it('turns a paused agent back on in place and then starts it again, with no second press', async () => {
    const path = '/identities/' + SCRIBE + '/transitions';
    const { host, posted, calls } = await shown(notActive('suspended'), { ['POST ' + path]: ok({}) });
    await click(host.querySelector('button'));
    expect(posted).toEqual([{ path, body: { transition: 'reinstate', reason: 'Turn back on from the start screen, to start Scribe' } }]);
    expect(calls.again).toBe(1);
  });

  it('turns on an agent that was never turned on', async () => {
    const path = '/identities/' + SCRIBE + '/transitions';
    const { host, posted, calls } = await shown(notActive('registered'), { ['POST ' + path]: ok({}) });
    expect(buttons(host)).toEqual(['Turn on']);
    await click(host.querySelector('button'));
    expect(posted[0]).toEqual({ path, body: { transition: 'activate', reason: 'Turn on from the start screen, to start Scribe' } });
    expect(calls.again).toBe(1);
  });

  it('offers nothing for a retired agent', async () => {
    const { host } = await shown(notActive('retired'));
    expect(host.textContent).toContain('Scribe is retired and cannot be started again.');
    expect(buttons(host)).toEqual([]);
  });

  it('says so when turning on is itself refused, keeps the button and does not start', async () => {
    const path = '/identities/' + SCRIBE + '/transitions';
    const { host, calls } = await shown(notActive('suspended'), { ['POST ' + path]: refused(403, 'NotAdmitted', 'only an administrator moves an identity\'s state') });
    await click(host.querySelector('button'));
    expect(host.textContent).toContain('That did not work. Only an administrator moves an identity\'s state. NotAdmitted');
    expect(buttons(host)).toEqual(['Turn back on']);
    expect(calls.again).toBe(0);
  });

  it('allows the agent on the one computer in place and then starts it again', async () => {
    const path = '/network/machines/' + computer + '/agents';
    const ward = machine(computer, 'Ward computer');
    const { host, posted, calls } = await shown({ refusal: 'MachineNotForAgent', reason: 'the machine may not run this agent' }, {
      '/network': ok({ machines: [ward], reports_served: true }),
      ['POST ' + path]: (body) => ok({ machine: { ...ward, may_run: [{ id: SCRIBE, display_name: 'Scribe', state: 'active' }] }, recorded: { ...(body as object), machine: computer, by: ADA, at: 1 } }),
    });
    expect(host.textContent).toContain('No computer is allowed to run Scribe.');
    expect(buttons(host)).toEqual(['Allow on this computer']);
    expect(host.textContent).toContain('Scribe will be allowed to run on Ward computer.');
    await click(host.querySelector('button'));
    expect(posted).toHaveLength(1);
    expect(posted[0].path).toBe(path);
    expect(posted[0].body).toMatchObject({ agent: SCRIBE, allow: true });
    expect(calls.again).toBe(1);
  });

  it('names each computer when there are several, and sends the same request on a second press', async () => {
    const other = 'op-' + '2'.repeat(32);
    const path = '/network/machines/' + other + '/agents';
    const { host, posted, calls } = await shown({ refusal: 'MachineUnavailable', reason: 'no computer admits this agent' }, {
      '/network': ok({ machines: [machine(computer, 'Ward computer'), machine(other, 'Lab computer'), { ...machine('op-' + '3'.repeat(32), 'Old computer'), state: 'retired' }], reports_served: true }),
      ['POST ' + path]: refused(503, 'NetworkUnavailable', 'the network record could not be written'),
    });
    expect(buttons(host)).toEqual(['Allow on Ward computer', 'Allow on Lab computer']);
    const lab = [...host.querySelectorAll('button')].find((entry) => entry.textContent?.includes('Lab'));
    await click(lab ?? null);
    await click(lab ?? null);
    expect(posted.map((entry) => entry.path)).toEqual([path, path]);
    expect(posted[1].body).toEqual(posted[0].body);
    expect(host.textContent).toContain('That did not work. The network record could not be written. NetworkUnavailable');
    expect(calls.again).toBe(0);
  });

  it('says there is no computer with Lys on it and where to add one', async () => {
    const { host } = await shown({ refusal: 'MachineUnavailable', reason: 'no computer admits this agent' }, { '/network': ok({ machines: [], reports_served: true }) });
    expect(host.textContent).toContain('No computer has Lys running on it yet.');
    expect(host.querySelector('a.btn')?.getAttribute('href')).toBe('#/network');
  });

  it('offers the same start again for a reason it has no fix for', async () => {
    const { host, calls } = await shown({ refusal: 'RuntimeUnavailable', reason: 'the runner did not answer' });
    expect(host.querySelector('[role="alert"] p')?.textContent).toBe('The runner did not answer. RuntimeUnavailable');
    await click(host.querySelector('button'));
    expect(calls.again).toBe(1);
  });
});
