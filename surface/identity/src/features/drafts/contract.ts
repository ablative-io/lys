/** The drafts an agent prepared for its responsible person to decide, as `GET /drafts` lists them, and what deciding one sends and answers. One file, so a member the service renames is one edit. */
import { readEveryPage } from '../../reads';
import type { Paged } from '../../reads';

export interface Named { id: string; display_name: string }

/** The resource and action the prepared request names. */
export interface DraftTarget { kind: string; id: string; action: string }

export type DraftState = 'waiting' | 'approved' | 'refused' | 'corrected';

/** The agent that prepared a draft; its name is null when the directory no longer holds it. */
export interface DraftAgent { id: string; display_name: string | null }

/** Who decided a draft: the person when the directory names one, and always the sign-in that did it. */
export interface DraftDecision {
  operation: string;
  by: Named | null;
  login: { provider: string; subject: string };
  /** Seconds since the epoch. */
  at: number;
  reason?: string;
  /** For an approval, the operation reserved for applying it. */
  application: string | null;
  /** For a correction, the draft that replaced this one. */
  replacement: string | null;
}

export interface Draft {
  /** The draft's operation id; never shown. */
  id: string;
  /** Null when the draft was not prepared by an agent the directory can name. */
  agent: DraftAgent | null;
  responsible: Named & { state: string };
  target: DraftTarget;
  /** The prepared request, kept as the agent wrote it. */
  method: string;
  path: string;
  body: string;
  note: string;
  /** Seconds since the epoch. */
  created_at: number;
  /** The digest a decision names, so it decides exactly this creation. */
  creation_hash: string;
  state: DraftState;
  decided?: DraftDecision;
}

export interface DraftList extends Paged { drafts: Draft[] }

/** Which drafts a list holds, as `GET /drafts?state=` takes it. */
export type DraftFilter = 'waiting' | 'decided';

/** `POST /drafts/{id}/approve`: the decision's own operation, the creation it decides, and the operation reserved for applying it. */
export interface ApproveBody { operation: string; creation_hash: string; application: string }
/** `POST /drafts/{id}/refuse`. */
export interface RefuseBody { operation: string; creation_hash: string; reason: string }

/** What either decision answers: where it was recorded, never that anything was applied. */
export interface DraftAnswer { draft: string; operation: string; creation_hash: string; index: number; tree_size: number; leaf_hash: string; replacement: string | null }

/** Every draft in `filter`, every page of it. */
export async function readDrafts(filter: DraftFilter): Promise<Draft[]> {
  const answer = await readEveryPage<DraftList>('/drafts?state=' + filter, (read, page) => {
    if (!page || !Array.isArray(page.drafts)) throw new Error('DraftsUnreadable: a later page did not answer a list of drafts.');
    return { ...read, drafts: [...read.drafts, ...page.drafts] };
  });
  if (!answer || !Array.isArray(answer.drafts)) throw new Error('DraftsUnreadable: the service did not answer a list of drafts.');
  return answer.drafts;
}

/** Whether an answer confirms exactly the decision that was sent. */
export const confirms = (id: string) => (answer: DraftAnswer, body: Record<string, unknown>): boolean =>
  answer.draft === id && answer.operation === body.operation && answer.creation_hash === body.creation_hash;
