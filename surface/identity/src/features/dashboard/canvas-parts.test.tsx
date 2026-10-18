/**
 * The canvas's Requests, Drafts, Budget and Goals widgets, which read the Dashboard's answer. The Dashboard itself says
 * these once, in Waiting on you, the agents table and Needs a look (tests/dashboard.test.tsx); these four are what a
 * person places in a window of the canvas, and they were pinned in the Dashboard's tests while the Dashboard drew them.
 */
import type { ReactNode } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { Refused } from '../../api';
import { ADA, ARCHIVIST, COURIER, SCRIBE, dashboard } from '../../../tests/fixtures';
import type { DashboardAgent } from './contract';
import type { Draft } from '../drafts/contract';
import type { AccessRequest } from '../requests/contract';
import { budgetPart, draftsPart, goalsPart, requestsPart } from './Widgets';

const at = (d: number) => new Date(2026, 9, d, 12, 0).getTime() / 1000;
const goal = (id: string, deadline: number | null, standing: string) => ({ goal: { id, kind: 'goal', words: 'a goal', deadline, active: true, evidence: null }, standing });
const scribeBudget = {
  holder: { kind: 'agent', id: SCRIBE }, warn_at: null, zone: 'UTC', version: 2, by: ADA, at: 1790000000, unavailable: [], within: [], unconfirmed: [],
  limits: [{ unit: 'dollars', amount: 500, period: 'week', act: 'stop' }, { unit: 'tokens', amount: 1000, period: 'day', act: 'notice' }],
  used: [{ unit: 'dollars', period: 'week', since_ms: null, figure: 100, unavailable: null }, { unit: 'tokens', period: 'day', since_ms: null, figure: 900, unavailable: null }],
};
const asked = { id: 'r-1', asked_by: SCRIBE, asked_by_name: 'Scribe', responsible: { id: ADA, display_name: 'Ada (test person)', state: 'active' }, resource: { kind: 'store', id: 's-1' },
  relation: 'reader', actions: ['read'], ends_at: null, why: 'to read the ward list', asked_at: 1790000000, state: 'waiting', approvers: [], sources: [], can_decide: true } as unknown as AccessRequest;
const decided = { ...asked, id: 'r-2', state: 'approved', why: 'already decided' } as AccessRequest;
const prepared = { id: 'op-' + '9'.repeat(32), agent: { id: SCRIBE, display_name: 'Scribe' }, responsible: { id: ADA, display_name: 'Ada (test person)', state: 'active' },
  target: { kind: 'grant', id: 'g-1', action: 'grant.give' }, method: 'POST', path: '/grants', body: '{}', note: 'for the night shift', created_at: 1790000000, creation_hash: 'h', state: 'waiting' } as Draft;
const rows = (more: Record<string, Record<string, unknown>> = {}) => dashboard(more).agents as unknown as DashboardAgent[];

/** A widget as the canvas draws it: its count and its body. */
const drawn = (part: { count: ReactNode; body: ReactNode }) => {
  const host = document.createElement('div');
  host.innerHTML = '<b class="dash-count">' + renderToStaticMarkup(<>{part.count}</>) + '</b>' + renderToStaticMarkup(<>{part.body}</>);
  return host;
};
/** What a person reads: a part that could not be read by its words alone, its refusal folded under them. */
const face = (element: Element | null | undefined): string => {
  if (!element) return '';
  const copy = element.cloneNode(true) as Element;
  for (const fold of copy.querySelectorAll('.dash-unread > :not(summary)')) fold.remove();
  return copy.textContent ?? '';
};
const folded = (element: Element | null | undefined): string => element?.querySelector('.dash-unread details .refusal-name')?.textContent ?? '';

