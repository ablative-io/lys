/** An agent's run is on the agent's own page: where it is running with Stop and Restart and a link to its terminal on the canvas, a stopped agent started in place with one press, and what stops a start said in one sentence with the button that fixes it. */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, text, leaveTheReachReadOutOfPosted, unmountAll } from './harness';
// These cases land on People and agents, which asks what each agent reaches; that read is not one of the flow's changes.
leaveTheReachReadOutOfPosted();
import { ADA, ARCHIVIST, COURIER, REVIEWER, SCRIBE, SCRIBE_VIEW, SERVICE, dashboard, ok, refused } from './fixtures';
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
const button = (words: string) => $$('.you-asked button, .agent-run button').find((el) => (el.getAttribute('aria-label') ?? el.textContent) === words) ?? null;

const routes = {
  ...SERVICE,
  '/teams': ok({ teams: [team('team-1', ADA, 'Ada team', [REVIEWER]), team('team-2', SCRIBE, 'Crew', [COURIER, ARCHIVIST])] }),
  '/runtime/live': ok({ sessions: [running], unanswered: [] }),
  '/dashboard': ok(dashboard({ [SCRIBE]: { sessions: [running] } })),
  '/network': ok({ machines: [machine(LAB, 'Lab', 'lys-runner', [SCRIBE]), machine(SHED, 'Shed', null, [])], reports_served: true }),
  ['POST /runtime/sessions/' + LIVE + '/resize']: receipt({ kind: 'resized' }),
  ['POST /runtime/sessions/' + LIVE + '/read-bytes']: refused(502, 'runner_unreachable', 'the read was closed'),
};
/** Scribe alone, stopped, with approved settings and one computer allowed to run it. */
const stopped = { ...routes, '/teams': ok({ teams: [] }), '/runtime/live': ok({ sessions: [], unanswered: [] }), '/dashboard': ok(dashboard()), ...provisioning(SCRIBE, profile()) };

