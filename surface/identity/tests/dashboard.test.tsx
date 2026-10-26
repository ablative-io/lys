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
const compactBudget = (figure: number | null = null) => ({
  ...scribeBudget,
  limits: [{ unit: 'context_percent', amount: 40, period: null, act: 'compact' }],
  used: [{ unit: 'context_percent', period: null, since_ms: null,
    ...(figure === null ? { figure, unavailable: 'context_percent was not reported' } : { figure, unavailable: null }) }],
});
const contextFigure = (figure: number | null, unavailable: string | null = null) => ({
  unit: 'context_percent', period: null, since_ms: null, figure, unavailable,
  context: { session: LIVE, read_at_ms: figure === null ? null : 1790000000000, reported_none: 0, model: 'test-model',
    context_tokens: figure === null ? null : figure * 1000, window_tokens: 100_000 },
});
/** One line of GET /waiting's answer; a request or a draft entry carries its record as its own list answers it. */
const waits = (kind: string, id: string, about: string, record: { request?: object; draft?: object } = {}) => ({ kind, id, asked_by: { id: SCRIBE, name: 'Scribe' }, about, asked_at: '2026-10-18T01:00:00Z', can_decide: true, ...record });
/** Scribe's request as GET /requests answers it, Ada able to decide it. */
const asked = (id: string, why: string) => ({ id, asked_by: SCRIBE, asked_by_name: 'Scribe', responsible: { id: ADA, display_name: 'Ada', state: 'active' }, resource: { kind: 'store', id: 's-1' },
  relation: 'reader', actions: ['read'], ends_at: null, why, asked_at: 1790000000, state: 'waiting', approvers: [{ id: ADA, display_name: 'Ada', state: 'active' }], sources: [], can_issue_root: true, can_decide: true, held_by: null, decision: null });
/** Scribe's draft as GET /drafts answers it. */
const prepared = (id: string, note: string) => ({ id, agent: { id: SCRIBE, display_name: 'Scribe' }, responsible: { id: ADA, display_name: 'Ada', state: 'active' }, target: { kind: 'person', id: ADA, action: 'person.profile.set' },
  method: 'POST', path: '/identities/' + ADA + '/profile', body: '{}', note, created_at: 1790000000, creation_hash: 'hash-' + id, state: 'waiting', execution: null });
/** A row's acts: its link's word, or the decision acts in the row by name. */
const actsOf = (tr: Element) => tr.querySelector('a.btn')?.textContent ?? [...tr.querySelectorAll('td.dash-decide button')].map((each) => each.getAttribute('aria-label')).join(' / ');
const names = () => $$('tr[data-href]').map((tr) => tr.querySelector('td')?.textContent);
const rowOf = (name: string) => $$('tr[data-href]').find((tr) => tr.querySelector('td')?.textContent === name) ?? null;
/**
 * What a person reads in an element: a part that could not be read by its plain words alone (what it means and its
 * refusal code stay folded under them), and the limits or goals of a cell, each on its own line, read as one line.
 */
const face = (element: Element | null | undefined): string => {
  if (!element) return '';
  const copy = element.cloneNode(true) as Element;
  for (const fold of copy.querySelectorAll('.dash-unread > :not(summary)')) fold.remove();
  const lines = [...copy.querySelectorAll('.dash-line')];
  for (const line of lines.slice(0, -1)) line.append(' · ');
  return copy.textContent ?? '';
};
/** The refusal code a folded part keeps under Error details, out of sight until opened. */
const folded = (element: Element | null | undefined): string => element?.querySelector('.dash-unread details .refusal-name')?.textContent ?? '';
const cells = (name: string) => [...(rowOf(name)?.querySelectorAll('td') ?? [])].map(face);

