/** A person's End now is the event that escalates a waiting stop. */
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, expect, it } from 'vitest';
import { StopWaiting } from '../src/features/run/StopWaiting';

describe('A waiting stop', () => {
  it('shows its start and sends End now only when pressed', async () => {
    const container = document.createElement('div');
    const root = createRoot(container);
    let asked = 0;
    await act(async () => root.render(<StopWaiting since={1000} ending={false} endNow={() => { asked += 1; }} />));
    expect(container.textContent).toContain('Waiting for the process to exit');
    expect(container.querySelector('time')?.dateTime).toBe(new Date(1000).toISOString());
    expect(asked).toBe(0);
    const end = container.querySelector<HTMLButtonElement>('button[aria-label="End now"]');
    expect(end?.dataset.symbol).toBe('stop');
    await act(async () => end?.click());
    expect(asked).toBe(1);
    await act(async () => root.unmount());
  });
});
