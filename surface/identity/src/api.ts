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
  constructor(status: number, refusal: Refusal) {
    super(refusal.reason);
    this.status = status;
    this.refusal = refusal;
  }
}

async function refusalOf(response: Response): Promise<Refused> {
  try {
    const body = (await response.json()) as Partial<Refusal>;
    if (typeof body.refusal === 'string' && typeof body.reason === 'string') {
      return new Refused(response.status, { refusal: body.refusal, reason: body.reason });
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
  return exchange<T>(body === undefined ? 'GET' : method, path, body, signal, false);
}

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