describe('The Dashboard is the front page', () => {
  it('is the rail\'s first entry, Dashboard, and #/ lands on it', async () => {
    await mount('#/', routes);
    // No heading of its own: the rail names the page, and nothing says Dashboard twice (Tom, 18 October 2026).
    expect($$('.page .page-header')).toEqual([]);
    expect($('.page.dash-page')).not.toBeNull();
    expect($('#rail a.on')?.dataset.nav).toBe('dashboard');
    // Home first, then what waits on you (CONSENSUS.md, 18 October 2026; it replaced Operations first, Tom, 4 October 2026).
    const [first, second] = $$('#rail a[data-nav]');
    expect([first.dataset.nav, first.getAttribute('href'), first.title]).toEqual(['dashboard', '#/', 'Dashboard (g d)']);
    expect([second.dataset.nav, second.title]).toEqual(['waiting', 'Waiting on you (g q)']);
    await click($('#rail a[data-nav="people"]'));
    await click($('#rail a[data-nav="dashboard"]'));
    expect(location.hash).toBe('#/');
    expect($('.page.dash-page')).not.toBeNull();
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
    expect(cells('Scribe').slice(2, 4)).toEqual(['900 tokens of 1,000 tokens today · US$100.00 of US$500.00 this week', '2 open, next due 6 Oct, 1 missed']);
    // A part that could not be read says so in plain words; its refusal is kept under Error details, never on the face.
    expect(cells('Courier').slice(1, 4)).toEqual(['Not turned on yet', 'Its use could not be read', '1 missed']);
    expect(cells('Archivist').slice(1, 4)).toEqual(['Paused', 'Its limits could not be read', 'No goal']);
    expect(folded(rowOf('Courier')?.querySelectorAll('td')[2])).toBe('UsageUnavailable: the usage store is closed');
    expect(folded(rowOf('Archivist')?.querySelectorAll('td')[2])).toBe('BudgetsUnavailable: the budgets store is closed');
    for (const name of ['Courier', 'Archivist']) expect(cells(name).join(' ')).not.toMatch(/Unavailable/);
    for (const name of ['Courier', 'Archivist']) expect(rowOf(name)?.querySelectorAll('td')[2].textContent).not.toMatch(/\b0\b|No limit/);
    // One read: no agent's usage, goals or budgets is asked for on its own.
    expect(requests.filter((path) => path === '/dashboard')).toHaveLength(1);
    // The person's own limit is the page's one other read; no agent's usage, goals or budgets is asked for on its own.
    expect(requests.filter((path) => /\/usage|\/goals|\/budgets\/(agent|team)/.test(path))).toEqual([]);
  });

  it('names the compact budget and says not reported when no current context reading was supplied', async () => {
    const { requests } = await mount('#/', { ...routes, '/dashboard': ok(dashboard({
      [SCRIBE]: { budget: compactBudget(), usage: { ...scribeUsage, used: compactBudget().used } },
    })) });
    expect(cells('Scribe')[2]).toBe('compacts at 40% · not reported yet');
    expect(rowOf('Scribe')?.textContent).not.toContain('40% of its context');
    expect(face(rowOf('Scribe')?.querySelector('td.you-budget'))).toBe('compacts at 40% · not reported yet');
    // The compact level is the limit working, never a level used against a limit: no bar is drawn for it.
    expect(rowOf('Scribe')?.querySelector('td.you-budget meter')).toBeNull();
    expect(requests.filter((path) => path === '/dashboard')).toHaveLength(1);
    expect(requests.filter((path) => /\/usage|\/budgets\/(agent|team)/.test(path))).toEqual([]);
  });

  it('shows current session context separately from the compact budget, using the reading rather than the budget-paired figure', async () => {
    await mount('#/', { ...routes, '/dashboard': ok(dashboard({
      [SCRIBE]: { budget: compactBudget(99), usage: { ...scribeUsage, used: compactBudget(99).used, figures: [contextFigure(23)] } },
    })) });
    expect(cells('Scribe')[2]).toBe('compacts at 40% · 23% now');
    expect(cells('Scribe')[2]).not.toContain('99%');
    expect(face(rowOf('Scribe')?.querySelector('td.you-budget'))).toBe('compacts at 40% · 23% now');
  });

  it('says 0% now when the current session reported zero context, without replacing it with the compact budget', async () => {
    await mount('#/', { ...routes, '/dashboard': ok(dashboard({
      [SCRIBE]: { budget: compactBudget(), usage: { ...scribeUsage, used: compactBudget().used, figures: [contextFigure(0)] } },
    })) });
    expect(cells('Scribe')[2]).toBe('compacts at 40% · 0% now');
  });

  it('keeps the context reading failure reason beside the compact budget instead of presenting the budget as usage', async () => {
    const reason = 'the runner did not retain the context window declaration';
    await mount('#/', { ...routes, '/dashboard': ok(dashboard({
      [SCRIBE]: { budget: compactBudget(), usage: { ...scribeUsage, used: compactBudget().used, figures: [contextFigure(null, reason)] } },
    })) });
    expect(cells('Scribe')[2]).toBe('compacts at 40% · context percentage unavailable: Scribe: ' + reason);
    expect(cells('Scribe')[2]).not.toContain('40% now');
  });

  it("shows an older scalar context percentage as the service's figure, since the reading carries no tokens or window to recompute it", async () => {
    await mount('#/', { ...routes, '/dashboard': ok(dashboard({
      [SCRIBE]: { budget: compactBudget(), usage: { ...scribeUsage, used: compactBudget().used,
        figures: [{ unit: 'context_percent', period: null, since_ms: null, figure: 23, unavailable: null }] } },
    })) });
    expect(cells('Scribe')[2]).toBe('compacts at 40% · 23% now');
    expect(cells('Scribe')[2]).not.toContain('not reported');
  });

  // The canvas's own Requests, Drafts, Budget and Goals widgets are pinned in src/features/dashboard/canvas-parts.test.tsx.
  it('says what waits on the person item by item, what needs a look worst first, each limit and goal in its row, and a part that could not be read in words', async () => {
    await mount('#/', { ...routes, '/waiting': ok({ waiting: [
      waits('request', 'r-1', 'Scribe asks for reader access to store s-1', { request: asked('r-1', 'to read the ward list') }),
      waits('agent_draft', 'op-' + '9'.repeat(32), 'Scribe prepared something for you to approve: for the night shift', { draft: prepared('op-' + '9'.repeat(32), 'for the night shift') }),
    ] }), '/dashboard': ok(dashboard({
      [SCRIBE]: { budget: scribeBudget, usage: scribeUsage, goals: { goals: [goal('g-1', at(9), 'open'), goal('g-2', at(6), 'open'), goal('g-3', null, 'missed'), goal('g-4', null, 'met')] } },
      [COURIER]: { goals: { goals: [goal('g-5', null, 'missed')] } },
      [ARCHIVIST]: { budget: { refusal: 'BudgetsUnavailable', reason: 'the budgets store is closed' }, goals: { refusal: 'GoalsUnavailable', reason: 'the goals store is closed' } },
    })) });
    const waiting = $('section[aria-label="Waiting on you"]');
    // One request the person may decide and one draft, as GET /waiting answers them: the number is the list's own.
    expect(waiting?.querySelector('[data-waiting="count"]')?.textContent).toBe('2');
    const request = waiting?.querySelector('tr[data-waiting-entry="request"]');
    expect(request?.querySelector('td')?.textContent).toBe('Scribe asks for reader access to store s-1to read the ward list');
    // Approve and Decline stand in the row itself; nothing is decided on another page (Tom, 18 October 2026).
    expect([...(request?.querySelectorAll('td.dash-decide button') ?? [])].map((act) => act.getAttribute('aria-label'))).toEqual(['Approve access', 'Decline request']);
    expect(request?.querySelector('a')).toBeNull();
    expect(waiting?.querySelectorAll('tr[data-waiting-entry="agent_draft"]')).toHaveLength(1);
    expect(waiting?.querySelector('tr[data-waiting-entry="agent_draft"] td')?.textContent).toBe('Scribe prepared something for you to approve: for the night shift');
    expect([...(waiting?.querySelectorAll('tr[data-waiting-entry="agent_draft"] td.dash-decide button') ?? [])].map((act) => act.getAttribute('aria-label'))).toEqual(['Approve', 'Decline']);
    expect(waiting?.querySelector('tr[data-waiting-entry="agent_draft"] a')).toBeNull();
    expect(waiting?.textContent).not.toMatch(/op-[0-9a-f]{32}/);
    // Needs a look: a missed goal before a limit near its end, each with Open to that agent's Limits and goals; nothing fine is listed.
    const look = $$('section[aria-label="Needs a look"] tr[data-exception]');
    expect(look.map((tr) => [tr.dataset.exception, tr.querySelector('td')?.textContent, tr.querySelector('a')?.getAttribute('href')])).toEqual([
      ['missed', 'Scribe missed a goal: a goal', '#/file/' + SCRIBE + '/budgets'],
      ['missed', 'Courier missed a goal: a goal', '#/file/' + COURIER + '/budgets'],
      ['near', 'Scribe has used 90% of its 1,000 tokens a day', '#/file/' + SCRIBE + '/budgets'],
    ]);
    expect($('section[aria-label="Needs a look"] .dash-count')?.textContent).toBe('3');
    // Each limit and goal is said in its agent's row: a part that could not be read in words, its refusal folded under them.
    expect(cells('Archivist').slice(2, 4)).toEqual(['Its limits could not be read', 'Its goals could not be read']);
    expect(folded(rowOf('Archivist')?.querySelector('td.you-budget'))).toBe('BudgetsUnavailable: the budgets store is closed');
    expect(folded(rowOf('Archivist')?.querySelector('td.you-goals'))).toBe('GoalsUnavailable: the goals store is closed');
    expect(rowOf('Scribe')?.querySelector('td.you-budget meter')?.getAttribute('aria-label')).toBe('90% of the nearest limit, near it');
    expect(cells('Courier').slice(2, 4)).toEqual(['No limit', '1 missed']);
    // The earliest open deadline is the one the row names.
    expect(cells('Scribe')[3]).toBe('2 open, next due 6 Oct, 1 missed');
    expect(unreachable()).toEqual([]);
  });

  it('says a list whose own read was refused in words where it would stand, with the rest of the Dashboard still shown', async () => {
    await mount('#/', { ...routes, '/waiting': ok({ waiting: [
      { kind: 'request', refusal: 'RequestsUnavailable', reason: 'the requests store is closed' },
      { kind: 'agent_draft', refusal: 'DraftsUnavailable', reason: 'closed' },
    ] }) });
    const waiting = $('section[aria-label="Waiting on you"]');
    expect(face(waiting?.querySelector('tr[data-waiting="request"]'))).toBe('Requests could not be readOpen');
    expect(face(waiting)).not.toContain('RequestsUnavailable');
    expect(folded(waiting?.querySelector('tr[data-waiting="request"]'))).toBe('RequestsUnavailable: the requests store is closed');
    expect(face(waiting?.querySelector('tr[data-waiting="agent_draft"]'))).toBe('Your agents’ drafts could not be readOpen');
    expect(folded(waiting?.querySelector('tr[data-waiting="agent_draft"]'))).toBe('DraftsUnavailable: closed');
    // No number and no "nothing" over a list that could not be read.
    expect(waiting?.querySelector('[data-waiting="count"]')).toBeNull();
    expect(waiting?.textContent).not.toContain('Nothing is waiting on you.');
    expect(rowOf('Scribe')).not.toBeNull();
    // With no limit and no goal anywhere, nothing needs a look and each row says so.
    expect($('section[aria-label="Needs a look"]')).toBeNull();
    for (const name of ['Scribe', 'Courier', 'Archivist']) expect(cells(name).slice(2, 4)).toEqual(['No limit', 'No goal']);
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
    // The table says every limit but the spending limit; the spending limit's card says that one, in the same words.
    const page = $$('.usage-budgets .usage-used, [data-limit="spending"] .usage-used').map((each) => each.textContent ?? '');
    expect(page).toHaveLength(2);
    expect([...line].sort()).toEqual([...page].sort());
  });

  it('lists what waits for the person as GET /waiting answers it alone, decides a request and a draft in their rows, opens the rest, and counts what it lists', async () => {
    const { requests } = await mount('#/', { ...routes, '/waiting': ok({ waiting: [
      waits('review', 'grant-1', 'Check the access Scribe holds: reader on store s-1'),
      waits('request', 'r-1', 'Scribe asks for reader access to store s-1', { request: asked('r-1', '') }),
      waits('agent_draft', 'op-' + '1'.repeat(32), 'Scribe prepared something for you to approve', { draft: prepared('op-' + '1'.repeat(32), '') }),
      waits('product_draft', 'op-' + '2'.repeat(32), 'Scribe asks to write notes 1 through notes'),
    ] }) });
    const rows = $$('section[aria-label="Waiting on you"] tbody tr');
    expect(rows.map((tr) => [tr.querySelector('td')?.textContent, actsOf(tr), tr.querySelector('a')?.getAttribute('href') ?? null])).toEqual([
      ['Check the access Scribe holds: reader on store s-1This Lys did not send the grants with it, so they are checked on their page.', 'Open', '#/reviews'],
      ['Scribe asks for reader access to store s-1', 'Approve access / Decline request', null],
      ['Scribe prepared something for you to approve', 'Approve / Decline', null],
      ['Scribe asks to write notes 1 through notesThis Lys did not send the app action with it, so it is decided on its page.', 'Open', '#/access/drafts'],
    ]);
    // The word Decide is nowhere on the page.
    expect($('.page.dash-page')?.textContent).not.toMatch(/Decide/);
    // Four lines, every one of them listed above, and the rail says the same number.
    expect($('section[aria-label="Waiting on you"] [data-waiting="count"]')?.textContent).toBe('4');
    expect($('#rail a[data-nav="waiting"] .cnt')?.textContent).toBe('4');
    expect(text()).not.toContain('Nothing is waiting on you.');
    // What waits is read through GET /waiting alone. The rail decides its Requests and Drafts entries by reading those
    // lists on every page (shell/NavigationAccess.tsx); the dashboard adds no read of either, so it reads each exactly as
    // often as another page does, and the one read of each is the rail's.
    expect(requests).toContain('/waiting');
    const lists = (paths: string[]) => paths.filter((path) => /^\/(requests|drafts)\b/.test(path)).sort();
    const here = lists(requests);
    unmountAll();
    const elsewhere = await mount('#/people', routes);
    expect(here).toEqual(lists(elsewhere.requests));
    expect(here).toEqual(['/drafts?state=waiting', '/requests']);
  });

  it('says once that nothing is waiting, and with nothing running offers no Stop, no Watch and no Stop all', async () => {
    await mount('#/', SERVICE);
    expect($$('tr[data-waiting], tr[data-waiting-entry]')).toEqual([]);
    expect(text().split('Nothing is waiting on you.')).toHaveLength(2);
    expect($('section[aria-label="Waiting on you"] tbody tr.empty td')?.textContent).toBe('Nothing is waiting on you.');
    expect($('section[aria-label="Waiting on you"] [data-waiting="count"]')).toBeNull();
    // What runs is said once, in each agent's row: nothing runs, so no row says it does and nothing offers to stop it.
    expect($('section[aria-label="Running now"]')).toBeNull();
    expect($$('.you-tree .dot').map((dot) => dot.getAttribute('aria-label'))).toEqual(['not running', 'not running', 'not running']);
    expect(cells('Scribe')[1]).toBe('Not running');
    expect($('[data-act="stop"], [data-act="watch"], [data-act="stop-everything"]')).toBeNull();
  });

  it('says in the running agent\'s own row where it runs, with Watch and Stop', async () => {
    await mount('#/', routes);
    const acts = $$('.you-tree .dash-acts[data-session]');
    expect(acts.map((each) => [each.closest('tr')?.querySelector('td')?.textContent, each.getAttribute('data-session')])).toEqual([['Scribe', LIVE]]);
    expect(cells('Scribe')[1]).toBe('on Lab');
    expect([...acts[0].querySelectorAll('button')].map((button) => button.dataset.act)).toEqual(['watch', 'stop']);
    expect(rowOf('Scribe')?.querySelector('.dot')?.getAttribute('aria-label')).toBe('running');
  });

  it('stops a running agent from its row, after asking, in a row under it, opened once', async () => {
    const { posted } = await mount('#/', { ...routes, ['POST /runtime/sessions/' + LIVE + '/end']: ok({ session: LIVE, answer: { kind: 'ended' }, receipt: { index: 1 } }) });
    await click(rowOf('Scribe')?.querySelector('button[data-act="stop"]') ?? null);
    expect($('section[aria-label="Agents"] .you-asked')?.textContent).toContain('Stop Scribe? This ends its run on Lab. What it has not saved is lost.');
    expect(rowOf('Scribe')?.nextElementSibling?.classList.contains('you-asked')).toBe(true);
    expect($$('.you-asked')).toHaveLength(1);
    expect(posted.filter((entry) => entry.path.endsWith('/end'))).toEqual([]);
    await click($$('section[aria-label="Agents"] .you-asked button').find((el) => (el.getAttribute('aria-label') ?? el.textContent) === 'Stop Scribe') ?? null);
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

    await click(rowOf('Courier')?.querySelector('[data-act="watch"]') ?? null);
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

  it('shows each live session of an agent with its own Watch and Stop, in its one row', async () => {
    const { posted } = await mount('#/', { ...routes, ...terminalOf(OTHER),
      '/dashboard': ok(dashboard({ [SCRIBE]: { sessions: [running, session(OTHER, SCRIBE, 'Shed')] } })),
      ['POST /runtime/sessions/' + OTHER + '/end']: ok({ session: OTHER, answer: { kind: 'ended' }, receipt: { index: 1 } }) });
    expect([...(rowOf('Scribe')?.querySelectorAll('td')[1].querySelectorAll('.dash-line') ?? [])].map((line) => line.textContent)).toEqual(['on Lab', 'on Shed']);
    expect([...(rowOf('Scribe')?.querySelectorAll('.dash-acts') ?? [])].map((acts) => acts.getAttribute('data-session'))).toEqual([LIVE, OTHER]);
    expect($$('.dash-acts[data-session]').map((acts) => [acts.getAttribute('data-session'), acts.closest('tr') === rowOf('Scribe')])).toEqual([[LIVE, true], [OTHER, true]]);
    await click(rowOf('Scribe')?.querySelector(`.dash-acts[data-session="${OTHER}"] [data-act="watch"]`) ?? null);
    expect($('.dash-picture')?.textContent).toContain('on Shed');
    await click(rowOf('Scribe')?.querySelector(`.dash-acts[data-session="${OTHER}"] [data-act="stop"]`) ?? null);
    await click($$('section[aria-label="Agents"] .you-asked button').find((el) => (el.getAttribute('aria-label') ?? el.textContent) === 'Stop Scribe') ?? null);
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
    expect(cells('Scribe')[1]).toBe('Not known');
    expect(folded(rowOf('Scribe')?.querySelectorAll('td')[1])).toBe('RuntimeUnavailable: no reports store');
    expect(rowOf('Scribe')?.querySelector('[data-act="start"], [data-act="stop"], [data-act="watch"]')).toBeNull();
    expect(cells('Scribe')[2]).toBe('No limit');
    const courier = [...($(`tr[data-href="#/file/${COURIER}"]`)?.querySelectorAll('td') ?? [])].map(face);
    expect(courier.slice(0, 2)).toEqual(['CourierIts teams could not be read', 'Not turned on yet']);
    expect(folded($(`tr[data-href="#/file/${COURIER}"] td`))).toBe('TeamsUnavailable: the teams store is closed');
    // An agent whose sessions could not be read is not counted as stopped: its row never says it is not running, and
    // Stop all stays in the Agents heading, since the off switch is needed most when what runs cannot be read.
    expect(rowOf('Scribe')?.textContent).not.toContain('Not running');
    expect(rowOf('Scribe')?.querySelector('.dot')?.getAttribute('aria-label')).toBe('not known');
    expect($('section[aria-label="Agents"] .section-h [data-act="stop-everything"]')).not.toBeNull();
  });

  it('says a part of what waits that could not be read in words, its link still there, never zero', async () => {
    await mount('#/', { ...routes, '/waiting': ok({ waiting: [
      { kind: 'product_draft', refusal: 'ProductDraftsUnavailable', reason: 'shut' },
      { kind: 'review', refusal: 'ReviewsUnavailable', reason: 'closed' },
    ] }) });
    const rows = $$('tr[data-waiting]');
    expect(rows.map((tr) => [tr.dataset.waiting, face(tr), tr.querySelector('a')?.getAttribute('href')])).toEqual([
      ['product_draft', 'Whether an app action waits for your approval could not be readOpen', '#/access/drafts'],
      ['review', 'Whether the access you gave is due a check could not be readOpen', '#/reviews'],
    ]);
    expect(rows.map(folded)).toEqual(['ProductDraftsUnavailable: shut', 'ReviewsUnavailable: closed']);
    expect($('[data-waiting="count"]')).toBeNull();
    expect(text()).not.toContain('Nothing is waiting on you.');
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
