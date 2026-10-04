/** The canvas is the person's own: they draw boxes with labels, notes and lines on it, find an agent from it, and save every layout by name, kept for them on the service. */
import { act } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, press, settle, text, type } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import { mockTerminal } from './terminal-double';
import { linkEnds, readArrangement, within } from '../src/features/runtime/canvas-marks';
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
    await click($('[data-act="add-note"]'));
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
    expect(keptHere().links).toEqual([expect.objectContaining({ from: node, to: note })]);
    // Picking a thing for a line opened no terminal and sent nothing.
    expect($('.terminal')).toBeNull();
    await click($('.canvas-note .canvas-mark-remove'));
    expect($$('path.person-line')).toHaveLength(0);
    expect(keptHere().links).toEqual([]);
  });

  it('leaves a tool with Escape and draws nothing', async () => {
    await mount('#/canvas', routes);
    await click($('[data-act="draw-line"]'));
    await pointer($('.session-canvas-node.sessions'), 'pointerdown', 0, 0);
    await press('Escape', {}, window as unknown as Element);
    expect($('[data-act="draw-line"]')?.getAttribute('aria-pressed')).toBe('false');
    await click($('[data-act="add-note"]'));
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
    expect($('.canvas-panel')?.className).toContain('open');
    expect(text()).toContain('kept on the service');
    await type($('.canvas-layouts input[name="name"]'), 'Morning');
    await click($('.canvas-layouts tfoot button'));
    expect($$('.canvas-layouts tbody tr[data-layout]').map((row) => row.getAttribute('data-layout'))).toEqual(['Morning']);
    expect((posted.find((entry) => entry.path === '/canvas/layouts')?.body as { name: string }).name).toBe('Morning');
    for (let step = 0; step < 3; step += 1) await act(async () => { one.querySelector('.session-canvas-bar')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true })); });
    expect(one.style.top).toBe('308px');
    await click($('[data-act="open-layout"]'));
    expect([one.style.left, one.style.top]).toEqual(['316px', '260px']);
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
    await click($('[data-act="add-note"]'));
    expect($('.canvas-dock [role="alert"]')?.textContent).toContain('The arrangement was not kept on the service.');
    expect($('.canvas-dock [role="alert"]')?.textContent).toContain('CanvasUnavailable: the canvas store is not open');
  });

  it('finds an agent from the dock: one that is running goes to its window, one that is not goes to its own page', async () => {
    await mount('#/canvas', routes);
    const find = $('.canvas-find input');
    await act(async () => { find?.focus(); });
    await settle();
    const all = $$('.canvas-find-list a');
    expect(all[0].getAttribute('data-find')).toBe('running');
    expect(all[0].getAttribute('href')).toBe('#/canvas/' + SCRIBE);
    expect(all[0].textContent).toContain('Running on Test runner');
    expect(all.some((each) => each.getAttribute('data-find') === 'idle' && each.getAttribute('href')?.startsWith('#/file/agent-'))).toBe(true);
    expect(all.filter((each) => each.getAttribute('href') === '#/file/' + SCRIBE)).toHaveLength(0);
    await type(find, 'scri');
    expect($$('.canvas-find-list a').map((each) => each.getAttribute('data-find'))).toEqual(['running']);
    await type(find, 'no such agent');
    expect($('.canvas-find-list')?.textContent).toBe('No agent by that name.');
    // The connections in words are in the panel, on the page whether it is open or away.
    expect($('.canvas-panel')?.className).not.toContain('open');
    await click($('[data-act="connections"]'));
    expect($('.canvas-panel')?.getAttribute('aria-label')).toBe('Connections');
    expect($('.canvas-panel .canvas-connections')).not.toBeNull();
    await click($('[data-act="close-panel"]'));
    expect($('.canvas-panel')?.className).not.toContain('open');
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
    expect(readArrangement({ boxes: { a: { x: 1, y: 2, w: 3, h: 4 } }, open: ['a'] })).toEqual({ boxes: { a: { x: 1, y: 2, w: 3, h: 4 } }, open: ['a'], groups: [], notes: [], links: [] });
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
});
