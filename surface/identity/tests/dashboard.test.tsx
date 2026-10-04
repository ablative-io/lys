/**
 * The front page is the Dashboard: the agents under their teams with where each runs, its budget and its goals from
 * one read; what waits for the person; what is running now; and one running agent's live terminal as a small picture
 * in the bottom right corner when the person presses Watch. You keeps only the account side.
 */
import { act } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { refreshLive } from '../src/live';
import { $, $$, click, mount, settle, text, unmountAll, unreachable } from './harness';
import { ADA, ARCHIVIST, COURIER, SCRIBE, SERVICE, dashboard, ok } from './fixtures';
import { mockTerminal } from './terminal-double';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const LIVE = 'op-' + '5'.repeat(32);
const OTHER = 'op-' + '6'.repeat(32);
const LAB = 'op-' + 'b'.repeat(32);
const session = (id: string, agent: string, machine: string) => ({ session: id, agent, machine: LAB, machine_name: machine, runtime: 'lys-runner', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'started', stopped: null, reported_by: 'the runner' });
const running = session(LIVE, SCRIBE, 'Lab');
const team = (id: string, owner: string, name: string, members: string[]) => ({ id, owner, name, description: '', members, state: 'active', created_by: { provider: 'p', subject: 's' }, created_at: 1790000000, retired_at: null });
const at = (d: number) => new Date(2026, 9, d, 12, 0).getTime() / 1000;
const goal = (id: string, deadline: number | null, standing: string) => ({ goal: { id, kind: 'goal', words: 'a goal', deadline, active: true, evidence: null }, standing });
const terminalOf = (id: string) => ({
  ['POST /runtime/sessions/' + id + '/resize']: ok({ receipt: { index: 1 } }),
  ['POST /runtime/sessions/' + id + '/read-bytes']: ok({ session: id, answer: { kind: 'bytes', output: { session: id, from: 0, cursor: 0, oldest: 0, data: [], ended: null } }, receipt: { index: 2 } }),
});

const routes = {
  ...SERVICE,
  '/teams': ok({ teams: [team('team-1', ADA, 'Crew', [COURIER, ARCHIVIST])] }),
  '/runtime/live': ok({ sessions: [running], unanswered: [] }),
  '/dashboard': ok(dashboard({ [SCRIBE]: { sessions: [running] }, [COURIER]: { teams: ['team-1'] }, [ARCHIVIST]: { teams: ['team-1'] } }, { requests: 1, reviews: 2 })),
};
const scribeBudget = {
  holder: { kind: 'agent', id: SCRIBE }, warn_at: null, zone: 'UTC', version: 2, by: ADA, at: 1790000000, unavailable: [], within: [], unconfirmed: [],
  limits: [{ unit: 'dollars', amount: 500, period: 'week', act: 'stop' }, { unit: 'tokens', amount: 1000, period: 'day', act: 'notice' }],
  used: [{ unit: 'dollars', period: 'week', since_ms: null, figure: 100, unavailable: null }, { unit: 'tokens', period: 'day', since_ms: null, figure: 900, unavailable: null }],
};
const scribeUsage = { agent: SCRIBE, receipts: [], last_reported_ms: 1790000000000, used: scribeBudget.used };
const names = () => $$('tr[data-href]').map((tr) => tr.querySelector('td')?.textContent);
const rowOf = (name: string) => $$('tr[data-href]').find((tr) => tr.querySelector('td')?.textContent === name) ?? null;
const cells = (name: string) => [...(rowOf(name)?.querySelectorAll('td') ?? [])].map((td) => td.textContent);

