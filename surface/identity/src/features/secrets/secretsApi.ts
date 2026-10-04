/**
 * The secrets routes beyond the listing, typed as the secrets broker writes
 * its answers. The identity service forwards each one to the broker
 * unchanged, on the signed-in person's behalf.
 */
import { request } from '../../api';

/**
 * One grant: who may do what with which secret, and who granted it.
 * `relation` is the text the broker's grants file holds; the broker writes
 * `use`, `read`, `lend` or `member`, and `use` for a row that names none.
 */
export interface SecretGrant {
  identity: string;
  secret: string;
  relation: string;
  granted_by: string | null;
}

/** The answer to GET /secrets/grants: the grants on secrets the caller may discover. */
export interface SecretGrantListing {
  grants: SecretGrant[];
}

/** What an audit line records, as the broker labels it. */
export type SecretAuditKind =
  | 'issue'
  | 'use'
  | 'drop'
  | 'seal'
  | 'rotation'
  | 'next_account'
  | 'settlement'
  | 'sealed_read'
  | 'refresh'
  | 'spawn_login';

/** One checked audit line, on a secret the caller may discover. */
export interface SecretAuditLine {
  /** The line's place in the log, counting from 0 at the first line. */
  index: number;
  kind: SecretAuditKind;
  /** The broker's clock, in milliseconds since the epoch. */
  at_ms: number;
  /** The handle's id, never the handle itself. */
  handle: string | null;
  identity: string | null;
  secret: string | null;
  /** The call's operation id, in hex. */
  operation: string | null;
  /** The lease's use count after this line. */
  uses: number | null;
  /** `issued`, `admitted`, `dropped`, or a refusal's name. */
  outcome: string;
}

/** The answer to GET /secrets/audit: the lines in log order, oldest first. */
export interface SecretAuditLog {
  /** The audit log's public verifying key, in hex. Not shown on screen. */
  verified_by: string;
  lines: SecretAuditLine[];
}

/** Where the provider's revocation of the grant behind a handle stands. */
export type UpstreamRevocation = 'not_asked' | 'unconfirmed' | 'confirmed';

/** The answer to GET /secrets/revocation?handle=<id>. */
export interface RevocationAnswer {
  handle: string;
  /** Whether use has stopped here: the handle, or one above it, dropped. */
  stopped_here: boolean;
  upstream: UpstreamRevocation;
  /** The provider's reason, only when `upstream` is `unconfirmed`. */
  upstream_reason: string | null;
}

/** The kind of scope a secret is set to, as the change is written. */
export type ScopeKind = 'personal' | 'team' | 'organisation';

/** The answer to POST /secrets/scope. */
export interface ScopeChanged {
  operation: string;
  repeated: boolean;
  secret: string;
  /** The new scope as the broker names it: `person/<id>`, `team/<name>` or `organisation/<name>`. */
  scope: string;
}

/** Who a secret may be handed to. */
export type Recipients = 'anyone' | 'people_only';

/** The answer to POST /secrets/recipients. */
export interface RecipientsChanged {
  operation: string;
  repeated: boolean;
  secret: string;
  recipients: Recipients;
}

/** GET /secrets/settings: discoverable metadata only, never a secret value. */
export interface SecretSettings {
  last_operation: string | null;
  secret: string;
  scope: string | null;
  recipients: Recipients;
}

/** The scope a change asks for, written `<kind>:<name>` as the broker reads it. */
export const scopeText = (kind: ScopeKind, name: string): string => `${kind}:${name}`;

/** The kinds of secret a person adds a value under. */
export type AddedClass = 'credential' | 'key';

/** POST /secrets/add: a secret with its value and where it is used. The value is sent once and never kept or shown. */
export interface SecretAdded {
  secret: string;
  class: AddedClass;
  owner: string;
  sequence: number;
  operation: string;
  repeated: boolean;
}

/** POST /secrets/replace: the secret's new recorded change. */
export interface SecretReplaced { secret: string; sequence: number; operation: string; repeated: boolean }

/** POST /secrets/retire: when the secret was retired, in milliseconds since the epoch. */
export interface SecretRetired { secret: string; retired_at: number; operation: string; repeated: boolean }

/** What a new secret is, without its value: what a pending addition keeps. */
export interface SecretAsked { name: string; class: AddedClass; upstream: string; header: string; prefix: string }

/** One identity's draws on a model account: the calls the broker counted and the draws still live. */
export interface ModelDraw { identity: string; calls: number; live: number }

/** One model account as GET /model-accounts lists it. No token. */
export interface ModelAccount {
  account: string;
  name: string;
  harness: string;
  registered_by: string;
  registered_at: number;
  retired_at: number | null;
  draws: ModelDraw[];
}

/** POST /model-accounts: the account registered. */
export interface ModelRegistered {
  account: string; name: string; harness: string; registered_by: string; registered_at: number; operation: string; repeated: boolean;
}

/** POST /model-accounts/retire. */
export interface ModelRetired { account: string; name: string; retired_at: number; operation: string; repeated: boolean }

export const secretsApi = {
  settings: (secret: string) => request<SecretSettings>('/secrets/settings?secret=' + encodeURIComponent(secret)),
  grants: () => request<SecretGrantListing>('/secrets/grants'),
  audit: () => request<SecretAuditLog>('/secrets/audit'),
  revocation: (handle: string) => request<RevocationAnswer>('/secrets/revocation?handle=' + encodeURIComponent(handle)),
  scope: (secret: string, kind: ScopeKind, name: string, operation: string) =>
    request<ScopeChanged>('/secrets/scope', { secret, scope: scopeText(kind, name), operation }),
  recipients: (secret: string, recipients: Recipients, operation: string) =>
    request<RecipientsChanged>('/secrets/recipients', { secret, recipients, operation }),
  add: (asked: SecretAsked, value: string, operation: string) =>
    request<SecretAdded>('/secrets/add', { operation, ...asked, value }),
  replace: (secret: string, value: string, operation: string) =>
    request<SecretReplaced>('/secrets/replace', { operation, secret, value }),
  retire: (secret: string, operation: string) =>
    request<SecretRetired>('/secrets/retire', { operation, secret }),
  accounts: () => request<{ accounts: ModelAccount[] }>('/model-accounts'),
  register: (name: string, harness: string, token: string, operation: string) =>
    request<ModelRegistered>('/model-accounts', { operation, name, harness, token }),
  retireAccount: (account: string, operation: string) =>
    request<ModelRetired>('/model-accounts/retire', { operation, account }),
};
