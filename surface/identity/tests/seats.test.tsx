/** The seats (AGENTS-002 R2, R6): every state named, unknown never shown as offline or zero, a send delivered or refused by name, and a forced stop offered only on seat_turn_in_progress. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import { afterEach, describe, expect, it } from 'vitest';
import { Seats } from '../src/features/sessions/Seats';
import type { SeatList, SeatView, UnregisteredSession } from '../src/api';
import { $, click, serve, settle, text, type } from './harness';
import { ok, refused } from './fixtures';
import type { Route } from './fixtures';

let root: Root | null = null;
let host: HTMLElement | null = null;
afterEach(() => { if (root) act(() => root?.unmount()); root = null; host?.remove(); host = null; });

const seat = (name: string, more: Partial<SeatView> = {}): SeatView => ({
  name, agent: 'agent-' + name, harness: 'claude', profile_version: 3, machine: 'machine-one', working_folder: '/work/' + name,
  responsible: 'person-one', session: 'session-' + name, harness_session: null, state: 'online-idle', last_signal_at: 1790000000000,
  created_by: 'person-one', created_at: 1790000000, revision: 1, ...more,
});
const read = (seats: SeatView[]): SeatList => ({ seats, runner: 'read' });
const stray: UnregisteredSession = { session: 'session-stray', machine: 'machine-one', agent: null, pid: 4242, started_at: 1790000000000 };

/** The seats alone, over a stubbed service. */
async function shown(routes: Record<string, Route>) {
  const posted: { path: string; body: unknown }[] = [];
  serve({ '/seats/unregistered': ok({ sessions: [] }), ...routes }, posted);
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
  await act(async () => { root?.render(<Seats />); });
  await settle();
  return { posted };
}

