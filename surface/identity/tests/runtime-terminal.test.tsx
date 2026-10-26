/** DIRECTORY-050 R7: the Running page lists every running session beside the canvas, opens one to its live terminal there, types a line, sends keys, asks before Stop by naming the agent, and names every refusal. */
import { act } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, text, settle } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { RuntimeSession } from '../src/features/runtime/RuntimeSessions';
import { byteOutput } from '../src/features/runtime/terminal-transport';
import { browser, cell, listeners, mockTerminal, ownCount, sent, sized, written } from './terminal-double';
import { MAC_KEYS } from '../src/features/runtime/GpuTerminal';
vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));

const ID = 'op-' + '5'.repeat(32);
const base = '/runtime/sessions/' + ID;
const running: RuntimeSession = { session: ID, agent: SCRIBE, machine: 'machine-one', machine_name: 'Dean laptop', runtime: 'sh', shown: 'running', last_reported: 'running', first_report_at: 1790000000, last_report_at: 1790000001, what: 'the runner started process 4242', stopped: null, reported_by: 'the runner of machine machine-one' };
const output = (from: number, text: string, ended: unknown = null) => ok({ session: ID, answer: { kind: 'bytes', output: { session: ID, from, cursor: from + new TextEncoder().encode(text).length, oldest: 0, data: Array.from(new TextEncoder().encode(text)), ended } }, receipt: { index: 1 } });
const exited = { how: 'exited', at: 1790000100000, status: 0, signal: null };

/** A read that answers the session's output once, then its end, so the live read stops. */
function reads(first: string): { route: Route; asked: unknown[] } {
  const asked: unknown[] = [];
  return {
    asked,
    route: (body) => {
      asked.push(body);
      return asked.length === 1 ? output(0, first) : output(first.length, '', exited);
    },
  };
}

/** A read that answers once and is then refused, as a read closed under it is, so the live read stops while the session stays open. */
function readOnce(first: string): Route {
  let asked = 0;
  return () => {
    asked += 1;
    return asked === 1 ? output(0, first) : refused(502, 'runner_unreachable', 'the read was closed');
  };
}

