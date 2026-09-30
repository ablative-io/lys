/** The front page shows the running agents as small views that never resize the session, what waits for the person, and the agents as a tree under their teams that folds. */
import { describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, text, unreachable } from './harness';
import { ADA, ARCHIVIST, COURIER, SCRIBE, SERVICE, ok } from './fixtures';
import { mockTerminal } from './terminal-double';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const LIVE = 'op-' + '5'.repeat(32);
const LAB = 'op-' + 'b'.repeat(32);
const running = { session: LIVE, agent: SCRIBE, machine: LAB, machine_name: 'Lab', runtime: 'lys-runner', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'started', stopped: null, reported_by: 'the runner' };
const team = (id: string, owner: string, name: string, members: string[]) => ({ id, owner, name, description: '', members, state: 'active', created_by: { provider: 'p', subject: 's' }, created_at: 1790000000, retired_at: null });
const person = { id: ADA, display_name: 'Ada (test person)', state: 'active' };
const ask = (id: string) => ({ id, asked_by: COURIER, asked_by_name: 'Courier', responsible: person, resource: { kind: 'project', id: 'atlas' }, relation: 'viewer', actions: [], ends_at: null, why: 'to read it', asked_at: 1790000000, state: 'waiting', approvers: [person], sources: [], decision: null });

const routes = {
  ...SERVICE,
  '/teams': ok({ teams: [team('team-1', ADA, 'Crew', [COURIER, ARCHIVIST])] }),
  '/runtime/live': ok({ sessions: [running], unanswered: [] }),
  '/requests': ok({ requests: [ask('r-1'), { ...ask('r-2'), state: 'approved' }] }),
  '/reviews': ok({ scope: 'personal', due: [{}, {}], unanswered: [], revision: 1, judged_at: 1790000000 }),
};

describe('Front page', () => {
  it('shows the running agent as a small view that opens its terminal and never resizes the session', async () => {
    localStorage.clear();
    const { posted } = await mount('#/me', routes);
    const peek = $('.peek') as HTMLAnchorElement | null;
    expect(peek?.getAttribute('href')).toBe('#/runtime/' + LIVE);
    expect(peek?.textContent).toContain('Scribe');
    expect(peek?.textContent).toContain('on Lab');
    expect($('.peek-screen')?.hasAttribute('inert')).toBe(true);
    expect(posted.filter((call) => call.path.endsWith('/resize'))).toEqual([]);
    expect(unreachable()).toEqual([]);
  });

  it('counts what waits for the person and links to it', async () => {
    await mount('#/me', routes);
    expect($('.you-waiting')?.textContent).toBe('1 request to decide · 2 reviews due');
    expect($$('.you-waiting a').map((a) => a.getAttribute('href'))).toEqual(['#/requests', '#/reviews']);
  });

  it('lists the agents under their teams, teams first, says which run, and folds a team away and back', async () => {
    localStorage.clear();
    await mount('#/me', routes);
    expect($$('tr[data-href]').map((tr) => tr.querySelector('td')?.textContent)).toEqual(['Courier', 'Archivist', 'Scribe']);
    expect($$('tr[data-href]')[2].textContent).toContain('running on Lab');
    expect($$('tr[data-href]')[0].textContent).not.toContain('running');
    expect($('.you-fold')?.textContent).toBe('Crew2');
    await click($('.you-fold'));
    expect($$('tr[data-href]').map((tr) => tr.querySelector('td')?.textContent)).toEqual(['Scribe']);
    expect(localStorage.getItem('iam.you-folded')).toBe('team-1');
    await click($('.you-fold'));
    expect($$('tr[data-href]')).toHaveLength(3);
    expect(text()).not.toContain('Ada team');
    localStorage.clear();
  });
});
