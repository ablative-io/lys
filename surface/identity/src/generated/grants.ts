// The grant routes' JSON, as crates/lys-identity-server/src/grant_contract.rs
// writes and reads it. Every member of every request body is required.

import type { IdentityId } from './index';

/** `ResourceWire`, and a grant's resource. */
export interface ResourceRef {
  kind: string;
  id: string;
}

export type RecipientKind = 'person' | 'agent';

/** `pass_on_json` and `PassOnWire`: what a holder may pass on, stated affirmatively. */
export type PassOn = { kind: 'use_only' } | { kind: 'to'; actions: string[]; recipients: RecipientKind[] };

/** `WindowWire`; `ends_at` null means no end of its own. */
export interface GrantWindow {
  starts_at: number;
  ends_at: number | null;
}

/** `grant_json`. */
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
  revoked: boolean;
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

/** `recorded_json`: a recorded change and its receipt. */
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

/** `permit_json`: a permitted decision and the authority path it rests on, root first. */
export interface Permit {
  permitted: true;
  grant: string;
  path: string[];
  responsible: IdentityId;
  scope: string[];
  model_version: number;
  revision: number;
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

export const resourceText = (r: ResourceRef): string => `${r.kind}:${r.id}`;
