import { act } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, expect, it, vi } from 'vitest';
import { FolderChooser } from '../src/features/provisioning/FolderChooser';

function deferred<T>() {
  let resolve: (value: T) => void = () => { throw new Error('promise not constructed'); };
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); });

it('reads suggestions on the parent change and cancels a read when the chooser closes', async () => {
  const pending = deferred<Response>();
  const fetch = vi.fn().mockReturnValue(pending.promise);
  vi.stubGlobal('fetch', fetch);
  const timer = vi.spyOn(globalThis, 'setTimeout');
  const container = document.createElement('div');
  document.body.append(container);
  const root = createRoot(container);
  try {
    await act(async () => { root.render(<FolderChooser computers={[{ id: 'computer', name: 'Computer' }]} chosen="" choose={() => undefined} />); });
    const open = container.querySelector<HTMLButtonElement>('button[aria-label="Choose a folder"]');
    if (!open) throw new Error('the chooser has no open button');
    await act(async () => { open.click(); });
    const input = container.querySelector<HTMLInputElement>('input[name="folder-path"]');
    if (!input) throw new Error('the chooser has no path field');
    await act(async () => {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
      if (!setter) throw new Error('the path field has no value setter');
      setter.call(input, '/work/project');
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(JSON.parse(fetch.mock.calls[0][1].body)).toEqual({ under: '/work' });
    expect(timer).not.toHaveBeenCalled();
    const signal = fetch.mock.calls[0][1].signal as AbortSignal;
    expect(signal.aborted).toBe(false);
    const cancel = container.querySelector<HTMLButtonElement>('button[aria-label="Cancel"]');
    if (!cancel) throw new Error('the chooser has no cancel button');
    await act(async () => { cancel.click(); });
    expect(signal.aborted).toBe(true);
    await act(async () => { pending.resolve(new Response(JSON.stringify({ machine: 'computer', under: '/work', folders: ['project'] }))); });
    expect(container.querySelector('datalist')).toBeNull();
  } finally { await act(async () => { root.unmount(); }); container.remove(); }
});
