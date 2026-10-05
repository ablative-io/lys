import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { $, serve, type, leaveTheReachReadOutOfPosted } from './harness';
// These cases land on People and agents, which asks what each agent reaches; that read is not one of the flow's changes.
leaveTheReachReadOutOfPosted();
import { ADA, RECEIPTS, SCRIBE_VIEW, SERVICE, ok, refused } from './fixtures';
import type { Answer, Route } from './fixtures';
import type { Machine, NameMachine } from '../src/features/network/contract';
import { addAndRunCapability } from '../src/features/people/registration-capability';
import { readAddAndRun } from '../src/features/people/add-and-run';

const agent = 'agent-' + 'f'.repeat(32);
const prefix = '/agents/' + agent;
const key = 'lys.add-and-run.' + ADA;
const stages = ['register', 'activate', 'profile', 'review', 'machine', 'runner', 'start'] as const;
type Stage = typeof stages[number] | 'admission';
const words: Record<Stage, string> = { register: 'registering the agent', activate: 'activating the agent', profile: 'saving the settings', review: 'approving the settings', machine: 'adding this computer', runner: 'recording this computer’s runner', admission: 'allowing the agent on this computer', start: 'starting the agent' };
const description = { models: { minimum: 1, maximum: 1, further_encoding: { kind: 'array' } }, permissions: { modes: ['default', 'workspace-write'], rule_forms: [] }, mcp: { transports: ['stdio'], working_directory: false, handle_variables: false, channel_policies: ['off'] }, rendering_contract: 'test' };
const program = { name: 'Care program', line: 'Agent program', description, models: [{ id: 'care', label: 'Care model' }], modes: [{ id: 'default', meaning: 'Ask before changes' }, { id: 'workspace-write', meaning: 'Works in its own folder; no internet.' }], instructions_modes: ['keep'], builds: [{ name: 'Installed', program: '/opt/bin/care', package: 'care', from: 'runner' }] };
let root: Root | null = null;
beforeEach(() => sessionStorage.clear());
afterEach(() => { if (root) act(() => root?.unmount()); root = null; });

function service(failure?: Stage, runnerState: 'running' | 'ended' | 'absent' = 'running', computers: Machine[] = []) {
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
    routes['/network/machines/' + machine.id + '/runner'] = ok({ machine: machine.id, runner: { kind: 'lys' } });
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

async function open(routes: Record<string, Route>) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve(routes, posted);
  history.replaceState(null, '', '/#/agents/new');
  const container = document.createElement('div'); document.body.appendChild(container); root = createRoot(container);
  await act(async () => { root?.render(<App />); });
  return { posted, requests };
}
async function remount(routes: Record<string, Route>) {
  if (root) act(() => root?.unmount()); root = null; document.body.innerHTML = '';
  return open(routes);
}
async function names() {
  await type($('[name="display_name"]'), 'Clover');
  await type($('[name="computer_name"]'), 'Ward computer');
}
function existingComputer(digit = 'e', name = 'Front desk'): Machine {
  return { id: 'op-' + digit.repeat(32), name, kind: 'Computer', runtime: 'lys-runner', slots: 0,
    may_run: [{ id: SCRIBE_VIEW.id, display_name: SCRIBE_VIEW.display_name, state: 'active' }], may_run_roles: [], may_reach: [],
    named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: 1 };
}
async function chooseComputer(value: string) {
  const select = $('[name="computer"]');
  if (!(select instanceof HTMLSelectElement)) throw new Error('The computer choice is missing');
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')?.set?.call(select, value);
    select.dispatchEvent(new Event('change', { bubbles: true }));
  });
}
async function submit(twice = false) {
  const form = $('form[aria-label="Add an agent"]');
  if (!form) throw new Error('The one-press form is missing');
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); if (twice) form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
}
function plain(stage: Stage, code = 'StepUnavailable') {
  const details = [...document.querySelectorAll('.refusal-name')].find((entry) => entry.textContent?.includes(code));
  expect(details?.textContent).toContain(code);
  const face = document.body.cloneNode(true) as HTMLElement;
  for (const detail of face.querySelectorAll('.refusal-name')) detail.remove();
  expect(face.textContent).not.toContain(code);
  expect(face.textContent).toContain(words[stage]);
}

