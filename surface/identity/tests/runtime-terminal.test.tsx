/** DIRECTORY-050 R7: the Sessions screen lists every running session, opens one to its live terminal, types a line, sends keys, asks before Stop by naming the agent, and names every refusal. */
import { act } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { $, $$, click, mount, settle, text } from './harness';
import { SCRIBE, SERVICE, ok, refused } from './fixtures';
import type { Route } from './fixtures';
import type { RuntimeSession } from '../src/features/runtime/RuntimeSessions';
import { byteOutput } from '../src/features/runtime/terminal-transport';
import { browser, disposed, listeners, mockTerminal, written } from './terminal-double';
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

async function submitLine(value: string) {
  const form = $('form[aria-label="Type to the session"]');
  const input = form?.querySelector<HTMLInputElement>('input[name="line"]');
  if (!form || !input) throw new Error('no line to type into');
  input.value = value;
  await act(async () => { form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await settle();
}

describe('Running sessions', () => {
  it('switches permitted sessions, disposes the old view, and does not stop either process', async () => {
    const second = 'op-' + '6'.repeat(32);
    const secondBase = '/runtime/sessions/' + second;
    const { posted } = await mount('#/runtime/' + ID, {
      ...SERVICE,
      '/runtime/live': ok({ sessions: [running, { ...running, session: second }], unanswered: [] }),
      ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }),
      ['POST ' + secondBase + '/resize']: ok({ receipt: { index: 1 } }),
      ['POST ' + base + '/read-bytes']: output(0, '', exited),
      ['POST ' + secondBase + '/read-bytes']: ok({ session: second, answer: { kind: 'bytes', output: { session: second, from: 0, cursor: 0, oldest: 0, data: [], ended: exited } }, receipt: { index: 2 } }),
    });
    await click($('a[href="#/runtime/' + second + '"]'));
    expect(disposed).toHaveBeenCalledOnce();
    expect($('a[aria-current="page"]')?.getAttribute('href')).toBe('#/runtime/' + second);
    expect(posted.some((entry) => entry.path === secondBase + '/read-bytes')).toBe(true);
    expect(posted.some((entry) => entry.path.endsWith('/end'))).toBe(false);
  });

  it('never opens a terminal that was not returned in the permitted session list', async () => {
    const { posted } = await mount('#/runtime/not-returned', { ...SERVICE, '/runtime/live': ok({ sessions: [running], unanswered: [] }) });
    expect(text()).toContain('Session not returned');
    expect(posted.some((entry) => entry.path.includes('/runtime/sessions/'))).toBe(false);
  });

  it('lists every running session the runners answered', async () => {
    const { requests } = await mount('#/runtime', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }) });
    expect(requests).toContain('/runtime/live');
    expect(text()).toContain('Dean laptop');
    expect(text()).toContain('Running');
    expect($('a[href="#/runtime/' + ID + '"]')).not.toBeNull();
  });

  it('refuses a stopped session listed as running, by name', async () => {
    await mount('#/runtime', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [{ ...running, shown: 'stopped' }], unanswered: [] }) });
    expect(text()).toContain('listed a stopped session as running');
  });

  it('names each session whose runner did not answer, and never shows it as running', async () => {
    const unanswered = [{ session: ID, machine: 'machine-one', refusal: 'runner_unreachable', reason: 'runner_unreachable: the socket is gone' }];
    await mount('#/runtime', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered }) });
    expect(text()).toContain('Runners that did not answer');
    expect(text()).toContain('runner_unreachable');
    expect(text()).toContain('Its runner did not answer; last reported running');
    expect($$('.runner-list td').some((cell) => cell.textContent?.startsWith('Running'))).toBe(false);
  });

  it('refuses a list that does not say which runners answered, by name', async () => {
    await mount('#/runtime', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running] }) });
    expect(text()).toContain('did not say which runners did not answer');
  });

  it('names an unavailable runtime', async () => {
    await mount('#/runtime', { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': refused(503, 'RuntimeUnavailable', 'No reports store') });
    expect(text()).toContain('RuntimeUnavailable');
    expect(text()).not.toContain('No running session was returned');
  });

  it('opens a session to its live output, read on from the cursor until it ends', async () => {
    const live = reads('$ echo hi\r\n\u001b[32mhi\u001b[0m\r\n');
    await mount('#/runtime/' + ID, { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }), ['POST ' + base + '/read-bytes']: live.route });
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
    await mount('#/runtime/' + ID, { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }), ['POST ' + base + '/read-bytes']: live.route });
    expect($('[role="alert"]')?.textContent ?? '').toBe('');
    expect(written.map((bytes) => new TextDecoder().decode(bytes)).join('')).toBe('$ echo hi\r\n');
    expect($('.terminal-screen')?.getAttribute('data-renderer')).toBe('webgl2');
    expect($('.terminal-renderer')?.textContent).toBe('This browser is not offering WebGPU, so the terminal is drawn with WebGL2 instead.');
  });

  it('says nothing of the renderer on WebGPU, and names the one it falls back to', async () => {
    await mount('#/runtime/' + ID, { ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }), ['POST ' + base + '/read-bytes']: reads('$ ').route });
    expect($('.terminal-screen')?.getAttribute('data-renderer')).toBe('webgpu');
    expect($('.terminal-renderer')).toBeNull();
    const changed = listeners.get('renderer');
    if (!changed) throw new Error('the terminal does not follow its renderer');
    act(() => changed({ backend: 'canvas2d', textShaping: 'browser-canvas' }));
    expect($('.terminal-screen')?.getAttribute('data-renderer')).toBe('canvas2d');
    expect($('.terminal-renderer')?.textContent).toBe('This browser is not offering WebGPU, so the terminal is drawn with Canvas 2D instead.');
  });

  it('types a line and sends keys to the session', async () => {
    const { posted } = await mount('#/runtime/' + ID, {
      ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }),
      ['POST ' + base + '/read-bytes']: readOnce('$ '),
      ['POST ' + base + '/input']: ok({ session: ID, answer: { kind: 'delivered', session: ID }, receipt: { index: 2 } }),
      ['POST ' + base + '/keys']: ok({ session: ID, answer: { kind: 'delivered', session: ID }, receipt: { index: 3 } }),
    });
    await submitLine('ls -l');
    expect(posted).toContainEqual({ path: base + '/input', body: { text: 'ls -l', enter: true } });
    await click($('button[data-key="ctrl_c"]'));
    expect(posted).toContainEqual({ path: base + '/keys', body: { keys: ['ctrl_c'] } });
    expect($$('button[data-key]').map((button) => button.getAttribute('data-key'))).toEqual(['enter', 'tab', 'escape', 'up', 'down', 'ctrl_c', 'ctrl_d']);
  });

  it('asks before Stop by naming the agent, then ends the session', async () => {
    const { posted } = await mount('#/runtime/' + ID, {
      ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }),
      ['POST ' + base + '/read-bytes']: readOnce('$ '),
      ['POST ' + base + '/end']: ok({ session: ID, answer: { kind: 'ended', session: ID, ended: { ...exited, status: null, signal: 'Killed: 9' } }, receipt: { index: 4 } }),
    });
    await click($('button[data-act="stop"]'));
    expect(posted.filter((entry) => entry.path === base + '/end')).toEqual([]);
    const dialog = $('[role="alertdialog"]');
    expect(dialog?.textContent).toContain('Stop Scribe?');
    await click($('button[data-act="confirm-stop"]'));
    expect(posted).toContainEqual({ path: base + '/end', body: {} });
    expect(text()).toContain('The process exited, Killed: 9');
    expect($('form[aria-label="Type to the session"]')).toBeNull();
  });

  it('names a refusal to type, by name', async () => {
    await mount('#/runtime/' + ID, {
      ...SERVICE, ['POST ' + base + '/resize']: ok({ receipt: { index: 0 } }), '/runtime/live': ok({ sessions: [running], unanswered: [] }),
      ['POST ' + base + '/read-bytes']: readOnce('$ '),
      ['POST ' + base + '/input']: refused(403, 'not_permitted', 'person-b does not hold operate on the agent'),
    });
    await submitLine('whoami');
    expect($('[role="alert"]')?.textContent).toBe('not_permitted: person-b does not hold operate on the agent');
  });

  it('preserves escape and invalid bytes instead of cleaning terminal output', () => {
    const data = [27, 91, 51, 49, 109, 255, 226];
    const value = { session: ID, answer: { kind: 'bytes', output: { session: ID, from: 0, cursor: data.length, oldest: 0, data, ended: null } } };
    expect(Array.from(byteOutput(value, ID, 0).data)).toEqual(data);
    expect(() => byteOutput(value, 'another-session', 0)).toThrow('another session');
    expect(() => byteOutput(value, ID, 1)).toThrow('invalid byte window');
  });
});
