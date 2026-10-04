/** A box round each team in one press: named for the team, its agents' windows in it, a team's own teams inside its box. */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount } from './harness';
import { SCRIBE, SERVICE, ok } from './fixtures';
import type { Route } from './fixtures';
import { mockTerminal } from './terminal-double';
import { within } from '../src/features/runtime/canvas-marks';
import type { Box, Marks } from '../src/features/runtime/canvas-marks';
import { teamBoxed } from '../src/features/runtime/canvas-teams';
import { tidied } from '../src/features/runtime/canvas-tidy';
import type { SessionEdge, SessionNode } from '../src/features/runtime/session-graph';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const OPEN = { w: 440, h: 34 };
const CARD = { w: 220, h: 64 };
const NONE: Marks = { groups: [], notes: [], links: [], widgets: [] };
const node = (id: string, column: SessionNode['column'], title: string): SessionNode => ({ id, column, title, detail: '' });
const member = (team: string, window: string): SessionEdge => ({ id: team + ':' + window, from: 'team:' + team, to: window, label: '', kind: 'membership', stands: true });
const same = (id: string) => id;

describe('Team boxes, worked out', () => {
  // Iridium has one team of its own, Osmium. Ada is in Iridium, Bea in Osmium, Cy in both, Dee in Zinc, Eve in none.
  const teams = { iridium: { name: 'Iridium', parent: null }, osmium: { name: 'Osmium', parent: 'iridium' }, zinc: { name: 'Zinc', parent: null }, idle: { name: 'Idle', parent: 'zinc' } };
  const nodes = [node('team:iridium', 'teams', 'Iridium'), node('team:osmium', 'teams', 'Osmium'), node('team:zinc', 'teams', 'Zinc'), ...['ada', 'bea', 'cy', 'dee', 'eve'].map((each) => node(each, 'sessions', each))];
  const edges = [member('iridium', 'ada'), member('osmium', 'bea'), member('iridium', 'cy'), member('osmium', 'cy'), member('zinc', 'dee')];
  const boxes: Record<string, Box> = {
    'team:iridium': { x: 0, y: 0, ...CARD }, 'team:osmium': { x: 0, y: 100, ...CARD }, 'team:zinc': { x: 0, y: 200, ...CARD },
    ada: { x: 300, y: 0, ...OPEN }, bea: { x: 300, y: 100, ...OPEN }, cy: { x: 300, y: 200, ...OPEN }, dee: { x: 300, y: 300, ...OPEN }, eve: { x: 300, y: 400, ...OPEN },
  };
  const marks: Marks = { ...NONE, groups: [{ id: 'group:mine', label: 'Mine', x: 250, y: 380, w: 540, h: 80 }], widgets: [{ id: 'widget:w', kind: 'usage', x: 800, y: 0, w: 340, h: 250 }], links: [{ id: 'link:1', from: 'ada', to: 'widget:w' }] };
  const columns = Object.fromEntries(nodes.map((each) => [each.id, each.column]));
  const boxOf = (groups: Marks['groups'], id: string) => groups.find((group) => group.id === id)!;

  it('draws a box named for each team with an agent running, and none for a team with nothing running', () => {
    const made = teamBoxed({ nodes, edges, teams }, boxes, marks)!;
    expect(made.marks.groups.map((group) => [group.id, group.label])).toEqual([['group:mine', 'Mine'], ['group:team:iridium', 'Iridium'], ['group:team:osmium', 'Osmium'], ['group:team:zinc', 'Zinc']]);
    // The person's own box, its window, and every other mark are as they were.
    expect(boxOf(made.marks.groups, 'group:mine')).toEqual(marks.groups[0]);
    expect(made.boxes.eve).toEqual(boxes.eve);
    expect([made.marks.widgets, made.marks.links, made.marks.notes]).toEqual([marks.widgets, marks.links, marks.notes]);
  });

  it('stands each window in its team\'s box, a team\'s own team inside it, and an agent in both in the one furthest down', () => {
    const made = teamBoxed({ nodes, edges, teams }, boxes, marks)!;
    const [iridium, osmium, zinc, mine] = ['group:team:iridium', 'group:team:osmium', 'group:team:zinc', 'group:mine'].map((id) => boxOf(made.marks.groups, id));
    const inside = (group: Box, ids: string[]) => ids.map((id) => within(group, made.boxes[id]));
    expect(inside(osmium, ['team:osmium', 'bea', 'cy'])).toEqual([true, true, true]);
    expect(inside(osmium, ['team:iridium', 'ada', 'dee', 'eve'])).toEqual([false, false, false, false]);
    expect(inside(iridium, ['team:iridium', 'ada', 'team:osmium', 'bea', 'cy'])).toEqual([true, true, true, true, true]);
    expect([within(iridium, osmium), osmium.w * osmium.h < iridium.w * iridium.h]).toEqual([true, true]);
    expect(inside(zinc, ['team:zinc', 'dee'])).toEqual([true, true]);
    expect(inside(zinc, ['ada', 'bea', 'cy', 'eve', 'team:iridium'])).toEqual([false, false, false, false, false]);
    // Nothing else lies in a team's box: not the widget, not the person's own box, not the agent in no team.
    for (const group of [iridium, osmium, zinc]) expect([within(group, made.marks.widgets[0]), within(group, mine), within(group, made.boxes.eve)]).toEqual([false, false, false]);
    // Windows keep their sizes.
    expect(Object.entries(made.boxes).every(([id, box]) => box.w === boxes[id].w && box.h === boxes[id].h)).toBe(true);
  });

  it('is what Tidy keeps: tidied, each window is still in its team\'s box and the widget still beside its window', () => {
    const made = teamBoxed({ nodes, edges, teams }, boxes, marks)!;
    const now = tidied(made.boxes, made.marks, columns, same, 16 / 9);
    const [iridium, osmium, zinc] = ['group:team:iridium', 'group:team:osmium', 'group:team:zinc'].map((id) => boxOf(now.groups, id));
    expect(['team:osmium', 'bea', 'cy'].map((id) => within(osmium, now.boxes[id]))).toEqual([true, true, true]);
    expect(['team:iridium', 'ada'].map((id) => within(iridium, now.boxes[id]) && !within(osmium, now.boxes[id]))).toEqual([true, true]);
    expect([within(iridium, osmium), within(zinc, now.boxes.dee), within(zinc, now.boxes['team:zinc'])]).toEqual([true, true, true]);
    expect([iridium, osmium, zinc].map((group) => within(group, now.boxes.eve))).toEqual([false, false, false]);
    expect([now.widgets[0].x, now.widgets[0].y]).toEqual([now.boxes.ada.x + 440 + 48, now.boxes.ada.y]);
  });

  it('draws the boxes afresh when pressed again, keeping a colour the person gave one, and never twice', () => {
    const once = teamBoxed({ nodes, edges, teams }, boxes, marks)!;
    const tinted = { ...once.marks, groups: once.marks.groups.map((group) => group.id === 'group:team:zinc' ? { ...group, colour: 'teal' } : group) };
    const twice = teamBoxed({ nodes, edges, teams }, once.boxes, tinted)!;
    expect(twice.marks.groups.map((group) => group.id).sort()).toEqual(once.marks.groups.map((group) => group.id).sort());
    expect(boxOf(twice.marks.groups, 'group:team:zinc').colour).toBe('teal');
  });

  it('draws a team above only for a team beneath it that has an agent running, and nothing when no team has', () => {
    const made = teamBoxed({ nodes, edges: [member('idle', 'eve')], teams }, boxes, NONE)!;
    expect(made.marks.groups.map((group) => group.label)).toEqual(['Zinc', 'Idle']);
    expect(within(made.marks.groups[0], made.marks.groups[1])).toBe(true);
    expect(teamBoxed({ nodes, edges: [], teams }, boxes, NONE)).toBeNull();
    // Teams that name each other as parent are neither inside the other: the one with an agent running is drawn alone, and the press still ends.
    const round = { a: { name: 'A', parent: 'b' }, b: { name: 'B', parent: 'a' } };
    expect(teamBoxed({ nodes, edges: [member('a', 'ada')], teams: round }, boxes, NONE)!.marks.groups.map((group) => group.label)).toEqual(['A']);
    // A team whose parent is not among the teams read is drawn on its own.
    expect(teamBoxed({ nodes, edges: [member('a', 'ada')], teams: { a: { name: 'A', parent: 'gone' } } }, boxes, NONE)!.marks.groups.map((group) => group.label)).toEqual(['A']);
  });
});

