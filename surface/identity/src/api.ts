import { useEffect, useState } from 'react';
import { subscribeChanges } from './live';
import type { AgentView, DirectoryRecord, MeView, PeopleView, ReceiptAnswer, Refusal, SignedIn } from './generated';
import type { ActionBody, DelegateBody, Grant, GrantList, GrantModel, Permit, ReachAnswer, ReachBody, Recorded, RevokeBody, WhoAnswer, WhoBody } from './generated/grants';

/** The service is reached through the page's own origin, under /api. */
export const API = '/api';

/** How access is managed, and the commit the answering service was built from. */
export type AuthorityAnswer = { authority: string; build: string };

/** A refusal the service answered, by name, with its HTTP status. */
export class Refused extends Error {
  readonly status: number;
  readonly refusal: Refusal;
  /** The whole refusal answer as the service wrote it, for a route that carries more beside the name and reason (a receipt of the steps already written); undefined when there was none. */
  readonly answer: unknown;
  constructor(status: number, refusal: Refusal, answer?: unknown) {
    super(refusal.reason);
    this.status = status;
    this.refusal = refusal;
    this.answer = answer;
  }
}

async function refusalOf(response: Response): Promise<Refused> {
  try {
    const body = (await response.json()) as Partial<Refusal>;
    if (typeof body.refusal === 'string' && typeof body.reason === 'string') {
      return new Refused(response.status, { refusal: body.refusal, reason: body.reason }, body);
    }
  } catch {
    // The body is not a refusal; it is named below by its status.
  }
  return new Refused(response.status, {
    refusal: 'Unanswered',
    reason: `the service answered ${response.status} without naming a refusal`,
  });
}

/** The one exchange with the service. An answer that is not a refusal and cannot be read is said, never passed on as nothing. */
async function exchange<T>(method: 'GET' | 'POST' | 'PUT', path: string, body: unknown, signal: AbortSignal | undefined, emptyAnswers: boolean): Promise<T> {
  let response: Response;
  const headers: Record<string, string> = body === undefined ? { accept: 'application/json' } : { accept: 'application/json', 'content-type': 'application/json' };
  const init: RequestInit = method === 'GET'
    ? { credentials: 'same-origin', headers }
    : { method, credentials: 'same-origin', headers, ...(body === undefined ? {} : { body: JSON.stringify(body) }) };
  try {
    response = await fetch(API + path, { ...init, signal });
  } catch (error) {
    throw new Refused(0, { refusal: 'ServiceUnreachable', reason: `the identity service could not be reached: ${String(error)}` });
  }
  if (!response.ok) throw await refusalOf(response);
  const unreadable = () => new Refused(response.status, {
    refusal: 'UnreadableResponse',
    reason: `the identity service answered ${response.status}, but its result could not be read; do not repeat a change whose outcome is unknown`,
  });
  let text: string;
  try { text = await response.text(); } catch { throw unreadable(); }
  if (text === '' && emptyAnswers) return null as T;
  try {
    return JSON.parse(text) as T;
  } catch {
    throw unreadable();
  }
}

/** Ask the service: a read without a body, else a change sent as `method`, POST unless named. */
export function request<T>(path: string, body?: unknown, method: 'POST' | 'PUT' = 'POST', signal?: AbortSignal): Promise<T> {
  if (body !== undefined || signal) return exchange<T>(body === undefined ? 'GET' : method, path, body, signal, false);
  const flying = reading.get(path);
  if (flying) return flying as Promise<T>;
  const read = exchange<T>('GET', path, undefined, undefined, false).finally(() => reading.delete(path));
  reading.set(path, read);
  return read;
}

/** Reads of one path in flight together are one request: the parts of a page that ask for the same list share its answer. Nothing is kept once it has answered. */
const reading = new Map<string, Promise<unknown>>();

/** The same exchange with the method named, for routes that take a POST with no body or answer with none. */
export function send<T>(method: 'GET' | 'POST' | 'PUT', path: string, body?: unknown): Promise<T> {
  return exchange<T>(method, path, body, undefined, true);
}

