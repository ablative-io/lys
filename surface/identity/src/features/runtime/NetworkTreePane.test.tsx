import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { NetworkTreePane } from './NetworkTreePane';
import { serve, type } from '../../../tests/harness';
import { ok, refused } from '../../../tests/fixtures';
import { mockTerminal, disposed } from '../../../tests/terminal-double';
import userEvent from '@testing-library/user-event';

vi.mock('@gespenst/core', () => ({ createTerminal: mockTerminal }));
const seat = (name: string, state = 'online-idle', transport = 'pty') => ({ name, agent: 'agent', state, transport, session: 'session-' + name });
let root: ReturnType<typeof createRoot> | null = null;
beforeEach(() => { root = null; });
afterEach(() => { if (root) act(() => root?.unmount()); });
async function render(name: string, state = 'online-idle', transport = 'pty') {
  if (!root) { const host = document.createElement('div'); document.body.append(host); root = createRoot(host); }
  await act(async () => { root?.render(<NetworkTreePane key={name} seat={seat(name, state, transport) as never} name={name} reason={null} change={vi.fn()} />); });
}
it('C738 switches PTY terminals and aborts the previous stream exactly once', async () => {
  const aborted = vi.fn();
  vi.stubGlobal('fetch', async (path: string, init: RequestInit) => {
    if (path.endsWith('/resize')) return new Response(JSON.stringify({ receipt: { index: 1 } }));
    init.signal?.addEventListener('abort', aborted, { once: true });
    return new Promise<Response>(() => {});
  });
  await render('first');
  expect(document.querySelector('[data-session="session-first"]')).not.toBeNull();
  await render('second');
  expect(aborted).toHaveBeenCalledTimes(1);
  expect(disposed).toHaveBeenCalledTimes(1);
  expect(document.querySelector('[data-session="session-second"]')).not.toBeNull();
});
it('C738 sends a user turn on Enter, clears only on delivery and names refusals', async () => {
  const posted: { path: string; body: unknown }[] = [];
  serve({ 'POST /seats/first/send': ok({ seat: seat('first'), session: 'session-first', delivered: true }) }, posted);
  await render('first', 'online-idle', 'managed');
  await type(document.querySelector('input'), 'hello');
  await act(async () => { await userEvent.setup().click(document.querySelector('input') as HTMLInputElement); await userEvent.setup().keyboard('{Enter}'); });
  expect(posted.find((each) => each.path === '/seats/first/send')?.body).toEqual({ operation: expect.any(String), text: 'hello' });
  expect((document.querySelector('input') as HTMLInputElement).value).toBe('');
  serve({ 'POST /seats/first/send': refused(409, 'seat_not_running', 'ended') });
  await type(document.querySelector('input'), 'keep this');
  await act(async () => { document.querySelector('form')?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  expect(document.body.textContent).toContain('seat_not_running');
  expect((document.querySelector('input') as HTMLInputElement).value).toBe('keep this');
});
it('C738 preserves a new draft written while the earlier user turn is being delivered', async () => {
  let delivered: ((answer: ReturnType<typeof ok>) => void) | undefined;
  serve({ 'POST /seats/first/send': () => new Promise<ReturnType<typeof ok>>((resolve) => { delivered = resolve; }) as never });
  await render('first', 'online-idle', 'managed');
  await type(document.querySelector('input'), 'first turn');
  await act(async () => { document.querySelector('form')?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })); });
  await type(document.querySelector('input'), 'next turn');
  await act(async () => { delivered?.(ok({ seat: seat('first'), session: 'session-first', delivered: true })); });
  expect((document.querySelector('input') as HTMLInputElement).value).toBe('next turn');
});
it('C738 stops through the seat route and shows confirmed end without a terminal', async () => {
  const posted: { path: string; body: unknown }[] = [];
  serve({ 'POST /seats/first/stop': ok({ seat: seat('first', 'offline'), session: 'session-first', ended: true }) }, posted);
  await render('first', 'online-idle', 'managed');
  await act(async () => { (document.querySelector('button[aria-label="Stop first"]') as HTMLElement).click(); });
  // Stop asks first; nothing is sent until the question is answered.
  expect(document.querySelector('[role="alertdialog"]')?.textContent).toContain('What it has not saved is lost.');
  await act(async () => { (document.querySelector('button[data-act="confirm-stop"]') as HTMLElement).click(); });
  expect(posted.find((each) => each.path === '/seats/first/stop')?.body).toEqual({ operation: expect.any(String), force: false });
  expect(document.querySelector('[role="status"]:not(.network-reason):not(.terminal-head [role="status"])')?.textContent).toBe('Stopped.');
  expect(document.querySelector('.terminal-screen')).toBeNull();
});
it('C738 offline and unknown leaves show their state, reason and Start without mounting a terminal', async () => {
  await render('first', 'offline');
  expect(document.querySelector('button[aria-label="Start first"]')).not.toBeNull();
  // The pane says what to do, not just a bare button.
  expect(document.querySelector('.network-start .note')?.textContent).toContain('is not running. Start it to watch its terminal here.');
  expect(document.querySelector('.terminal-head [role="status"]')?.textContent).toBe('Offline');
  expect(document.querySelector('.terminal-screen')).toBeNull();
  await act(async () => { root?.render(<NetworkTreePane key="unknown" seat={seat('unknown', 'unknown') as never} name="unknown" reason="runner_unreachable: socket absent" change={vi.fn()} />); });
  // The reason is said in plain words; the service's own words, refusal name and all, stay in its title.
  expect(document.querySelector('.network-reason')?.textContent).toBe('Lys could not reach the chosen computer’s runner. Check that the computer and its runner are connected.');
  expect(document.querySelector('.network-reason')?.getAttribute('title')).toBe('runner_unreachable: socket absent');
  expect(document.querySelector('.terminal-head [role="status"]')?.textContent).toBe('Unknown: its runner could not be read');
  expect(document.querySelector('.terminal-screen')).toBeNull();
  // Whether an unknown seat runs was not read: Start is offered, and the service refuses it if the seat is running.
  expect(document.querySelector('button[aria-label="Start unknown"]')).not.toBeNull();
});
it('C738 follows managed attach lines, preserves the cursor, and names the stream end', async () => {
  const posted: { path: string; body: unknown }[] = [];
  let reads = 0;
  serve({ 'POST /seats/first/attach': () => ok({ seat: seat('first'), session: 'session-first', lines: reads++ === 0 ? [{ at: 1, kind: 'assistant', text: 'answer' }] : [], cursor: 1, ended: reads === 2 }) }, posted);
  await render('first', 'online-idle', 'managed');
  expect(document.body.textContent).toContain('answer');
  expect(document.body.textContent).toContain('Session session-first ended');
  expect(posted.filter((each) => each.path === '/seats/first/attach').map((each) => each.body)).toEqual([{ follow: true }, { cursor: 1, follow: true }]);
  expect(document.querySelector('.terminal-screen')).toBeNull();
});
it('C738 swaps managed streams, aborts the old read once and refuses lines from another session', async () => {
  const aborted = vi.fn();
  vi.stubGlobal('fetch', async (_path: string, init: RequestInit) => {
    init.signal?.addEventListener('abort', aborted, { once: true });
    return new Promise<Response>(() => {});
  });
  await render('first', 'online-idle', 'managed');
  await render('second', 'online-idle', 'managed');
  expect(aborted).toHaveBeenCalledTimes(1);
  serve({ 'POST /seats/third/attach': ok({ seat: seat('third'), session: 'wrong-session', cursor: 1, ended: true, lines: [{ at: 1, kind: 'assistant', text: 'wrong output' }] }) });
  await render('third', 'online-idle', 'managed');
  expect(document.body.textContent).toContain('Lys answered, but the agent\'s output could not be read. Choose the seat again to reconnect.');
  expect(document.body.textContent).not.toContain('wrong output');
});
it('C738 keeps the terminal while stop is unconfirmed and never invents a missing transport', async () => {
  serve({ 'POST /seats/first/stop': ok({ seat: seat('first', 'online-idle', 'managed'), session: 'session-first', ended: false }) });
  await render('first', 'online-idle', 'managed');
  await act(async () => { (document.querySelector('button[aria-label="Stop first"]') as HTMLElement).click(); });
  // Stop asks first; nothing is sent until the question is answered.
  expect(document.querySelector('[role="alertdialog"]')?.textContent).toContain('What it has not saved is lost.');
  await act(async () => { (document.querySelector('button[data-act="confirm-stop"]') as HTMLElement).click(); });
  expect(document.body.textContent).toContain('Asked to stop. Its computer has not yet confirmed it stopped.');
  expect(document.querySelector('[role="log"]')).not.toBeNull();
  await render('second', 'online-idle', '');
  expect(document.body.textContent).toContain('This seat is running, but Lys does not know how to show its terminal.');
  expect(document.body.textContent).not.toContain('SeatTransportUnknown');
  expect(document.querySelector('.terminal-screen')).toBeNull();
  expect(document.querySelector('[role="log"]')).toBeNull();
});

it('offers Start again beside Try again when a known conversation is missing and sends a fresh restart', async () => {
  const posted: { path: string; body: unknown }[] = [];
  serve({
    'POST /seats/first/start': refused(409, 'pty_resume_missing', 'the earlier conversation transcript is gone; choose Start again'),
    'POST /seats/first/restart': ok({ seat: seat('first'), session: 'session-first' }),
  }, posted);
  await render('first', 'offline');
  await act(async () => { (document.querySelector('button[aria-label="Start first"]') as HTMLElement).click(); });
  expect(document.querySelector('button[aria-label="Start again"]')).not.toBeNull();
  expect(document.querySelector('button[aria-label="Try again"]')).not.toBeNull();
  await act(async () => { (document.querySelector('button[aria-label="Start again"]') as HTMLElement).click(); });
  expect(posted.filter((entry) => entry.path.endsWith('/restart'))).toMatchObject([{ path: '/seats/first/restart', body: { force: false, operation: expect.stringMatching(/^op-[0-9a-f]{32}$/) } }]);
});