describe('Running sessions', () => {
  it('lists two permitted sessions, each opening on the canvas, and stops neither process by looking', async () => {
    const second = 'op-' + '6'.repeat(32);
    const secondBase = '/runtime/sessions/' + second;
    const { posted } = await mount('#/canvas?view=canvas', {
      ...SERVICE,
      '/runtime/live': ok({ sessions: [running, { ...running, session: second }], unanswered: [] }),
      ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }),
      ['POST ' + secondBase + '/resize']: ok({ receipt: { index: 1 } }),
      ['POST ' + base + '/read-bytes']: output(0, '', exited),
      ['POST ' + secondBase + '/read-bytes']: ok({ session: second, answer: { kind: 'bytes', output: { session: second, from: 0, cursor: 0, oldest: 0, data: [], ended: exited } }, receipt: { index: 2 } }),
    });
    expect($$('.running-list .running-chip a[href="#/canvas/' + SCRIBE + '"]')).toHaveLength(2);
    expect(posted.some((entry) => entry.path.endsWith('/end'))).toBe(false);
  });

  it('sends a Mac\'s Command and Option editing keys as the bytes a line editor acts on, and leaves every other key to the terminal', async () => {
    // The session's first output is answered and the next read waits, as a live read does while nothing is printed.
    let asked = 0;
    const live = (() => { asked += 1; return asked === 1 ? output(0, '$ ') : new Promise(() => {}); }) as unknown as Route;
    await mount('#/canvas/' + SCRIBE, { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }), ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), ['POST ' + base + '/read-bytes']: live });
    const screen = $('.terminal-screen');
    expect(screen).not.toBeNull();
    // The terminal takes keys once it has opened.
    for (let turn = 0; turn < 20 && screen?.getAttribute('data-renderer') === 'starting'; turn++) await settle();
    expect(screen?.getAttribute('data-renderer')).toBe('webgpu');
    const press = (key: string, more: KeyboardEventInit) => { const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...more }); screen?.dispatchEvent(event); return event.defaultPrevented; };
    expect(press('Backspace', { metaKey: true })).toBe(true);
    expect(press('Delete', { metaKey: true })).toBe(true);
    expect(press('ArrowLeft', { metaKey: true })).toBe(true);
    expect(press('ArrowRight', { metaKey: true })).toBe(true);
    expect(press('Backspace', { altKey: true })).toBe(true);
    expect(press('Delete', { altKey: true })).toBe(true);
    expect(press('ArrowLeft', { altKey: true })).toBe(true);
    expect(press('ArrowRight', { altKey: true })).toBe(true);
    expect(sent).toEqual(['\x15', '\x0b', '\x01', '\x05', '\x1b\x7f', '\x1bd', '\x1bb', '\x1bf']);
    expect(Object.keys(MAC_KEYS)).toHaveLength(8);
    // A plain key, a Control key and a Command letter are the terminal's own: nothing is sent from here and nothing is held back.
    expect(press('Backspace', {})).toBe(false);
    expect(press('ArrowLeft', { ctrlKey: true, altKey: true })).toBe(false);
    expect(press('c', { metaKey: true })).toBe(false);
    expect(sent).toHaveLength(8);
  });

  it('gives a terminal as many columns as its window has room for, counted from the window and not from how the canvas is zoomed', async () => {
    // A window 800 by 510 on a screen of two device pixels to one, in a font whose cell is 16 by 34 device pixels: 100 columns, 30 rows.
    cell.cellWidthPx = 16; cell.cellHeightPx = 34;
    vi.stubGlobal('devicePixelRatio', 2);
    const width = vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(function (this: HTMLElement) { return this.classList.contains('terminal-screen') ? 800 : 0; });
    const height = vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockImplementation(function (this: HTMLElement) { return this.classList.contains('terminal-screen') ? 510 : 0; });
    // The canvas is zoomed out to a half: the window is drawn 400 wide. That is not what its columns are counted from.
    const drawn = vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({ x: 0, y: 0, left: 0, top: 0, right: 400, bottom: 255, width: 400, height: 255, toJSON: () => ({}) });
    let asked = 0;
    const live = (() => { asked += 1; return asked === 1 ? output(0, '$ ') : new Promise(() => {}); }) as unknown as Route;
    const { posted } = await mount('#/canvas/' + SCRIBE, { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }), ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), ['POST ' + base + '/read-bytes']: live });
    for (let turn = 0; turn < 20 && $('.terminal-screen')?.getAttribute('data-renderer') === 'starting'; turn++) await settle();
    expect(sized).toEqual([[100, 30]]);
    expect(ownCount.disconnect).toHaveBeenCalledTimes(1);
    const sizes = posted.filter((entry) => entry.path === base + '/resize').map((entry) => entry.body);
    expect(sizes.at(-1)).toEqual({ columns: 100, rows: 30 });
    width.mockRestore(); height.mockRestore(); drawn.mockRestore();
    vi.unstubAllGlobals();
  });

  it('never opens a terminal that was not returned in the permitted session list', async () => {
    const { posted } = await mount('#/canvas/agent-' + 'e'.repeat(32), { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }) });
    expect($('.terminal')).toBeNull();
    expect($('.running-list a[aria-current="page"]')).toBeNull();
    expect(posted.some((entry) => entry.path.includes('/runtime/sessions/'))).toBe(false);
  });

  it('lists every running session the runners answered', async () => {
    const { requests } = await mount('#/canvas?view=canvas', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }) });
    expect(requests).toContain('/runtime/live');
    expect(text()).toContain('Dean laptop');
    expect(text()).toContain('Running');
    expect($('.running-list a[href="#/canvas/' + SCRIBE + '"]')?.textContent).toBe('Scribe');
    expect($('.running-list')?.textContent).toContain('Dean laptop');
  });

  it('refuses a stopped session listed as running, by name', async () => {
    await mount('#/canvas?view=sessions', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [{ ...running, shown: 'stopped' }], unanswered: [] }) });
    expect(text()).toContain('listed a stopped session as running');
  });

  it('names each session whose runner did not answer, and never shows it as running', async () => {
    const unanswered = [{ session: ID, machine: 'machine-one', refusal: 'runner_unreachable', reason: 'runner_unreachable: the socket is gone' }];
    await mount('#/canvas?view=canvas', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered }) });
    expect(text()).toContain('Lys could not reach the chosen computer’s runner. Check that the computer and its runner are connected.');
    expect($('.session-canvas-strip [title^="runner_unreachable"]')).not.toBeNull();
    expect(text()).toContain('Its computer did not answer, so whether it is still running is not known.');
    expect($$('.running-list .running-state')).toHaveLength(1);
    expect($$('.running-list .running-state').some((cell) => cell.textContent?.startsWith('Running'))).toBe(false);
  });

  it('refuses a list that does not say which runners answered, by name', async () => {
    await mount('#/canvas?view=sessions', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running] }) });
    expect(text()).toContain('did not say which runners did not answer');
  });

  it('names an unavailable runtime', async () => {
    await mount('#/canvas?view=canvas', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': refused(503, 'RuntimeUnavailable', 'No reports store') });
    expect(text()).toContain('RuntimeUnavailable');
    expect(text()).not.toContain('Nothing is running.');
  });

  it('opens a session to its live output, read on from the cursor until it ends', async () => {
    const live = reads('$ echo hi\r\n\u001b[32mhi\u001b[0m\r\n');
    await mount('#/canvas/' + SCRIBE, { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }), ['POST ' + base + '/read-bytes']: live.route });
    expect($('[role="alert"]')?.textContent ?? '').toBe('');
    expect(live.asked[0]).toEqual({ cursor: null, follow: true });
    expect(live.asked[1]).toEqual({ cursor: '$ echo hi\r\n\u001b[32mhi\u001b[0m\r\n'.length, follow: true });
    expect(live.asked).toHaveLength(2);
    expect(written.map((bytes) => new TextDecoder().decode(bytes)).join('')).toBe('$ echo hi\r\n\u001b[32mhi\u001b[0m\r\n');
    expect(text()).toContain('The process exited, status 0');
  });

  it('shows the session in a browser without WebGPU and says what draws it instead', async () => {
    browser.webgpu = false;
    const live = reads('$ echo hi\r\n');
    await mount('#/canvas/' + SCRIBE, { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }), ['POST ' + base + '/read-bytes']: live.route });
    expect($('[role="alert"]')?.textContent ?? '').toBe('');
    expect(written.map((bytes) => new TextDecoder().decode(bytes)).join('')).toBe('$ echo hi\r\n');
    expect($('.terminal-screen')?.getAttribute('data-renderer')).toBe('webgl2');
    expect($('.terminal-renderer')?.textContent).toBe('This browser is not offering WebGPU, so the terminal is drawn with WebGL2 instead.');
  });

  it('says nothing of the renderer on WebGPU, and names the one it falls back to', async () => {
    await mount('#/canvas/' + SCRIBE, { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }), ['POST ' + base + '/read-bytes']: reads('$ ').route });
    expect($('.terminal-screen')?.getAttribute('data-renderer')).toBe('webgpu');
    expect($('.terminal-renderer')).toBeNull();
    const changed = listeners.get('renderer');
    if (!changed) throw new Error('the terminal does not follow its renderer');
    act(() => changed({ backend: 'canvas2d', textShaping: 'browser-canvas' }));
    expect($('.terminal-screen')?.getAttribute('data-renderer')).toBe('canvas2d');
    expect($('.terminal-renderer')?.textContent).toBe('This browser is not offering WebGPU, so the terminal is drawn with Canvas 2D instead.');
  });

  it('keeps direct typing and sends each terminal key button to its session', async () => {
    const { posted } = await mount('#/canvas/' + SCRIBE, {
      ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }),
      ['POST ' + base + '/read-bytes']: readOnce('$ '),
      ['POST ' + base + '/keys']: ok({ receipt: { index: 3 } }),
    });
    expect($('.terminal')).not.toBeNull();
    expect($('.terminal form')).toBeNull();
    expect($('.terminal input')).toBeNull();
    // Each name is one the runner publishes for its keys act (crates/lys-runner/src/published.rs, "keys").
    const keys = [['Escape', 'escape'], ['Enter', 'enter'], ['Up', 'up'], ['Down', 'down'], ['Left', 'left'], ['Right', 'right'], ['Line start', 'home'], ['Line end', 'end'], ['Tab', 'tab'], ['Ctrl-C', 'ctrl_c']];
    expect($$('button[data-key]').map((button) => button.textContent)).toEqual(keys.map(([label]) => label));
    expect($('.terminal-head')?.textContent).toContain('Scribe');
    expect($('.terminal-head')?.textContent).toContain('on Dean laptop');
    expect($('.terminal-head')?.textContent).not.toContain(ID);
    expect(posted.filter((entry) => entry.path.endsWith('/input') || entry.path.endsWith('/keys'))).toEqual([]);
    for (const [label, key] of keys) {
      await click($$('button[data-key]').find((button) => button.textContent === label) ?? null);
      expect(posted.at(-1)).toEqual({ path: base + '/keys', body: { keys: [key] } });
    }
    expect(posted.filter((entry) => entry.path === base + '/keys')).toHaveLength(10);
  });


  it('confirms Restart, sends the agent and session once, and cancels without a request', async () => {
    const { posted } = await mount('#/canvas/' + SCRIBE, {
      ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }),
      '/runtime/live': ok({ sessions: [running], unanswered: [] }),
      ['POST ' + base + '/read-bytes']: readOnce('$ '),
      ['POST /agents/' + SCRIBE + '/restart']: ok({ session: 'op-' + '9'.repeat(32) }),
    });
    await click($('button[data-act="restart"]'));
    expect($('[role="alertdialog"]')?.textContent).toContain('Restart Scribe?');
    expect(posted.some((entry) => entry.path.endsWith('/restart'))).toBe(false);
    await click($('button[data-act="keep-running"]'));
    expect($('[role="alertdialog"]')).toBeNull();
    await click($('button[data-act="restart"]'));
    await click($('button[data-act="confirm-restart"]'));
    const restarts = posted.filter((entry) => entry.path === '/agents/' + SCRIBE + '/restart');
    expect(restarts).toHaveLength(1);
    expect(restarts[0].body).toEqual({ session: ID, operation: expect.stringMatching(/^op-[0-9a-f]{32}$/) });
  });

  it('names a refused terminal key and a refused restart without claiming either succeeded', async () => {
    const { posted } = await mount('#/canvas/' + SCRIBE, {
      ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }),
      '/runtime/live': ok({ sessions: [running], unanswered: [] }),
      ['POST ' + base + '/read-bytes']: readOnce('$ '),
      ['POST ' + base + '/keys']: refused(403, 'NotAdmitted', 'keys are not granted'),
      ['POST /agents/' + SCRIBE + '/restart']: refused(409, 'conversation_held', 'another run holds it'),
    });
    await click($('button[data-key="ctrl_c"]'));
    expect(text()).toContain('NotAdmitted: keys are not granted');
    expect($('.terminal [role="alert"] p')?.textContent).toBe('You do not have permission to do this. Ask the directory administrator to make the change.');
    await click($('button[data-act="restart"]'));
    await click($('button[data-act="confirm-restart"]'));
    expect(text()).toContain('conversation_held: another run holds it');
    expect($('.terminal [role="alert"] p')?.textContent).not.toContain('conversation_held');
    expect($('[role="alertdialog"]')).not.toBeNull();
    expect(posted.filter((entry) => entry.path.endsWith('/restart'))).toHaveLength(1);
  });

  it('gives an unrecognised terminal refusal its server reason and keeps the code in details', async () => {
    await mount('#/canvas/' + SCRIBE, {
      ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }),
      '/runtime/live': ok({ sessions: [running], unanswered: [] }),
      ['POST ' + base + '/read-bytes']: readOnce('$ '),
      ['POST ' + base + '/keys']: refused(409, 'NewTerminalRefusal', 'the terminal cannot accept this key'),
    });
    await click($('button[data-key="ctrl_c"]'));
    expect($('.terminal [role="alert"] p')?.textContent).toBe('Lys refused: the terminal cannot accept this key');
    expect($('.terminal [role="alert"]')?.textContent).toContain('NewTerminalRefusal');
  });

  it('asks before Stop by naming the agent, then ends the session', async () => {
    const { posted } = await mount('#/canvas/' + SCRIBE, {
      ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }),
      ['POST ' + base + '/read-bytes']: readOnce('$ '),
      ['POST ' + base + '/stop']: ok({ session: ID, answer: { kind: 'ended', session: ID, ended: { ...exited, status: null, signal: 'Killed: 9' } }, receipt: { index: 4 } }),
    });
    await click($('button[data-act="stop"]'));
    expect(posted.filter((entry) => entry.path === base + '/stop')).toEqual([]);
    const dialog = $('[role="alertdialog"]');
    expect(dialog?.textContent).toContain('Stop Scribe?');
    await click($('button[data-act="confirm-stop"]'));
    expect(posted).toContainEqual({ path: base + '/stop', body: {} });
    expect(text()).toContain('The process exited, Killed: 9');
  });

  it('preserves escape and invalid bytes instead of cleaning terminal output', () => {
    const data = [27, 91, 51, 49, 109, 255, 226];
    const value = { session: ID, answer: { kind: 'bytes', output: { session: ID, from: 0, cursor: data.length, oldest: 0, data, ended: null } } };
    expect(Array.from(byteOutput(value, ID, 0).data)).toEqual(data);
    expect(() => byteOutput(value, 'another-session', 0)).toThrow('another session');
    expect(() => byteOutput(value, ID, 1)).toThrow('invalid byte window');
  });
});