const get = request;

/**
 * The administrator's wider view where the caller is admitted to it, and the
 * caller's own records otherwise. Admission is the server's; this only picks
 * the route.
 */
async function widest<T>(directory: string, personal: string): Promise<T> {
  try {
    return await get<T>(directory);
  } catch (error) {
    if (error instanceof Refused && error.refusal.refusal === 'NotAdmitted') return get<T>(personal);
    throw error;
  }
}

export const api = {
  me: () => get<MeView>('/me'),
  people: () => widest<PeopleView>('/directory/people', '/people'),
  ownPeople: () => get<PeopleView>('/people'),
  agent: (id: string) => widest<AgentView>('/directory/agents/' + encodeURIComponent(id), '/agents/' + encodeURIComponent(id)),
  identity: (id: string) => get<DirectoryRecord>('/identities/' + encodeURIComponent(id)),
  receipt: (index: number) => get<ReceiptAnswer>('/receipts/' + index),
  grants: () => get<GrantList>('/grants'),
  model: async () => {
    const model = await get<GrantModel>('/grants/model');
    if (!model.action_sentences || typeof model.action_sentences !== 'object'
      || Object.values(model.action_sentences).some((sentence) => typeof sentence !== 'string' || !sentence.trim())) {
      throw new Error('PermissionDescriptionsMissing: the service did not return valid action sentences.');
    }
    return model;
  },
  grant: (id: string) => get<Grant>('/grants/' + encodeURIComponent(id)),
  delegate: (body: DelegateBody) => get<Recorded>('/grants', body),
  why: (body: ActionBody) => get<Permit>('/grants/why', body),
  who: (body: WhoBody) => get<WhoAnswer>('/grants/who', body),
  reach: (body: ReachBody) => get<ReachAnswer>('/grants/reach', body),
  revoke: (id: string, body: RevokeBody) => get<Recorded>('/grants/' + encodeURIComponent(id) + '/revoke', body),
  callback: (search: string) => get<SignedIn>('/callback' + search),
  authority: () => get<AuthorityAnswer>('/authority'),
};

/** A new operation id, `op-` and 32 hex digits, kept across retries of one change. */
export function operationId(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return 'op-' + [...bytes].map((b) => b.toString(16).padStart(2, '0')).join('');
}

export type Load<T> = { status: 'loading' } | { status: 'ok'; data: T } | { status: 'refused'; refused: Refused };

const asRefused = (error: unknown): Refused =>
  error instanceof Refused ? error : new Refused(0, { refusal: 'Unanswered', reason: String(error) });

/** Live reads are driven by a bounded change signal, never a polling clock. */
export function useLive<T>(read: () => Promise<T>, key: string): Load<T> {
  const [state, setState] = useState<Load<T>>({ status: 'loading' });
  useEffect(() => {
    let live = true;
    let reading = false;
    let again = false;
    let feedFailure: Refused | null = null;
    setState({ status: 'loading' });
    const once = async () => {
      if (reading) { again = true; return; }
      reading = true;
      try {
        do {
          again = false;
          try {
            const data = await read();
            if (live && !feedFailure) setState({ status: 'ok', data });
          } catch (error) {
            if (live) setState({ status: 'refused', refused: asRefused(error) });
          }
        } while (live && again && document.visibilityState !== 'hidden');
      } finally { reading = false; }
    };
    const unsubscribe = subscribeChanges(() => { feedFailure = null; void once(); }, (error) => {
      feedFailure = asRefused(error);
      if (live) setState({ status: 'refused', refused: feedFailure });
    });
    return () => { live = false; unsubscribe(); };
  }, [key]);
  return state;
}

