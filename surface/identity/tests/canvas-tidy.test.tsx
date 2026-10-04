/** Tidy puts everything on the canvas into rows in one press: each window with its own widgets, each box round what sits in it, and it can be put back. */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, click, mount, text } from './harness';
import { SCRIBE, SERVICE, ok } from './fixtures';
import type { Route } from './fixtures';
import { mockTerminal } from './terminal-double';
import { within } from '../src/features/runtime/canvas-marks';
import type { Box, Marks } from '../src/features/runtime/canvas-marks';
import { tidied } from '../src/features/runtime/canvas-tidy';
import type { Column, Tidied } from '../src/features/runtime/canvas-tidy';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const WIDE = 16 / 9;
const same = (id: string) => id;
const OPEN = { w: 760, h: 480 };
const pill = (id: string, x: number, y: number) => ({ id, kind: 'usage', x, y, w: 340, h: 250 });
const placeOf = (all: { id: string; x: number; y: number }[], id: string) => { const each = all.find((one) => one.id === id); return [each?.x, each?.y]; };
/** What was tidied, as the canvas it now is, to tidy again. */
const again = (was: Tidied, marks: Marks, columns: Record<string, Column>, here: (id: string) => string, shape: number) =>
  tidied(was.boxes, { ...marks, groups: was.groups, notes: was.notes, widgets: was.widgets }, columns, here, shape);

describe('Tidy, worked out', () => {
  const boxes: Record<string, Box> = { team: { x: 900, y: 50, w: 220, h: 64 }, a: { x: 300, y: 700, ...OPEN }, b: { x: 10, y: 10, ...OPEN }, resource: { x: 0, y: 300, w: 220, h: 64 } };
  const columns: Record<string, Column> = { team: 'teams', a: 'sessions', b: 'sessions', resource: 'resources' };
  const marks: Marks = {
    groups: [],
    notes: [{ id: 'note:n', text: 'A note', x: 900, y: 2000, w: 260, h: 180 }],
    widgets: [pill('widget:a1', 5000, 20), pill('widget:a2', 40, 900), { ...pill('widget:b', 3000, 3000), view: 'detail' }, pill('widget:loose', 0, 2000), { ...pill('widget:sum', 500, 2000), kind: 'formula', locked: true }],
    links: [
      { id: 'link:1', from: 'a', to: 'widget:a1' }, { id: 'link:2', from: 'widget:a2', to: 'a' },
      // A line's end kept under the agent's name is that agent's window.
      { id: 'link:3', from: 'agent:B', to: 'widget:b' },
      // A formula's lines are to widgets: it is no one window's, and the widgets it takes stay with their windows.
      { id: 'link:4', from: 'widget:a1', to: 'widget:sum' }, { id: 'link:5', from: 'widget:b', to: 'widget:sum' },
    ],
  };
  const here = (id: string) => id === 'agent:B' ? 'b' : id;

  it('stands each window with its own widgets at its right, teams down the left, resources down the right, and what is no one window\'s beneath', () => {
    const now = tidied(boxes, marks, columns, here, WIDE);
    // The whole starts where the leftmost and the topmost thing stood. Two windows this large are nearest the page's shape one under the other, in the order they were read in.
    expect(now.boxes).toEqual({ team: { x: 0, y: 10, w: 220, h: 64 }, b: { x: 280, y: 10, ...OPEN }, a: { x: 280, y: 530, ...OPEN }, resource: { x: 1488, y: 10, w: 220, h: 64 } });
    // Each window's widgets are a column 48 from its right edge, 10 apart, in the order they stood; a widget opened out keeps its size.
    expect(placeOf(now.widgets, 'widget:b')).toEqual([1088, 10]);
    expect(placeOf(now.widgets, 'widget:a1')).toEqual([1088, 530]);
    expect(placeOf(now.widgets, 'widget:a2')).toEqual([1088, 574]);
    expect(now.widgets.find((each) => each.id === 'widget:b')).toMatchObject({ view: 'detail', w: 340, h: 250 });
    // Beneath, in a row from the left: the widget no line feeds, the formula (locked, and still arranged), the note.
    expect(placeOf(now.widgets, 'widget:loose')).toEqual([0, 1050]);
    expect(placeOf(now.widgets, 'widget:sum')).toEqual([268, 1050]);
    expect(now.widgets.find((each) => each.id === 'widget:sum')?.locked).toBe(true);
    expect(placeOf(now.notes, 'note:n')).toEqual([536, 1050]);
    expect(now.extent).toEqual({ x: 0, y: 10, w: 1708, h: 1220 });
  });

  it('changes nothing when what it tidied is tidied again', () => {
    const once = tidied(boxes, marks, columns, here, WIDE);
    expect(again(once, marks, columns, here, WIDE)).toEqual(once);
  });

  it('makes its rows as long as the page\'s shape asks: one column on a tall page, two by two on a wide one, one row on a very wide one', () => {
    const four: Record<string, Box> = { one: { x: 0, y: 0, ...OPEN }, two: { x: 30, y: 2000, ...OPEN }, three: { x: 60, y: 4000, ...OPEN }, four: { x: 90, y: 6000, ...OPEN } };
    const none: Marks = { groups: [], notes: [], links: [], widgets: [] };
    const places = (shape: number) => Object.values(tidied(four, none, {}, same, shape).boxes).map((box) => [box.x, box.y]);
    expect(places(0.4)).toEqual([[0, 0], [0, 520], [0, 1040], [0, 1560]]);
    expect(places(WIDE)).toEqual([[0, 0], [800, 0], [0, 520], [800, 520]]);
    expect(places(6)).toEqual([[0, 0], [800, 0], [1600, 0], [2400, 0]]);
  });
});