describe('Seats', () => {
  it('names every state with its last signal, and lists a session held for no seat as seen but unregistered', async () => {
    await shown({
      '/seats': ok(read([
        seat('waffles', { state: 'online-working' }), seat('archie'), seat('daisy', { state: 'offline' }),
        seat('gaia', { state: 'not-seen', session: null, last_signal_at: null }),
      ])),
      '/seats/unregistered': ok({ sessions: [stray] }),
    });
    expect($('tr[data-seat="waffles"]')?.textContent).toContain('Online, working');
    expect($('tr[data-seat="archie"]')?.textContent).toContain('Online, idle');
    expect($('tr[data-seat="daisy"]')?.textContent).toContain('Offline');
    expect($('tr[data-seat="gaia"]')?.textContent).toContain('Not seen');
    expect($('tr[data-seat="gaia"]')?.textContent).toContain('No signal seen');
    expect($('tr[data-seat="archie"]')?.textContent).toContain('Last signal');
    expect(text()).toContain('Seen but unregistered');
    expect($('tr[data-unregistered="session-stray"]')?.textContent).toContain('4242');
    expect(text()).not.toContain('Runner unknown');
  });

  it('names a runner that could not be read, and shows its seats unknown, never offline or zero', async () => {
    await shown({ '/seats': ok({ seats: [seat('waffles', { state: 'unknown', last_signal_at: null })], runner: 'unknown', reason: 'runner_unreachable: the runner on machine-one did not answer' }) });
    expect(text()).toContain('Runner unknown');
    expect(text()).toContain('runner_unreachable: the runner on machine-one did not answer');
    expect($('tr[data-seat="waffles"]')?.textContent).toContain('Unknown: its runner could not be read');
    expect($('tr[data-seat="waffles"]')?.textContent).not.toContain('Offline');
    expect($('form[aria-label="Send to waffles"]')).toBeNull();
  });

  it('refuses an answer with a state it does not know, by name', async () => {
    await shown({ '/seats': ok(read([seat('waffles', { state: 'asleep' as unknown as SeatView['state'] })])) });
    expect(text()).toContain('SeatsUnreadable');
    expect($('tr[data-seat="waffles"]')).toBeNull();
  });

  it('posts a send with its text and an operation id, and shows it delivered', async () => {
    const { posted } = await shown({
      '/seats': ok(read([seat('waffles')])),
      'POST /seats/waffles/send': () => ok({ seat: seat('waffles'), session: 'session-waffles', delivered: true }),
    });
    await type($('input[aria-label="Message for waffles"]'), 'Read the brief on main.');
    await settle();
    await click($('button[aria-label="Send to waffles"]'));
    const sent = posted.find((entry) => entry.path === '/seats/waffles/send');
    expect(sent?.body).toMatchObject({ text: 'Read the brief on main.' });
    expect((sent?.body as { operation: string }).operation).toMatch(/^op-[0-9a-f]{32}$/);
    expect($('tr[data-seat="waffles"]')?.textContent).toContain('Delivered to session session-waffles as a user turn.');
  });

  it('shows a refused send by its name and words, and offers no send box to a seat that is not running', async () => {
    await shown({
      '/seats': ok(read([seat('waffles'), seat('daisy', { state: 'offline' })])),
      'POST /seats/waffles/send': () => refused(409, 'seat_not_running', 'the seat waffles has no running session'),
    });
    expect($('form[aria-label="Send to daisy"]')).toBeNull();
    await type($('input[aria-label="Message for waffles"]'), 'hello');
    await settle();
    await click($('button[aria-label="Send to waffles"]'));
    const alert = $('tr[data-seat="waffles"] [role="alert"]')?.textContent ?? '';
    expect(alert).toContain('seat_not_running');
    expect(alert).toContain('the seat waffles has no running session');
    expect(text()).not.toContain('Delivered');
  });

  it('offers a forced stop only when the stop is refused seat_turn_in_progress, under a new operation', async () => {
    let asked = 0;
    const { posted } = await shown({
      '/seats': ok(read([seat('waffles', { state: 'online-working' })])),
      'POST /seats/waffles/stop': (body) => {
        asked += 1;
        return (body as { force: boolean }).force
          ? ok({ seat: seat('waffles', { state: 'offline' }), session: 'session-waffles', ended: true })
          : refused(409, 'seat_turn_in_progress', 'the seat waffles is in a turn; stop with force to end it');
      },
    });
    expect($('button[aria-label="Stop waffles anyway"]')).toBeNull();
    await click($('button[aria-label="Stop waffles"]'));
    expect($('tr[data-seat="waffles"] [role="alert"]')?.textContent).toContain('seat_turn_in_progress');
    await click($('button[aria-label="Stop waffles anyway"]'));
    const stops = posted.filter((entry) => entry.path === '/seats/waffles/stop').map((entry) => entry.body as { operation: string; force: boolean });
    expect(asked).toBe(2);
    expect(stops.map((entry) => entry.force)).toEqual([false, true]);
    expect(stops[0].operation).not.toBe(stops[1].operation);
    expect($('tr[data-seat="waffles"]')?.textContent).toContain('Session session-waffles ended.');
  });

  it('does not offer a forced stop for any other refusal', async () => {
    await shown({
      '/seats': ok(read([seat('waffles')])),
      'POST /seats/waffles/stop': () => refused(403, 'NotAdmitted', 'seat.stop is not granted for /api/seats/waffles/stop'),
    });
    await click($('button[aria-label="Stop waffles"]'));
    expect(text()).toContain('NotAdmitted');
    expect($('button[aria-label="Stop waffles anyway"]')).toBeNull();
  });

  it('starts a seat with a fresh operation id', async () => {
    const { posted } = await shown({
      '/seats': ok(read([seat('gaia', { state: 'not-seen', session: null, last_signal_at: null })])),
      'POST /seats/gaia/start': () => ok({ seat: seat('gaia'), session: 'session-gaia' }),
    });
    await click($('button[aria-label="Start gaia"]'));
    expect((posted.find((entry) => entry.path === '/seats/gaia/start')?.body as { operation: string }).operation).toMatch(/^op-[0-9a-f]{32}$/);
    expect(text()).toContain('Started as session session-gaia.');
  });
});