describe('The Dashboard is the front page', () => {
  it('is the rail\'s first entry, and #/ lands on it', async () => {
    await mount('#/', routes);
    expect($('.page h1')?.textContent).toBe('Dashboard');
    expect($('#rail a.on')?.dataset.nav).toBe('dashboard');
    const first = $$('#rail a[data-nav]')[0];
    expect(first.dataset.nav).toBe('dashboard');
    expect(first.getAttribute('href')).toBe('#/dashboard');
    expect(first.title).toBe('Dashboard (g d)');
    await click($('#rail a[data-nav="people"]'));
    await click($('#rail a[data-nav="dashboard"]'));
    expect(location.hash).toBe('#/dashboard');
    expect($('.page h1')?.textContent).toBe('Dashboard');
  });

  it('shows no terminal until Watch is pressed, and resizes no session', async () => {
    const { posted } = await mount('#/', routes);
    expect($('.peek')).toBeNull();
    expect($('.terminal')).toBeNull();
    expect($('.view')).toBeNull();
    expect($('.dash-picture')).toBeNull();
    expect(posted.filter((call) => call.path.endsWith('/resize'))).toEqual([]);
    expect(unreachable()).toEqual([]);
  });

  it('lists the agents under their teams, teams first, says which run, and folds a team away and back', async () => {
    await mount('#/', routes);
    expect(names()).toEqual(['Courier', 'Archivist', 'Scribe']);
    expect(rowOf('Scribe')?.textContent).toContain('on Lab');
    expect(rowOf('Courier')?.textContent).not.toContain('running');
    expect($('.you-fold')?.textContent).toBe('Crew2');
    await click($('.you-fold'));
    expect(names()).toEqual(['Scribe']);
    expect(localStorage.getItem('iam.you-folded')).toBe('team-1');
    await click($('.you-fold'));
    expect($$('tr[data-href]')).toHaveLength(3);
    expect(text()).not.toContain('Ada team');
    expect($$('.you-tree thead th').map((th) => th.textContent)).toEqual(['Agent', 'Running', 'Budget', 'Goals', '']);
  });

  it('shows each agent\'s budget, tightest first, and its goals from the one read, and a refused budget or usage by its refusal name with the rest of the row still shown', async () => {
    const { requests } = await mount('#/', { ...routes, '/dashboard': ok(dashboard({
      [SCRIBE]: { budget: scribeBudget, usage: scribeUsage, goals: { goals: [goal('g-1', at(9), 'open'), goal('g-2', at(6), 'open'), goal('g-3', null, 'missed'), goal('g-4', null, 'met')] } },
      [COURIER]: { usage: { refusal: 'UsageUnavailable', reason: 'the usage store is closed' }, goals: { goals: [goal('g-5', null, 'missed')] } },
      [ARCHIVIST]: { budget: { refusal: 'BudgetsUnavailable', reason: 'the budgets store is closed' } },
    })) });
    expect(cells('Scribe').slice(2, 4)).toEqual(['900 tokens of 1,000 tokens today · $100 of $500 this week', '2 open, next due 6 Oct, 1 missed']);
    expect(cells('Courier').slice(1, 4)).toEqual(['registered', 'UsageUnavailable', '1 missed']);
    expect(cells('Archivist').slice(1, 4)).toEqual(['suspended', 'BudgetsUnavailable', 'No goal']);
    for (const name of ['Courier', 'Archivist']) expect(rowOf(name)?.querySelectorAll('td')[2].textContent).not.toMatch(/\b0\b|No limit/);
    // One read: no agent's usage, goals or budgets is asked for on its own.
    expect(requests.filter((path) => path === '/dashboard')).toHaveLength(1);
    expect(requests.filter((path) => /\/usage|\/goals|\/budgets/.test(path))).toEqual([]);
  });

  it('shows the widgets: waiting requests and drafts item by item, budgets nearest the limit first, goals missed first, and a part that could not be read by its refusal name', async () => {
    const asked = { id: 'r-1', asked_by: SCRIBE, asked_by_name: 'Scribe', responsible: { id: ADA, display_name: 'Ada (test person)', state: 'active' }, resource: { kind: 'store', id: 's-1' },
      relation: 'reader', actions: ['read'], ends_at: null, why: 'to read the ward list', asked_at: 1790000000, state: 'waiting', approvers: [], sources: [], can_decide: true };
    const decided = { ...asked, id: 'r-2', state: 'approved', why: 'already decided' };
    const prepared = { id: 'op-' + '9'.repeat(32), agent: { id: SCRIBE, display_name: 'Scribe' }, responsible: { id: ADA, display_name: 'Ada (test person)', state: 'active' },
      target: { kind: 'grant', id: 'g-1', action: 'grant.give' }, method: 'POST', path: '/grants', body: '{}', note: 'for the night shift', created_at: 1790000000, creation_hash: 'h', state: 'waiting' };
    await mount('#/', { ...routes, '/requests': ok({ requests: [decided, asked] }), '/drafts?state=waiting': ok({ drafts: [prepared] }), '/dashboard': ok(dashboard({
      [SCRIBE]: { budget: scribeBudget, usage: scribeUsage, goals: { goals: [goal('g-1', at(9), 'open'), goal('g-2', at(6), 'open'), goal('g-3', null, 'missed'), goal('g-4', null, 'met')] } },
      [COURIER]: { goals: { goals: [goal('g-5', null, 'missed')] } },
      [ARCHIVIST]: { budget: { refusal: 'BudgetsUnavailable', reason: 'the budgets store is closed' }, goals: { refusal: 'GoalsUnavailable', reason: 'the goals store is closed' } },
    })) });
    const widget = (label: string) => $('section.dash-widget[aria-label="' + label + '"]');
    expect(widget('Requests')?.querySelector('.dash-count')?.textContent).toBe('1');
    const request = widget('Requests')?.querySelector('tr[data-request="r-1"]');
    expect([...(request?.querySelectorAll('td') ?? [])].map((td) => td.textContent)).toEqual(['Scribe', 'reader: readto read the ward list', 'Decide']);
    expect(request?.querySelector('a')?.getAttribute('href')).toBe('#/requests');
    expect(widget('Requests')?.textContent).not.toContain('already decided');
    expect(widget('Drafts')?.querySelector('.dash-count')?.textContent).toBe('1');
    expect([...(widget('Drafts')?.querySelectorAll('tr[data-draft] td') ?? [])].map((td) => td.textContent)).toEqual(['Scribe', 'grant.give on grantfor the night shift', 'Decide']);
    expect(widget('Drafts')?.textContent).not.toMatch(/op-[0-9a-f]{32}/);
    // Budget: the part that could not be read first, by its name; then the agent nearest its limit; the rest counted once.
    expect(widget('Budget')?.querySelector('.dash-count')?.textContent).toBe('1 limited');
    expect([...(widget('Budget')?.querySelectorAll('tr[data-budget]') ?? [])].map((tr) => tr.getAttribute('data-budget'))).toEqual([ARCHIVIST, SCRIBE]);
    expect(widget('Budget')?.querySelector('tr[data-budget="' + ARCHIVIST + '"]')?.textContent).toBe('ArchivistBudgetsUnavailable');
    expect(widget('Budget')?.querySelector('tr[data-budget="' + SCRIBE + '"] .dash-bar')?.getAttribute('aria-label')).toBe('90% of the nearest limit');
    expect(widget('Budget')?.querySelector('tr.empty')?.textContent).toBe('1 other agent has no limit.');
    // Goals: missed first, then open by deadline, then met; an agent whose goals could not be read is named.
    expect(widget('Goals')?.querySelector('.dash-count')?.textContent).toBe('2 open, 2 missed');
    expect([...(widget('Goals')?.querySelectorAll('tr[data-goal]') ?? [])].map((tr) => tr.getAttribute('data-goal'))).toEqual(['missed', 'missed', 'open', 'open', 'met', 'unread']);
    const goals = widget('Goals')?.textContent ?? '';
    expect(goals.indexOf('due 6 Oct')).toBeGreaterThan(-1);
    expect(goals.indexOf('due 6 Oct')).toBeLessThan(goals.indexOf('due 9 Oct'));
    expect(widget('Goals')?.querySelector('tr[data-goal="unread"]')?.textContent).toBe('ArchivistIts goals could not be read. GoalsUnavailable');
    expect(unreachable()).toEqual([]);
  });

  it('says a widget whose own read was refused by the refusal name, with the rest of the Dashboard still shown', async () => {
    await mount('#/', { ...routes, '/requests': { status: 503, body: { refusal: 'RequestsUnavailable', reason: 'the requests store is closed' } }, '/drafts?state=waiting': { status: 503, body: { refusal: 'DraftsUnavailable', reason: 'closed' } } });
    expect($('section.dash-widget[aria-label="Requests"]')?.textContent).toContain('Requests could not be read. RequestsUnavailable');
    expect($('section.dash-widget[aria-label="Drafts"]')?.textContent).toContain('Drafts could not be read. DraftsUnavailable');
    expect($('section.dash-widget[aria-label="Requests"] .dash-count')?.textContent).toBe('?');
    expect(rowOf('Scribe')).not.toBeNull();
    expect($('section.dash-widget[aria-label="Budget"] tr.empty')?.textContent).toBe('No agent has a limit.');
    expect($('section.dash-widget[aria-label="Goals"] tr.empty')?.textContent).toBe('No goal is set.');
  });

  it('says No limit and No goal for an agent with neither', async () => {
    await mount('#/', routes);
    expect(cells('Courier').slice(2, 4)).toEqual(['No limit', 'No goal']);
  });

  it('says each limit used of limit exactly as the agent\'s Limits and goals page says it, for the same answers', async () => {
    await mount('#/', { ...routes, '/dashboard': ok(dashboard({ [SCRIBE]: { sessions: [running], budget: scribeBudget, usage: scribeUsage } })) });
    const line = cells('Scribe')[2]?.split(' · ') ?? [];
    unmountAll();
    await mount('#/file/' + SCRIBE + '/budgets', { ...routes, ['/budgets/agent/' + SCRIBE]: ok(scribeBudget), ['/agents/' + SCRIBE + '/usage']: ok(scribeUsage), ['/agents/' + SCRIBE + '/goals']: ok({ goals: [] }) });
    const page = $$('.usage-budgets .usage-used').map((td) => td.textContent ?? '');
    expect(page).toHaveLength(2);
    expect([...line].sort()).toEqual([...page].sort());
  });

  it('counts what waits for the person, one in the singular, each a link to its place', async () => {
    await mount('#/', { ...routes, '/dashboard': ok(dashboard({}, { requests: 1, drafts: 2, reviews: 1 })) });
    const rows = $$('section[aria-label="Waiting for you"] tr[data-waiting]');
    expect(rows.map((tr) => [tr.querySelector('.dash-num')?.textContent, tr.querySelector('a')?.textContent, tr.querySelector('a')?.getAttribute('href')])).toEqual([
      ['1', 'request to decide', '#/requests'], ['2', 'drafts to approve', '#/access/drafts'], ['1', 'review due', '#/reviews'],
    ]);
    expect(text()).not.toContain('Nothing is waiting.');
  });

  it('says once that nothing is waiting and nothing is running, in the tables\' bodies', async () => {
    await mount('#/', SERVICE);
    expect($$('tr[data-waiting]')).toEqual([]);
    expect(text().split('Nothing is waiting.')).toHaveLength(2);
    expect($('section[aria-label="Waiting for you"] tbody tr.empty td')?.textContent).toBe('Nothing is waiting.');
    expect(text().split('Nothing is running.')).toHaveLength(2);
    expect($('section[aria-label="Running now"] tbody tr.empty td')?.textContent).toBe('Nothing is running.');
  });

  it('lists what is running now, each with Watch and Stop', async () => {
    await mount('#/', routes);
    const rows = $$('section[aria-label="Running now"] tr[data-running]');
    expect(rows.map((tr) => tr.querySelector('td')?.textContent)).toEqual(['Scribe']);
    expect(rows.map((tr) => tr.dataset.running)).toEqual([LIVE]);
    expect(rows[0].textContent).toContain('on Lab');
    expect([...rows[0].querySelectorAll('button')].map((button) => button.dataset.act)).toEqual(['watch', 'stop']);
  });

  it('stops a running agent from Running now, after asking, in a row under it', async () => {
    const { posted } = await mount('#/', { ...routes, ['POST /runtime/sessions/' + LIVE + '/end']: ok({ session: LIVE, answer: { kind: 'ended' }, receipt: { index: 1 } }) });
    await click($('section[aria-label="Running now"] button[data-act="stop"]'));
    expect($('section[aria-label="Running now"] .you-asked')?.textContent).toContain('Stop Scribe? What it has not saved is lost.');
    expect($('section[aria-label="Agents"] .you-asked')).toBeNull();
    expect(posted.filter((entry) => entry.path.endsWith('/end'))).toEqual([]);
    await click($$('section[aria-label="Running now"] .you-asked button').find((el) => el.textContent === 'Stop Scribe') ?? null);
    expect(posted.filter((entry) => entry.path.endsWith('/end')).map((entry) => entry.path)).toEqual(['/runtime/sessions/' + LIVE + '/end']);
  });

  it('opens one picture of a running agent\'s terminal on Watch, switches it on a second Watch, keeps it while the page reads again, and closes it leaving the agent running', async () => {
    const two = {
      ...routes, ...terminalOf(LIVE), ...terminalOf(OTHER),
      ['/directory/agents/' + COURIER]: ok({ id: COURIER, display_name: 'Courier' }),
      '/dashboard': ok(dashboard({ [SCRIBE]: { sessions: [running] }, [COURIER]: { sessions: [session(OTHER, COURIER, 'Shed')] } })),
    };
    const { posted, requests } = await mount('#/', two);
    await click(rowOf('Scribe')?.querySelector('[data-act="watch"]') ?? null);
    expect($$('.dash-picture')).toHaveLength(1);
    expect($('.dash-picture h2')?.textContent).toBe('Scribe');
    expect($('.dash-picture .terminal')).not.toBeNull();
    expect($('.dash-picture a[data-act="full-size"]')?.getAttribute('href')).toBe('#/canvas/' + SCRIBE);
    expect(rowOf('Scribe')?.querySelector('[data-act="watch"]')?.getAttribute('aria-pressed')).toBe('true');
    expect(posted.filter((call) => call.path.endsWith('/resize')).map((call) => call.path)).toEqual(['/runtime/sessions/' + LIVE + '/resize']);

    await click($('section[aria-label="Running now"] tr[data-running="' + OTHER + '"] [data-act="watch"]'));
    expect($$('.dash-picture')).toHaveLength(1);
    expect($('.dash-picture h2')?.textContent).toBe('Courier');
    expect($('.dash-picture a[data-act="full-size"]')?.getAttribute('href')).toBe('#/canvas/' + COURIER);

    const reads = requests.filter((path) => path === '/dashboard').length;
    await act(async () => { refreshLive(); });
    await settle();
    expect(requests.filter((path) => path === '/dashboard').length).toBeGreaterThan(reads);
    expect($('.dash-picture h2')?.textContent).toBe('Courier');

    await click($('.dash-picture [data-act="close-picture"]'));
    expect($('.dash-picture')).toBeNull();
    expect(rowOf('Courier')?.querySelector('[data-act="stop"]')).not.toBeNull();
    expect(rowOf('Courier')?.textContent).toContain('on Shed');
    expect(posted.filter((call) => call.path.endsWith('/end'))).toEqual([]);
    expect(unreachable()).toEqual([]);
  });

  it('sends a registered person, whose account waits for activation, to You', async () => {
    await mount('#/', { ...SERVICE, '/me': ok({ person: { id: ADA, display_name: 'Ada (test person)', state: 'registered' }, signed_in: { provider: 'p', subject: 's' }, sign_in_identities: [], service_accounts: [] }) });
    expect(location.hash).toBe('#/me');
  });
});

