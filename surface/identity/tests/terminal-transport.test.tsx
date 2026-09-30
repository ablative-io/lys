/** Raw terminal transport rejects ambiguous windows and stops input after an uncertain outcome. */
import { describe, expect, it, vi } from 'vitest';
import { byteOutput, terminalRequest, terminalStreams } from '../src/features/runtime/terminal-transport';

const session = 'running-fixture';
const envelope = (patch: Record<string, unknown> = {}) => ({
  session, answer: { kind: 'bytes', output: { session, from: 0, cursor: 2, oldest: 0, data: [255, 226], ended: null, ...patch } },
});

describe('Raw terminal transport', () => {
  it('does not permit truncation, cursor jumps, or invalid bytes', () => {
    for (const patch of [{ cursor: 3 }, { from: 1 }, { oldest: 1 }, { data: [256, 1] }, { data: [-1, 1] }, { cursor: 0, data: [] }]) {
      expect(() => byteOutput(envelope(patch), session, 0)).toThrow();
    }
  });

  it('requires a receipt instead of reporting unrecorded input as accepted', async () => {
    vi.stubGlobal('fetch', async () => new Response(JSON.stringify({ session, answer: { kind: 'delivered', session } })));
    await expect(terminalRequest('/input-bytes', { data: [65] })).rejects.toThrow();
  });

  it('retains the named permission refusal', async () => {
    vi.stubGlobal('fetch', async () => new Response(JSON.stringify({ refusal: 'not_permitted', reason: 'No operate grant for this agent' }), { status: 403 }));
    await expect(terminalRequest('/input-bytes', { data: [65] })).rejects.toMatchObject({ status: 403, refusal: { refusal: 'not_permitted' } });
  });

  it('never sends queued input after the first input has an unknown outcome', async () => {
    const sent: unknown[] = [];
    vi.stubGlobal('fetch', (path: string, init?: RequestInit) => {
      if (path.endsWith('/read-bytes')) return Promise.resolve(new Response(JSON.stringify({ ...envelope(), receipt: { index: 1 } })));
      sent.push(JSON.parse(String(init?.body)));
      return Promise.reject(new Error('lost response'));
    });
    const streams = terminalStreams(session, new AbortController(), () => {});
    const writer = streams.writable.getWriter();
    const first = writer.write(new Uint8Array([0, 255]));
    const second = writer.write(new Uint8Array([66]));
    const firstFailed = expect(first).rejects.toThrow('lost response');
    const secondFailed = expect(second).rejects.toThrow('lost response');
    await Promise.all([firstFailed, secondFailed]);
    expect(sent).toEqual([{ data: [0, 255] }]);
  });

  it('does not send input after the runner reports the process ended', async () => {
    const fetcher = vi.fn(async () => new Response(JSON.stringify({ ...envelope({ data: [], cursor: 0, ended: { how: 'exited', at: 1, status: 0, signal: null } }), receipt: { index: 1 } })));
    vi.stubGlobal('fetch', fetcher);
    const ended = vi.fn();
    const streams = terminalStreams(session, new AbortController(), ended);
    expect(await streams.readable.getReader().read()).toEqual({ done: true, value: undefined });
    await expect(streams.writable.getWriter().write(new Uint8Array([65]))).rejects.toThrow('session has ended');
    expect(ended).toHaveBeenCalledOnce();
    expect(fetcher).toHaveBeenCalledOnce();
  });

  it('disconnects input without sending it when a session view is closed', async () => {
    vi.stubGlobal('fetch', async () => new Response(JSON.stringify({ ...envelope(), receipt: { index: 1 } })));
    const controller = new AbortController();
    const streams = terminalStreams(session, controller, () => {});
    await streams.readable.cancel();
    expect(controller.signal.aborted).toBe(true);
    await expect(streams.writable.getWriter().write(new Uint8Array([65]))).rejects.toThrow('disconnected');
  });
});
