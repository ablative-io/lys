/**
 * The three dashboards of 18 October, drawn from Cherie's fixture under `?option=A|B|C`: each is whole on the screen
 * with nothing scrolling top to bottom (no `.dash-body`), its tiles are equal cells of one grid, what waits on her comes
 * first, and the alarms and security events are one line each.
 */
import { renderToStaticMarkup } from 'react-dom/server';
import { MemoryRouter } from 'react-router';
import { describe, expect, it } from 'vitest';
import type { RowContext } from './AgentTree';
import { DashboardOption, dashOptionOf } from './DashboardOptions';
import type { DashOption } from './DashboardOptions';
import { cherie } from './dashboard-cherie.fixture';

const data = cherie().data;
const rows = data.answer.agents;
const drawn = (option: DashOption) => {
  const context: RowContext = { rows: new Map(rows.map((row) => [row.agent.id, row])), me: data.me.person.id, asked: null, ask: () => undefined, changed: () => undefined, watch: () => undefined, watching: null, stopped: null };
  const branches = [{ team: null, members: rows.map((row) => ({ entry: { id: row.agent.id, display_name: row.agent.display_name, state: row.agent.state, kind: 'agent' as const, role: null, person: null }, branches: [] })) }];
  const host = document.createElement('div');
  host.innerHTML = renderToStaticMarkup(<MemoryRouter><DashboardOption option={option} shown={{ data, context, branches, reload: () => undefined, fixture: true, stoppable: true, openStop: () => undefined }} /></MemoryRouter>);
  return host;
};
const labels = (host: HTMLElement) => [...host.querySelectorAll('.dash-tile > section')].map((each) => each.getAttribute('aria-label'));

describe('The dashboard options', () => {
  it('reads only A, B or C; absent or unknown is today’s page', () => {
    expect([null, 'A', 'B', 'C', 'a', 'D'].map(dashOptionOf)).toEqual([null, 'A', 'B', 'C', null, null]);
  });

  it('A: four equal tiles, waiting on her first, then look out for, agents, the week; nothing scrolls the page', () => {
    const host = drawn('A');
    expect(host.querySelector('.dash-body')).toBeNull();
    expect(labels(host)).toEqual(['Waiting on you', 'Look out for', 'Agents', 'This week']);
  });

  it('Look out for says what is wrong one line each: Helper near its limit, Researcher’s missed goal, and the security events as one line', () => {
    const host = drawn('A');
    const lines = [...host.querySelectorAll('.dash-lookout tbody tr td:first-child')].map((td) => td.textContent);
    expect(lines.some((words) => words?.startsWith('Helper has used 84% of its'))).toBe(true);
    expect(lines).toContain('Researcher missed a goal: Sort the receipts folder by Friday');
    expect(host.querySelector('.dash-lookout .dash-count')?.textContent).toBe('2');
    expect(host.querySelector('.dash-security-line')?.textContent).toBe('No security events this week.');
  });

  it('B: one line of agents over one column; the strip counts her agents and what runs', () => {
    const host = drawn('B');
    expect(host.querySelector('.dash-strip')?.textContent).toContain('3 agents');
    expect(host.querySelector('.dash-strip')?.textContent).toContain('1 running');
    expect(host.querySelector('.dash-strip')?.textContent).toContain('this week US$4.95');
    expect([...host.querySelectorAll('.dash-column > section')].map((each) => each.getAttribute('aria-label'))).toEqual(['Waiting on you', 'Look out for']);
  });

  it('C: six equal tiles, the alarms and the security events each their own line', () => {
    const host = drawn('C');
    expect(labels(host)).toEqual(['Waiting on you', 'Look out for', 'Agents', 'This week', 'Alarms', 'Security events']);
    expect(host.querySelector('.dash-alarms-tile')?.textContent).toContain('No open alarms.');
    expect(host.querySelector('.dash-lookout .dash-security-line')).toBeNull();
  });
});
