/** A byte sink for UI transport tests, never evidence that a GPU or VT emulator ran. */
import { beforeEach, vi } from 'vitest';

export const written: Uint8Array[] = [];
export const disposed = vi.fn();
/** What the double's browser offers: without WebGPU a strict WebGPU request fails as the library's does, and 'auto' draws with WebGL2. */
export const browser = { webgpu: true };
/** The listener the terminal holds for each event, so a test can raise one as the library would. */
export const listeners = new Map<string, (value: unknown) => void>();
beforeEach(() => { written.length = 0; disposed.mockClear(); browser.webgpu = true; listeners.clear(); });

export async function mockTerminal(options: { renderer?: string }) {
  if (options.renderer === 'webgpu' && !browser.webgpu) throw new Error('WebGPU is unavailable');
  return {
    renderer: { backend: browser.webgpu ? 'webgpu' : 'webgl2' }, geometry: { cols: 80, rows: 24 },
    on(type: string, listener: (value: unknown) => void) { listeners.set(type, listener); return { dispose() {} }; },
    blur() {}, dispose: disposed,
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
