import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from '../src/App';
import { $, $$, choose, click, serve, settle, text } from './harness';
import { ADA, ME, SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';

let root: Root | null = null;
afterEach(() => { if (root) act(() => root?.unmount()); root = null; });

const prefix = '/agents/' + SCRIBE;
const session = {
  session: 'op-' + 'a'.repeat(32), agent: SCRIBE, machine: 'computer-one', machine_name: 'Ward computer',
  runtime: 'runner', shown: 'running', last_reported: 'running', first_report_at: 1790000000,
  last_report_at: 1790000200, what: 'Runner confirmed the process', stopped: null, reported_by: ADA,
};
const ready = { version: 1, operation: 'op-' + 'b'.repeat(32), harness: { name: 'Claude Code', description: {}, program: '/opt/claude', package: 'claude' }, model_access: ['model-one'], runs_on: 'computer-one',
  tools: [], skills: [], mcp_servers: [], instructions: '', note: '', set_by: ADA, set_at: 1790000000, reviewed_by: ADA, reviewed_at: 1790000000, working_folder: '/Users/ada/work' };
const notRunning = { '/runtime/live': ok({ sessions: [], unanswered: [] }), [prefix + '/provisioning']: ok({ agent: SCRIBE, enforced: false, versions: [], profile: ready }) };
const run = () => $('section[aria-label="Run"]');
const routes = {
  ...SERVICE,
  '/teams': ok({ teams: [{ id: 'team-one', name: 'Care team', state: 'active', members: [SCRIBE] }] }),
  '/network': ok({ machines: [{ id: 'computer-one', name: 'Ward computer', state: 'in_use', runtime: 'runner', may_run: [{ id: SCRIBE }], may_run_roles: [] }], reports_served: true }),
  [prefix + '/provisioning']: ok({ agent: SCRIBE, enforced: false, versions: [], profile: {
    version: 1, operation: 'op-' + 'b'.repeat(32), model_access: ['model-one'], runs_on: 'computer-one',
    tools: [], skills: [], mcp_servers: [], instructions: '', note: '', set_by: ADA, set_at: 1790000000,
  } }),
  '/runtime/live': ok({ sessions: [session], unanswered: [] }),
  '/harnesses': ok({ programs: [{ name: 'Claude Code', line: 'claude', models: [{ id: 'model-one', label: 'Model One' }], modes: [], builds: [] }] }),
};

async function open(overrides: Record<string, Route> = {}, hash = '#/file/' + SCRIBE) {
  const posted: { path: string; body: unknown }[] = [];
  const requests = serve({ ...routes, ...overrides }, posted);
  history.replaceState(null, '', '/' + hash);
  const container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => { root?.render(<App />); });
  return { posted, requests };
}

async function follow(link: HTMLElement | null) {
  if (!(link instanceof HTMLAnchorElement)) throw new Error('Navigation link is missing');
  await act(async () => {
    const changed = new Promise<void>((resolve) => window.addEventListener('hashchange', () => resolve(), { once: true }));
    link.click();
    await changed;
  });
}

