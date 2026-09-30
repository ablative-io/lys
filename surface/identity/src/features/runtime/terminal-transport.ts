/** The terminal preserves byte cursors and requires a receipt for every request of input; input typed while one is in flight goes in the next, so pointer motion never holds typing back. */
import { API, Refused } from '../../api';
import type { SessionEnd } from './Terminal';

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('The terminal response is not an object.');
  return value as Record<string, unknown>;
}

async function recordedRequest(path: string, body: unknown, signal?: AbortSignal): Promise<Record<string, unknown>> {
  const response = await fetch(API + path, {
    method: 'POST', credentials: 'same-origin', signal,
    headers: { accept: 'application/json', 'content-type': 'application/json' }, body: JSON.stringify(body),
  });
  const value = record(await response.json());
  if (!response.ok) throw new Refused(response.status, {
    refusal: typeof value.refusal === 'string' ? value.refusal : 'Unanswered',
    reason: typeof value.reason === 'string' ? value.reason : 'The terminal request was refused without a reason.',
  });
  const receipt = record(value.receipt);
  if (!Number.isSafeInteger(receipt.index) || Number(receipt.index) < 0) throw new Error('The terminal request has no receipt. Do not repeat input whose outcome is unknown.');
  return value;
}

export async function terminalRequest(path: string, body: unknown, signal?: AbortSignal): Promise<Record<string, unknown>> {
  try {
    return await recordedRequest(path, body, signal);
  } catch (error) {
    if (error instanceof Refused || signal?.aborted) throw error;
    throw new Error(`The terminal request's outcome could not be confirmed. Do not repeat input or controls without checking the session. ${String(error)}`, { cause: error });
  }
}

export function byteOutput(value: Record<string, unknown>, session: string, cursor: number | null) {
  const answer = record(value.answer);
  const output = record(answer.output);
  if (value.session !== session || answer.kind !== 'bytes' || output.session !== session) throw new Error('The terminal response belongs to another session or is not raw bytes.');
  const { from, cursor: next, oldest, data, ended } = output;
  if (!Number.isSafeInteger(from) || !Number.isSafeInteger(next) || !Number.isSafeInteger(oldest)
    || Number(oldest) < 0 || Number(from) < Number(oldest) || Number(next) < Number(from)
    || (cursor !== null && from !== cursor) || !Array.isArray(data)
    || !data.every((byte: unknown) => typeof byte === 'number' && Number.isInteger(byte) && byte >= 0 && byte <= 255)
    || Number(next) - Number(from) !== data.length) throw new Error('The terminal response has an invalid byte window.');
  let end: SessionEnd | null = null;
  if (ended !== null) {
    const entry = record(ended);
    if (!['exited', 'ended_by_runner_restart', 'accounts_exhausted'].includes(String(entry.how))
      || !Number.isSafeInteger(entry.at) || Number(entry.at) < 0
      || (entry.status !== null && !Number.isInteger(entry.status))
      || (entry.signal !== null && typeof entry.signal !== 'string')) throw new Error('The terminal response has invalid exit evidence.');
    end = entry as unknown as SessionEnd;
  }
  if (data.length === 0 && !end) throw new Error('A following terminal read returned no output or exit.');
  return { data: Uint8Array.from(data), cursor: Number(next), ended: end };
}

/** One pointer-motion report, SGR (`CSI < b;x;y M`) or X10 (`CSI M b x y`), whose button code carries the motion bit; other encodings are sent as they come. */
export function motionReport(data: Uint8Array): boolean {
  if (data.length === 6 && data[0] === 27 && data[1] === 91 && data[2] === 77) return ((data[3] - 32) & 32) !== 0;
  const sgr = data.length <= 24 ? /^\u001b\[<(\d+);\d+;\d+[Mm]$/.exec(String.fromCharCode(...data)) : null;
  return sgr !== null && (Number(sgr[1]) & 32) !== 0;
}

export function terminalStreams(session: string, controller: AbortController, ended: (value: SessionEnd) => void) {
  const base = '/runtime/sessions/' + encodeURIComponent(session);
  let cursor: number | null = null;
  let finished = false;
  let output: ReadableStreamDefaultController<Uint8Array> | undefined;
  let input: WritableStreamDefaultController | undefined;
  let queued: { data: Uint8Array; motion: boolean }[] = [];
  let sending = false;
  // One input request at a time; everything queued behind it goes in the next single request, in order.
  const deliver = async () => {
    while (queued.length && !controller.signal.aborted) {
      if (finished) throw new Error('The session has ended. The input queued for it was not sent.');
      const batch = queued;
      queued = [];
      const value = await terminalRequest(base + '/input-bytes', { data: batch.flatMap((entry) => Array.from(entry.data)) });
      const answer = record(value.answer);
      if (value.session !== session || answer.kind !== 'delivered' || answer.session !== session) throw new Error('Terminal input was not confirmed. Do not resend it.');
    }
    queued = [];
    sending = false;
  };
  // An uncertain outcome errors both directions, so the terminal disconnects with the reason; queued input is never sent after it.
  const failed = (error: unknown) => {
    queued = [];
    input?.error(error);
    output?.error(error);
  };
  return {
    readable: new ReadableStream<Uint8Array>({
      start(stream) { output = stream; },
      async pull(stream) {
        const answer = await terminalRequest(base + '/read-bytes', { cursor, follow: true }, controller.signal);
        if (controller.signal.aborted) return;
        const read = byteOutput(answer, session, cursor);
        cursor = read.cursor;
        if (read.data.length) stream.enqueue(read.data);
        if (read.ended) { finished = true; ended(read.ended); stream.close(); }
      },
      cancel() { controller.abort(); },
    }),
    writable: new WritableStream<Uint8Array>({
      start(stream) { input = stream; },
      // Accepting a chunk queues it, so the next can join it while a request is in flight; a motion report replaces the one queued before it.
      write(data) {
        if (controller.signal.aborted) throw new Error('The terminal is disconnected.');
        if (finished) throw new Error('The session has ended. No input was sent.');
        const motion = motionReport(data);
        if (motion) queued = queued.filter((entry) => !entry.motion);
        queued.push({ data, motion });
        if (sending) return;
        sending = true;
        void deliver().catch(failed);
      },
    }),
  };
}