export function useLoad<T>(read: () => Promise<T>, key: string): Load<T> {
  const [state, setState] = useState<Load<T>>({ status: 'loading' });
  useEffect(() => {
    let live = true;
    setState({ status: 'loading' });
    read().then(
      (data) => live && setState({ status: 'ok', data }),
      (error: unknown) => live && setState({ status: 'refused', refused: asRefused(error) }),
    );
    return () => {
      live = false;
    };
  }, [key]);
  return state;
}

export type ControlPhase = 'unknown' | 'idle' | 'reserved' | 'active' | 'closed';
export type ControlReason = 'measurement_unavailable' | 'measurement_future' | 'post_measurement_missing' | 'stop_pending' | 'delivery_uncertain' | 'above_limit' | 'authority_unavailable' | 'reason_unrecognised';
export type AppliedContext = { state: 'released' } | { state: 'compact'; crossing: string } | { state: 'held'; crossing: string | null; reason: ControlReason } | { state: 'unavailable'; reason: ControlReason };
export type ReminderReference = { goal: string; occurrence: string; version: string; prior: string | null };
export type CurrentControl = { generation: number; phase: ControlPhase; active: string | null; context: AppliedContext | null; boundary: string | null; crossing: string | null; queued: { operation: string; reference: ReminderReference }[] };
export type PersonDecision = { operation: string; by: string; at: number; decision: { choice: 'seen' | 'not_seen' } | { choice: 'resent'; occurrence: string } };
export type PublicControlReceipt = {
  operation: string; session: string; request: string; state: 'accepted' | 'delivering' | 'delivered' | 'confirmed' | 'uncertain' | 'refused'; at: number;
  text: { length: number; sha256: string } | null; prepared: boolean; certainty: 'safely_unsent' | 'possibly_sent' | 'observed' | null;
  generation: number | null; uuid: string | null; reference: ReminderReference | null; admitted: boolean; reconciled: PersonDecision | null;
};
export type ResendOccurrence = { operation: string; session: string };
export type ResendLookup = { goal: string; prior: string; occurrence: ResendOccurrence | null };
export type ControlSessionsPage = { agent: string; sessions: string[]; after: string | null };
export type ControlReceiptsPage = { session: string; receipts: PublicControlReceipt[]; after: string | null };

