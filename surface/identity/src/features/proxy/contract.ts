/**
 * The model calls an agent's runs made, as Lys's proxy recorded them: one row for each call, and one call whole with
 * its request and its response as the provider's own JSON. Read by whoever may read that agent's terminal.
 */
import { request } from '../../api';

/** The token figures a response reported; a figure it did not report is absent, never zero. */
export interface CallTokens { input?: number; output?: number; cache_creation?: number; cache_read?: number; reasoning?: number }

/** How a call ended, in the record's own word. */
export type CallStatus = 'complete' | 'cancelled' | 'partial' | 'unrecorded' | 'lost';

export interface CallRow {
  call_id: string;
  /** When the call started, RFC 3339. */
  started_at: string;
  duration_ms: number | null;
  provider: string;
  api: string;
  model: string | null;
  status: CallStatus;
  stream: boolean;
  /** The provider's HTTP status; none when no response head arrived. */
  http_status: number | null;
  usage: CallTokens | null;
  request_id: string | null;
  /** Why the call is unrecorded, in the words of the step that found it. */
  unrecorded_reason: string | null;
}

/** An agent's calls, newest first. `after` is there when earlier calls exist: asked for again with it, they follow. */
export interface CallsView { agent: string; calls: CallRow[]; after: string | null }

/** One side's headers: every name in the order received, and the values the record keeps. A credential's value is never kept. */
export interface HeadSide { names: string[]; values: Record<string, string[]> }

/** One call whole. A body that could not be read as JSON is absent, with why. A streamed response is the list of its events. */
export interface CallView {
  call: CallRow;
  request_head: HeadSide | null;
  response_head: HeadSide | null;
  request: unknown;
  response: unknown;
  request_unreadable: string | null;
  response_unreadable: string | null;
}

const part = encodeURIComponent;
export const readCalls = (agent: string, after?: string): Promise<CallsView> => request<CallsView>('/agents/' + part(agent) + '/calls' + (after ? '?after=' + part(after) : ''));
export const readCall = (agent: string, call: string): Promise<CallView> => request<CallView>('/agents/' + part(agent) + '/calls/' + part(call));
