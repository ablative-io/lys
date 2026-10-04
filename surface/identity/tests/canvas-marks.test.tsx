/** The canvas is the person's own: they draw boxes with labels, notes and lines on it, find an agent from it, and save every layout by name, kept for them on the service. */
import { act } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, press, settle, text, type, unmountAll } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { mockTerminal } from './terminal-double';
import { linkEnds, linkRoute, nearestSide, readArrangement, within } from '../src/features/runtime/canvas-marks';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const session = 'op-' + '7'.repeat(32);
const node = 'session:' + session;
const running = { session, agent: SCRIBE, machine: 'machine-one', machine_name: 'Test runner', runtime: 'sh', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'process 4242', stopped: null, reported_by: 'test runner' };
const routes: Record<string, Route> = { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }) };
const pointer = (target: Element | null, kind: string, x: number, y: number) => act(async () => { target?.dispatchEvent(new MouseEvent(kind, { bubbles: true, button: 0, clientX: x, clientY: y })); });
const leave = (field: Element | null) => act(async () => { field?.dispatchEvent(new FocusEvent('focusout', { bubbles: true })); });
const write = (area: Element | null, words: string) => act(async () => {
  Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')?.set?.call(area, words);
  area?.dispatchEvent(new Event('input', { bubbles: true }));
});
const keptHere = () => JSON.parse(localStorage.getItem('lys.canvas') ?? 'null') as { groups: { label: string; x: number; y: number; w: number; h: number }[]; notes: { text: string }[]; links: { from: string; to: string }[]; boxes: Record<string, { x: number; y: number }> };

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  localStorage.clear();
});

/** Holds the note tool and presses the surface at (x, y): the note goes there. */
async function placeNote(x = 400, y = 300) {
  const surface = $('.session-canvas-scroll');
  await click($('[data-act="draw-note"]'));
  await pointer(surface, 'pointerdown', x, y);
  await pointer(surface, 'pointerup', x, y);
}

/** Draws a box from (100, 100) to (700, 500) on the surface, whose view starts at (24, 24), and labels it. */
async function drawBox(label: string) {
  const surface = $('.session-canvas-scroll');
  await click($('[data-act="draw-box"]'));
  await pointer(surface, 'pointerdown', 100, 100);
  await pointer(surface, 'pointermove', 700, 500);
  await pointer(surface, 'pointerup', 700, 500);
  await type($('.canvas-group input'), label);
  await leave($('.canvas-group input'));
}

