/** HOME-040: elapsed display never judges a wait, and each cancel is an Act. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it } from 'vitest';
import { WaitsList } from '../src/features/health/Health';

describe('Health waits', () => {
  it('keeps the peer, subject and start visible and cancels only on a person’s act', async () => {
    const container = document.createElement('div');
    const root = createRoot(container);
    const cancelled: string[] = [];
    await act(async () => root.render(<WaitsList waits={[
      { id: 'held', service: 'mail', peer: 'smtp.test:25', act: 'email_change', subject: 'Ada', started_at: 1 },
    ]} cancel={async (id) => { cancelled.push(id); }} />));
    expect(container.textContent).toContain('smtp.test:25');
    expect(container.textContent).toContain('Ada');
    expect(container.querySelector('time')?.getAttribute('dateTime')).toBe(new Date(1000).toISOString());
    expect(cancelled).toEqual([]);
    const cancel = container.querySelector<HTMLButtonElement>('button[aria-label="Cancel mail email_change for Ada"]');
    expect(cancel?.dataset.symbol).toBe('close');
    await act(async () => cancel?.click());
    expect(cancelled).toEqual(['held']);
    await act(async () => root.unmount());
  });
});
