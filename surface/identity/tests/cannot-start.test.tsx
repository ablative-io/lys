import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { CannotStart, whyNot } from '../src/features/runtime/CannotStart';
import type { StartRefusal } from '../src/features/runtime/CannotStart';
import type { Machine, NetworkView } from '../src/features/network/contract';
import { request } from '../src/api';
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
  // The computers come from the start that was refused, which read them; here that read is made once, as Start makes it.
  const { machines } = await request<NetworkView>('/network').catch(() => ({ machines: [] as NetworkView['machines'] }));
  await act(async () => { root?.render(<CannotStart agent={SCRIBE} name="Scribe" refusal={refusal} machines={machines} again={() => { calls.again += 1; }} />); });
  await settle();
  return { host: shownIn, posted, calls };
}
const buttons = (scope: HTMLElement) => [...scope.querySelectorAll('button, a.btn')].map((entry) => (entry.getAttribute('aria-label') ?? entry.textContent)?.trim());

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
    expect(host.querySelector('a.btn')?.getAttribute('href')).toBe('#/network?add=computer');
  });

  it('offers the same start again for a reason it has no fix for', async () => {
    const { host, calls } = await shown({ refusal: 'RuntimeUnavailable', reason: 'the runner did not answer' });
    expect(host.querySelector('[role="alert"] p')?.textContent).toBe('The runner did not answer. RuntimeUnavailable');
    await click(host.querySelector('button'));
    expect(calls.again).toBe(1);
  });
});