describe('The canvas widgets that read the Dashboard', () => {
  it('says waiting requests and drafts item by item, budgets nearest the limit first, goals missed first, and a part that could not be read in words', () => {
    const requests = drawn(requestsPart({ requests: [decided, asked], me: ADA }));
    expect(requests.querySelector('.dash-count')?.textContent).toBe('1');
    const request = requests.querySelector('tr[data-request="r-1"]');
    expect([...(request?.querySelectorAll('td') ?? [])].map((td) => td.textContent)).toEqual(['Scribe', 'reader: readto read the ward list', 'Decide']);
    expect(request?.querySelector('a')?.getAttribute('href')).toBe('#/requests/r-1');
    expect(requests.textContent).not.toContain('already decided');
    const drafts = drawn(draftsPart({ drafts: [prepared] }));
    expect(drafts.querySelector('.dash-count')?.textContent).toBe('1');
    expect([...drafts.querySelectorAll('tr[data-draft] td')].map((td) => td.textContent)).toEqual(['Scribe', 'grant.give on grantfor the night shift', 'Decide']);
    expect(drafts.textContent).not.toMatch(/op-[0-9a-f]{32}/);

    const agents = rows({
      [SCRIBE]: { budget: scribeBudget, goals: { goals: [goal('g-1', at(9), 'open'), goal('g-2', at(6), 'open'), goal('g-3', null, 'missed'), goal('g-4', null, 'met')] } },
      [COURIER]: { goals: { goals: [goal('g-5', null, 'missed')] } },
      [ARCHIVIST]: { budget: { refusal: 'BudgetsUnavailable', reason: 'the budgets store is closed' }, goals: { refusal: 'GoalsUnavailable', reason: 'the goals store is closed' } },
    });
    // Budget: the part that could not be read first, in words; then the agent nearest its limit; the rest counted once.
    const budget = drawn(budgetPart({ rows: agents }));
    expect(budget.querySelector('.dash-count')?.textContent).toBe('1 limited');
    expect([...budget.querySelectorAll('tr[data-budget]')].map((tr) => tr.getAttribute('data-budget'))).toEqual([ARCHIVIST, SCRIBE]);
    expect(face(budget.querySelector('tr[data-budget="' + ARCHIVIST + '"]'))).toBe('ArchivistIts limits could not be read');
    expect(folded(budget.querySelector('tr[data-budget="' + ARCHIVIST + '"]'))).toBe('BudgetsUnavailable: the budgets store is closed');
    expect(budget.querySelector('tr[data-budget="' + SCRIBE + '"] a')?.getAttribute('href')).toBe('#/file/' + SCRIBE + '/budgets');
    expect(budget.querySelector('tr[data-budget="' + SCRIBE + '"] meter')?.getAttribute('aria-label')).toBe('90% of the nearest limit, near it');
    expect(budget.querySelector('tr.empty')?.textContent).toBe('1 other agent has no limit.');
    // Goals: missed first, then open by deadline, then met; an agent whose goals could not be read is named.
    const goals = drawn(goalsPart({ rows: agents }));
    expect(goals.querySelector('.dash-count')?.textContent).toBe('2 open, 2 missed');
    expect([...goals.querySelectorAll('tr[data-goal]')].map((tr) => tr.getAttribute('data-goal'))).toEqual(['missed', 'missed', 'open', 'open', 'met', 'unread']);
    const words = goals.textContent ?? '';
    expect(words.indexOf('due 6 Oct')).toBeGreaterThan(-1);
    expect(words.indexOf('due 6 Oct')).toBeLessThan(words.indexOf('due 9 Oct'));
    expect(face(goals.querySelector('tr[data-goal="unread"]'))).toBe('ArchivistIts goals could not be read');
    expect(folded(goals.querySelector('tr[data-goal="unread"]'))).toBe('GoalsUnavailable: the goals store is closed');
  });

  it('says a widget whose own read was refused in words, with its refusal folded, never a count of nothing', () => {
    const requests = drawn(requestsPart({ requests: new Refused(503, { refusal: 'RequestsUnavailable', reason: 'the requests store is closed' }), me: ADA }));
    expect(face(requests)).toContain('Requests could not be read.');
    expect(face(requests)).not.toContain('RequestsUnavailable');
    expect(folded(requests)).toBe('RequestsUnavailable: the requests store is closed');
    expect(requests.querySelector('.dash-count')?.textContent).toBe('?');
    const drafts = drawn(draftsPart({ drafts: new Refused(503, { refusal: 'DraftsUnavailable', reason: 'closed' }) }));
    expect(face(drafts)).toContain('Drafts could not be read.');
    expect(folded(drafts)).toBe('DraftsUnavailable: closed');
    expect(drawn(budgetPart({ rows: rows() })).querySelector('tr.empty')?.textContent).toBe('No agent has a limit.');
    expect(drawn(goalsPart({ rows: rows() })).querySelector('tr.empty')?.textContent).toBe('No goal is set.');
  });
});