const session = 'op-' + '7'.repeat(32);
const running = { session, agent: SCRIBE, machine: 'machine-one', machine_name: 'Test runner', runtime: 'sh', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'process 4242', stopped: null, reported_by: 'test runner' };
const team = (id: string, name: string, parent: string | null, members: string[]) => ({ id, owner: 'person-ada', parent, lead: null, name, description: '', members, state: 'active' });

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  localStorage.clear();
});

describe('Team boxes on the canvas', () => {
  const sized = () => {
    const surface = $('.session-canvas-scroll') as HTMLElement;
    Object.defineProperty(surface, 'clientWidth', { configurable: true, value: 1600 });
    Object.defineProperty(surface, 'clientHeight', { configurable: true, value: 900 });
  };

  it('are drawn in one press from the tools bar, the agent\'s window inside its team\'s box inside the team above, and can be put back', async () => {
    const routes: Record<string, Route> = { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }), '/teams': ok({ teams: [team('team-top', 'Iridium', null, []), team('team-low', 'Osmium', 'team-top', [SCRIBE])] }) };
    await mount('#/canvas', routes);
    sized();
    expect($$('[data-group]').length).toBe(0);
    expect($('[data-act="teams"]')?.closest('[role="toolbar"]')?.getAttribute('aria-label')).toBe('Canvas tools');
    await click($('[data-act="teams"]'));
    const labels = $$('[data-group] input').map((each) => (each as HTMLInputElement).value);
    expect(labels.sort()).toEqual(['Iridium', 'Osmium']);
    const rect = (element: Element | null): Box => { const style = (element as HTMLElement).style; return { x: parseFloat(style.left), y: parseFloat(style.top), w: parseFloat(style.width), h: parseFloat(style.height) }; };
    const [top, low] = ['Iridium', 'Osmium'].map((label) => rect($$('[data-group]').find((each) => (each.querySelector('input') as HTMLInputElement).value === label) ?? null));
    const window = rect($('[data-node="session:' + session + '"]'));
    expect([within(low, { ...window, h: 34 }), within(top, low), low.w < top.w]).toEqual([true, true, true]);
    await click($('[data-act="untidy"]'));
    expect($$('[data-group]').length).toBe(0);
  });

  it('are not offered when no team has an agent running', async () => {
    await mount('#/canvas', { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }) });
    expect($('[data-act="tidy"]')).not.toBeNull();
    expect($('[data-act="teams"]')).toBeNull();
  });
});
