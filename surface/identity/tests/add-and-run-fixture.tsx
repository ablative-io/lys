import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, expect } from 'vitest';
import { App } from '../src/App';
import { $, serve, type, leaveTheReachReadOutOfPosted } from './harness';
// These cases land on People and agents, which asks what each agent reaches; that read is not one of the flow's changes.
leaveTheReachReadOutOfPosted();
import { ADA, RECEIPTS, SCRIBE_VIEW, SERVICE, ok, refused } from './fixtures';
import type { Answer, Route } from './fixtures';
import type { Machine, NameMachine } from '../src/features/network/contract';

export const agent = 'agent-' + 'f'.repeat(32);
export const prefix = '/agents/' + agent;
export const key = 'lys.add-and-run.' + ADA;
export const stages = ['register', 'activate', 'profile', 'review', 'machine', 'runner', 'start'] as const;
export type Stage = typeof stages[number] | 'admission';
export const words: Record<Stage, string> = { register: 'registering the agent', activate: 'activating the agent', profile: 'saving the settings', review: 'approving the settings', machine: 'adding this computer', runner: 'recording this computer’s runner', admission: 'allowing the agent on this computer', start: 'starting the agent' };
export const description = { models: { minimum: 1, maximum: 1, further_encoding: { kind: 'array' } }, permissions: { modes: ['default', 'workspace-write'], rule_forms: [] }, mcp: { transports: ['stdio'], working_directory: false, handle_variables: false, channel_policies: ['off'] }, rendering_contract: 'test' };
export const program = { name: 'Care program', line: 'Agent program', description, models: [{ id: 'care', label: 'Care model' }], modes: [{ id: 'default', meaning: 'Ask before changes' }, { id: 'workspace-write', meaning: 'Works in its own folder; no internet.' }], instructions_modes: ['keep'], builds: [{ name: 'Installed', program: '/opt/bin/care', package: 'care', from: 'runner' }] };
let root: Root | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { if (root) act(() => root?.unmount()); root = null; });

export function service(failure?: Stage, runnerState: 'running' | 'ended' | 'absent' = 'running', computers: Machine[] = []) {
  let profile: Record<string, unknown> | null = null;
  let computer: Record<string, unknown> | null = null;
  let session = '';
  let sessionMachine = '';
  let existing = computers.map((machine) => ({ ...machine, may_run: [...machine.may_run] }));
  let failed = false;
  const applied = new Map<string, Answer>();
  const routes: Record<string, Route> = { ...SERVICE,
    '/surface-contract': ok({ paths: {
      '/agents': { post: { requestBody: { content: { 'application/json': { schema: { properties: { operation: {}, display_name: {} } } } } } } },
      ...(computers.length ? { '/network/machines/{id}/agents': { post: {
        requestBody: { content: { 'application/json': { schema: { type: 'object', required: ['operation', 'agent', 'allow'], properties: { operation: { type: 'string' }, agent: { type: 'string' }, allow: { type: 'boolean' } } } } } },
        responses: { '200': { content: { 'application/json': { schema: { type: 'object', required: ['machine', 'recorded'], properties: { machine: { type: 'object' }, recorded: { type: 'object' } } } } } } },
      } } } : {}),
    } }),
    '/network': () => ok({ machines: existing.length ? existing : computer ? [computer] : [], reports_served: true }),
    '/harnesses': ok({ programs: [program] }), '/skills': ok({ skills: [] }), '/secrets': ok({ secrets: [] }),
    ['/directory/agents/' + agent]: ok({ ...SCRIBE_VIEW, id: agent, display_name: 'Clover' }),
    [prefix + '/provisioning']: () => ok({ agent, profile, versions: [], enforced: false }),
    [prefix + '/runtime/sessions']: () => ok({ sessions: session && runnerState === 'running' ? [{ agent, session, machine: sessionMachine, machine_name: computer?.name ?? existing.find((machine) => machine.id === sessionMachine)?.name, runtime: 'lys-runner', shown: 'running', last_report_at: 1, stopped: null, stop_asked_at: null }] : [] }),
  };
  const answer = (stage: Stage, body: unknown, build: () => Answer): Answer => {
    const stamp = stage + JSON.stringify(body);
    let result = applied.get(stamp);
    if (!result) { result = build(); applied.set(stamp, result); }
    if (failure === stage && !failed) { failed = true; return refused(503, 'StepUnavailable', 'The step outcome is unknown'); }
    return result;
  };
  routes['POST /agents'] = (body) => answer('register', body, () => ok({ agent, responsible: ADA, receipt: { ...RECEIPTS[4].receipt, operation: (body as { operation: string }).operation, identity: agent } }));
  routes['POST /identities/' + agent + '/transitions'] = (body) => answer('activate', body, () => ok({ receipt: { ...RECEIPTS[5].receipt, operation: (body as { operation: string }).operation, identity: agent } }));
  routes['POST ' + prefix + '/provisioning'] = (body) => answer('profile', body, () => {
    profile = { ...(body as object), version: 1, set_by: ADA, set_at: 1, reviewed_by: null, session: null };
    return ok({ agent, profile, recorded: { operation: profile.operation, version: 1 } });
  });
  routes['POST ' + prefix + '/provisioning/1/review'] = (body) => answer('review', body, () => {
    profile = { ...profile, reviewed_by: ADA };
    return ok({ agent, profile, recorded: { operation: (body as { operation: string }).operation, version: 1 } });
  });
  routes['POST /network/machines'] = (body) => answer('machine', body, () => {
    const given = body as NameMachine;
    computer = { ...given, id: given.operation, may_run: [{ id: agent, display_name: 'Clover', state: 'active' }], named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: null };
    routes['POST /network/machines/' + given.operation + '/runner'] = (runner) => answer('runner', runner, () => ok({ machine: given.operation, ...(runner as object) }));
    return ok(computer);
  });
  for (const machine of computers) {
    routes['/network/machines/' + machine.id + '/runner'] = ok({ machine: machine.id, runner: { kind: 'lys' }, answers: true });
    routes['POST /network/machines/' + machine.id + '/agents'] = (body) => answer('admission', body, () => {
      const given = body as { operation: string; agent: string; allow: boolean };
      const original = existing.find((entry) => entry.id === machine.id);
      if (!original) throw new Error('The computer fixture is missing');
      const current = { ...original, may_run: [...original.may_run, { id: given.agent, display_name: 'Clover', state: 'active' as const }] };
      existing = existing.map((entry) => entry.id === machine.id ? current : entry);
      return ok({ machine: current, recorded: { ...given, machine: machine.id, by: ADA, at: 1, original_may_run: original.may_run.map((entry) => entry.id) } });
    });
  }
  routes['POST ' + prefix + '/start-command'] = (body) => answer('start', body, () => {
    const given = body as { operation: string; machine: string }; session = given.operation; sessionMachine = given.machine;
    return ok({ agent, machine: given.machine, runtime: 'lys-runner', session, provisioning_version: 1, harness: program.name, handles: [], template: '{}', template_sha256: 'digest', command: 'care', left_out: [], executed: false,
      ...(runnerState === 'absent' ? {} : { runner: { session, state: runnerState, pid: 1, started_at: 1 } }) });
  });
  return { routes, applied };
}

