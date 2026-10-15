import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { seats } from '../../api';
import { NetworkTreePane } from './NetworkTreePane';
import { serve } from '../../../tests/harness';
import { ok } from '../../../tests/fixtures';

const seat = { name: 'fixture', agent: 'agent', machine: 'machine', state: 'online-idle', transport: 'acp', session: 'fixture-session' };

it('admits the declared ACP seat transport in a seat list', async () => {
  serve({ '/seats': ok({ seats: [seat], runner: 'read' }) });
  expect((await seats.list()).seats[0].transport).toBe('acp');
});

it('shows ACP answer lines through the seat transcript', async () => {
  let reads = 0;
  serve({ 'POST /seats/fixture/attach': () => reads++ === 0 ? ok({ seat, session: seat.session, cursor: 1, ended: false, lines: [{ at: 1, kind: 'assistant', text: 'first ACP answer' }] }) : new Promise<ReturnType<typeof ok>>(() => {}) as never });
  const host = document.createElement('div');document.body.append(host);
  const root = createRoot(host);
  try {
    await act(async () => { root.render(<NetworkTreePane seat={seat as never} name="Fixture" reason={null} change={vi.fn()} />); });
    expect(host.querySelector('[role="log"]')?.textContent).toContain('first ACP answer');
    expect(host.querySelector('input')).not.toBeNull();
  } finally { await act(async () => { root.unmount(); }); }
});
