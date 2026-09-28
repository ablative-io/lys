// The directory's JSON as crates/lys-identity-server answers it. Each type
// mirrors the Rust that writes it, named beside it: the screens' views are
// crates/lys-identity-server/src/read_api.rs. Change them together.

/** `IdentityId` text form: `person-` or `agent-` and 32 lowercase hex digits. */
export type IdentityId = string;

/** `LifecycleState`, as its Display writes it. */
export type LifecycleState = 'registered' | 'active' | 'suspended' | 'retired';

export const LIFECYCLE_STATES: readonly LifecycleState[] = ['registered', 'active', 'suspended', 'retired'];

/** `login_json`: a sign-in identity, the issuer as its provider. */
export interface Login {
  provider: string;
  subject: string;
}

/** `person_summary`. */
export interface PersonSummary {
  id: IdentityId;
  display_name: string;
  state: LifecycleState;
}

/** An agent in `agents_of`. */
export type AgentSummary = PersonSummary;

/** `person_with_agents`. */
export interface PersonWithAgents extends PersonSummary {
  agents: AgentSummary[];
}

/** Whose records a view shows: the signed-in person's own, or the administrator's whole directory. */
export type Scope = 'personal' | 'directory';

/** GET /people and GET /directory/people. */
export interface PeopleView {
  scope: Scope;
  people: PersonWithAgents[];
}

/**
 * A service account the person may use. The directory records none yet and
 * answers an empty list; these members are the mock-up's columns, to be
 * fixed with the server when it records them.
 */
export interface ServiceAccount {
  id: string;
  owner: string;
  name: string;
  description: string;
  state: string;
  created_by: Login;
  created_at: number;
  retired_by: Login | null;
  retired_at: number | null;
}

/** GET /me. */
export interface MeView {
  person: PersonSummary;
  signed_in: Login;
  sign_in_identities: Login[];
  service_accounts: ServiceAccount[];
}

/** GET /agents/{id} and GET /directory/agents/{id}, from `agent_view`. */
export interface AgentView {
  id: IdentityId;
  display_name: string;
  person: PersonSummary;
  needs_new_person: boolean;
  /** Not recorded by the directory yet; always null for now. */
  role: string | null;
  /** Not recorded by the directory yet; always null for now. */
  version: number | null;
  state: LifecycleState;
  provenance: {
    registered_by: Login;
    events: number[];
    registration: Receipt | null;
  };
}

/** `ServerError` as its IntoResponse writes it. */
export interface Refusal {
  refusal: string;
  reason: string;
}

/** `receipt_json`. */
export interface Receipt {
  version: number;
  operation: string;
  actor: { issuer: string; subject: string; authenticated_at: number };
  identity: IdentityId;
  change_kind: number;
  payload_commitment: string;
  payload_commitment_hash: string;
  log: { index: number; tree_size: number; root: string; leaf_hash: string };
}

/** GET /receipts/{index} */
export interface ReceiptAnswer {
  receipt: Receipt;
  message: string;
  checkpoint: { tree_size: number; root: string };
  inclusion_proof: string;
}

/** `event::wire` change kinds. */
export const CHANGE_KINDS: Readonly<Record<number, string>> = {
  7: 'Setup completed',
  1: 'registered',
  2: 'registered, under its person',
  3: 'profile changed',
  4: 'login bound',
  5: 'lifecycle moved',
  6: 'link audit accepted',
};

/** GET /callback */
export interface SignedIn {
  signed_in: { issuer: string; subject: string };
  authority: string;
}

export type IdentityKind = 'person' | 'agent';

export const kindOf = (id: IdentityId): IdentityKind => (id.startsWith('agent-') ? 'agent' : 'person');

/** GET /identities/{id}: record_json in crates/lys-identity-server/src/routes.rs. Administrator only. */
export interface DirectoryRecord {
  id: IdentityId;
  display_name: string;
  state: LifecycleState;
  responsible: IdentityId | null;
  logins: { issuer: string; subject: string }[];
  events: number[];
}

// The start route's JSON (DIRECTORY-029). The library writes every byte of
// it, in crates/lys-identity/src/start/ (give.rs, error.rs, checks.rs and
// state.rs), and crates/lys-identity-server/src/start.rs answers it as it
// came. Change them together.

/** `Check::name`, the five checks in the order a start runs them (`Check::ALL`). */
export const START_CHECKS: readonly string[] = [
  'the agent is active',
  'its profile version is reviewed',
  'the machine is allowed for the role',
  'its virtual credentials are valid',
  'the machine may reach what the profile needs',
];

/** One check as `checks_json` writes it: `passed`, or the refusal's name. */
export interface StartCheck {
  check: string;
  result: string;
  words: string;
}

/** `record_json`: a kept launch record. It holds ids, never a credential value. */
export interface LaunchRecordView {
  id: string;
  agent: string;
  machine: string;
  executable: string;
  working_directory: string;
  profile_version: string;
  given_by: string;
  arguments: string[];
  credential_ids: string[];
  given_at: number;
  copied_from: string | null;
}

/** `LaunchState::name`. */
export type LaunchStateName = 'unconfirmed' | 'running' | 'withdrawn';

/** `Given::to_json`: POST /agents/{id}/start and POST /launch-records/{id}/start-again. */
export interface StartGiven {
  command: string;
  working_directory: string;
  launch_record: LaunchRecordView;
  state: LaunchStateName;
  words: string;
  checks: StartCheck[];
}

/** `Refusal::to_json`. */
export interface StartRefusal {
  refusal: string;
  words: string;
}

/** `Refused::to_json`: a start refused, with every check that ran. */
export interface StartRefused {
  refused: StartRefusal[];
  checks: StartCheck[];
}

/** `StartError::to_json` for a failure that is not a refusal. */
export interface StartFailure {
  error: string;
  words: string;
}

/** `LaunchState::to_json`: GET /launch-records/{id}/state and POST /launch-records/{id}/withdraw. */
export interface LaunchStateView {
  launch_record: string;
  agent: string;
  state: LaunchStateName;
  words: string;
  session: string | null;
  withdrawal: { by: string; at: number } | null;
}