export async function open(routes: Record<string, Route>) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve(routes, posted);
  history.replaceState(null, '', '/#/agents/new');
  const container = document.createElement('div'); document.body.appendChild(container); root = createRoot(container);
  await act(async () => { root?.render(<App />); });
  return { posted, requests };
}
export async function remount(routes: Record<string, Route>) {
  if (root) act(() => root?.unmount()); root = null; document.body.innerHTML = '';
  return open(routes);
}
export async function names() {
  await type($('[name="display_name"]'), 'Clover');
  await type($('[name="computer_name"]'), 'Ward computer');
}
export function existingComputer(digit = 'e', name = 'Front desk'): Machine {
  return { id: 'op-' + digit.repeat(32), name, kind: 'Computer', runtime: 'lys-runner', slots: 0,
    may_run: [{ id: SCRIBE_VIEW.id, display_name: SCRIBE_VIEW.display_name, state: 'active' }], may_run_roles: [], may_reach: [],
    named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: 1 };
}
export async function chooseComputer(value: string) {
  const select = $('[name="computer"]');
  if (!(select instanceof HTMLSelectElement)) throw new Error('The computer choice is missing');
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')?.set?.call(select, value);
    select.dispatchEvent(new Event('change', { bubbles: true }));
  });
}
export async function submit(twice = false) {
  const form = $('form[aria-label="Add an agent"]');
  if (!form) throw new Error('The one-press form is missing');
  const name = form.querySelector<HTMLInputElement>('[name="display_name"]');
  if (name && !name.value) await type(name, 'Clover');
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); if (twice) form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
}
export function plain(stage: Stage, code = 'StepUnavailable') {
  const details = [...document.querySelectorAll('.refusal-name')].find((entry) => entry.textContent?.includes(code));
  expect(details?.textContent).toContain(code);
  const face = document.body.cloneNode(true) as HTMLElement;
  for (const detail of face.querySelectorAll('.refusal-name')) detail.remove();
  expect(face.textContent).not.toContain(code);
  expect(face.textContent).toContain(words[stage]);
}