describe('The canvas is the person\'s own', () => {
  it('draws a labelled box where the person drags, and the box takes the windows in it along when it is moved', async () => {
    localStorage.setItem('lys.canvas', JSON.stringify({ boxes: { [node]: { x: 200, y: 200, w: 440, h: 34 } }, open: [], view: { x: 24, y: 24 } }));
    await mount('#/canvas', routes);
    await drawBox('Iridium');
    expect($('[data-act="draw-box"]')?.getAttribute('aria-pressed')).toBe('false');
    const box = $('.canvas-group') as HTMLElement;
    expect([box.style.left, box.style.top, box.style.width, box.style.height]).toEqual(['76px', '76px', '600px', '400px']);
    expect(box.getAttribute('aria-label')).toBe('Box: Iridium');
    expect(keptHere().groups).toEqual([expect.objectContaining({ label: 'Iridium', x: 76, y: 76, w: 600, h: 400 })]);
    // Moved by its bar, the box and the window inside it go the same distance.
    const surface = $('.session-canvas-scroll');
    await pointer($('.canvas-group-bar'), 'pointerdown', 300, 110);
    await pointer(surface, 'pointermove', 350, 140);
    await pointer(surface, 'pointerup', 350, 140);
    expect([box.style.left, box.style.top]).toEqual(['126px', '106px']);
    const one = $('.session-canvas-node.sessions') as HTMLElement;
    expect([one.style.left, one.style.top]).toEqual(['250px', '230px']);
    // Taken away, it is gone from the surface and from what is kept; the window stays.
    await click($('.canvas-group .canvas-mark-remove'));
    expect($('.canvas-group')).toBeNull();
    expect(keptHere().groups).toEqual([]);
    expect($('.session-canvas-node.sessions')).not.toBeNull();
  });

  it('puts a note on the canvas, keeps what is written in it, and draws a line from a window to it that goes when the note does', async () => {
    await mount('#/canvas', routes);
    expect($('[data-act="draw-note"]')?.getAttribute('aria-pressed')).toBe('false');
    await click($('[data-act="draw-note"]'));
    expect($('[data-act="draw-note"]')?.getAttribute('aria-pressed')).toBe('true');
    expect(text()).toContain('Press on the canvas where the note goes.');
    expect($('.canvas-note')).toBeNull();
    await pointer($('.session-canvas-scroll'), 'pointerdown', 424, 324);
    await pointer($('.session-canvas-scroll'), 'pointerup', 424, 324);
    // The note is where the press landed (the view starts at 24, 64, beneath the swap), and the tool is let go.
    expect([($('.canvas-note') as HTMLElement).style.left, ($('.canvas-note') as HTMLElement).style.top]).toEqual(['400px', '260px']);
    expect($('[data-act="draw-note"]')?.getAttribute('aria-pressed')).toBe('false');
    await write($('.canvas-note textarea'), 'Ask Scribe about the build');
    await leave($('.canvas-note textarea'));
    expect(keptHere().notes).toEqual([expect.objectContaining({ text: 'Ask Scribe about the build' })]);
    await click($('[data-act="draw-line"]'));
    expect(text()).toContain('Press the thing the line starts from.');
    await pointer($('.session-canvas-node.sessions'), 'pointerdown', 0, 0);
    expect(text()).toContain('Now press the thing the line goes to.');
    await pointer($('.canvas-note'), 'pointerdown', 0, 0);
    expect($$('path.person-line')).toHaveLength(1);
    expect($('[data-act="draw-line"]')?.getAttribute('aria-pressed')).toBe('false');
    const note = $('.canvas-note')?.getAttribute('data-note');
    // What is kept names the window by its agent, so the line holds when the agent has been started again.
    expect(keptHere().links).toEqual([expect.objectContaining({ from: 'agent:' + SCRIBE, to: note })]);
    // Picking a thing for a line opened no terminal and sent nothing.
    expect($('.terminal')).toBeNull();
    await click($('.canvas-note .canvas-mark-remove'));
    expect($$('path.person-line')).toHaveLength(0);
    expect(keptHere().links).toEqual([]);
  });

  it('draws a line by dragging from a dot on a thing\'s edge to another thing, and draws none when it is let go over nothing', async () => {
    await mount('#/canvas', routes);
    await placeNote();
    const [one, note, surface] = [$('.session-canvas-node.sessions'), $('.canvas-note'), $('.session-canvas-scroll')];
    expect(one?.querySelectorAll('.canvas-anchor')).toHaveLength(4);
    expect(note?.querySelectorAll('.canvas-anchor')).toHaveLength(4);
    const over = vi.fn<(x: number, y: number) => Element | null>(() => null);
    Object.defineProperty(document, 'elementFromPoint', { configurable: true, value: over });
    await pointer(one?.querySelector('.canvas-anchor') ?? null, 'pointerdown', 300, 200);
    await pointer(surface, 'pointermove', 500, 400);
    expect($$('path.drawing-line')).toHaveLength(1);
    await pointer(surface, 'pointerup', 500, 400);
    expect($$('path.drawing-line')).toHaveLength(0);
    expect($$('path.person-line')).toHaveLength(0);
    over.mockReturnValue(note?.querySelector('textarea') ?? null);
    await pointer(one?.querySelector('.canvas-anchor') ?? null, 'pointerdown', 300, 200);
    await pointer(surface, 'pointermove', 420, 320);
    await pointer(surface, 'pointerup', 420, 320);
    expect($$('path.person-line')).toHaveLength(1);
    // It leaves the edge whose dot it was dragged from (the first dot is the top one) and meets the edge it was let go nearest; its way turns square corners.
    expect(keptHere().links).toEqual([expect.objectContaining({ from: 'agent:' + SCRIBE, to: note?.getAttribute('data-note'), from_side: 'top', to_side: 'left' })]);
    expect($('path.person-line')?.getAttribute('d')).toMatch(/^M ([-\d.]+) [-\d.]+ L \1 [-\d.]+ Q \1 ([-\d.]+) [-\d.]+ \2 L [-\d.]+ \2$/);
    // The window was not moved and no terminal was opened by the drag.
    expect($('.terminal')).toBeNull();
    Reflect.deleteProperty(document, 'elementFromPoint');
  });

  it('stays where it is when the page is read again and when the agent has been started again', async () => {
    await mount('#/canvas', routes);
    await drawBox('Iridium');
    const one = $('.session-canvas-node.sessions') as HTMLElement;
    await act(async () => { one.querySelector('.session-canvas-bar')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true })); });
    const [left, top] = [one.style.left, one.style.top];
    expect(Object.keys(keptHere().boxes)).toContain('agent:' + SCRIBE);
    expect(Object.keys(keptHere().boxes).some((id) => id.startsWith('session:'))).toBe(false);
    unmountAll();
    // The agent was stopped and started: its session has another name. Its window is where the agent's was, and the box is still there.
    const again = 'op-' + '9'.repeat(32);
    await mount('#/canvas', { ...routes, '/runtime/live': ok({ sessions: [{ ...running, session: again }], unanswered: [] }) });
    const now = $('.session-canvas-node.sessions') as HTMLElement;
    expect(now.getAttribute('data-node')).toBe('session:' + again);
    expect([now.style.left, now.style.top]).toEqual([left, top]);
    expect($('.canvas-group')?.getAttribute('aria-label')).toBe('Box: Iridium');
  });

  it('keeps each bar in the corner the person moved it to: dragged by its grip, or pressed for the next corner round', async () => {
    await mount('#/canvas', routes);
    const corner = (bar: string) => $('[data-move="' + bar + '"]')?.closest('.canvas-corner')?.getAttribute('data-corner');
    const corners = () => ['look', 'draw', 'find'].map(corner);
    expect(corners()).toEqual(['bottom-left', 'bottom-right', 'bottom-right']);
    // Pressed without dragging, a bar goes to the next corner round.
    await pointer($('[data-move="look"]'), 'pointerdown', 20, 780);
    await pointer($('[data-move="look"]'), 'pointerup', 20, 780);
    expect(corners()).toEqual(['top-left', 'bottom-right', 'bottom-right']);
    // Dragged, it goes to the corner nearest where it is let go; while it is dragged each corner shows it can go there.
    const canvas = $('.canvas-corner')?.parentElement as HTMLElement;
    vi.spyOn(canvas, 'getBoundingClientRect').mockReturnValue({ left: 0, top: 0, width: 1000, height: 800, right: 1000, bottom: 800, x: 0, y: 0, toJSON: () => ({}) });
    const grip = $('[data-move="find"]');
    await pointer(grip, 'pointerdown', 900, 780);
    expect($$('.canvas-corner-offer').map((each) => each.getAttribute('data-corner'))).toEqual(['top-left', 'top-right', 'bottom-right', 'bottom-left']);
    await pointer(grip, 'pointerup', 880, 60);
    expect([corners(), $$('.canvas-corner-offer').length]).toEqual([['top-left', 'bottom-right', 'top-right'], 0]);
    // The panel opens beside the bar it belongs to: the layouts by the tools, the widgets by the drawing bar.
    await click($('[data-act="layouts"]'));
    expect([$('.canvas-pop')?.getAttribute('aria-label'), $('.canvas-dock')?.getAttribute('data-corner')]).toEqual(['Layouts', 'top-right']);
    await click($('[data-act="draw-widget"]'));
    expect([$('.canvas-pop')?.getAttribute('aria-label'), $('.canvas-dock')?.getAttribute('data-corner')]).toEqual(['Widgets', 'bottom-right']);
    // Kept in this browser: read again, each bar is where it was put; a corner it does not know is the bar's own.
    expect(JSON.parse(localStorage.getItem('lys.canvas.bars') ?? '{}')).toEqual({ look: 'top-left', draw: 'bottom-right', find: 'top-right' });
    localStorage.setItem('lys.canvas.bars', JSON.stringify({ look: 'top-left', draw: 'middle', find: 'top-right' }));
    unmountAll();
    document.body.innerHTML = '';
    await mount('#/canvas', routes);
    expect(corners()).toEqual(['top-left', 'bottom-right', 'top-right']);
  });

  it('leaves a tool with Escape and draws nothing', async () => {
    await mount('#/canvas', routes);
    await click($('[data-act="draw-line"]'));
    await pointer($('.session-canvas-node.sessions'), 'pointerdown', 0, 0);
    await press('Escape', {}, window as unknown as Element);
    expect($('[data-act="draw-line"]')?.getAttribute('aria-pressed')).toBe('false');
    await placeNote();
    await pointer($('.canvas-note'), 'pointerdown', 0, 0);
    expect($$('path.person-line')).toHaveLength(0);
  });

  it('keeps the arrangement and every named layout on the service, under the agent\'s name so it holds across a restart', async () => {
    const saved: { name: string; saved_at: number; arrangement: unknown }[] = [];
    const { posted } = await mount('#/canvas', {
      ...routes,
      '/canvas': ok({ arrangement: { boxes: { ['agent:' + SCRIBE]: { x: 300, y: 260, w: 440, h: 34 } }, open: [], groups: [{ id: 'group:a', label: 'Haematite', x: 250, y: 200, w: 600, h: 300 }], notes: [], links: [] }, layouts: [] }),
      'PUT /canvas': ok({}),
      'POST /canvas/layouts': ((body: { name: string; arrangement: unknown }) => { saved.push({ name: body.name, saved_at: 1790000500, arrangement: body.arrangement }); return ok({ layouts: saved }); }) as Route,
      'POST /canvas/layouts/remove': ((body: { name: string }) => { saved.splice(saved.findIndex((each) => each.name === body.name), 1); return ok({ layouts: saved }); }) as Route,
    });
    // What the service kept for this person is what the canvas shows: the agent's window where it was, and the box.
    const one = $('.session-canvas-node.sessions') as HTMLElement;
    expect([one.style.left, one.style.top]).toEqual(['300px', '260px']);
    expect($('.canvas-group')?.getAttribute('aria-label')).toBe('Box: Haematite');
    expect(posted.filter((entry) => entry.path === 'PUT /canvas')).toHaveLength(0);
    // A change is kept there once it is whole, with the window under its agent's name, never its session's.
    await act(async () => { one.querySelector('.session-canvas-bar')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true })); });
    await settle();
    const puts = posted.filter((entry) => entry.path === 'PUT /canvas');
    expect(puts).toHaveLength(1);
    const kept = readArrangement((puts[0].body as { arrangement: unknown }).arrangement);
    expect(kept?.boxes['agent:' + SCRIBE]).toEqual({ x: 316, y: 260, w: 440, h: 34 });
    expect(Object.keys(kept?.boxes ?? {}).some((id) => id.startsWith('session:'))).toBe(false);
    expect(kept?.groups).toHaveLength(1);
    // Saved by name, listed, opened again after the window was moved away, and removed.
    await click($('[data-act="layouts"]'));
    expect($('.canvas-pop')?.className).toContain('open');
    expect($('.canvas-pop')?.getAttribute('aria-label')).toBe('Layouts');
    expect(text()).toContain('kept on the service');
    await type($('.canvas-layouts input[name="name"]'), 'Morning');
    await click($('.canvas-layouts tfoot button'));
    expect($$('.canvas-layouts tbody tr[data-layout]').map((row) => row.getAttribute('data-layout'))).toEqual(['Morning']);
    expect((posted.find((entry) => entry.path === '/canvas/layouts')?.body as { name: string }).name).toBe('Morning');
    for (let step = 0; step < 3; step += 1) await act(async () => { one.querySelector('.session-canvas-bar')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true })); });
    expect(one.style.top).toBe('308px');
    // A layout's name is its link: it opens the layout, the panel stays out, and the layout's name does not stay in the address.
    expect($('[data-act="open-layout"]')?.getAttribute('href')).toBe('#/canvas?panel=layouts&layout=Morning');
    await click($('[data-act="open-layout"]'));
    expect([one.style.left, one.style.top]).toEqual(['316px', '260px']);
    expect(location.hash).toBe('#/canvas?panel=layouts');
    expect($('.canvas-pop')?.className).toContain('open');
    await click($('[data-act="remove-layout"]'));
    expect($$('.canvas-layouts tbody tr[data-layout]')).toHaveLength(0);
    expect(text()).toContain('No layout is saved yet.');
  });

  it('says by name when the service will not keep the canvas, and keeps it and its layouts in this browser', async () => {
    await mount('#/canvas', { ...routes, '/canvas': refused(503, 'CanvasUnavailable', 'the canvas store is not open') });
    await click($('[data-act="layouts"]'));
    expect(text()).toContain('Kept in this browser only');
    expect(text()).toContain('CanvasUnavailable: the canvas store is not open');
    await type($('.canvas-layouts input[name="name"]'), 'Here');
    await click($('.canvas-layouts tfoot button'));
    expect($$('.canvas-layouts tbody tr[data-layout]').map((row) => row.getAttribute('data-layout'))).toEqual(['Here']);
    expect(JSON.parse(localStorage.getItem('lys.canvas.layouts') ?? '[]')).toHaveLength(1);
  });

  it('says that a change was not kept when the service that keeps the canvas refuses it, with the refusal by name', async () => {
    await mount('#/canvas', { ...routes, '/canvas': ok({ arrangement: null, layouts: [] }), 'PUT /canvas': refused(503, 'CanvasUnavailable', 'the canvas store is not open') });
    await placeNote();
    expect($('.canvas-dock [role="alert"]')?.textContent).toContain('The arrangement was not kept on the service.');
    expect($('.canvas-dock [role="alert"]')?.textContent).toContain('CanvasUnavailable: the canvas store is not open');
  });

  it('keeps the agents behind one button: the running ones go to their windows, the others to their own pages, and a name typed finds one', async () => {
    await mount('#/canvas', routes);
    // The bar is symbols only, each named for a reader; nothing is laid over the canvas until it is asked for.
    expect($$('.canvas-bar button').every((each) => !!each.getAttribute('aria-label'))).toBe(true);
    expect($('[data-act="agents"]')?.textContent).toBe('1');
    expect($('.canvas-pop')?.className).not.toContain('open');
    await click($('[data-act="agents"]'));
    expect(location.hash).toBe('#/canvas?panel=agents');
    expect($('.canvas-pop')?.getAttribute('aria-label')).toBe('Agents');
    expect(document.activeElement).toBe($('.canvas-find input'));
    const all = $$('.canvas-find-list a');
    expect(all[0].getAttribute('href')).toBe('#/canvas/' + SCRIBE);
    expect(all[0].closest('.running-chip')?.textContent).toContain('Test runner');
    expect(all.some((each) => each.getAttribute('data-find') === 'idle' && each.getAttribute('href')?.startsWith('#/file/agent-'))).toBe(true);
    expect(all.filter((each) => each.getAttribute('href') === '#/file/' + SCRIBE)).toHaveLength(0);
    await type($('.canvas-find input'), 'scri');
    expect($$('.canvas-find-list a').map((each) => each.getAttribute('href'))).toEqual(['#/canvas/' + SCRIBE]);
    await type($('.canvas-find input'), 'no such agent');
    expect($$('.canvas-find-list a')).toHaveLength(0);
    expect($('.canvas-find-list p')?.textContent).toBe('No agent by that name.');
    // The connections in words are in the same panel, on the page whether it is open or away; Escape puts it away.
    await click($('[data-act="connections"]'));
    expect($('.canvas-pop')?.getAttribute('aria-label')).toBe('Connections');
    expect($('.canvas-pop .canvas-connections')).not.toBeNull();
    await press('Escape', {}, window as unknown as Element);
    expect($('.canvas-pop')?.className).not.toContain('open');
    expect($('.canvas-pop .canvas-connections')).not.toBeNull();
    expect(location.hash).toBe('#/canvas');
  });
});