describe('An agent\'s run on People and agents', () => {
  beforeEach(() => { sessionStorage.clear(); });

  it.each([FOLDER, '/earlier/project'])('sends the displayed folder and explains a change from %s', async (earlier) => {
    const { posted } = await mount('#/file/' + SCRIBE, {
      ...stopped,
      ['/agents/' + SCRIBE + '/start-command?machine=' + LAB]: ok({ machine: LAB, directory: earlier }),
      ['POST /agents/' + SCRIBE + '/start-command']: started(SCRIBE),
    });
    const words = 'This starts a new conversation in ' + FOLDER + '; the earlier one stays in ' + earlier;
    if (earlier === FOLDER) expect(text()).not.toContain(words);
    else expect(text()).toContain(words);
    await click(button('Start'));
    expect(posted.find((entry) => entry.path === '/agents/' + SCRIBE + '/start-command')?.body).toMatchObject({ machine: LAB, directory: FOLDER });
  });

  it('has no page of its own: the Dashboard lists your agents, each with Start or Stop in its own row, and the agent\'s own page says where it runs with its acts, with no terminal and no side pane on People and agents', async () => {
    await mount('#/', routes);
    // Stop all sits in the Agents heading while something runs (Scribe does here).
    expect($('section[aria-label="Agents"] .section-h')?.textContent).toBe('Agents');
    expect($('.team-screen')).toBeNull();
    expect($('a[href="#/team"]')).toBeNull();
    expect($$('tr[data-href]').find((row) => row.textContent?.includes('Scribe'))?.querySelector('[data-act="stop"]')).not.toBeNull();
    expect($$('button[data-act="start"]').length).toBeGreaterThan(0);
    await mount('#/people/' + SCRIBE, routes);
    expect($('.page .head h1')?.textContent).toBe('People and agents');
    expect($('.detail')).toBeNull();
    expect($('.agent-run')).toBeNull();
    expect($('.terminal')).toBeNull();
    await mount('#/file/' + SCRIBE, routes);
    const run = $('section[aria-label="Run"]');
    expect(run?.textContent).toContain('Scribe is running on Lab');
    expect(run?.querySelector('.terminal')).toBeNull();
    expect(run?.querySelector('[data-act="watch"]')?.getAttribute('href')).toBe('#/canvas/' + SCRIBE);
    expect($$('.file nav.tabs a').map((el) => el.getAttribute('href'))).toContain('#/file/' + SCRIBE + '/provisioning');
  });

  it('offers Set up, not Start, on the row of an agent with no program chosen, and Set up opens the Settings tab of its own page', async () => {
    await mount('#/', { ...stopped, ...provisioning(SCRIBE, null) });
    const row = () => $$('tr[data-href]').find((each) => each.textContent?.includes('Scribe'));
    expect(row()?.querySelector('[data-act="start"]')).toBeNull();
    expect(row()?.querySelector('[data-act="setup"]')?.getAttribute('href')).toBe('#/file/' + SCRIBE + '/provisioning');
    // The settings form has one home. No copy of it opens under the row.
    expect($('.you-setup')).toBeNull();
    expect($('.you-asked')).toBeNull();
    unmountAll(); document.body.innerHTML = '';
    await mount('#/file/' + SCRIBE + '/provisioning', { ...stopped, ...provisioning(SCRIBE, null) });
    expect($('section[aria-label="Settings of this agent"]')).not.toBeNull();
    expect($$('section[aria-label="Settings of this agent"] button[type="submit"]').map((el) => [el.getAttribute('aria-label'), el.textContent])).toEqual([['Save these settings', 'Save']]);
  });

  it('stops a running agent from its row on the front page, after asking', async () => {
    const { posted } = await mount('#/', { ...routes, ['POST /runtime/sessions/' + LIVE + '/stop']: receipt({ kind: 'ended' }) });
    await click($('button[data-act="stop"]'));
    expect(text()).toContain('Stop Scribe? This ends its run on Lab. What it has not saved is lost. You can start it again afterwards.');
    expect(posted.filter((entry) => entry.path.endsWith('/stop'))).toEqual([]);
    await click(button('Stop Scribe'));
    expect(posted.filter((entry) => entry.path.endsWith('/stop')).map((entry) => entry.path)).toEqual(['/runtime/sessions/' + LIVE + '/stop']);
  });

  it('keeps suspension in the named menu action and gives Stop only to the live run', async () => {
    await mount('#/file/' + SCRIBE, routes);
    expect($('.file .head [data-act="stop"]')).toBeNull();
    expect($('section[aria-label="Run"] [data-act="stop"]')?.getAttribute('aria-label')).toBe('Stop');
    await click($('.file .head [aria-label="More actions"]'));
    expect($('.file .head [data-act="suspend"]')?.textContent).toBe('Withdraw');
    await click($('.file .head [data-act="suspend"]'));
    expect($('form[aria-label="Confirm withdraw"]')?.textContent).toContain('withdraws its certificates');
    expect($('form[aria-label="Confirm withdraw"] button[type="submit"]')?.getAttribute('aria-label')).toBe('Withdraw this agent now');
  });

  it('starts a stopped agent from its row on the front page with one press', async () => {
    let live = false;
    const start = started(SCRIBE);
    const { posted } = await mount('#/', {
      ...stopped,
      '/runtime/live': () => ok({ sessions: live ? [running] : [], unanswered: [] }),
      '/dashboard': () => ok(dashboard(live ? { [SCRIBE]: { sessions: [running] } } : {})),
      ['POST /agents/' + SCRIBE + '/start-command']: (body) => { live = true; return start(body); },
    });
    await click($$('tr[data-href]').find((row) => row.textContent?.includes('Scribe'))?.querySelector('[data-act="start"]') ?? null);
    const starts = posted.filter((entry) => entry.path.startsWith('/agents/'));
    expect(starts).toHaveLength(1);
    expect(starts[0]).toMatchObject({ path: '/agents/' + SCRIBE + '/start-command', body: { machine: LAB } });
    expect($$('tr[data-href]').find((row) => row.textContent?.includes('Scribe'))?.querySelector('[data-act="stop"]')).not.toBeNull();
  });

  it('starts through the start route when capability reads are refused, and says the refusal beside Start', async () => {
    const { posted } = await mount('#/file/' + SCRIBE, {
      ...stopped,
      '/surface-contract': refused(503, 'CapabilityUnavailable', 'the permission schema is unavailable'),
      ['/harnesses?machine=' + LAB]: refused(503, 'runner_unreachable', 'the catalogue read was refused'),
      ['POST /agents/' + SCRIBE + '/start-command']: started(SCRIBE),
    });
    // The sentence and its fix are said; the service's own words stay under Error details.
    expect(text()).toContain('On Lab: Lys could not reach the chosen computer’s runner. Check that the computer and its runner are connected.');
    expect($('.run-start .why-not[role="alert"] > p')?.textContent).not.toContain('runner_unreachable');
    expect($('.run-start .why-not details .refusal-name')?.textContent).toBe('runner_unreachable: the catalogue read was refused');
    expect(text()).toContain('CapabilityUnavailable');
    expect((button('Start') as HTMLButtonElement).disabled).toBe(false);
    await click(button('Start'));
    expect(posted.filter((entry) => entry.path.endsWith('/start-command'))).toHaveLength(1);
    expect(posted.some((entry) => entry.path.endsWith('/transitions'))).toBe(false);
  });

  it('restarts a running agent in place, after asking, with one request', async () => {
    const { posted } = await mount('#/file/' + SCRIBE, { ...routes, ['POST /agents/' + SCRIBE + '/restart']: ok({}) });
    await click($('section[aria-label="Run"] [data-act="restart"]'));
    expect(text()).toContain('Restart Scribe? This ends its run and starts it again in the same folder, on Lab. What it has not saved is lost.');
    expect(posted.filter((entry) => entry.path.endsWith('/restart'))).toEqual([]);
    await click(button('Restart Scribe'));
    expect(posted.filter((entry) => entry.path.endsWith('/restart'))).toMatchObject([{ path: '/agents/' + SCRIBE + '/restart', body: { session: LIVE } }]);
  });

  // A clinician reads the question at a glance: that the work in progress is not saved first is a line of its own, never inside the sentence.
  it('says on a line of its own, before Stop or Restart, that Lys does not save the agent’s work first and how to keep it', async () => {
    await mount('#/file/' + SCRIBE, routes);
    await click($('section[aria-label="Run"] [data-act="stop"]'));
    expect($('[role="alertdialog"] strong.run-loss')?.textContent).toBe('Lys does not save Scribe’s work first. To keep it, choose Keep it running and let Scribe finish.');
    await click(button('Keep it running'));
    await click($('section[aria-label="Run"] [data-act="restart"]'));
    expect($('[role="alertdialog"] strong.run-loss')?.textContent).toBe('Lys does not save Scribe’s work first. To keep it, choose Keep it running and let Scribe finish.');
  });

  it('stops a running agent by ending its run, after asking, and never by the emergency stop', async () => {
    const { posted } = await mount('#/file/' + SCRIBE, { ...routes, ['POST /runtime/sessions/' + LIVE + '/stop']: receipt({ kind: 'ended' }) });
    await click($('section[aria-label="Run"] [data-act="stop"]'));
    expect(text()).toContain('Stop Scribe? This ends its run on Lab. What it has not saved is lost. You can start it again afterwards.');
    expect(posted.filter((entry) => entry.path.endsWith('/stop'))).toEqual([]);
    await click(button('Stop Scribe'));
    expect(posted.filter((entry) => entry.path.endsWith('/stop')).map((entry) => entry.path)).toEqual(['/runtime/sessions/' + LIVE + '/stop']);
    expect(posted.some((entry) => entry.path === '/agents/' + SCRIBE + '/stop')).toBe(false);
  });

  it('starts a stopped agent in its own pane with one press, on its one allowed computer, in its saved folder', async () => {
    let live = false;
    const start = started(SCRIBE);
    const { posted } = await mount('#/file/' + SCRIBE, {
      ...stopped,
      '/runtime/live': () => ok({ sessions: live ? [running] : [], unanswered: [] }),
      '/dashboard': () => ok(dashboard(live ? { [SCRIBE]: { sessions: [running] } } : {})),
      ['POST /agents/' + SCRIBE + '/start-command']: (body) => { live = true; return start(body); },
    });
    expect(location.hash).toBe('#/file/' + SCRIBE);
    expect(text()).toContain('Scribe is not running.');
    expect(text()).toContain('On Lab, in the folder ' + FOLDER + '.');
    expect($('.run-start select')).toBeNull();
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
    const { posted } = await mount('#/file/' + SCRIBE, {
      ...stopped, ...provisioning(SCRIBE, profile({ reviewed_by: null, reviewed_at: null })),
      ['POST ' + review]: (body) => ok({ agent: SCRIBE, profile: profile(), recorded: { operation: (body as { operation: string }).operation, version: 1 }, versions: [], enforced: false }),
      ['POST /agents/' + SCRIBE + '/start-command']: started(SCRIBE),
    });
    await click(button('Start'));
    expect(posted.map((entry) => entry.path)).toEqual([review, '/agents/' + SCRIBE + '/start-command']);
  });

  it('offers each computer as its own Start when several are allowed and none is saved, and uses the saved one when there is one', async () => {
    const two = { ...stopped, '/network': ok({ machines: [machine(LAB, 'Lab', 'lys-runner', [SCRIBE]), machine(SHED, 'Shed', 'lys-runner', [SCRIBE])], reports_served: true }) };
    await mount('#/file/' + SCRIBE, two);
    expect($$('.run-start-acts button').map((el) => el.textContent)).toEqual(['Start on Lab', 'Start on Shed']);
    await mount('#/file/' + SCRIBE, { ...two, ...provisioning(SCRIBE, profile({ runs_on: SHED })) });
    expect(text()).toContain('On Shed, in the folder ' + FOLDER + '.');
  });

  it('says a paused agent is paused, turns it back on in place and starts it without a second press', async () => {
    let state = 'suspended';
    const { posted } = await mount('#/file/' + ARCHIVIST, {
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
    await mount('#/file/' + SCRIBE, { ...stopped, '/network': ok({ machines: [], reports_served: true }) });
    expect(text()).toContain('No computer is allowed to run Scribe.');
    expect(button('Start')).toBeNull();
    await mount('#/file/' + SCRIBE, { ...stopped, ...provisioning(SCRIBE, profile({ working_folder: undefined })) });
    expect(text()).toContain('Scribe has no folder to work in.');
    await mount('#/file/' + SCRIBE, { ...stopped, ...provisioning(SCRIBE, null) });
    expect(text()).toContain('Scribe has no program chosen yet.');
    await click(button('Choose its program'));
    // Its settings have one home: the Settings tab of its own page.
    expect(location.hash).toBe('#/file/' + SCRIBE + '/provisioning');
    expect($('.you-setup')).toBeNull();
    expect($('section[aria-label="Settings of this agent"]')).not.toBeNull();
  });

  it('says the service\'s own reason when it refuses the start, with Try again, and no greyed button', async () => {
    await mount('#/file/' + SCRIBE, { ...stopped, ['POST /agents/' + SCRIBE + '/start-command']: refused(409, 'SecretsUnavailable', 'the secrets store is closed') });
    await click(button('Start'));
    // The sentence and its fix are said; the service's own words stay under closed Error details.
    expect(text()).toContain('Lys could not reach or use its secret storage service. Ask the administrator to check it.');
    expect(text()).toContain('SecretsUnavailable: the secrets store is closed');
    expect((button('Try again') as HTMLButtonElement).disabled).toBe(false);
    expect($$('button').filter((el) => (el as HTMLButtonElement).disabled && el.closest('.run-start'))).toEqual([]);
    expect($$('.run-start details').map((each) => [each.querySelector('summary')?.textContent, each.hasAttribute('open')])).toEqual([['Error details', false]]);
  });

  it('sends the old start addresses to the agent\'s own page, where Start is', async () => {
    await mount('#/file/' + SCRIBE + '/start', stopped);
    expect(location.hash).toBe('#/file/' + SCRIBE);
    expect($('section[aria-label="Run"]')?.textContent).toContain('Scribe is not running.');
    await mount('#/team/' + SCRIBE, stopped);
    expect(location.hash).toBe('#/file/' + SCRIBE);
  });

  it('says so when it cannot read whether the agent is running, and offers no Start it cannot stand behind', async () => {
    await mount('#/file/' + SCRIBE, { ...routes, '/runtime/live': refused(503, 'RuntimeUnavailable', 'no reports store') });
    expect(text()).toContain('Lys could not read whether Scribe is running');
    expect(text()).toContain('RuntimeUnavailable');
    expect(button('Start')).toBeNull();
  });
});