function controlUnreadable(name: string, reason: string): never { throw new Refused(0, { refusal: name, reason }); }
function controlObject(value: unknown, name: string): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) controlUnreadable(name, 'The control answer is not an object.');
  return value as Record<string, unknown>;
}
function controlId(value: unknown, name: string): string {
  if (typeof value !== 'string' || !value.trim()) controlUnreadable(name, 'A control identity is absent.');
  return value;
}
function controlOptionalId(value: unknown, name: string): string | null { return value === null ? null : controlId(value, name); }
function controlNumber(value: unknown, name: string): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0) controlUnreadable(name, 'A control count or instant is invalid.');
  return value;
}
function controlBoolean(value: unknown, name: string): boolean {
  if (typeof value !== 'boolean') controlUnreadable(name, 'A control evidence flag is absent.');
  return value;
}
function controlChoice<T extends string>(value: unknown, choices: readonly T[], name: string): T {
  if (typeof value !== 'string' || !choices.includes(value as T)) controlUnreadable(name, 'An unknown control phase, outcome or reason was answered; its state is not assumed idle or complete.');
  return value as T;
}
function controlReference(value: unknown, name: string): ReminderReference {
  const row = controlObject(value, name);
  return { goal: controlId(row.goal, name), occurrence: controlId(row.occurrence, name), version: controlId(row.version, name), prior: controlOptionalId(row.prior, name) };
}
function appliedContext(value: unknown, name: string): AppliedContext | null {
  if (value === null) return null;
  const row = controlObject(value, name);
  const state = controlChoice(row.state, ['released', 'compact', 'held', 'unavailable'] as const, name);
  if (state === 'released') return { state };
  if (state === 'compact') return { state, crossing: controlId(row.crossing, name) };
  const reason = controlChoice(row.reason, ['measurement_unavailable', 'measurement_future', 'post_measurement_missing', 'stop_pending', 'delivery_uncertain', 'above_limit', 'authority_unavailable', 'reason_unrecognised'] as const, name);
  return state === 'held' ? { state, crossing: controlOptionalId(row.crossing, name), reason } : { state, reason };
}
function publicControlReceipt(value: unknown, session: string): PublicControlReceipt {
  const name = 'ControlReceiptUnreadable';
  const row = controlObject(value, name);
  if (row.session !== session) controlUnreadable(name, 'The receipt belongs to another session.');
  let text: PublicControlReceipt['text'] = null;
  if (row.text !== null) {
    const digest = controlObject(row.text, name);
    const sha256 = controlId(digest.sha256, name);
    if (!/^[0-9a-f]{64}$/.test(sha256)) controlUnreadable(name, 'The payload digest is invalid.');
    text = { length: controlNumber(digest.length, name), sha256 };
  }
  let reconciled: PersonDecision | null = null;
  if (row.reconciled !== null) {
    const kept = controlObject(row.reconciled, name);
    const decision = controlObject(kept.decision, name);
    const choice = controlChoice(decision.choice, ['seen', 'not_seen', 'resent'] as const, name);
    reconciled = { operation: controlId(kept.operation, name), by: controlId(kept.by, name), at: controlNumber(kept.at, name), decision: choice === 'resent' ? { choice, occurrence: controlId(decision.occurrence, name) } : { choice } };
  }
  return {
    operation: controlId(row.operation, name), session, request: controlId(row.request, name),
    state: controlChoice(row.state, ['accepted', 'delivering', 'delivered', 'confirmed', 'uncertain', 'refused'] as const, name), at: controlNumber(row.at, name), text,
    prepared: controlBoolean(row.prepared, name), certainty: row.certainty === null ? null : controlChoice(row.certainty, ['safely_unsent', 'possibly_sent', 'observed'] as const, name),
    generation: row.generation === null ? null : controlNumber(row.generation, name), uuid: controlOptionalId(row.uuid, name),
    reference: row.reference === null ? null : controlReference(row.reference, name), admitted: controlBoolean(row.admitted, name), reconciled,
  };
}
function controlCursor(value: unknown, last: string | undefined, name: string): string | null {
  const after = controlOptionalId(value, name);
  if (after !== null && after !== last) controlUnreadable(name, 'The continuation does not name the last entry in this bounded page.');
  return after;
}

