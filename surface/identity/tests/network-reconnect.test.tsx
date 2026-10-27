/** A computer's owner can end its runner's reconnect wait explicitly. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { MachineDetail } from '../src/features/network/MachineDetail';
import type { Machine } from '../src/features/network/contract';
let root: ReturnType<typeof createRoot> | null = null;
afterEach(() => { if (root) act(() => root?.unmount()); root = null; vi.resetAllMocks(); vi.useRealTimers(); document.body.replaceChildren(); });
const machine: Machine = { id: 'own', name: 'Own laptop', kind: 'computer', runtime: 'lys-runner', slots: 0,
  may_run: [], may_reach: [], named_by: 'person', named_at: 1, state: 'in_use', retired_at: null, last_report_at: null };

it('shows the named refusal schedule and Reconnect asks Lys to end the runner wait', async () => {
  const lost = new Date(2026, 9, 16, 3, 52, 10).getTime();
  const now = lost + 8000;
  // Fake timers first: they put back the real Date the shared setup pins,
  // and this test drives Date.now itself.
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
  const clock = vi.spyOn(Date, 'now').mockReturnValue(now);
  vi.stubGlobal('IntersectionObserver', class {
    constructor(private readonly observed: IntersectionObserverCallback) {}
    observe(target: Element) { this.observed([{ target, isIntersecting: true } as IntersectionObserverEntry], this as unknown as IntersectionObserver); }
    disconnect() {}
  });
  const words = 'Lys refused at ' + lost + ', next dial at ' + (now + 12000) + ', attempt 3';
  const retry = { refused_at: lost, next_dial: now + 12000, attempt: 3, reason: 'The service closed the connection.' };
  const posted: string[] = [];
  vi.stubGlobal('fetch', async (input: string, init?: RequestInit) => {
    if (init?.method === 'POST') posted.push(input);
    if (input.includes('/changes')) return new Promise<Response>((resolve, reject) => {
      void resolve;
      init?.signal?.addEventListener('abort', () => reject(new DOMException('left', 'AbortError')), { once: true });
    });
    const answer = input.endsWith('/completions') ? { machine: 'own', tickets: [] }
      : input.endsWith('/reconnect') ? { reconnecting: true }
      : input.includes('/people') ? { people: [{ id: 'person', agents: [] }] }
      : { state: 'refused', words, retry };
    return new Response(JSON.stringify(answer), { headers: { 'content-type': 'application/json' } });
  });
  const container = document.createElement('div'); document.body.append(container); root = createRoot(container);
  await act(async () => root?.render(<MachineDetail computer={{ machine, asked: 'answered', runner: { kind: 'dialled', key: 'public', runner: 'runner' },
    answers: false, reason: null, refusal: null, joining: false, running: [] }} admin={false} me="person" teams={[]} names={new Map()}
    changed={() => {}} connecting={null} connect={() => {}} done={() => {}} />));
  expect(container.textContent).toContain('Lost connection to Lys at 3:52:10 am. Next try in 12 s (attempt 3).');
  expect(container.textContent).toContain('The service closed the connection.');
  expect(container.textContent).not.toContain(words);
  clock.mockReturnValue(now + 1000);
  act(() => vi.advanceTimersByTime(1000));
  expect(container.textContent).toContain('Next try in 11 s (attempt 3).');
  expect(posted).toEqual([]);
  const button = container.querySelector('button[aria-label="Reconnect Own laptop"]');
  expect(button).not.toBeNull();
  await act(async () => button?.dispatchEvent(new MouseEvent('click', { bubbles: true })));
  expect(posted).toContain('/api/network/machines/own/reconnect');
  expect(container.textContent).toContain('Reconnecting');
});
