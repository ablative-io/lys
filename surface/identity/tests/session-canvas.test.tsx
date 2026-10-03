/** The canvas starts with a returned running session and never turns membership or missing reads into authority. */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { $, click, mount, text } from './harness';
import { ADA, GRANTS, SCRIBE, SERVICE, ok, refused } from './fixtures';
import { act } from 'react';
import { mockTerminal } from './terminal-double';
import { lineBetween, placed } from '../src/features/runtime/SessionCanvas';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const session = 'op-' + '7'.repeat(32);
const running = { session, agent: SCRIBE, machine: 'machine-one', machine_name: 'Test runner', runtime: 'sh', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'process 4242', stopped: null, reported_by: 'test runner' };
const routes = {
  ...SERVICE,
  '/runtime/live': ok({ sessions: [running], unanswered: [] }),
  '/teams': ok({ teams: [{ id: 'team-a', name: 'Delivery', description: '', state: 'active', members: [SCRIBE] }] }),
};

beforeEach(() => {
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
  // The arrangement is kept in the browser; each test starts from none.
  localStorage.clear();
});

describe('Agent canvas', () => {
  it('shows only recorded team and grant connections for returned sessions', async () => {
    const { posted } = await mount('#/runtime/canvas', routes);
    expect(text()).toContain('Scribe');
    expect(text()).toContain('Test runner');
    expect(text()).toContain('Delivery');
    expect(text()).toContain('Member; no permission implied');
    expect(text()).toContain('View this resource on project identity');
    expect(text()).toContain('Message connections unavailable');
    expect(document.querySelectorAll('.canvas-connections li')).toHaveLength(2);
    expect(posted).toEqual([]);
  });

  it('draws the returned message addresses between identities without calling them terminal deliveries', async () => {
    await mount('#/runtime/canvas', {
      ...routes,
      '/runtime/message-edges': ok({ places: ['chat'], messages: [], roots: [], next: null, unmapped: [] }),
      '/runtime/message-edges?stream=chat': ok({ places: [], messages: [{ message: 'message-one', stream: 'chat', source: ADA, recipients: [SCRIBE], addressing: 'direct', at: 1 }], roots: [], next: null, unmapped: [] }),
    });
    expect(text()).toContain('Direct message message-one in chat');
    expect(text()).toContain('not whether a person read the message or an agent consumed it');
    expect(document.querySelectorAll('.canvas-connections li[data-kind="message"]')).toHaveLength(1);
    expect(document.querySelectorAll('.canvas-connections li[data-kind="identity"]')).toHaveLength(1);
  });

  it('keeps the server refusal on a grant instead of drawing it as authority', async () => {
    await mount('#/runtime/canvas', { ...routes, '/grants': ok({ grants: GRANTS.map((grant) => grant.holder === SCRIBE ? { ...grant, standing: { stands: false, refusal: 'GrantRevoked', grant: grant.id, reason: 'Its source was revoked' } } : grant), revision: 8 }) });
    expect(text()).toContain('GrantRevoked: Its source was revoked');
    expect($('.session-canvas-lines path[data-kind="grant"]')?.getAttribute('data-standing')).toBe('false');
  });

  it('keeps sessions and teams visible when reading grants is refused', async () => {
    await mount('#/runtime/canvas', { ...routes, '/grants': refused(403, 'GrantReadRefused', 'These grants are not visible') });
    expect(text()).toContain('GrantReadRefused: These grants are not visible');
    expect(text()).toContain('Delivery');
    expect(text()).toContain('Test runner');
    expect(document.querySelectorAll('.canvas-connections li[data-kind="grant"]')).toHaveLength(0);
  });

  it('does not use a failed live-session read as an empty graph', async () => {
    await mount('#/runtime/canvas', { ...routes, '/runtime/live': refused(503, 'RuntimeUnavailable', 'Reports could not be read') });
    expect(text()).toContain('RuntimeUnavailable');
    expect(text()).not.toContain('No running sessions were returned');
    expect($('.session-canvas')).toBeNull();
  });

  it('does not label an unanswered runner as currently running', async () => {
    await mount('#/runtime/canvas', { ...routes, '/runtime/live': ok({ sessions: [running], unanswered: [{ session, machine: 'machine-one', refusal: 'runner_unreachable', reason: 'Socket closed' }] }) });
    expect(text()).toContain('Runner did not answer; current state unknown');
    expect(text()).toContain('runner_unreachable: Socket closed');
  });

  it('keeps saying the runner did not answer in the window when its terminal is open', async () => {
    const base = '/runtime/sessions/' + session;
    await mount('#/runtime/canvas', { ...routes, '/runtime/live': ok({ sessions: [running], unanswered: [{ session, machine: 'machine-one', refusal: 'runner_unreachable', reason: 'Socket closed' }] }),
      ['POST ' + base + '/resize']: ok({ receipt: { index: 1 } }),
      ['POST ' + base + '/read-bytes']: refused(503, 'runner_unreachable', 'Socket closed') });
    const one = $('.session-canvas-node.sessions') as HTMLElement;
    await click(one.querySelector('button'));
    expect(one.querySelector('.terminal')).not.toBeNull();
    expect(one.classList.contains('unanswered')).toBe(true);
    expect(one.querySelector('.session-canvas-unanswered')?.textContent).toBe('Runner did not answer; current state unknown');
  });

  it('opens the returned session terminal and closing the view never ends its process', async () => {
    const base = '/runtime/sessions/' + session;
    const { posted } = await mount('#/runtime/canvas', {
      ...routes,
      ['POST ' + base + '/resize']: ok({ receipt: { index: 1 } }),
      ['POST ' + base + '/read-bytes']: ok({ session, answer: { kind: 'bytes', output: { session, from: 0, cursor: 0, oldest: 0, data: [], ended: { how: 'exited', at: 1, status: 0, signal: null } } }, receipt: { index: 2 } }),
    });
    await click($('.session-canvas-node button'));
    expect($('.terminal')).not.toBeNull();
    expect(posted.some((entry) => entry.path === base + '/read-bytes')).toBe(true);
    await click($('.session-canvas-node button'));
    expect($('.terminal')).toBeNull();
    expect(posted.some((entry) => entry.path.endsWith('/end'))).toBe(false);
  });

  it('is its own place in the rail, and an agent opens straight onto its terminal', async () => {
    await mount('#/canvas/' + SCRIBE, routes);
    expect($('#rail a.on')?.dataset.nav).toBe('canvas');
    expect($('button[aria-expanded="true"]')?.textContent).toBe('Close terminal view');
    expect([...document.querySelectorAll('button')].map((button) => button.textContent ?? '').filter((words) => /refresh|again/i.test(words))).toEqual([]);
  });

  it('is reached from an agent\'s file', async () => {
    await mount('#/file/' + SCRIBE, routes);
    expect($('a[data-act="canvas"]')?.getAttribute('href')).toBe('#/canvas/' + SCRIBE);
  });
  it('opens more than one terminal at once, each in its own window', async () => {
    const other = 'op-' + '8'.repeat(32);
    const bytes = (id: string) => ok({ session: id, answer: { kind: 'bytes', output: { session: id, from: 0, cursor: 0, oldest: 0, data: [], ended: { how: 'exited', at: 1, status: 0, signal: null } } }, receipt: { index: 2 } });
    await mount('#/runtime/canvas', {
      ...routes,
      '/runtime/live': ok({ sessions: [running, { ...running, session: other }], unanswered: [] }),
      ...Object.fromEntries([session, other].flatMap((id) => [['POST /runtime/sessions/' + id + '/resize', ok({ receipt: { index: 1 } })], ['POST /runtime/sessions/' + id + '/read-bytes', bytes(id)]])),
    });
    const windows = [...document.querySelectorAll<HTMLElement>('.session-canvas-node.sessions')];
    expect(windows).toHaveLength(2);
    for (const one of windows) await click(one.querySelector('button'));
    expect(document.querySelectorAll('.session-canvas-node.open .terminal')).toHaveLength(2);
  });

  it('moves a window with the arrow keys and keeps where it was put', async () => {
    await mount('#/runtime/canvas', routes);
    const one = $('.session-canvas-node.sessions') as HTMLElement;
    const before = parseFloat(one.style.left);
    await act(async () => { one.querySelector('.session-canvas-bar')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true })); });
    expect(parseFloat(one.style.left)).toBe(before + 16);
    const kept = JSON.parse(localStorage.getItem('lys.canvas') ?? 'null') as { boxes: Record<string, { x: number }> };
    expect(kept.boxes['session:' + session].x).toBe(before + 16);
  });

  it('opens the agent the route names even when its window was kept closed, at the open size', async () => {
    const base = '/runtime/sessions/' + session;
    localStorage.setItem('lys.canvas', JSON.stringify({ boxes: { ['session:' + session]: { x: 900, y: 700, w: 280, h: 34 } }, open: [], view: { x: 0, y: 0 } }));
    await mount('#/canvas/' + SCRIBE, {
      ...routes,
      ['POST ' + base + '/resize']: ok({ receipt: { index: 1 } }),
      ['POST ' + base + '/read-bytes']: ok({ session, answer: { kind: 'bytes', output: { session, from: 0, cursor: 0, oldest: 0, data: [], ended: { how: 'exited', at: 1, status: 0, signal: null } } }, receipt: { index: 2 } }),
    });
    const one = $('.session-canvas-node.sessions') as HTMLElement;
    expect(one.querySelector('.terminal')).not.toBeNull();
    expect([one.style.left, one.style.top, one.style.width, one.style.height]).toEqual(['900px', '700px', '760px', '480px']);
  });

  it('drags a window by its bar and drags the surface by its background', async () => {
    await mount('#/runtime/canvas', routes);
    const one = $('.session-canvas-node.sessions') as HTMLElement;
    const surface = $('.session-canvas-scroll') as HTMLElement;
    const pointer = (target: Element, type: string, x: number, y: number) => act(async () => { target.dispatchEvent(new MouseEvent(type, { bubbles: true, button: 0, clientX: x, clientY: y })); });
    const [left, top] = [parseFloat(one.style.left), parseFloat(one.style.top)];
    await pointer(one.querySelector('.session-canvas-bar') as Element, 'pointerdown', 10, 10);
    await pointer(surface, 'pointermove', 50, 30);
    await pointer(surface, 'pointerup', 50, 30);
    expect([parseFloat(one.style.left), parseFloat(one.style.top)]).toEqual([left + 40, top + 20]);
    const space = $('.session-canvas') as HTMLElement;
    const before = space.style.transform;
    await pointer(surface, 'pointerdown', 5, 5);
    await pointer(surface, 'pointermove', 105, 55);
    await pointer(surface, 'pointerup', 105, 55);
    expect(before).toBe('translate(24px, 24px)');
    expect(space.style.transform).toBe('translate(124px, 74px)');
    expect([parseFloat(one.style.left), parseFloat(one.style.top)]).toEqual([left + 40, top + 20]);
  });

  it('still works, and says so, when the browser refuses to keep the arrangement', async () => {
    const set = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('storage is full'); });
    try {
      await mount('#/runtime/canvas', routes);
      expect(text()).toContain('This browser will not keep the arrangement: storage is full');
      expect($('.session-canvas-node.sessions')).not.toBeNull();
    } finally { set.mockRestore(); }
  });

  it('sizes an open terminal from the keyboard and never below the size its bar needs', async () => {
    const base = '/runtime/sessions/' + session;
    await mount('#/runtime/canvas', { ...routes, ['POST ' + base + '/resize']: ok({ receipt: { index: 1 } }),
      ['POST ' + base + '/read-bytes']: ok({ session, answer: { kind: 'bytes', output: { session, from: 0, cursor: 0, oldest: 0, data: [], ended: { how: 'exited', at: 1, status: 0, signal: null } } }, receipt: { index: 2 } }) });
    const one = $('.session-canvas-node.sessions') as HTMLElement;
    await click(one.querySelector('button'));
    const key = (name: string) => act(async () => { one.querySelector('.session-canvas-bar')?.dispatchEvent(new KeyboardEvent('keydown', { key: name, shiftKey: true, bubbles: true })); });
    await key('ArrowDown');
    expect([one.style.width, one.style.height]).toEqual(['760px', '496px']);
    for (let press = 0; press < 20; press += 1) await key('ArrowLeft');
    expect(one.style.width).toBe('560px');
  });

  it('keeps the place of a team card whose read was refused this visit', async () => {
    const place = { x: 40, y: 400, w: 220, h: 64 };
    localStorage.setItem('lys.canvas', JSON.stringify({ boxes: { 'team:team-a': place }, open: [], view: { x: 24, y: 24 } }));
    await mount('#/runtime/canvas', { ...routes, '/teams': refused(503, 'TeamsUnavailable', 'Teams could not be read') });
    expect(text()).toContain('TeamsUnavailable');
    expect($('[data-node="team:team-a"]')).toBeNull();
    const kept = JSON.parse(localStorage.getItem('lys.canvas') ?? 'null') as { boxes: Record<string, unknown> };
    expect(kept.boxes['team:team-a']).toEqual(place);
  });

  it('says on each window what it is', async () => {
    await mount('#/runtime/canvas', routes);
    expect([...document.querySelectorAll('.session-canvas-kind')].map((kind) => kind.textContent).sort()).toEqual(['Agent', 'Resource or recipient', 'Team or sender']);
  });

  it('places every node once, leaves a moved one where it is, and joins two windows edge to edge', () => {
    const node = (id: string, column: 'teams' | 'sessions' | 'resources') => ({ id, column, title: id, detail: '' });
    const graph = { nodes: [node('t', 'teams'), node('a', 'sessions'), node('b', 'sessions'), node('r', 'resources')], edges: [], notices: [], unanswered: [], names: {} };
    const boxes = placed(graph, { b: { x: 5, y: 6, w: 7, h: 8 } }, new Set(['a']));
    expect(boxes.b).toEqual({ x: 5, y: 6, w: 7, h: 8 });
    expect(boxes.t.x < boxes.a.x && boxes.a.x + boxes.a.w < boxes.r.x).toBe(true);
    expect([boxes.a.w, boxes.a.h]).toEqual([760, 480]);
    expect(lineBetween({ x: 0, y: 0, w: 10, h: 20 }, { x: 110, y: 100, w: 10, h: 20 })).toBe('M 10 10 C 60 10, 60 110, 110 110');
  });
});