describe('Addresses on the canvas', () => {
  it('goes straight to a saved layout or to a panel by its address, and says when the layout named is not saved', async () => {
    const layout = { name: 'Morning', saved_at: 1790000500, arrangement: { boxes: { ['agent:' + SCRIBE]: { x: 640, y: 480, w: 440, h: 34 } }, open: [], groups: [{ id: 'group:a', label: 'Iridium', x: 600, y: 400, w: 600, h: 300 }], notes: [], links: [] } };
    const kept = { ...routes, '/canvas': ok({ arrangement: null, layouts: [layout] }), 'PUT /canvas': ok({}) };
    await mount('#/canvas?layout=Morning', kept);
    const one = $('.session-canvas-node.sessions') as HTMLElement;
    expect([one.style.left, one.style.top]).toEqual(['640px', '480px']);
    expect($('.canvas-group')?.getAttribute('aria-label')).toBe('Box: Iridium');
    expect(location.hash).toBe('#/canvas');
  });

  it('opens the panel the address names, and names a layout that is not saved', async () => {
    await mount('#/canvas?panel=connections&layout=Nowhere', routes);
    expect($('.canvas-pop')?.className).toContain('open');
    expect($('.canvas-pop')?.getAttribute('aria-label')).toBe('Connections');
    expect($('.canvas-dock [role="alert"]')?.textContent).toBe('No layout is saved as Nowhere.');
    expect(location.hash).toBe('#/canvas?panel=connections');
  });
});