export const controls = {
  async resendLookup(receipt: PublicControlReceipt): Promise<ResendLookup> {
    const name = 'GoalResendLookupUnreadable';
    if (!receipt.reference) controlUnreadable(name, 'The uncertain receipt names no goal.');
    const goal = receipt.reference.goal;
    const row = controlObject(await request<unknown>('/goals/' + encodeURIComponent(goal) + '/resends/' + encodeURIComponent(receipt.operation)), name);
    if (row.goal !== goal || row.prior !== receipt.operation) controlUnreadable(name, 'The resend lookup names another aim or prior.');
    const occurrence = row.occurrence === null ? null : controlObject(row.occurrence, name);
    return { goal, prior: receipt.operation, occurrence: occurrence === null ? null : { operation: controlId(occurrence.operation, name), session: controlId(occurrence.session, name) } };
  },
  async resend(receipt: PublicControlReceipt, occurrence: ResendOccurrence): Promise<PublicControlReceipt> {
    const name = 'GoalResendUnreadable';
    if (!receipt.reference) controlUnreadable(name, 'The uncertain receipt names no goal.');
    const goal = receipt.reference.goal;
    const row = controlObject(await request<unknown>('/goals/' + encodeURIComponent(goal) + '/resend', { operation: occurrence.operation, prior: receipt.operation, session: occurrence.session }), name);
    const answer = publicControlReceipt(row.reconciliation, receipt.session);
    const decision = answer.reconciled?.decision;
    if (row.goal !== goal || row.prior !== receipt.operation || row.occurrence !== occurrence.operation || row.session !== occurrence.session || typeof row.operation !== 'string' || !row.operation || row.operation === receipt.operation || answer.operation !== receipt.operation || answer.state !== receipt.state || answer.admitted !== receipt.admitted || answer.reconciled?.operation !== occurrence.operation || decision?.choice !== 'resent' || decision.occurrence !== occurrence.operation) controlUnreadable(name, 'The answer did not confirm the kept occurrence and its separate personal decision; retry under the same identity.');
    return answer;
  },
  async sessions(agent: string, after: string | null = null): Promise<ControlSessionsPage> {
    const name = 'ControlSessionsUnreadable';
    const row = controlObject(await request<unknown>('/agents/' + encodeURIComponent(agent) + '/control-sessions' + (after ? '?after=' + encodeURIComponent(after) : '')), name);
    if (row.agent !== agent || !Array.isArray(row.sessions) || row.sessions.length > 256) controlUnreadable(name, 'The session page is unbounded or names another agent.');
    const sessions = row.sessions.map((value) => controlId(value, name));
    if (new Set(sessions).size !== sessions.length || sessions.includes(after ?? '')) controlUnreadable(name, 'The session page repeats an identity or its cursor.');
    return { agent, sessions, after: controlCursor(row.after, sessions.at(-1), name) };
  },
  async status(session: string): Promise<CurrentControl | null> {
    const name = 'ControlStatusUnreadable';
    const answer = controlObject(await request<unknown>('/runtime/sessions/' + encodeURIComponent(session) + '/controls'), name);
    if (answer.session !== session) controlUnreadable(name, 'The current control belongs to another session.');
    if (answer.control === null) return null;
    const row = controlObject(answer.control, name);
    if (!Array.isArray(row.queued)) controlUnreadable(name, 'Queued occurrence identities are absent.');
    return {
      generation: controlNumber(row.generation, name), phase: controlChoice(row.phase, ['unknown', 'idle', 'reserved', 'active', 'closed'] as const, name),
      active: controlOptionalId(row.active, name), context: appliedContext(row.context, name), boundary: controlOptionalId(row.boundary, name), crossing: controlOptionalId(row.crossing, name),
      queued: row.queued.map((value) => { const entry = controlObject(value, name); return { operation: controlId(entry.operation, name), reference: controlReference(entry.reference, name) }; }),
    };
  },
  async receipts(session: string, after: string | null = null): Promise<ControlReceiptsPage> {
    const name = 'ControlReceiptsUnreadable';
    const row = controlObject(await request<unknown>('/runtime/sessions/' + encodeURIComponent(session) + '/control-receipts' + (after ? '?after=' + encodeURIComponent(after) : '')), name);
    if (row.session !== session || !Array.isArray(row.receipts) || row.receipts.length > 256) controlUnreadable(name, 'The receipt page is unbounded or names another session.');
    const receipts = row.receipts.map((value) => publicControlReceipt(value, session));
    const ids = receipts.map((receipt) => receipt.operation);
    if (new Set(ids).size !== ids.length || ids.includes(after ?? '')) controlUnreadable(name, 'The receipt page repeats an operation or its cursor.');
    return { session, receipts, after: controlCursor(row.after, ids.at(-1), name) };
  },
  async decide(receipt: PublicControlReceipt, operation: string, choice: 'seen' | 'not_seen'): Promise<PublicControlReceipt> {
    const answer = publicControlReceipt(await request<unknown>('/runtime/sessions/' + encodeURIComponent(receipt.session) + '/control-receipts/' + encodeURIComponent(receipt.operation) + '/reconcile', { operation, decision: choice }), receipt.session);
    if (answer.operation !== receipt.operation || answer.state !== receipt.state || answer.admitted !== receipt.admitted || answer.reconciled?.operation !== operation || answer.reconciled.decision.choice !== choice) controlUnreadable('ControlDecisionUnreadable', 'The answer did not confirm this personal decision separately from unchanged harness evidence; keep the same request when retrying.');
    return answer;
  },
};

