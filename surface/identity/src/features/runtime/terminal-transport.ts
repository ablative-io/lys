/** The terminal preserves byte cursors and requires a receipt for every accepted input. */
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

export function terminalStreams(session: string, controller: AbortController, ended: (value: SessionEnd) => void) {
  const base = '/runtime/sessions/' + encodeURIComponent(session);
  let cursor: number | null = null;
  let finished = false;
  return {
    readable: new ReadableStream<Uint8Array>({
      async pull(stream) {
        const answer = await terminalRequest(base + '/read-bytes', { cursor, follow: true }, controller.signal);
        if (controller.signal.aborted) return;
        const output = byteOutput(answer, session, cursor);
        cursor = output.cursor;
        if (output.data.length) stream.enqueue(output.data);
        if (output.ended) { finished = true; ended(output.ended); stream.close(); }
      },
      cancel() { controller.abort(); },
    }),
    writable: new WritableStream<Uint8Array>({
      async write(data) {
        if (controller.signal.aborted) throw new Error('The terminal is disconnected.');
        if (finished) throw new Error('The session has ended. No input was sent.');
        // Writes serialize in the stream; an uncertain outcome errors it, never retries it.
        const value = await terminalRequest(base + '/input-bytes', { data: Array.from(data) });
        const answer = record(value.answer);
        if (value.session !== session || answer.kind !== 'delivered' || answer.session !== session) throw new Error('Terminal input was not confirmed. Do not resend it.');
      },
    }),
  };
}