describe('The command palette', () => {
  it('lists each agent running now first, and choosing one goes to its terminal on the canvas', async () => {
    await mount('#/people', routes);
    await press('k', { metaKey: true }, document.body);
    const rows = $$('#palList > *');
    expect(rows[0].textContent).toBe('Running now');
    expect(rows[1].textContent).toContain('Scribe: terminal');
    expect(rows[1].textContent).toContain('on Test runner');
    expect(rows[1].getAttribute('data-to')).toBe('#/canvas/' + SCRIBE);
    await type($('#palIn'), 'scribe: term');
    expect($('#palList .it')?.textContent).toContain('Scribe: terminal');
    expect($$('#palList .it')).toHaveLength(1);
    await click($('#palList .it'));
    expect(location.hash).toBe('#/canvas/' + SCRIBE);
  });
});

describe('What a person drew, as it is kept', () => {
  it('reads an arrangement kept before there were marks, and refuses one that is not an arrangement', () => {
    expect(readArrangement({ boxes: { a: { x: 1, y: 2, w: 3, h: 4 } }, open: ['a'] })).toEqual({ boxes: { a: { x: 1, y: 2, w: 3, h: 4 } }, open: ['a'], groups: [], notes: [], links: [], widgets: [] });
    expect(readArrangement({ boxes: {}, open: [], widgets: [{ id: 'widget:a', kind: 'usage', x: 0, y: 0, w: 1, h: 1 }] })?.widgets).toHaveLength(1);
    expect(readArrangement({ boxes: {}, open: [], widgets: [{ id: 'widget:a', x: 0, y: 0, w: 1, h: 1 }] })).toBeNull();
    expect(readArrangement({ boxes: {}, open: [], groups: [{ id: 'group:a', x: 0, y: 0, w: 1, h: 1 }] })).toBeNull();
    expect(readArrangement({ boxes: {}, open: [], links: [{ id: 'link:a', from: 'a' }] })).toBeNull();
    expect(readArrangement({ boxes: { a: { x: 'left' } }, open: [] })).toBeNull();
    expect(readArrangement(null)).toBeNull();
  });

  it('counts a thing as in a box by its middle, and ends a line on the facing edges', () => {
    const box = { x: 0, y: 0, w: 100, h: 100 };
    expect(within(box, { x: 80, y: 80, w: 30, h: 30 })).toBe(true);
    expect(within(box, { x: 90, y: 90, w: 30, h: 30 })).toBe(false);
    expect(linkEnds(box, { x: 300, y: 20, w: 100, h: 100 })).toEqual([100, 50, 300, 70]);
    expect(linkEnds(box, { x: -300, y: 20, w: 100, h: 100 })).toEqual([0, 50, -200, 70]);
    expect(linkEnds(box, { x: 20, y: 300, w: 100, h: 100 })).toEqual([50, 100, 70, 300]);
    expect(linkEnds(box, { x: 20, y: -300, w: 100, h: 100 })).toEqual([50, 0, 70, -200]);
  });

  it('routes a line from the edge it was drawn from with square corners, never as the crow flies', () => {
    const [from, to] = [{ x: 0, y: 0, w: 100, h: 100 }, { x: 300, y: 200, w: 100, h: 100 }];
    // With no edge named the facing edges are taken: out to the right, in from the left, turning half way.
    expect(linkRoute(from, to, {})).toEqual({ d: 'M 100 50 L 186 50 Q 200 50 200 64 L 200 236 Q 200 250 214 250 L 300 250', middle: [200, 150] });
    expect(linkRoute(from, to, { from_side: 'bottom', to_side: 'top' })).toEqual({ d: 'M 50 100 L 50 136 Q 50 150 64 150 L 336 150 Q 350 150 350 164 L 350 200', middle: [200, 150] });
    expect(linkRoute(from, to, { from_side: 'right', to_side: 'top' })).toEqual({ d: 'M 100 50 L 336 50 Q 350 50 350 64 L 350 200', middle: [350, 50] });
    expect(linkRoute(from, to, { from_side: 'bottom', to_side: 'left' })).toEqual({ d: 'M 50 100 L 50 236 Q 50 250 64 250 L 300 250', middle: [50, 250] });
    expect(nearestSide(to, [310, 250])).toBe('left');
    expect(nearestSide(to, [350, 290])).toBe('bottom');
    expect(readArrangement({ boxes: {}, open: [], links: [{ id: 'link:a', from: 'a', to: 'b', from_side: 'up' }] })).toBeNull();
  });
});