/** crates/lys-identity-server/src/seats_api.rs (AGENTS-002): a seat's state as its runner knows it; unknown when the runner could not be read. */
export type SeatState = 'online-working' | 'online-idle' | 'offline' | 'not-seen' | 'unknown';
const SEAT_STATES: readonly SeatState[] = ['online-working', 'online-idle', 'offline', 'not-seen', 'unknown'];
/** One seat: its record and its liveness. `last_signal_at` is milliseconds since the epoch, as the runner keeps it. */
export interface SeatView {
  name: string; agent: string; harness: string; profile_version: number; machine: string; working_folder: string; account?: string | null;
  responsible: string; session?: string | null; harness_session?: string | null; state: SeatState; last_signal_at?: number | null;
  created_by: string; created_at: number; revision: number;
}
/** GET /seats: every seat, and whether the runner was read; when it was not, `reason` says why and every state is unknown. */
export interface SeatList { seats: SeatView[]; runner: 'read' | 'unknown'; reason?: string }
/** A session a runner holds for no seat: seen but unregistered. `started_at` is milliseconds since the epoch. */
export interface UnregisteredSession { session: string; machine: string; agent?: string | null; pid: number; started_at: number }

function seatsUnreadable(reason: string): never { throw new Refused(0, { refusal: 'SeatsUnreadable', reason }); }
const seatPath = (name: string, act: string) => '/seats/' + encodeURIComponent(name) + '/' + act;
function seatAnswer<T extends { seat: SeatView; session: string }>(answer: T, name: string): T {
  if (typeof answer !== 'object' || answer === null || answer.seat?.name !== name || typeof answer.session !== 'string' || !answer.session) seatsUnreadable('The answer did not name this seat and its session; its outcome is not assumed.');
  return answer;
}

/** The seat calls: the list, the sessions held for no seat, and the four acts, each carrying its operation id. */
export const seats = {
  list: async (): Promise<SeatList> => {
    const answer = await request<SeatList>('/seats');
    if (!Array.isArray(answer.seats) || (answer.runner !== 'read' && answer.runner !== 'unknown')) seatsUnreadable('The service did not answer a seat list with whether its runner was read.');
    if (answer.seats.some((seat) => typeof seat.name !== 'string' || !seat.name || !SEAT_STATES.includes(seat.state))) seatsUnreadable('A seat has no name or an unknown state; its state is not assumed.');
    return answer;
  },
  unregistered: async (): Promise<UnregisteredSession[]> => {
    const answer = await request<{ sessions: UnregisteredSession[] }>('/seats/unregistered');
    if (!Array.isArray(answer.sessions)) seatsUnreadable('The service did not answer the sessions held for no seat.');
    return answer.sessions;
  },
  start: async (name: string, operation: string) =>
    seatAnswer(await request<{ seat: SeatView; session: string; harness_session?: string | null }>(seatPath(name, 'start'), { operation }), name),
  stop: async (name: string, operation: string, force: boolean) => {
    const answer = seatAnswer(await request<{ seat: SeatView; session: string; ended: boolean }>(seatPath(name, 'stop'), { operation, force }), name);
    if (typeof answer.ended !== 'boolean') seatsUnreadable('The stop answer did not say whether the session ended.');
    return answer;
  },
  restart: async (name: string, operation: string, force: boolean) =>
    seatAnswer(await request<{ seat: SeatView; session: string }>(seatPath(name, 'restart'), { operation, force }), name),
  send: async (name: string, operation: string, text: string) => {
    const answer = seatAnswer(await request<{ seat: SeatView; session: string; delivered: true }>(seatPath(name, 'send'), { operation, text }), name);
    if (answer.delivered !== true) seatsUnreadable('The send answer did not confirm delivery; the message is not assumed delivered.');
    return answer;
  },
};