describe('Add and run on the first computer', () => {
  it('requires the empty computer name and offered choices before enabling its one button', async () => {
    const { posted, requests } = await open(service().routes);
    const button = $('form button[type="submit"]') as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    expect(($('[name="computer_name"]') as HTMLInputElement).value).toBe('');
    expect(($('[name="program"]') as HTMLSelectElement).value).toBe(program.name);
    expect(($('[name="model"]') as HTMLSelectElement).value).toBe('care');
    await names();
    expect(button.disabled).toBe(false);
    expect([button.getAttribute('aria-label'), button.textContent]).toEqual(['Add Clover and run it on this computer', 'Add and run']);
    expect(document.querySelectorAll('form button[type="submit"]')).toHaveLength(1);
    expect(posted).toEqual([]);
    expect(requests.filter((path) => path === '/network')).toHaveLength(1);
  });

  it('names the served repair steps instead of adding a second roleless computer', async () => {
    const server = service();
    const existing = { id: 'machine-existing', name: 'Front desk', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: [], named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: 1 };
    server.routes['/network'] = ok({ machines: [existing], reports_served: true });
    const { posted } = await open(server.routes);
    await type($('[name="display_name"]'), 'Clover');
    expect($('[name="computer_name"]')).toBeNull();
    expect($('form button[type="submit"]')?.textContent).not.toContain('and run');
    expect(document.body.textContent).toContain('Front desk runs only the agents named when it was added. To add and run from here, add this computer again in Network with a role, then retire Front desk.');
    expect($('a[href="#/network"]')).not.toBeNull();
    expect(posted).toEqual([]);
    await submit();
    expect(posted.filter((entry) => entry.path === '/network/machines' || entry.path.endsWith('/start-command'))).toEqual([]);
  });

  it.each([
    { grantTemplates: [] },
    { grantTemplates: [{ resource: { kind: 'project', id: 'records' }, relation: 'editor', days: null }] },
  ])('does not assign a computer role when its actual grants are not served', async ({ grantTemplates }) => {
    const server = service();
    const role = 'role-' + 'a'.repeat(32);
    const existing = { id: 'op-' + 'b'.repeat(32), name: 'Front desk', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [role], may_reach: [], named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: 1 };
    server.routes['/network'] = ok({ machines: [existing], reports_served: true });
    server.routes['/network/machines/' + existing.id + '/runner'] = ok({ machine: existing.id, runner: { kind: 'lys' } });
    server.routes['/roles'] = ok({ roles: [{ id: role, name: 'Computer operator', latest: 1, policy: 'stays_until_moved', holders: [], versions: [{ number: 1, responsibilities: '', goals: '', practice: '', profile: '', grant_templates: grantTemplates, note: '', made_by: ADA, made_at: 1 }] }] });
    const { posted } = await open(server.routes);
    await type($('[name="display_name"]'), 'Clover');
    expect($('[name="computer_name"]')).toBeNull();
    expect($('form button[type="submit"]')?.textContent).not.toContain('and run');
    expect(document.body.textContent).toContain("Lys cannot confirm what Front desk's roles grant, so it won't give one to Clover. Add the agent here, then start it from its page once it is admitted to Front desk.");
    expect(document.body.textContent).not.toContain('Front desk runs only the agents named');
    expect(posted).toEqual([]);
    await submit();
    expect(posted.filter((entry) => entry.path === '/network/machines' || entry.path.endsWith('/holders') || entry.path.endsWith('/start-command'))).toEqual([]);
  });

  it('keeps the typed name path when every recorded computer is retired', async () => {
    const server = service();
    server.routes['/network'] = ok({ machines: [{ id: 'op-' + 'c'.repeat(32), name: 'Old computer', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: [], named_by: ADA, named_at: 1, state: 'retired', retired_at: 1, last_report_at: null }], reports_served: true });
    const { posted } = await open(server.routes);
    expect(($('[name="computer_name"]') as HTMLInputElement).value).toBe('');
    await names(); await submit();
    expect(posted).toHaveLength(7);
    expect(posted[4].body).toMatchObject({ name: 'Ward computer', may_run: [agent] });
    expect(location.hash).toBe('#/file/' + agent);
  });

  it('holds an earlier planned computer addition when another computer is now in use', async () => {
    const server = service('profile');
    await open(server.routes); await names(); await submit();
    const saved = sessionStorage.getItem(key);
    expect(saved).not.toBeNull();
    server.routes['/network'] = ok({ machines: [{ id: 'op-' + 'd'.repeat(32), name: 'Front desk', kind: 'Computer', runtime: 'lys-runner', slots: 0, may_run: [], may_run_roles: [], may_reach: [], named_by: ADA, named_at: 1, state: 'in_use', retired_at: null, last_report_at: 1 }], reports_served: true });
    const next = await remount(server.routes);
    expect(document.body.textContent).toContain('RetainedComputerAdditionHeld');
    expect(document.body.textContent).toContain('This saved request would add a second computer; it has not been sent.');
    await submit();
    expect(next.posted).toEqual([]);
    expect(sessionStorage.getItem(key)).toBe(saved);
    expect(location.hash).toBe('#/agents/new');
  });

  it('records all seven stages in order and lands on the real running session', async () => {
    const server = service(); const { posted } = await open(server.routes); await names(); await submit(true);
    expect(posted).toHaveLength(7);
    const computer = (posted[4].body as NameMachine).operation;
    expect(posted.map((entry) => entry.path)).toEqual(['/agents', '/identities/' + agent + '/transitions', prefix + '/provisioning', prefix + '/provisioning/1/review', '/network/machines', '/network/machines/' + computer + '/runner', prefix + '/start-command']);
    expect(posted[2].body).toMatchObject({ from_version: 0, model_access: ['care'], permissions: { default_mode: 'workspace-write' }, harness: { name: program.name, program: '/opt/bin/care' } });
    expect(posted[4].body).toMatchObject({ name: 'Ward computer', runtime: 'lys-runner', slots: 0, may_run: [agent], may_run_roles: [], may_reach: [] });
    expect(posted[5].body).toEqual({ runner: { kind: 'lys' } });
    expect(posted[6].body).toMatchObject({ machine: computer });
    expect(server.applied.size).toBe(7);
    expect(sessionStorage.getItem(key)).toBeNull();
    expect(location.hash).toBe('#/file/' + agent);
    expect(posted[6].path).toBe(prefix + '/start-command');
  });

  it.each(stages)('replays only the saved %s request after an unknown outcome and remount', async (stage) => {
    const server = service(stage); const first = await open(server.routes); await names(); await submit();
    const index = stages.indexOf(stage);
    expect(first.posted).toHaveLength(index + 1);
    expect(sessionStorage.getItem(key)).not.toBeNull();
    plain(stage);
    const original = first.posted[index];
    const next = await remount(server.routes); await submit();
    expect(next.posted[0]).toEqual(original);
    expect(next.posted).toHaveLength(7 - index);
    expect(server.applied.size).toBe(7);
    expect(sessionStorage.getItem(key)).toBeNull();
    expect(location.hash).toBe('#/file/' + agent);
  });

  it('shows a start refusal in its own words, not as an unconfirmed step', async () => {
    const server = service();
    server.routes['POST /agents/' + agent + '/start-command'] = () => refused(400, 'PolicyUnrepresentable', 'The settings file cannot express these permissions for this program.');
    await open(server.routes); await names(); await submit();
    const face = document.body.cloneNode(true) as HTMLElement;
    for (const detail of face.querySelectorAll('.refusal-name')) detail.remove();
    expect(face.textContent).toContain('The settings file cannot express these permissions for this program.');
    expect(face.textContent).not.toContain('could not confirm');
    expect(face.textContent).not.toContain('PolicyUnrepresentable');
  });

  it('does not navigate or claim running without a confirmed runner start', async () => {
    const { posted } = await open(service(undefined, 'absent').routes); await names(); await submit();
    expect(posted).toHaveLength(7);
    expect(location.hash).toBe('#/agents/new');
    expect(sessionStorage.getItem(key)).not.toBeNull();
    plain('start', 'RunnerStartUnconfirmed');
    expect(document.body.textContent).toContain('Lys admitted the start, but no runner ran it');
  });

  it('names an ended runner session without claiming it is still running', async () => {
    const { posted } = await open(service(undefined, 'ended').routes); await names(); await submit();
    expect(posted).toHaveLength(7);
    expect(location.hash).toBe('#/agents/new');
    expect(sessionStorage.getItem(key)).not.toBeNull();
    plain('start', 'RunnerStartEnded');
    expect(document.body.textContent).toContain('The runner confirmed this session already ended');
  });

  it('blocks corrupt retained state without sending a replacement operation', async () => {
    sessionStorage.setItem(key, '{');
    const { posted } = await open(service().routes);
    expect(($('form button[type="submit"]') as HTMLButtonElement).disabled).toBe(true);
    expect(document.body.textContent).toContain('PendingAddAndRunUnreadable');
    await submit(); expect(posted).toEqual([]);
  });

  it('preselects an offered workspace program when the service offers more than one', async () => {
    const server = service(); server.routes['/harnesses'] = ok({ programs: [program, { ...program, name: 'Other program' }] });
    const { posted } = await open(server.routes); await names();
    expect(($('[name="program"]') as HTMLSelectElement).value).toBe(program.name);
    expect(($('form button[type="submit"]') as HTMLButtonElement).disabled).toBe(false);
    expect(posted).toEqual([]);
  });
});

