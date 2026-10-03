/** The front page: the real teams, people and live sessions as a tree of names, the first running agent opened by itself, a stopped agent started in its own pane with one press, and what stops a start said in one sentence with the button that fixes it. */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, text } from './harness';
import { ADA, ARCHIVIST, COURIER, REVIEWER, SCRIBE, SCRIBE_VIEW, SERVICE, ok, refused } from './fixtures';
import { mockTerminal } from './terminal-double';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const LIVE = 'op-' + '5'.repeat(32);
const LAB = 'op-' + 'b'.repeat(32);
const SHED = 'op-' + 'c'.repeat(32);
const FOLDER = '/Users/ada/work';

const running = { session: LIVE, agent: SCRIBE, machine: LAB, machine_name: 'Lab', runtime: 'lys-runner', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'started', stopped: null, reported_by: 'the runner' };
const machine = (id: string, name: string, runtime: string | null, mayRun: string[]) => ({ id, name, kind: 'laptop', runtime, slots: 1, may_run: mayRun.map((agent) => ({ id: agent, display_name: 'x', state: 'active' })), may_reach: [], named_by: ADA, named_at: 1790000000, state: 'in_use', retired_at: null, last_report_at: null });
const team = (id: string, owner: string, name: string, members: string[]) => ({ id, owner, name, description: '', members, state: 'active', created_by: { provider: 'p', subject: 's' }, created_at: 1790000000, retired_at: null });
const receipt = (answer: unknown) => ok({ session: LIVE, answer, receipt: { index: 1 } });
const profile = (more: Record<string, unknown> = {}) => ({ version: 1, operation: 'op-' + 'a'.repeat(32), harness: { name: 'Claude Code', description: {}, program: '/opt/claude', package: 'claude' }, model_access: ['default'], tools: [], skills: [], mcp_servers: [], instructions: '', note: '', set_by: ADA, set_at: 1790000000, reviewed_by: ADA, reviewed_at: 1790000000, working_folder: FOLDER, ...more });
const provisioning = (agent: string, saved: unknown) => ({ ['/agents/' + agent + '/provisioning']: ok({ agent, profile: saved, versions: [], enforced: false }) });
const started = (agent: string) => (body: unknown) => {
  const sent = body as { machine: string; operation: string };
  return ok({ agent, machine: sent.machine, runtime: 'lys-runner', session: sent.operation, provisioning_version: 1, harness: 'Claude Code', handles: [], template: '', template_sha256: '', command: 'claude', left_out: [], executed: false, runner: { session: sent.operation, state: 'running', pid: 7, started_at: 1790000003 } });
};
const button = (words: string) => $$('button').find((el) => el.textContent === words) ?? null;

const routes = {
  ...SERVICE,
  '/teams': ok({ teams: [team('team-1', ADA, 'Ada team', [REVIEWER]), team('team-2', SCRIBE, 'Crew', [COURIER, ARCHIVIST])] }),
  '/runtime/live': ok({ sessions: [running], unanswered: [] }),
  '/network': ok({ machines: [machine(LAB, 'Lab', 'lys-runner', [SCRIBE]), machine(SHED, 'Shed', null, [])], reports_served: true }),
  ['POST /runtime/sessions/' + LIVE + '/resize']: receipt({ kind: 'resized' }),
  ['POST /runtime/sessions/' + LIVE + '/read-bytes']: refused(502, 'runner_unreachable', 'the read was closed'),
};
/** Scribe alone, stopped, with approved settings and one computer allowed to run it. */
const stopped = { ...routes, '/teams': ok({ teams: [] }), '/runtime/live': ok({ sessions: [], unanswered: [] }), ...provisioning(SCRIBE, profile()) };

