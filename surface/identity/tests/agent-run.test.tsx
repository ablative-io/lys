/** People and agents is the front page, and an agent's run is beside the list there: a running agent's terminal with Stop and Restart, a stopped agent started in place with one press, and what stops a start said in one sentence with the button that fixes it. */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, text, leaveTheReachReadOutOfPosted } from './harness';
// These cases land on People and agents, which asks what each agent reaches; that read is not one of the flow's changes.
leaveTheReachReadOutOfPosted();
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
// A button inside what one agent's Start or Stop opened: the row under it on the front page, or its pane on People and agents. Every stopped agent's row has its own Start, so the whole page is never searched.
const button = (words: string) => $$('.you-asked button, .agent-run button').find((el) => el.textContent === words) ?? null;

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

describe('An agent\'s run on People and agents', () => {
  beforeEach(() => { sessionStorage.clear(); });

  it('has no page of its own: the front page is your agents, each with Start or Stop in its own row, and People and agents shows a chosen agent\'s terminal beside the list', async () => {
    await mount('#/', routes);
    expect(text()).toContain('Your agents');
    expect($('.team-screen')).toBeNull();
    expect($('a[href="#/team"]')).toBeNull();
    expect($$('tr[data-href]').find((row) => row.textContent?.includes('Scribe'))?.querySelector('[data-act="stop"]')).not.toBeNull();
    expect($$('button[data-act="start"]').length).toBeGreaterThan(0);
    await mount('#/people/' + SCRIBE, routes);
    expect($('.page .head h1')?.textContent).toBe('People and agents');
    expect($('.detail h2')?.textContent).toBe('Scribe');
    expect($('.detail .team-pane .terminal')).not.toBeNull();
    expect($('.team-foot')?.textContent).toContain('Lab');
    expect($$('.team-foot-act').find((el) => el.textContent === 'Settings')?.getAttribute('href')).toBe('#/file/' + SCRIBE + '/provisioning');
  });

  it('offers Set up, not Start, on the row of an agent with no program chosen, and sets it up under that row', async () => {
    await mount('#/', { ...stopped, ...provisioning(SCRIBE, null) });
    const row = () => $$('tr[data-href]').find((each) => each.textContent?.includes('Scribe'));
    expect(row()?.querySelector('[data-act="start"]')).toBeNull();
    await click(row()?.querySelector('[data-act="setup"]') ?? null);
    expect($('.you-setup section[aria-label="Settings of this agent"]')).not.toBeNull();
    // One button under the form: it saves and goes on to the start. There is no second button to find afterwards.
    expect(button('Save and start Scribe')).not.toBeNull();
    expect(button('Save these settings')).toBeNull();
    expect(button('Done, back to Start')).toBeNull();
    expect(location.hash).toBe('#/');
    await click(button('Close'));
    expect($('.you-asked')).toBeNull();
    expect(row()?.querySelector('[data-act="setup"]')).not.toBeNull();
  });

  it('stops a running agent from its row on the front page, after asking', async () => {
    const { posted } = await mount('#/', { ...routes, ['POST /runtime/sessions/' + LIVE + '/end']: receipt({ kind: 'ended' }) });
    await click($('button[data-act="stop"]'));
    expect(text()).toContain('Stop Scribe? What it has not saved is lost. You can start it again afterwards.');
    expect(posted.filter((entry) => entry.path.endsWith('/end'))).toEqual([]);
    await click(button('Stop Scribe'));
    expect(posted.filter((entry) => entry.path.endsWith('/end')).map((entry) => entry.path)).toEqual(['/runtime/sessions/' + LIVE + '/end']);
  });

  it('starts a stopped agent from its row on the front page with one press', async () => {
    let live = false;
    const start = started(SCRIBE);
    const { posted } = await mount('#/', {
      ...stopped,
      '/runtime/live': () => ok({ sessions: live ? [running] : [], unanswered: [] }),
      ['POST /agents/' + SCRIBE + '/start-command']: (body) => { live = true; return start(body); },
    });
    await click($$('tr[data-href]').find((row) => row.textContent?.includes('Scribe'))?.querySelector('[data-act="start"]') ?? null);
    const starts = posted.filter((entry) => entry.path.startsWith('/agents/'));
    expect(starts).toHaveLength(1);
    expect(starts[0]).toMatchObject({ path: '/agents/' + SCRIBE + '/start-command', body: { machine: LAB } });
    expect($$('tr[data-href]').find((row) => row.textContent?.includes('Scribe'))?.querySelector('[data-act="stop"]')).not.toBeNull();
  });

  it('restarts a running agent in place, after asking, with one request', async () => {
    const { posted } = await mount('#/people/' + SCRIBE, { ...routes, ['POST /agents/' + SCRIBE + '/restart']: ok({}) });
    await click($$('.team-foot-act').find((el) => el.textContent === 'Restart') ?? null);
    expect(text()).toContain('Restart Scribe? This ends the run and starts it again in the same folder, on Lab.');
    expect(posted.filter((entry) => entry.path.endsWith('/restart'))).toEqual([]);
    await click(button('Restart Scribe'));
    expect(posted.filter((entry) => entry.path.endsWith('/restart'))).toMatchObject([{ path: '/agents/' + SCRIBE + '/restart', body: { session: LIVE } }]);
  });

  it('stops a running agent by ending its run, after asking, and never by the emergency stop', async () => {
    const { posted } = await mount('#/people/' + SCRIBE, { ...routes, ['POST /runtime/sessions/' + LIVE + '/end']: receipt({ kind: 'ended' }) });
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
    const { posted } = await mount('#/team/' + SCRIBE, {
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
    expect($('button[data-act="stop"]')).not.toBeNull();
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
    // Setting it up happens under its row on the front page; the person is not sent to another page.
    expect(location.hash).toBe('#/team/' + SCRIBE);
    expect($('.you-setup section[aria-label="Settings of this agent"]')).not.toBeNull();
    // Earlier mounts in this case leave their own Close on the page, so the one under the form is named.
    await click($$('.you-setup button').find((el) => el.textContent === 'Close') ?? null);
    expect($('.you-setup')).toBeNull();
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
    expect(text()).toContain('Your agents');
    expect(text()).toContain('Scribe is not running.');
  });

  it('says so when it cannot read whether the agent is running, and offers no Start it cannot stand behind', async () => {
    await mount('#/people/' + SCRIBE, { ...routes, '/runtime/live': refused(503, 'RuntimeUnavailable', 'no reports store') });
    expect(text()).toContain('Lys could not read whether Scribe is running');
    expect(text()).toContain('RuntimeUnavailable');
    expect(button('Start')).toBeNull();
  });
});