describe('Add and run on an existing computer', () => {
  it('is live with served defaults and a known computer, without a sandbox picker', async () => {
    const computer = existingComputer();
    const { posted } = await open(service(undefined, 'running', [computer]).routes);
    expect(($('form button[type="submit"]') as HTMLButtonElement).disabled).toBe(false);
    expect(($('[name="display_name"]') as HTMLInputElement).value).toBe('New agent');
    expect($('[name="computer_name"]')).toBeNull();
    expect($('[name="mode"]')).toBeNull();
    expect(document.body.textContent).toContain('Computer: Front desk');
    expect(document.body.textContent).toContain('Works in its own folder; no internet.');
    expect(document.body.textContent).not.toContain('Choose the settings before adding');
    expect(posted).toEqual([]);
  });

  it('names an unavailable workspace setting instead of starting an unrestricted program', async () => {
    const server = service(undefined, 'running', [existingComputer()]);
    server.routes['/harnesses'] = ok({ programs: [{ ...program, modes: [{ id: 'danger-full-access', meaning: 'All access' }] }] });
    const { posted } = await open(server.routes);
    expect(($('form button[type="submit"]') as HTMLButtonElement).disabled).toBe(true);
    expect(document.body.textContent).toContain('This program has no setting that keeps it to its own folder with internet off.');
    expect(document.body.textContent).not.toContain('WorkspaceOnlyUnavailable');
    expect(document.body.textContent).not.toContain('Works in its own folder; no internet.');
    // Nothing is folded away anywhere in the form.
    expect(document.querySelector('form details')).toBeNull();
    await submit();
    expect(posted).toEqual([]);
  });

  it('keeps both capabilities unknown when their single schema read fails', async () => {
    const requests = serve({ '/surface-contract': refused(503, 'SchemaUnavailable', 'The served schema could not be read') });
    const capability = await addAndRunCapability();
    expect(capability.answersTo).toBeNull();
    expect(capability.machineAdmission).toBeNull();
    expect(capability.problem).toMatchObject({ refusal: { refusal: 'SchemaUnavailable' } });
    expect(capability.admissionProblem).toBe(capability.problem);
    expect(requests).toEqual(['/surface-contract']);
  });

  it('reads one saved format: an earlier envelope is said to be unreadable, is kept as it was, and nothing is sent', () => {
    const old = { version: 2, person: ADA, step: 'registration', pending: null,
      registration: { name: 'Clover', register: 'op-' + '1'.repeat(32), activate: 'op-' + '2'.repeat(32), agent: null, answersTo: ADA, team: null, membership: null, activated: false },
      settings: { harness: { name: program.name, program: '/opt/bin/care', package: 'care', description }, model_access: ['care'], permissions: { default_mode: 'default' }, tools: [], skills: [], mcp_servers: [], instructions: '', instructions_mode: 'keep', note: '' },
      placement: { kind: 'existing', computer: existingComputer(), admission: null },
    };
    const raw = JSON.stringify(old); sessionStorage.setItem(key, raw);
    const posted: { path: string; body: unknown }[] = []; serve(service().routes, posted);
    let refusal = '';
    try { readAddAndRun(key, ADA); } catch (error) { refusal = (error as { refusal?: { refusal?: string } }).refusal?.refusal ?? String(error); }
    expect(refusal).toBe('PendingAddAndRunUnreadable');
    expect(sessionStorage.getItem(key)).toBe(raw);
    expect(posted).toEqual([]);
  });

  it('uses six confirmed stages on the named computer without creating or assigning a role', async () => {
    const computer = existingComputer();
    const server = service(undefined, 'running', [computer]);
    const { posted, requests } = await open(server.routes);
    const start = server.routes['POST ' + prefix + '/start-command'];
    if (typeof start !== 'function') throw new Error('The start fixture is missing');
    let readsAtStart: string[] = [];
    server.routes['POST ' + prefix + '/start-command'] = (body) => {
      const answer = start(body); readsAtStart = [...requests]; return answer;
    };
    expect($('[name="computer_name"]')).toBeNull();
    expect(document.body.textContent).toContain(computer.name);
    expect(($('[name="display_name"]') as HTMLInputElement).value).toBe('New agent');
    await type($('[name="display_name"]'), 'Clover');
    expect(($('form button[type="submit"]') as HTMLButtonElement).disabled).toBe(false);
    expect([$('form button[type="submit"]')?.getAttribute('aria-label'), $('form button[type="submit"]')?.textContent]).toEqual(['Add Clover and run it on this computer', 'Add and run']);
    expect(requests.filter((path) => path === '/surface-contract')).toHaveLength(1);
    await submit(true);
    expect(posted.map((entry) => entry.path)).toEqual(['/agents', '/identities/' + agent + '/transitions', prefix + '/provisioning', prefix + '/provisioning/1/review', '/network/machines/' + computer.id + '/agents', prefix + '/start-command']);
    expect(posted[4].body).toEqual({ operation: expect.stringMatching(/^op-[0-9a-f]{32}$/), agent, allow: true });
    expect(posted[5].body).toMatchObject({ machine: computer.id });
    expect(server.applied.size).toBe(6);
    expect(sessionStorage.getItem(key)).toBeNull();
    expect(location.hash).toBe('#/file/' + agent);
    expect(posted[5].path).toBe(prefix + '/start-command');
    expect(readsAtStart.filter((path) => path === '/network')).toHaveLength(1);
    // The add reads the computers once, and the agent's own page it lands on reads them once for all its parts.
    expect(requests.filter((path) => path === '/network')).toHaveLength(2);
  });

  it.each(['admission', 'start'] as const)('replays the exact saved %s on the original computer after remount', async (stage) => {
    const computer = existingComputer();
    const server = service(stage, 'running', [computer]);
    const first = await open(server.routes);
    await type($('[name="display_name"]'), 'Clover'); await submit();
    const index = stage === 'admission' ? 4 : 5;
    expect(first.posted).toHaveLength(index + 1);
    expect(sessionStorage.getItem(key)).not.toBeNull();
    plain(stage);
    const original = first.posted[index];
    const next = await remount(server.routes); await submit();
    expect(next.posted[0]).toEqual(original);
    expect(next.posted).toHaveLength(6 - index);
    expect(next.posted[next.posted.length - 1].body).toMatchObject({ machine: computer.id });
    expect(server.applied.size).toBe(6);
    expect(sessionStorage.getItem(key)).toBeNull();
    expect(location.hash).toBe('#/file/' + agent);
  });

  it.each([
    { field: 'operation', value: 'op-' + '0'.repeat(32) },
    { field: 'machine', value: 'op-' + '0'.repeat(32) },
    { field: 'agent', value: SCRIBE_VIEW.id },
    { field: 'allow', value: false },
    { field: 'by', value: 'person-' + '0'.repeat(32) },
  ])('does not start when the allowance receipt has a different $field', async ({ field, value }) => {
    const computer = existingComputer();
    const server = service(undefined, 'running', [computer]);
    const path = 'POST /network/machines/' + computer.id + '/agents';
    const route = server.routes[path];
    if (typeof route !== 'function') throw new Error('The allowance fixture is missing');
    server.routes[path] = (body) => {
      const answer = route(body);
      const payload = answer.body as { machine: Machine; recorded: Record<string, unknown> };
      return ok({ ...payload, recorded: { ...payload.recorded, [field]: value } });
    };
    const { posted } = await open(server.routes);
    await type($('[name="display_name"]'), 'Clover'); await submit();
    expect(posted).toHaveLength(5);
    expect(posted[4].path).toBe('/network/machines/' + computer.id + '/agents');
    expect(location.hash).toBe('#/agents/new');
    expect(sessionStorage.getItem(key)).not.toBeNull();
    plain('admission', 'MachineAdmissionReceiptMismatch');
  });

  it('requires an explicit computer choice when more than one local runner is served', async () => {
    const first = existingComputer('a', 'Front desk');
    const second = existingComputer('b', 'Ward computer');
    const { posted } = await open(service(undefined, 'running', [first, second]).routes);
    expect($('[name="computer_name"]')).toBeNull();
    expect(($('[name="computer"]') as HTMLSelectElement).value).toBe('');
    await type($('[name="display_name"]'), 'Clover');
    expect(($('form button[type="submit"]') as HTMLButtonElement).disabled).toBe(true);
    expect(posted).toEqual([]);
    await chooseComputer(second.id);
    expect(($('form button[type="submit"]') as HTMLButtonElement).disabled).toBe(false);
    await submit();
    expect(posted).toHaveLength(6);
    expect(posted[4].path).toBe('/network/machines/' + second.id + '/agents');
    expect(posted[5].body).toMatchObject({ machine: second.id });
    expect(location.hash).toBe('#/file/' + agent);
  });

  it.each([
    { kind: 'socket', path: '/tmp/runner.sock' },
    null,
  ])('does not offer add and run when an in-use computer has no confirmed local runner', async (runner) => {
    const computer = existingComputer();
    const server = service(undefined, 'running', [computer]);
    server.routes['/network/machines/' + computer.id + '/runner'] = ok({ machine: computer.id, runner });
    const { posted } = await open(server.routes);
    await type($('[name="display_name"]'), 'Clover');
    expect($('[name="computer_name"]')).toBeNull();
    expect($('form button[type="submit"]')?.textContent).not.toContain('and run');
    expect(document.body.textContent).toContain('runner');
    await submit();
    expect(posted.filter((entry) => entry.path === '/network/machines' || entry.path.endsWith('/agents') && entry.path.startsWith('/network/') || entry.path.endsWith('/start-command'))).toEqual([]);
  });
});
