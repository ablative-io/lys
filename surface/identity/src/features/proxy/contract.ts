/**
 * The model calls an agent's runs made, as Lys's proxy recorded them: one row for each call, and one call whole with
 * its request and its response as the provider's own JSON. Read by whoever may read that agent's terminal.
 */
import { request } from '../../api';

/** The token figures a response reported; a figure it did not report is absent, never zero. */
export interface CallTokens { input?: number; output?: number; cache_creation?: number; cache_read?: number; reasoning?: number }

/** How a call ended, in the record's own word. */
export type CallStatus = 'complete' | 'cancelled' | 'partial' | 'unrecorded' | 'lost';

/** One call as the list shows it: read from the uses Lys keeps, so a figure the response did not report is null, never zero. */
export interface CallRow {
  call_id: string;
  /** The Lys session that made it. */
  session: string | null;
  /** When the call ended, in milliseconds since the Unix epoch. */
  at_ms: number;
  model: string | null;
  input_tokens: number | null;
  output_tokens: number | null;
  cache_creation_tokens: number | null;
  cache_read_tokens: number | null;
  account: string | null;
  status: CallStatus | null;
  duration_ms: number | null;
}

/** An agent's calls, newest first. `next` is there when earlier calls exist: asked for again with it, they follow. */
export interface CallsView { agent: string; calls: CallRow[]; next: string | null }

/** One side's headers: every name in the order received, and the values the record keeps. A credential's value is never kept. */
export interface HeadSide { names: string[]; values: Record<string, string[]> }

/** A call's record as the proxy keeps it. */
export interface CallRecord {
  call_id: string;
  provider: string;
  api: string;
  model?: string;
  status: CallStatus;
  stream: boolean;
  started_at: string;
  duration_ms?: number;
  usage?: CallTokens;
  request_id?: string;
  message_id?: string;
  /** Why the call is unrecorded, in the words of the step that found it. */
  unrecorded_reason?: string;
  /** The provider's HTTP status and both sides' headers; absent when no head was recorded. */
  head?: { status?: number | null; request: HeadSide; response: HeadSide };
}

/** One call whole. A body that could not be read as JSON is null, with why. A streamed response is the list of its events. */
export interface CallView {
  call: CallRecord;
  request: unknown;
  response: unknown;
  request_unreadable: string | null;
  response_unreadable: string | null;
}

const part = encodeURIComponent;
export const readCalls = (agent: string, after?: string): Promise<CallsView> => request<CallsView>('/agents/' + part(agent) + '/calls' + (after ? '?after=' + part(after) : ''));
export const readCall = (agent: string, call: string): Promise<CallView> => request<CallView>('/agents/' + part(agent) + '/calls/' + part(call));