describe('What the Dashboard reads, as the service answers it', () => {
  it('lists an agent under every team it is in', async () => {
    await mount('#/', { ...routes,
      '/teams': ok({ teams: [team('team-1', ADA, 'Crew', []), team('team-2', ADA, 'Night shift', [])] }),
      '/dashboard': ok(dashboard({ [COURIER]: { teams: ['team-1', 'team-2'] }, [ARCHIVIST]: { teams: ['team-1'] } })) });
    expect($$('.you-fold').map((button) => button.textContent)).toEqual(['Crew2', 'Night shift1']);
    expect(names()).toEqual(['Courier', 'Archivist', 'Courier', 'Scribe']);
  });

  it('lists a team led by someone outside the person\'s tree with the agents in it', async () => {
    await mount('#/', { ...routes, '/teams': ok({ teams: [team('team-9', 'person-' + '9'.repeat(32), 'Borrowed', [])] }),
      '/dashboard': ok(dashboard({ [COURIER]: { teams: ['team-9'] } })) });
    expect($$('.you-fold').map((button) => button.textContent)).toEqual(['Borrowed1']);
    expect(names()).toEqual(['Courier', 'Scribe', 'Archivist']);
  });

  it('shows each live session of an agent with its own Watch and Stop, and Running now lists sessions', async () => {
    const { posted } = await mount('#/', { ...routes, ...terminalOf(OTHER),
      '/dashboard': ok(dashboard({ [SCRIBE]: { sessions: [running, session(OTHER, SCRIBE, 'Shed')] } })),
      ['POST /runtime/sessions/' + OTHER + '/end']: ok({ session: OTHER, answer: { kind: 'ended' }, receipt: { index: 1 } }) });
    expect([...(rowOf('Scribe')?.querySelectorAll('.dash-line') ?? [])].map((line) => line.textContent)).toEqual(['on Lab', 'on Shed']);
    expect([...(rowOf('Scribe')?.querySelectorAll('.dash-acts') ?? [])].map((acts) => acts.getAttribute('data-session'))).toEqual([LIVE, OTHER]);
    expect($$('section[aria-label="Running now"] tr[data-running]').map((tr) => [tr.dataset.running, tr.textContent?.includes('on Shed')])).toEqual([[LIVE, false], [OTHER, true]]);
    await click(rowOf('Scribe')?.querySelector(`.dash-acts[data-session="${OTHER}"] [data-act="watch"]`) ?? null);
    expect($('.dash-picture')?.textContent).toContain('on Shed');
    await click(rowOf('Scribe')?.querySelector(`.dash-acts[data-session="${OTHER}"] [data-act="stop"]`) ?? null);
    await click($$('section[aria-label="Agents"] .you-asked button').find((el) => el.textContent === 'Stop Scribe') ?? null);
    expect(posted.filter((entry) => entry.path.endsWith('/end')).map((entry) => entry.path)).toEqual(['/runtime/sessions/' + OTHER + '/end']);
  });

  it('opens Stop under the one listing it was pressed in when an agent is under two teams', async () => {
    await mount('#/', { ...routes, '/teams': ok({ teams: [team('team-1', ADA, 'Crew', []), team('team-2', ADA, 'Night shift', [])] }),
      '/dashboard': ok(dashboard({ [SCRIBE]: { sessions: [running], teams: ['team-1', 'team-2'] } })) });
    await click($$('tr[data-href]').filter((tr) => tr.querySelector('td')?.textContent === 'Scribe')[1].querySelector('[data-act="stop"]'));
    expect($$('.you-asked')).toHaveLength(1);
  });

  it('says a refused teams or sessions part by its refusal name in its row, and offers no Start it cannot stand behind', async () => {
    await mount('#/', { ...routes, '/dashboard': ok(dashboard({
      [SCRIBE]: { sessions: { refusal: 'RuntimeUnavailable', reason: 'no reports store' } },
      [COURIER]: { teams: { refusal: 'TeamsUnavailable', reason: 'the teams store is closed' } },
    })) });
    expect(cells('Scribe')[1]).toBe('RuntimeUnavailable');
    expect(rowOf('Scribe')?.querySelector('[data-act="start"], [data-act="stop"], [data-act="watch"]')).toBeNull();
    expect(cells('Scribe')[2]).toBe('No limit');
    const courier = [...($(`tr[data-href="#/file/${COURIER}"]`)?.querySelectorAll('td') ?? [])].map((td) => td.textContent);
    expect(courier.slice(0, 2)).toEqual(['Courier teams: TeamsUnavailable', 'registered']);
    // An agent whose sessions could not be read is not counted as stopped: Running now names it and never says nothing runs.
    const panel = $('section[aria-label="Running now"]');
    expect(panel?.textContent).not.toContain('Nothing is running.');
    expect(panel?.querySelector('tr[data-unread="' + SCRIBE + '"]')?.textContent).toBe('ScribeWhether it is running could not be read. RuntimeUnavailable');
    expect(rowOf('Scribe')?.querySelector('.dot')?.getAttribute('aria-label')).toBe('not known');
  });

  it('says a waiting count that could not be read by its refusal name, its link still there, never zero', async () => {
    await mount('#/', { ...routes, '/dashboard': ok(dashboard({}, { requests: { refusal: 'RequestsUnavailable', reason: 'closed' } as unknown as number, drafts: 0, reviews: 0 })) });
    const rows = $$('tr[data-waiting]');
    expect(rows.map((tr) => [tr.querySelector('.dash-num')?.textContent, tr.querySelector('a')?.getAttribute('href')])).toEqual([
      ['RequestsUnavailable', '#/requests'], ['0', '#/access/drafts'], ['0', '#/reviews'],
    ]);
    expect(text()).not.toContain('Nothing is waiting.');
  });

  it('reads every page of agents, one after another, until there is no next page', async () => {
    const first = dashboard();
    const { requests } = await mount('#/', { ...routes,
      '/dashboard': ok({ ...first, agents: first.agents.slice(0, 2), total: 3, next: 'page-2' }),
      '/dashboard?after=page-2': ok({ ...first, agents: first.agents.slice(2), total: 3, next: null }) });
    expect(names()).toEqual(['Scribe', 'Courier', 'Archivist']);
    expect(requests.filter((path) => path.startsWith('/dashboard'))).toEqual(['/dashboard', '/dashboard?after=page-2']);
  });
});

describe('You keeps the account side', () => {
  it('lists no agents and has no Agents tab; #/me and #/me?tab=account show the same page', async () => {
    await mount('#/me', routes);
    expect($('.page h1')?.textContent).toBe('Ada (test person)');
    expect($('.you-page')).not.toBeNull();
    expect($('.you-tree')).toBeNull();
    expect($$('tr[data-href^="#/file/agent-"]')).toEqual([]);
    expect($$('#screen .tabs a').map((a) => a.textContent)).not.toContain('Agents');
    expect(text()).not.toContain('Your agents');
    expect($('section[aria-label="What you hold"]')).not.toBeNull();
    const shown = $('#screen')?.textContent;
    location.hash = '#/me?tab=account';
    await settle();
    expect($('#screen')?.textContent).toBe(shown);
  });
});
