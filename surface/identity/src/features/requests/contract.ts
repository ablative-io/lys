/** Access requests as requests_views.rs writes them; asking never grants access. */
import type { PersonSummary } from '../../generated';
import type { ResourceRef } from '../../generated/grants';

export interface AccessRequest {
  id: string;
  asked_by: string;
  asked_by_name: string | null;
  responsible: PersonSummary;
  resource: ResourceRef;
  relation: string;
  actions: string[];
  ends_at: number | null;
  why: string;
  asked_at: number;
  state: 'waiting' | 'approved' | 'declined';
  approvers: PersonSummary[];
  sources: string[];
  /** New server capability; older responses offer no root issuance control. */
  can_issue_root?: boolean;
  /** The service may admit a root decider even when it cannot lend to an agent. */
  can_decide?: boolean;
  decision: { by: string; note: string; grant: string | null; decided_at: number } | null;
}

export interface Ask {
  operation: string;
  resource: ResourceRef;
  relation: string;
  ends_at: number | null;
  why: string;
}

/** Check a read-back against all of the retained request, including its authenticated asker. */
export function matchesAsk(value: AccessRequest, asked: Ask, person: string): boolean {
  return value.id === asked.operation && value.asked_by === person
    && value.resource.kind === asked.resource.kind && value.resource.id === asked.resource.id
    && value.relation === asked.relation && value.ends_at === asked.ends_at && value.why === asked.why;
}
