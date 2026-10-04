/** A byte sink for UI transport tests, never evidence that a GPU or VT emulator ran. */
import { beforeEach, vi } from 'vitest';

export const written: Uint8Array[] = [];
export const disposed = vi.fn();
/** What was sent to the session as typed text, in order. */
export const sent: string[] = [];
/** Each size the page set on the terminal, as (columns, rows), and whether it switched the library's own count off. */
export const sized: [number, number][] = [];
export const ownCount = { disconnect: vi.fn() };
/** The cell the double's font measures, in device pixels; none until a test gives one, as a terminal with nothing to measure. */
export const cell: { cellWidthPx?: number; cellHeightPx?: number } = {};
/** What the double's browser offers: without WebGPU a strict WebGPU request fails as the library's does, and 'auto' draws with WebGL2. */
export const browser = { webgpu: true };
/** The listener the terminal holds for each event, so a test can raise one as the library would. */
export const listeners = new Map<string, (value: unknown) => void>();
beforeEach(() => {
  written.length = 0; sent.length = 0; sized.length = 0; disposed.mockClear(); ownCount.disconnect.mockClear(); browser.webgpu = true; listeners.clear();
  delete cell.cellWidthPx; delete cell.cellHeightPx;
  if (typeof globalThis.ResizeObserver !== 'function') vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} });
});

export async function mockTerminal(options: { renderer?: string }) {
  if (options.renderer === 'webgpu' && !browser.webgpu) throw new Error('WebGPU is unavailable');
  return {
    renderer: { backend: browser.webgpu ? 'webgpu' : 'webgl2' }, get geometry() { return { cols: sized.at(-1)?.[0] ?? 80, rows: sized.at(-1)?.[1] ?? 24, ...cell }; },
    resizeObserver: ownCount, resize(cols: number, rows: number) { sized.push([cols, rows]); listeners.get('resize')?.({ cols, rows }); },
    on(type: string, listener: (value: unknown) => void) { listeners.set(type, listener); return { dispose() {} }; },
    blur() {}, dispose: disposed, sendText(data: string) { sent.push(data); },
    connect(transport: { readable: ReadableStream<Uint8Array>; writable: WritableStream<Uint8Array> }) {
      const reader = transport.readable.getReader();
      const closed = (async () => {
        for (;;) {
          const value = await reader.read();
          if (value.done) return;
          written.push(value.value);
        }
      })();
      return { closed, onStatusChange() { return { dispose() {} }; } };
    },
  };
}
