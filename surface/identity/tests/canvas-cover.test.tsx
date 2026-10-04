/** A box covers what is let go in it, and is held by its whole outline. */
import { act } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, mount } from './harness';
import { SCRIBE, SERVICE, ok } from './fixtures';
import type { Route } from './fixtures';
import { mockTerminal } from './terminal-double';
import { covering } from '../src/features/runtime/canvas-marks';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

describe('Covering, worked out', () => {
  const box = { id: 'group:a', label: 'A', x: 100, y: 100, w: 300, h: 200 };

  it('draws a box out round a thing whose middle is in it, clear of it by 20 and 40 under its bar, and never draws one in', () => {
    // The thing's middle (380, 280) is inside; it reaches past the right edge and the foot.
    expect(covering([box], [{ x: 300, y: 250, w: 160, h: 60 }])).toEqual([{ ...box, w: 380, h: 230 }]);
    // Past the left edge and above the bar's foot.
    expect(covering([box], [{ x: 60, y: 110, w: 100, h: 40 }])).toEqual([{ ...box, x: 40, y: 70, w: 360, h: 230 }]);
    // A thing whose middle is outside is not the box's, and a thing well inside changes nothing: the same box is given back.
    const same = covering([box], [{ x: 390, y: 150, w: 200, h: 40 }, { x: 150, y: 160, w: 100, h: 40 }]);
    expect(same[0]).toBe(box);
  });

  it('grows a box round the box inside it after that one has grown', () => {
    const outer = { id: 'group:o', label: 'O', x: 0, y: 0, w: 500, h: 400 };
    // The thing's middle (400, 170) is in both; it reaches to 550. The inner box is drawn out to 570, and the outer round the inner to 590.
    const [inner, around] = covering([box, outer], [{ x: 250, y: 150, w: 300, h: 40 }]);
    expect(inner).toEqual({ ...box, w: 470 });
    expect(around).toEqual({ ...outer, w: 590 });
  });
});

const session = 'op-' + '7'.repeat(32);
const node = 'session:' + session;
const running = { session, agent: SCRIBE, machine: 'machine-one', machine_name: 'Test runner', runtime: 'sh', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'process 4242', stopped: null, reported_by: 'test runner' };
const routes: Record<string, Route> = { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }) };
const pointer = (target: Element | null, kind: string, x: number, y: number) => act(async () => { target?.dispatchEvent(new MouseEvent(kind, { bubbles: true, button: 0, clientX: x, clientY: y })); });
const rect = (element: Element | null) => { const style = (element as HTMLElement).style; return [parseFloat(style.left), parseFloat(style.top), parseFloat(style.width), parseFloat(style.height)]; };

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  localStorage.clear();
  localStorage.setItem('lys.canvas', JSON.stringify({ boxes: { [node]: { x: 900, y: 100, w: 440, h: 34 } }, open: [], view: { x: 0, y: 0 }, groups: [{ id: 'group:a', label: 'Iridium', x: 100, y: 300, w: 400, h: 200 }] }));
});

describe('A box on the canvas', () => {
  it('is drawn out round a window let go with its middle in it', async () => {
    await mount('#/canvas', routes);
    const surface = $('.session-canvas-scroll');
    expect(rect($('[data-group="group:a"]'))).toEqual([100, 300, 400, 200]);
    // The window is dragged from (900, 100) to (200, 380): its middle (420, 397) is in the box and its right edge is past the box's.
    await pointer($('[data-node="' + node + '"] .session-canvas-bar'), 'pointerdown', 950, 110);
    await pointer(surface, 'pointermove', 250, 390);
    await pointer(surface, 'pointerup', 250, 390);
    expect(rect($('[data-node="' + node + '"]')).slice(0, 2)).toEqual([200, 380]);
    expect(rect($('[data-group="group:a"]'))).toEqual([100, 300, 560, 200]);
  });

  it('is sized one way by its right edge and its foot, and moved by its left edge', async () => {
    await mount('#/canvas', routes);
    const surface = $('.session-canvas-scroll');
    await pointer($('[data-group="group:a"] .canvas-group-edge.right'), 'pointerdown', 500, 400);
    await pointer(surface, 'pointermove', 560, 470);
    await pointer(surface, 'pointerup', 560, 470);
    expect(rect($('[data-group="group:a"]'))).toEqual([100, 300, 460, 200]);
    await pointer($('[data-group="group:a"] .canvas-group-edge.foot'), 'pointerdown', 300, 500);
    await pointer(surface, 'pointermove', 380, 540);
    await pointer(surface, 'pointerup', 380, 540);
    expect(rect($('[data-group="group:a"]'))).toEqual([100, 300, 460, 240]);
    await pointer($('[data-group="group:a"] .canvas-group-edge.left'), 'pointerdown', 100, 400);
    await pointer(surface, 'pointermove', 130, 410);
    await pointer(surface, 'pointerup', 130, 410);
    expect(rect($('[data-group="group:a"]'))).toEqual([130, 310, 460, 240]);
  });
});
