import type { AgentView, MeView, PeopleView, ReceiptAnswer } from '../src/generated';

// Fixture API responses in the shapes read_api.rs answers. Names are test names.

const hex = (n: number) => n.toString(16).padStart(32, '0');
export const ADA = 'person-' + hex(1);
export const BEA = 'person-' + hex(2);
export const SCRIBE = 'agent-' + hex(11);
export const COURIER = 'agent-' + hex(12);
export const ARCHIVIST = 'agent-' + hex(13);
export const REVIEWER = 'agent-' + hex(21);
export const LAMPLIGHTER = 'agent-' + hex(22);

export const ISSUER = 'http://issuer.test:9000';

export const DIRECTORY: PeopleView = {
  scope: 'directory',
  people: [
    {
      id: ADA, display_name: 'Ada (test person)', state: 'active',
      agents: [
        { id: SCRIBE, display_name: "Ada's scribe", state: 'active' },
        { id: COURIER, display_name: "Ada's courier", state: 'registered' },
        { id: ARCHIVIST, display_name: "Ada's archivist", state: 'suspended' },
      ],
    },
    {
      id: BEA, display_name: 'Bea (test person)', state: 'retired',
      agents: [
        { id: REVIEWER, display_name: "Bea's reviewer", state: 'active' },
        { id: LAMPLIGHTER, display_name: "Bea's lamplighter", state: 'retired' },
      ],
    },
  ],
};

export const OWN: PeopleView = { scope: 'personal', people: [DIRECTORY.people[0]] };

const receipt = (index: number, kind: number, at: number): ReceiptAnswer => ({
  receipt: {
    version: 1,
    operation: 'op-' + hex(100 + index),
    actor: { issuer: ISSUER, subject: 'ada', authenticated_at: at },
    identity: SCRIBE,
    change_kind: kind,
    payload_commitment: 'aa'.repeat(32),
    payload_commitment_hash: 'sha-256',
    log: { index, tree_size: index + 1, root: 'bb'.repeat(32), leaf_hash: 'cc'.repeat(32) },
  },
  message: 'dd',
  checkpoint: { tree_size: 10, root: 'ee'.repeat(32) },
  inclusion_proof: 'ff',
});

// 22 September 2026, 09:14 and 09:20 local time.
const REGISTERED = new Date(2026, 8, 22, 9, 14).getTime() / 1000;
const ACTIVATED = new Date(2026, 8, 22, 9, 20).getTime() / 1000;

export const RECEIPTS: Record<number, ReceiptAnswer> = { 4: receipt(4, 2, REGISTERED), 5: receipt(5, 5, ACTIVATED) };

export const SCRIBE_VIEW: AgentView = {
  id: SCRIBE,
  display_name: "Ada's scribe",
  person: { id: ADA, display_name: 'Ada (test person)', state: 'active' },
  needs_new_person: false,
  role: null,
  version: null,
  state: 'active',
  provenance: { registered_by: { provider: ISSUER, subject: 'ada' }, events: [4, 5], registration: RECEIPTS[4].receipt },
};

export const REVIEWER_VIEW: AgentView = {
  ...SCRIBE_VIEW,
  id: REVIEWER,
  display_name: "Bea's reviewer",
  person: { id: BEA, display_name: 'Bea (test person)', state: 'retired' },
  needs_new_person: true,
  provenance: { registered_by: { provider: ISSUER, subject: 'ada' }, events: [], registration: null },
};

export const ME: MeView = {
  person: { id: ADA, display_name: 'Ada (test person)', state: 'active' },
  signed_in: { provider: ISSUER, subject: 'ada' },
  sign_in_identities: [{ provider: ISSUER, subject: 'ada' }],
  service_accounts: [],
};

export type Answer = { status: number; body: unknown };
export const ok = (body: unknown): Answer => ({ status: 200, body });
export const refused = (status: number, refusal: string, reason: string): Answer => ({ status, body: { refusal, reason } });

/** Every route the screens read, answered as the service answers them. */
export const SERVICE: Record<string, Answer> = {
  '/directory/people': ok(DIRECTORY),
  '/people': ok(OWN),
  ['/directory/agents/' + SCRIBE]: ok(SCRIBE_VIEW),
  ['/directory/agents/' + REVIEWER]: ok(REVIEWER_VIEW),
  '/receipts/4': ok(RECEIPTS[4]),
  '/receipts/5': ok(RECEIPTS[5]),
  '/me': ok(ME),
  '/authority': ok('Step 1 of the directory has one administrator.'),
};