describe('an agent with no folder to work in', () => {
  const folders = 'POST /network/machines/' + computer + '/folders';
  const provisioning = '/agents/' + SCRIBE + '/provisioning';
  const profile = { version: 2, operation: 'op-' + 'a'.repeat(32), model_access: ['model-one'], tools: [], skills: [], mcp_servers: [], instructions: '', note: 'Start this agent', set_by: ADA, set_at: 1, runs_on: computer, permissions: { default_mode: 'default' } };
  const held = (under: string, names: string[]) => ok({ machine: computer, under, folders: names });
  const looked: Route = (body) => {
    const under = (body as { under?: string }).under;
    if (!under) return held('/Users/ada', ['.cache', 'Developer', 'Music']);
    if (under === '/Users/ada/Developer') return held(under, ['.git', 'receipts']);
    if (under === '/Users/ada/Developer/receipts') return held(under, []);
    return refused(409, 'folder_unreadable', under + ' could not be read');
  };
  const button = (scope: HTMLElement, label: string) => [...scope.querySelectorAll('button')].find((entry) => (entry.getAttribute('aria-label') ?? entry.textContent)?.trim() === label) ?? null;

  it('is chosen from the computer’s own folders, saved, and the start follows without a second press', async () => {
    const { host, posted, calls } = await shown({ refusal: 'WorkingFolderUnnamed', reason: 'the launch names no working folder' }, {
      '/network': ok({ machines: [machine(computer, 'Ada’s laptop')], reports_served: true }),
      [folders]: looked, [provisioning]: ok({ agent: SCRIBE, profile, versions: [], enforced: false }),
      ['POST ' + provisioning]: (body) => ok({ agent: SCRIBE, profile: { ...profile, ...(body as object), version: 3 }, recorded: { operation: (body as { operation: string }).operation, version: 3 }, versions: [], enforced: false }),
    });
    expect(host.textContent).toContain('Scribe has no folder to work in.');
    expect(host.querySelector('input')).toBeNull();
    expect(posted).toEqual([]);
    await click(button(host, 'Choose a folder'));
    await settle();
    await settle();
    expect(host.textContent).toContain('On Ada’s laptop, in /Users/ada');
    expect(buttons(host)).toEqual(expect.arrayContaining(['Developer', 'Music']));
    expect(buttons(host)).not.toContain('.cache');
    await click(button(host, 'Developer'));
    await settle();
    expect(host.textContent).toContain('On Ada’s laptop, in /Users/ada/Developer');
    expect(buttons(host)).toContain('receipts');
    expect(buttons(host)).not.toContain('.git');
    expect(buttons(host)).toContain('Back to ada');
    await click(button(host, 'receipts'));
    await settle();
    expect(host.textContent).toContain('There are no folders inside this one.');
    await click(button(host, 'Work in receipts'));
    await settle();
    await settle();
    const saved = posted.find((entry) => entry.path === provisioning)?.body as Record<string, unknown>;
    expect(saved.working_folder).toBe('/Users/ada/Developer/receipts');
    expect(saved.from_version).toBe(2);
    expect(saved.runs_on).toBe(computer);
    expect(saved.model_access).toEqual(['model-one']);
    expect(calls.again).toBe(1);
    expect(host.querySelector('input')).toBeNull();
  });

  it('says so when the computer cannot be looked in, and saves nothing', async () => {
    const { host, posted, calls } = await shown({ refusal: 'WorkingFolderUnnamed', reason: 'the launch names no working folder' }, {
      '/network': ok({ machines: [machine(computer, 'Ada’s laptop')], reports_served: true }),
      [folders]: refused(409, 'runner_absent', 'machine names no runner'), [provisioning]: ok({ agent: SCRIBE, profile, versions: [], enforced: false }),
    });
    await click(button(host, 'Choose a folder'));
    await settle();
    expect(host.textContent).toContain('Lys could not look at the folders there.');
    expect(host.textContent).toContain('runner_absent');
    expect(posted.some((entry) => entry.path === provisioning)).toBe(false);
    expect(calls.again).toBe(0);
  });

  it('settles the computer first when neither the settings nor the allowances name one', async () => {
    const other = 'op-' + '2'.repeat(32);
    const { host, posted } = await shown({ refusal: 'WorkingFolderUnnamed', reason: 'the launch names no working folder' }, {
      '/network': ok({ machines: [machine(computer, 'Ada’s laptop'), machine(other, 'Ward desk')], reports_served: true }),
      [provisioning]: ok({ agent: SCRIBE, profile: { ...profile, runs_on: undefined }, versions: [], enforced: false }),
    });
    expect(host.textContent).toContain('Lys does not know yet which computer Scribe runs on.');
    expect(button(host, 'Choose a folder')).toBeNull();
    expect(host.querySelector('select')).toBeNull();
    expect(posted).toEqual([]);
  });

  it('looks on the one computer allowed to run the agent when the settings name none', async () => {
    const other = 'op-' + '2'.repeat(32);
    const allowed = { ...machine(other, 'Ward desk'), may_run: [{ id: SCRIBE, display_name: 'Scribe', state: 'active' }] };
    const { host, posted } = await shown({ refusal: 'WorkingFolderUnnamed', reason: 'the launch names no working folder' }, {
      '/network': ok({ machines: [machine(computer, 'Ada’s laptop'), allowed], reports_served: true }),
      [provisioning]: ok({ agent: SCRIBE, profile: { ...profile, runs_on: undefined }, versions: [], enforced: false }),
      ['POST /network/machines/' + other + '/folders']: ok({ machine: other, under: '/home/ward', folders: ['notes'] }),
    });
    await click(button(host, 'Choose a folder'));
    await settle();
    expect(host.textContent).toContain('On Ward desk, in /home/ward');
    expect(posted.map((entry) => entry.path)).toEqual(['/network/machines/' + other + '/folders']);
  });

  it('looks on the one computer a role the agent holds allows it on, and not on one a role it no longer holds allowed', async () => {
    const other = 'op-' + '2'.repeat(32);
    const byRole = { ...machine(other, 'Ward desk'), may_run_roles: ['role-ward'] };
    const byOldRole = { ...machine(computer, 'Ada’s laptop'), may_run_roles: ['role-old'] };
    const roles = [{ id: 'role-ward', holders: [{ holder: SCRIBE, state: 'holding' }] }, { id: 'role-old', holders: [{ holder: SCRIBE, state: 'ended' }] }];
    const { host, posted } = await shown({ refusal: 'WorkingFolderUnnamed', reason: 'the launch names no working folder' }, {
      '/network': ok({ machines: [byOldRole, byRole], reports_served: true }), '/roles': ok({ roles }),
      [provisioning]: ok({ agent: SCRIBE, profile: { ...profile, runs_on: undefined }, versions: [], enforced: false }),
      ['POST /network/machines/' + other + '/folders']: ok({ machine: other, under: '/home/ward', folders: ['notes'] }),
    });
    await click(button(host, 'Choose a folder'));
    await settle();
    expect(host.textContent).toContain('On Ward desk, in /home/ward');
    expect(posted.map((entry) => entry.path)).toEqual(['/network/machines/' + other + '/folders']);
  });

  it('sends every setting it read back unchanged, with the folder, when it saves', async () => {
    const full = { ...profile, tools: ['reader'], skills: ['review'], mcp_servers: [{ name: 'Cambium', url: 'http://localhost:6010', channel: 'wake' }],
      instructions: 'Check every receipt', instructions_mode: 'append', session: { compact: 'compact' }, writable: '/srv/out',
      harness: { name: 'Claude Code', program: '/usr/local/bin/claude', package: 'claude-code', description: { rendering_contract: 'claude-code/template-v1' } },
      permissions: { allow: ['Read'], deny: ['Bash(rm:*)'], ask: ['Edit'], default_mode: 'acceptEdits', additional_directories: ['/srv/a', '/srv/b'] } };
    const { host, posted } = await shown({ refusal: 'WorkingFolderUnnamed', reason: 'the launch names no working folder' }, {
      '/network': ok({ machines: [machine(computer, 'Ada’s laptop')], reports_served: true }),
      [folders]: looked, [provisioning]: ok({ agent: SCRIBE, profile: full, versions: [], enforced: false }),
      ['POST ' + provisioning]: (body) => ok({ agent: SCRIBE, profile: { ...full, ...(body as object), version: 3 }, recorded: { operation: (body as { operation: string }).operation, version: 3 }, versions: [], enforced: false }),
    });
    await click(button(host, 'Choose a folder'));
    await settle();
    await click(button(host, 'Work in ada'));
    await settle();
    await settle();
    const saved = posted.find((entry) => entry.path === provisioning)?.body as Record<string, unknown>;
    const { operation, from_version, note, working_folder, ...kept } = saved;
    expect(working_folder).toBe('/Users/ada');
    expect(from_version).toBe(2);
    expect(typeof operation).toBe('string');
    expect(note).toBe('Choose the folder this agent works in');
    const own = ['version', 'operation', 'note', 'set_by', 'set_at'];
    expect(kept).toEqual(Object.fromEntries(Object.entries(full).filter(([member]) => !own.includes(member))));
  });
});