describe('Tidy and the boxes a person drew', () => {
  const boxes: Record<string, Box> = { a: { x: 100, y: 100, ...OPEN }, b: { x: 900, y: 600, w: 440, h: 34 }, c: { x: 3000, y: 0, ...OPEN } };
  const marks: Marks = {
    groups: [
      { id: 'group:team', label: 'Team', x: 0, y: 0, w: 2000, h: 1200 },
      { id: 'group:pair', label: 'Pair', x: 850, y: 550, w: 600, h: 200 },
      { id: 'group:empty', label: 'Later', x: 5000, y: 5000, w: 520, h: 360 },
    ],
    notes: [],
    // One widget sits in the big box and no line feeds it: it counts that box's agents. The other is fed by a line from the big box itself.
    widgets: [pill('widget:in', 50, 1000), pill('widget:of', 2500, 50)],
    links: [{ id: 'link:1', from: 'group:team', to: 'widget:of' }],
  };

  it('lays each box out inside and draws it round what sits in it, so what sat in a box still does and nothing else has come to', () => {
    const now = tidied(boxes, marks, {}, same, WIDE);
    const [team, pair, empty] = now.groups;
    // The small box is drawn round its one window: 20 clear at each side and beneath, 40 under its label.
    expect(pair).toMatchObject({ x: 20, y: 560, w: 480, h: 94 });
    expect(now.boxes.b).toEqual({ x: 40, y: 600, w: 440, h: 34 });
    // The big box holds the open window, the small box under it, and beneath them the widget that sat in it.
    expect(team).toMatchObject({ x: 0, y: 0, w: 800, h: 748 });
    expect(now.boxes.a).toEqual({ x: 20, y: 40, ...OPEN });
    expect(placeOf(now.widgets, 'widget:in')).toEqual([20, 694]);
    // The widget a line from the box feeds stands at the box's right, the window outside the box after that, and the empty box keeps its size on the next row.
    expect(placeOf(now.widgets, 'widget:of')).toEqual([848, 0]);
    expect(now.boxes.c).toEqual({ x: 1136, y: 0, ...OPEN });
    expect(empty).toMatchObject({ x: 0, y: 788, w: 520, h: 360 });
    const pillAt = (id: string): Box => ({ ...now.widgets.find((each) => each.id === id)!, w: 248, h: 34 });
    expect([within(team, now.boxes.a), within(team, pair), within(pair, now.boxes.b), within(team, now.boxes.b), within(team, pillAt('widget:in'))]).toEqual([true, true, true, true, true]);
    expect([within(team, now.boxes.c), within(team, pillAt('widget:of')), within(pair, now.boxes.a), within(empty, now.boxes.c)]).toEqual([false, false, false, false]);
    expect(again(now, marks, {}, same, WIDE)).toEqual(now);
  });
});

