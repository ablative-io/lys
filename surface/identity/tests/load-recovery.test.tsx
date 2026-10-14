import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { expect, it, vi } from 'vitest';
import { Refused, useLoad } from '../src/api';
import { serve } from './harness';

it('retries a first-load service failure when the next change signal arrives', async () => {
  serve({});
  const read = vi.fn<() => Promise<string>>()
    .mockRejectedValueOnce(new Refused(502, { refusal: 'Unanswered', reason: 'the tunnel was restarting' }))
    .mockResolvedValue('read after the restart');
  function Reading() {
    const load = useLoad(read, 'first-load-recovery');
    return <p>{load.status === 'ok' ? load.data : load.status}</p>;
  }
  const host = document.createElement('div');
  document.body.append(host);
  const root = createRoot(host);
  try {
    await act(async () => { root.render(<Reading />); });
    expect(read).toHaveBeenCalledTimes(2);
    expect(host.textContent).toBe('read after the restart');
  } finally { act(() => root.unmount()); }
});