describe('Team', () => {
  beforeEach(() => { sessionStorage.clear(); });

  it('is the front page, draws the tree of names and opens the first running agent by itself', async () => {
    await mount('#/', routes);
    expect(location.hash).toBe('#/team/' + SCRIBE);
    expect($$('.tree-name').map((el) => el.textContent)).toEqual(['Reviewer', 'Scribe', 'Courier', 'Archivist']);
    expect($$('.tree-team').map((el) => el.textContent)).toEqual(['Ada team', 'Crew']);
    expect(text()).not.toContain('Ada (test person)');
    expect($('.tree-name[aria-current="page"]')?.textContent).toBe('Scribe');
    expect($('.team-foot')?.textContent).toContain('Lab');
    expect($('.tree-new')?.getAttribute('href')).toBe('#/agents/new');
  });

  it('folds a team away and back, and hides the tree from the foot line', async () => {
    localStorage.clear();
    await mount('#/team/' + SCRIBE, routes);
    await click($$('.tree-team').find((el) => el.textContent === 'Crew') ?? null);
    expect($$('.tree-name').map((el) => el.textContent)).toEqual(['Reviewer', 'Scribe']);
    expect(localStorage.getItem('iam.team-folded')).toBe('team-2');
    await click($$('.tree-team').find((el) => el.textContent === 'Crew2') ?? null);
    expect($$('.tree-name').map((el) => el.textContent)).toEqual(['Reviewer', 'Scribe', 'Courier', 'Archivist']);
    await click($('.team-foot-tree'));
    expect($('.team-screen')?.getAttribute('data-tree')).toBe('hidden');
    await click($('.team-foot-tree'));
    expect($('.team-screen')?.getAttribute('data-tree')).toBe('shown');
    localStorage.clear();
  });

  it('opens settings over the agent\'s own terminal and closes them from the button', async () => {
    await mount('#/team/' + SCRIBE, routes);
    expect($('.team-settings')).toBeNull();
    await click($$('.team-foot-act').find((el) => el.textContent === 'Settings') ?? null);
    expect($('.team-settings')?.getAttribute('aria-label')).toBe('Scribe settings');
    expect(text()).toContain('Restart Scribe');
    expect($('.team-pane .terminal')).not.toBeNull();
    await click($$('.team-settings .btn').find((el) => el.textContent === 'Back to the terminal') ?? null);
    expect($('.team-settings')).toBeNull();
  });

  it('stops a running agent by ending its run, after asking, and never by the emergency stop', async () => {
    const { posted } = await mount('#/team/' + SCRIBE, { ...routes, ['POST /runtime/sessions/' + LIVE + '/end']: receipt({ kind: 'ended' }) });
    await click($$('.team-foot-act').find((el) => el.textContent === 'Stop') ?? null);
    expect(text()).toContain('Stop Scribe? What it has not saved is lost. You can start it again afterwards.');
    expect(posted.filter((entry) => entry.path.endsWith('/end'))).toEqual([]);
    await click(button('Stop Scribe'));
    expect(posted.filter((entry) => entry.path.endsWith('/end')).map((entry) => entry.path)).toEqual(['/runtime/sessions/' + LIVE + '/end']);
    expect(posted.some((entry) => entry.path === '/agents/' + SCRIBE + '/stop')).toBe(false);
  });

  it('starts a stopped agent in its own pane with one press, on its one allowed computer, in its saved folder', async () => {
    let live = false;
    const start = started(SCRIBE);
    const { posted } = await mount('#/', {
      ...stopped,
      '/runtime/live': () => ok({ sessions: live ? [running] : [], unanswered: [] }),
      ['POST /agents/' + SCRIBE + '/start-command']: (body) => { live = true; return start(body); },
    });
    expect(location.hash).toBe('#/team/' + SCRIBE);
    expect(text()).toContain('Scribe is not running.');
    expect(text()).toContain('On Lab, in the folder ' + FOLDER + '.');
    expect($('.team-start select')).toBeNull();
    expect((button('Start') as HTMLButtonElement).disabled).toBe(false);
    await click(button('Start'));
    // The terminal that opens afterwards sends its own requests; the start itself is one request.
    const starts = posted.filter((entry) => entry.path.startsWith('/agents/'));
    expect(starts).toHaveLength(1);
    expect(starts[0]).toMatchObject({ path: '/agents/' + SCRIBE + '/start-command', body: { machine: LAB } });
    expect($('.team-pane .terminal')).not.toBeNull();
    expect($$('.team-foot-act').map((el) => el.textContent)).toContain('Stop');
    expect(sessionStorage.getItem('lys.pending.agent-start.' + ADA + '.' + SCRIBE)).toBeNull();
  });

  it('approves unapproved settings as the person and starts, in the same one press', async () => {
    const review = '/agents/' + SCRIBE + '/provisioning/1/review';
    const { posted } = await mount('#/team/' + SCRIBE, {
      ...stopped, ...provisioning(SCRIBE, profile({ reviewed_by: null, reviewed_at: null })),
      ['POST ' + review]: (body) => ok({ agent: SCRIBE, profile: profile(), recorded: { operation: (body as { operation: string }).operation, version: 1 }, versions: [], enforced: false }),
      ['POST /agents/' + SCRIBE + '/start-command']: started(SCRIBE),
    });
    await click(button('Start'));
    expect(posted.map((entry) => entry.path)).toEqual([review, '/agents/' + SCRIBE + '/start-command']);
  });

  it('offers each computer as its own Start when several are allowed and none is saved, and uses the saved one when there is one', async () => {
    const two = { ...stopped, '/network': ok({ machines: [machine(LAB, 'Lab', 'lys-runner', [SCRIBE]), machine(SHED, 'Shed', 'lys-runner', [SCRIBE])], reports_served: true }) };
    await mount('#/team/' + SCRIBE, two);
    expect($$('.team-start-acts button').map((el) => el.textContent)).toEqual(['Start on Lab', 'Start on Shed']);
    await mount('#/team/' + SCRIBE, { ...two, ...provisioning(SCRIBE, profile({ runs_on: SHED })) });
    expect(text()).toContain('On Shed, in the folder ' + FOLDER + '.');
  });

  it('says a paused agent is paused, turns it back on in place and starts it without a second press', async () => {
    let state = 'suspended';
    const { posted } = await mount('#/team/' + ARCHIVIST, {
      ...stopped, '/teams': ok({ teams: [team('team-2', ADA, 'Crew', [ARCHIVIST])] }),
      '/network': ok({ machines: [machine(LAB, 'Lab', 'lys-runner', [ARCHIVIST])], reports_served: true }),
      ...provisioning(ARCHIVIST, profile()),
      ['/directory/agents/' + ARCHIVIST]: () => ok({ ...SCRIBE_VIEW, id: ARCHIVIST, display_name: 'Archivist', state }),
      ['POST /identities/' + ARCHIVIST + '/transitions']: () => { state = 'active'; return ok({}); },
      ['POST /agents/' + ARCHIVIST + '/start-command']: started(ARCHIVIST),
    });
    expect(text()).toContain('Archivist is paused.');
    expect(button('Start')).toBeNull();
    await click(button('Turn back on'));
    expect(posted.map((entry) => entry.path)).toEqual(['/identities/' + ARCHIVIST + '/transitions', '/agents/' + ARCHIVIST + '/start-command']);
    expect(posted[0].body).toMatchObject({ transition: 'reinstate' });
  });

  it('says what stops a start before any press: no computer, no folder, no program', async () => {
    await mount('#/team/' + SCRIBE, { ...stopped, '/network': ok({ machines: [], reports_served: true }) });
    expect(text()).toContain('No computer is allowed to run Scribe.');
    expect(button('Start')).toBeNull();
    await mount('#/team/' + SCRIBE, { ...stopped, ...provisioning(SCRIBE, profile({ working_folder: undefined })) });
    expect(text()).toContain('Scribe has no folder to work in.');
    await mount('#/team/' + SCRIBE, { ...stopped, ...provisioning(SCRIBE, null) });
    expect(text()).toContain('Scribe has no program chosen yet.');
    await click(button('Choose its program'));
    expect($('.team-settings')?.getAttribute('aria-label')).toBe('Scribe settings');
  });

  it('says the service\'s own reason when it refuses the start, with Try again, and no greyed button', async () => {
    await mount('#/team/' + SCRIBE, { ...stopped, ['POST /agents/' + SCRIBE + '/start-command']: refused(409, 'SecretsUnavailable', 'the secrets store is closed') });
    await click(button('Start'));
    expect(text()).toContain('The secrets store is closed.');
    expect(text()).toContain('SecretsUnavailable');
    expect((button('Try again') as HTMLButtonElement).disabled).toBe(false);
    expect($$('button').filter((el) => (el as HTMLButtonElement).disabled && el.closest('.team-start'))).toEqual([]);
    expect($('.team-start details')).toBeNull();
  });

  it('sends the old start address to the agent\'s pane on the front page', async () => {
    await mount('#/file/' + SCRIBE + '/start', stopped);
    expect(location.hash).toBe('#/team/' + SCRIBE);
  });

  it('names the refusal when the caller is not signed in', async () => {
    await mount('#/team', { ...routes, '/teams': refused(401, 'NotSignedIn', 'sign in first') });
    expect(text()).toContain('not signed in');
  });
});