const session = 'op-' + '7'.repeat(32);
const node = 'session:' + session;
const running = { session, agent: SCRIBE, machine: 'machine-one', machine_name: 'Test runner', runtime: 'sh', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'process 4242', stopped: null, reported_by: 'test runner' };
const routes: Record<string, Route> = { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }) };
const keptHere = () => JSON.parse(localStorage.getItem('lys.canvas') ?? 'null') as { boxes: Record<string, { x: number; y: number }>; widgets: { x: number; y: number }[] };
const at = (element: Element | null) => { const style = (element as HTMLElement).style; return [parseFloat(style.left), parseFloat(style.top)]; };

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  localStorage.clear();
});

describe('Tidy on the canvas', () => {
  it('tidies in one press from the tools bar, keeps what it did, and puts it back as it was when asked', async () => {
    localStorage.setItem('lys.canvas', JSON.stringify({
      boxes: { [node]: { x: 200, y: 200, w: 440, h: 34 } }, open: [], view: { x: 24, y: 24 },
      widgets: [pill('widget:one', 900, 900)], links: [{ id: 'link:one', from: 'agent:' + SCRIBE, to: 'widget:one' }],
    }));
    await mount('#/canvas', routes);
    const surface = $('.session-canvas-scroll') as HTMLElement;
    Object.defineProperty(surface, 'clientWidth', { configurable: true, value: 1600 });
    Object.defineProperty(surface, 'clientHeight', { configurable: true, value: 900 });
    const [window, widget, drawing] = [$('[data-node="' + node + '"]'), $('[data-widget="widget:one"]'), $('.session-canvas') as HTMLElement];
    const before = drawing.style.transform;
    expect([at(window), at(widget)]).toEqual([[200, 200], [900, 900]]);
    expect($('[data-act="untidy"]')).toBeNull();
    expect($('[data-act="tidy"]')?.closest('[role="toolbar"]')?.getAttribute('aria-label')).toBe('Canvas tools');
    await click($('[data-act="tidy"]'));
    // The widget the agent feeds stands at its window's right, their tops level; the view has moved to hold the whole.
    const [left, top] = at(window);
    expect(at(widget)).toEqual([left + 440 + 48, top]);
    expect([left, top]).not.toEqual([200, 200]);
    expect(drawing.style.transform).not.toBe(before);
    expect(keptHere().boxes['agent:' + SCRIBE]).toEqual(expect.objectContaining({ x: left, y: top }));
    expect(keptHere().widgets[0]).toEqual(expect.objectContaining({ x: left + 488, y: top }));
    expect(text()).toContain('Tidied.');
    await click($('[data-act="untidy"]'));
    expect([at(window), at(widget)]).toEqual([[200, 200], [900, 900]]);
    expect(drawing.style.transform).toBe(before);
    expect(keptHere().boxes['agent:' + SCRIBE]).toEqual(expect.objectContaining({ x: 200, y: 200 }));
    expect($('[data-act="untidy"]')).toBeNull();
  });

  it('offers to put it back only until the person changes something else', async () => {
    await mount('#/canvas', routes);
    const surface = $('.session-canvas-scroll') as HTMLElement;
    Object.defineProperty(surface, 'clientWidth', { configurable: true, value: 1600 });
    Object.defineProperty(surface, 'clientHeight', { configurable: true, value: 900 });
    await click($('[data-act="tidy"]'));
    expect($('[data-act="untidy"]')).not.toBeNull();
    await click($('[data-act="toggle"]'));
    expect($('[data-act="untidy"]')).toBeNull();
  });
});
