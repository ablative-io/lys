// The grant routes' JSON, as crates/lys-identity-server/src/grant_contract/
// (requests.rs and views.rs) writes and reads it. Every member of every request body is required.

import type { IdentityId } from './index';

/** `ResourceWire`, and a grant's resource. */
export interface ResourceRef {
  kind: string;
  id: string;
}

export type RecipientKind = 'person' | 'agent' | 'service_account';

/** `pass_on_json` and `PassOnWire`: what a holder may pass on, stated affirmatively. */
export type PassOn = { kind: 'use_only' } | { kind: 'to'; actions: string[]; recipients: RecipientKind[] };

/** `WindowWire`; `ends_at` null means no end of its own. */
export interface GrantWindow {
  starts_at: number;
  ends_at: number | null;
}

/** `UnreportedView`: permitted exercises whose use events could not be recorded. */
export interface Unreported {
  count: number;
  at: number;
  route: RouteWire;
  reason: string;
}

/**
 * `LastUseView`: when a grant was last seen exercised at an enforcement point.
 * Not seen says only that no exercise was observed, never that it was never used.
 * `recorded` counts the use events in the grant log; `source` is `reported` when
 * every permitted exercise the service has seen since it opened has one, and
 * `missing` when some have none, so a zero is then not no use.
 */
export type LastUse = ({ seen: false } | { seen: true; at: number; route: RouteWire; use_event: number }) & { recorded: number } & (
  | { source: 'reported' }
  | { source: 'missing'; unreported: Unreported }
);

/** `GrantView`. */
export interface Grant {
  id: string;
  issuer: IdentityId;
  holder: IdentityId;
  responsible: IdentityId;
  resource: ResourceRef;
  relation: string;
  actions: string[];
  pass_on: PassOn;
  /** The grant it derives from; null for a root grant. */
  source: string | null;
  window: GrantWindow;
  model_version: number;
  operation: string;
  /** Whether it was revoked directly. */
  revoked: boolean;
  /** When it was revoked directly, in seconds; null while it stands. */
  revoked_at: number | null;
  /** The grants' revision once its revocation was committed; null while it stands. */
  revoked_revision: number | null;
  last_use: LastUse;
  /**
   * `StandingView`: whether it stands, judged by the service over its whole
   * chain at its clock. A refusal that names a grant or identity the caller
   * may not see keeps its name, with a null grant and the withheld reason.
   */
  standing: { stands: true } | { stands: false; refusal: string; grant: string | null; reason: string };
  /** The earliest end on its chain, which admission keeps its own end; null when nothing on it ends. */
  effective_ends_at: number | null;
}

/** GET /grants */
export interface GrantList {
  grants: Grant[];
  revision: number;
}

/** `RouteWire`: how a request arrived. Recorded, never a decision. */
export type RouteWire = 'browser' | 'api' | 'tool';

/** POST /grants: `DelegateBody`. */
export interface DelegateBody {
  operation: string;
  route: RouteWire;
  source: string;
  recipient: IdentityId;
  responsible: IdentityId;
  resource: ResourceRef;
  relation: string;
  pass_on: PassOn;
  window: GrantWindow;
}

/** POST /grants/{id}/revoke: `RevokeBody`. The reason is 1 to 1024 bytes. */
export interface RevokeBody {
  operation: string;
  route: RouteWire;
  reason: string;
}

/** POST /grants/why: `ActionBody`. */
export interface ActionBody {
  route: RouteWire;
  resource: ResourceRef;
  action: string;
}

/** POST /grants/who: `WhoBody`. */
export interface WhoBody extends ActionBody {
  page_size: number;
  after: string | null;
}

/** `RecordedView`: a recorded change and its receipt. */
export interface Recorded {
  operation: string;
  grant: string;
  index: number;
  receipt: {
    version: number;
    caller: IdentityId;
    change_kind: number;
    payload_commitment: string;
    payload_commitment_hash: string;
    revision: number;
    log: { index: number; tree_size: number; root: string; leaf_hash: string };
  };
}

/** `UseEventView`: whether a check's use was recorded, and why not when it was not. */
export type UseEvent = { recorded: true; index: number } | { recorded: false; reason: string };

/**
 * `PermitView`: a permitted decision and the authority path it rests on. POST
 * /grants/why answers it and records nothing; POST /grants/check answers it
 * with `use_event` and records a use, so a question never calls it.
 */
export interface Permit {
  permitted: true;
  grant: string;
  path: string[];
  responsible: IdentityId;
  scope: string[];
  model_version: number;
  revision: number;
  use_event?: UseEvent;
}

/** GET /grants/model: `ModelView`, each relation with the actions it carries. */
export interface GrantModel {
  action_sentences: Record<string, string>;
  version: number;
  relations: Record<string, string[]>;
}

/** POST /grants/who answers one page of holders, each with its permit. */
export interface WhoAnswer {
  holders: (Permit & { holder: IdentityId })[];
  revision: number;
  complete: boolean;
  next: string | null;
}

/** `PAGE_MAX`. */
export const PAGE_MAX = 100;

/** POST /grants/reach: `ReachBody`, each resource with the actions asked about it. */
export interface ReachBody {
  route: RouteWire;
  resources: (ResourceRef & { actions: string[] })[];
}

/** POST /grants/reach answers each resource, in the order asked, with every visible holder and the actions it may take. */
export interface ReachAnswer {
  revision: number;
  resources: (ResourceRef & { holders: { holder: IdentityId; actions: string[] }[] })[];
}

/** `REACH_MAX`. */
export const REACH_MAX = 500;

export const resourceText = (r: ResourceRef): string => `${r.kind}:${r.id}`;
