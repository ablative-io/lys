/** A byte sink for UI transport tests, never evidence that a GPU or VT emulator ran. */
import { beforeEach, vi } from 'vitest';

export const written: Uint8Array[] = [];
export const disposed = vi.fn();
beforeEach(() => { written.length = 0; disposed.mockClear(); });

export async function mockTerminal() {
  return {
    renderer: { backend: 'webgpu' }, geometry: { cols: 80, rows: 24 },
    on() { return { dispose() {} }; },
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