describe('An agent page explains the agent before its controls', () => {
  it('shows its run with its acts first, then its person, team, computer and saved model, and repeats no tab as a card', async () => {
    const { posted, requests } = await open();
    const overview = $('[aria-label="About this agent"]');
    expect(overview?.textContent).toContain('Ada (test person)');
    expect(overview?.textContent).toContain('Care team');
    expect(overview?.textContent).toContain('Ward computer');
    expect(overview?.textContent).toContain('Model One');
    expect(overview?.textContent).not.toContain('model-one');
    expect(run()?.textContent).toContain('Scribe is running on Ward computer, as its runner last reported.');
    expect(run()?.querySelector('[data-act="watch"]')?.getAttribute('href')).toBe('#/canvas/' + SCRIBE);
    expect(run()?.querySelector('[data-act="restart"]')).not.toBeNull();
    expect(run()?.querySelector('[data-act="stop"]')).not.toBeNull();
    expect(run()?.querySelector('.terminal')).toBeNull();
    expect((run()?.compareDocumentPosition(overview as Node) ?? 0) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect($('[aria-label="Next steps"]')).toBeNull();
    expect($$('.file nav.tabs a').map((entry) => entry.getAttribute('href'))).toEqual(expect.arrayContaining(['#/file/' + SCRIBE + '/budgets', '#/file/' + SCRIBE + '/access']));
    expect($('[data-act="start"]')).toBeNull();
    expect(run()?.textContent).not.toContain('is not running');
    expect($('.file .head')?.textContent).toContain('Edit name');
    expect($('.file .head [data-act="suspend"]')).not.toBeNull();
    expect(requests).not.toContain('/receipts/4');
    expect($('.agent-details')).toBeNull();
    expect($('.file details')).toBeNull();
    expect($('.file .head .fileno')?.textContent).toBe(SCRIBE);
    expect(posted).toEqual([]);
  });

  it('adds the agent to a team and removes it from one on its own page, each as one confirmed change', async () => {
    const teams = [{ id: 'team-one', name: 'Care team', state: 'active', owner: ADA, members: [SCRIBE] }, { id: 'team-two', name: 'Night team', state: 'active', owner: ADA, members: [] as string[] }];
    const recorded = (body: unknown, act: string) => ({ operation: (body as { operation: string }).operation, act, member: SCRIBE, by: ME.signed_in, at: 1790000000 });
    const { posted } = await open({ '/teams': () => ok({ teams }),
      'POST /teams/team-two/members': (body) => { teams[1].members = [SCRIBE]; return ok({ ...teams[1], recorded: recorded(body, 'added') }); },
      ['POST /teams/team-one/members/' + SCRIBE + '/remove']: (body) => { teams[0].members = []; return ok({ ...teams[0], recorded: recorded(body, 'removed') }); } });
    await settle();
    const about = () => $('[aria-label="About this agent"]');
    await choose($('select[aria-label="Add to a team"]'), 'team-two');
    expect(about()?.textContent).toContain('Add Scribe to Night team?');
    expect(posted).toEqual([]);
    await click($$('[aria-label="Confirm team change"] button').find((button) => button.textContent === 'Confirm add member') ?? null);
    await settle();
    expect(posted.at(-1)).toMatchObject({ path: '/teams/team-two/members', body: { member: SCRIBE } });
    expect($('button[aria-label="Remove from Night team"]')).not.toBeNull();
    expect($('select[aria-label="Add to a team"]')).toBeNull();
    await click($('button[aria-label="Remove from Care team"]'));
    expect(about()?.textContent).toContain('Remove Scribe from Care team?');
    await click($$('[aria-label="Confirm team change"] button').find((button) => button.textContent === 'Confirm remove member') ?? null);
    await settle();
    expect(posted.at(-1)?.path).toBe('/teams/team-one/members/' + SCRIBE + '/remove');
    expect(posted).toHaveLength(2);
    expect($('button[aria-label="Remove from Care team"]')).toBeNull();
    expect(text()).not.toContain('team-one');
  });

  it('says a saved model no program lists is unlisted instead of showing a bare id as its name', async () => {
    await open({ '/harnesses': ok({ programs: [{ name: 'Claude Code', line: 'claude', models: [{ id: 'model-two', label: 'Model Two' }], modes: [], builds: [] }] }) });
    expect($('[aria-label="About this agent"]')?.textContent).toContain('model-one (no program lists this model now)');
  });

  it('names an unreadable model list instead of showing the saved id', async () => {
    await open({ '/runtime/live': ok({ sessions: [], unanswered: [] }), '/harnesses': refused(503, 'HarnessesUnavailable', 'The program list could not be read') });
    const overview = $('[aria-label="About this agent"]');
    expect(overview?.textContent).toContain('Lys could not read model names just now');
    expect(overview?.textContent).not.toContain('model-one');
    expect($('[aria-label="Read problems"]')?.textContent).toContain('HarnessesUnavailable');
    expect(run()?.textContent).toContain('Scribe is not running.');
    expect(run()?.textContent).toContain('Scribe has no program chosen yet.');
  });

  it('asks for a computer and model when none is saved', async () => {
    await open({ '/runtime/live': ok({ sessions: [], unanswered: [] }), [prefix + '/provisioning']: ok({ agent: SCRIBE, enforced: false, versions: [], profile: null }) });
    expect($('[aria-label="About this agent"]')?.textContent).toContain('Choose a model when you start');
    expect(run()?.textContent).toContain('Scribe is not running.');
    expect(run()?.textContent).toContain('Scribe has no program chosen yet.');
  });

  it('says an agent no runner lists is not running, apart from the identity being active, and offers Start in place', async () => {
    await open(notRunning);
    expect(run()?.textContent).toContain('Scribe is not running.');
    expect(run()?.textContent).toContain('On Ward computer, in the folder /Users/ada/work.');
    expect([...(run()?.querySelectorAll('button') ?? [])].map((entry) => entry.textContent)).toEqual(['Start']);
    expect(run()?.textContent).not.toContain('Stopped');
    expect($('#state')?.textContent).toBe('Active');
  });

  it('never says running for a run whose runner did not answer or whose state is unconfirmed', async () => {
    await open({ '/runtime/live': ok({ sessions: [session], unanswered: [{ session: session.session, machine: 'computer-one', refusal: 'runner_unreachable', reason: 'no answer' }] }) });
    expect(run()?.textContent).toContain('Whether Scribe is still running on Ward computer is not confirmed');
    expect(run()?.textContent).not.toContain('Scribe is running on');
    act(() => root?.unmount()); root = null;
    await open({ '/runtime/live': ok({ sessions: [{ ...session, shown: 'unconfirmed' }], unanswered: [] }) });
    expect(run()?.textContent).toContain('is not confirmed');
    expect(run()?.textContent).not.toContain('Scribe is running on');
  });

  it('names a failed runtime read instead of inventing an empty or stopped state', async () => {
    await open({ '/runtime/live': refused(503, 'RunnerUnavailable', 'The runner did not answer') });
    expect($('.agent-run')?.textContent).toContain('Lys could not read whether Scribe is running: The runner did not answer');
    expect($('.agent-run')?.textContent).toContain('RunnerUnavailable');
    expect($('.agent-run')?.textContent).not.toContain('is not running');
    expect($('.agent-run button')).toBeNull();
    expect($('[data-act="watch"]')).toBeNull();
  });

  it('does not take another agent’s run for this agent’s', async () => {
    await open({ ...notRunning, '/runtime/live': ok({ sessions: [{ ...session, agent: 'agent-' + 'f'.repeat(32) }], unanswered: [] }) });
    expect(run()?.textContent).toContain('Scribe is not running.');
    expect(run()?.textContent).not.toContain('Scribe is running on');
    expect($('[data-act="watch"]')).toBeNull();
  });

  it('reads receipts only when the Record tab is opened and keeps lifecycle changes in the head', async () => {
    const first = await open();
    expect(first.requests).not.toContain('/receipts/4');
    expect($('.file .head [data-act="suspend"]')).not.toBeNull();
    expect(first.posted).toEqual([]);
    act(() => root?.unmount()); root = null;
    const { posted, requests } = await open({}, '#/file/' + SCRIBE + '/record');
    expect(requests).toContain('/receipts/4');
    expect(requests).toContain('/receipts/5');
    expect($('.file .pane')?.textContent).toContain('registered, under its person');
    // A line signed for the signed-in person's own sign-in names that person, not the account's subject.
    await settle();
    expect($('.file .pane')?.textContent).toContain('by Ada (test person)');
    expect($('.file .pane')?.textContent).not.toContain('by ada');
    expect($('.file .head [data-act="suspend"]')).not.toBeNull();
    expect($('.file details.agent-details')).toBeNull();
    expect(posted).toEqual([]);
  });

  it('names the missing computer permission without presenting activation as readiness, with the fix in place', async () => {
    const { posted } = await open({ ...notRunning, '/network': ok({ machines: [{ id: 'computer-one', name: 'Ward computer', state: 'in_use', runtime: 'runner', may_run: [], may_run_roles: [] }], reports_served: true }) });
    expect($('.file .head')?.textContent).toContain('Scribe is added.');
    expect($('.file .head')?.textContent).not.toContain('finish setting it up');
    expect($('.file .head')?.textContent).not.toContain('Scribe is ready');
    expect(run()?.textContent).toContain('No computer is allowed to run Scribe.');
    expect([...(run()?.querySelectorAll('button') ?? [])].map((entry) => entry.textContent)).toEqual(['Allow on this computer']);
    expect(run()?.querySelector('a[href^="#/team"]')).toBeNull();
    expect(posted).toEqual([]);
  });

  it.each([
    { name: 'no computers', machines: [] },
    { name: 'only a retired computer', machines: [{ id: 'computer-one', name: 'Ward computer', state: 'retired', runtime: 'runner', may_run: [{ id: SCRIBE }], may_run_roles: [] }] },
  ])('opens the existing add form when Lys has $name', async ({ machines }) => {
    const { posted } = await open({ ...notRunning, '/network': ok({ machines, reports_served: true }), '/network/machines/computer-one/runner': ok({ runner: null }) });
    expect(run()?.textContent).toContain('No computer is allowed to run Scribe.');
    expect(run()?.textContent).toContain('No computer has Lys running on it yet.');
    const add = [...(run()?.querySelectorAll('a') ?? [])].find((link) => link.textContent === 'Add a computer') ?? null;
    expect(add?.getAttribute('href')).toBe('#/network?add=computer');
    expect($('[data-act="start"]')).toBeNull();
    await follow(add);
    expect($('form[aria-label="Add a computer"]')).not.toBeNull();
    expect(posted).toEqual([]);
  });

  it('keeps an unreadable computer list unknown instead of offering to add a first computer', async () => {
    const { posted } = await open({ ...notRunning, '/network': refused(503, 'NetworkUnavailable', 'The computer list could not be read') });
    expect($('[aria-label="Read problems"]')?.textContent).toContain('NetworkUnavailable');
    expect(run()?.textContent).toContain('Lys could not read what Scribe needs to start.');
    expect(run()?.textContent).not.toContain('No computer has Lys running on it yet');
    expect($('a[href="#/network?add=computer"]')).toBeNull();
    expect([...(run()?.querySelectorAll('button') ?? [])].map((entry) => entry.textContent)).toEqual(['Try again']);
    expect(posted).toEqual([]);
  });

  it('keeps the add-computer deep link administrator-only', async () => {
    const { posted } = await open({ '/network': ok({ machines: [], reports_served: true }), '/directory/people': refused(403, 'NotAdmitted', 'not administrator') }, '#/network?add=computer');
    expect($('.page h1')?.textContent).toBe('Computers');
    expect($('form[aria-label="Add a computer"]')).toBeNull();
    expect(posted).toEqual([]);
  });

  it.each([
    { tab: 'provisioning', control: '[aria-label="Settings of this agent"]' },
    { tab: 'budgets', control: 'section.usage' },
    { tab: 'access', control: '[data-act="grant"]' },
  ])('shows $tab in the open, under visible tabs, without reading receipts', async ({ tab, control }) => {
    const { posted, requests } = await open({}, '#/file/' + SCRIBE + '/' + tab);
    const form = $(control);
    const tabs = $('.file nav.tabs');
    if (!form || !tabs) throw new Error('Agent settings or tabs are missing');
    expect(form.closest('details')).toBeNull();
    expect(tabs.closest('details')).toBeNull();
    expect(tabs.compareDocumentPosition(form) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect($('.agent-details')).toBeNull();
    expect(requests).not.toContain('/receipts/4');
    expect(requests).not.toContain('/receipts/5');
    const stop = $('.file .head [data-act="stop"]');
    expect(stop).not.toBeNull();
    expect(stop?.closest('details')).toBeNull();
    expect(posted).toEqual([]);
  });
});
