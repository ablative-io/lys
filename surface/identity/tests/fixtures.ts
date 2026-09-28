import type { AgentView, MeView, PeopleView, ReceiptAnswer } from '../src/generated';
import type { ActionBody, Grant, GrantModel, Permit, WhoBody } from '../src/generated/grants';

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
        { id: SCRIBE, display_name: "Scribe", state: 'active' },
        { id: COURIER, display_name: "Courier", state: 'registered' },
        { id: ARCHIVIST, display_name: "Archivist", state: 'suspended' },
      ],
    },
    {
      id: BEA, display_name: 'Bea (test person)', state: 'retired',
      agents: [
        { id: REVIEWER, display_name: "Reviewer", state: 'active' },
        { id: LAMPLIGHTER, display_name: "Lamplighter", state: 'retired' },
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
  display_name: "Scribe",
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
  display_name: "Reviewer",
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

export const ROOT_G = 'grant-' + hex(31);
export const LEDGER_G = 'grant-' + hex(32);
export const SCRIBE_G = 'grant-' + hex(33);
const at = (d: number, m: number) => new Date(2026, m - 1, d, 12, 0).getTime() / 1000;

const grant = (g: Partial<Grant> & Pick<Grant, 'id' | 'holder' | 'relation' | 'actions' | 'pass_on' | 'source'>): Grant => ({
  issuer: ADA, responsible: ADA, resource: { kind: 'project', id: 'identity' }, window: { starts_at: at(27, 9), ends_at: null },
  model_version: 1, operation: 'op-' + hex(200), revoked: false, revoked_at: null, revoked_revision: null, last_use: { seen: false }, ...g,
});

export const GRANTS: Grant[] = [
  grant({ id: ROOT_G, holder: ADA, relation: 'owner', actions: ['edit', 'grant', 'view'], pass_on: { kind: 'to', actions: ['edit', 'view'], recipients: ['agent'] }, source: null, window: { starts_at: at(27, 9), ends_at: at(27, 10) } }),
  grant({ id: LEDGER_G, holder: ADA, relation: 'viewer', actions: ['view'], pass_on: { kind: 'use_only' }, source: null, resource: { kind: 'project', id: 'ledger' } }),
  grant({ id: SCRIBE_G, holder: SCRIBE, relation: 'viewer', actions: ['view'], pass_on: { kind: 'use_only' }, source: ROOT_G, window: { starts_at: at(27, 9), ends_at: at(4, 10) }, last_use: { seen: true, at: at(27, 9), route: 'tool', use_event: 5 } }),
];

/** GET /grants/model: the relations the service's model defines. */
export const MODEL: GrantModel = { version: 3, relations: { editor: ['edit', 'view'], owner: ['edit', 'grant', 'view'], viewer: ['view'] } };

const permit = (path: string[], scope: string[]): Permit => ({ permitted: true, grant: path[path.length - 1], path, responsible: ADA, scope, model_version: 1, revision: 7 });

/** /grants/why for Ada: view and edit on project:identity through her root grant; anything else refused. */
export const why = (body: unknown): Answer => {
  const b = body as ActionBody;
  if (b.resource.id === 'identity') return ok(permit([ROOT_G], ['edit', 'grant', 'view']));
  if (b.resource.id === 'ledger' && b.action === 'view') return ok(permit([LEDGER_G], ['view']));
  return refused(409, 'NotHeld', `NotHeld: ${ADA} holds no grant of ${b.action} on project:${b.resource.id}`);
};

/** /grants/who: Ada on everything she holds, the scribe on view of project:identity. */
export const who = (body: unknown): Answer => {
  const b = body as WhoBody;
  const holders = [];
  if (b.resource.id === 'identity') holders.push({ ...permit([ROOT_G], ['edit', 'grant', 'view']), holder: ADA });
  if (b.resource.id === 'ledger' && b.action === 'view') holders.push({ ...permit([LEDGER_G], ['view']), holder: ADA });
  if (b.resource.id === 'identity' && b.action === 'view') holders.push({ ...permit([ROOT_G, SCRIBE_G], ['view']), holder: SCRIBE });
  return ok({ holders, revision: 7, complete: true, next: null });
};

export type Answer = { status: number; body: unknown };
/** A route's answer, or a function of the posted body for routes that are asked. */
export type Route = Answer | ((body: unknown) => Answer);
export const ok = (body: unknown): Answer => ({ status: 200, body });
export const refused = (status: number, refusal: string, reason: string): Answer => ({ status, body: { refusal, reason } });

/** Every route the screens read, answered as the service answers them. */
export const SERVICE: Record<string, Route> = {
  '/directory/people': ok(DIRECTORY),
  '/people': ok(OWN),
  ['/directory/agents/' + SCRIBE]: ok(SCRIBE_VIEW),
  ['/directory/agents/' + REVIEWER]: ok(REVIEWER_VIEW),
  '/receipts/4': ok(RECEIPTS[4]),
  '/receipts/5': ok(RECEIPTS[5]),
  '/me': ok(ME),
  '/roles': ok({ roles: [] }),
  '/authority': ok('Step 1 of the directory has one administrator.'),
  '/grants': ok({ grants: GRANTS, revision: 7 }),
  '/grants/model': ok(MODEL),
  'POST /grants/why': why,
  'POST /grants/who': who,
  'POST /grants': (body) => ok({ operation: (body as { operation: string }).operation, grant: 'grant-' + hex(34), index: 3, receipt: { caller: ADA } }),
};
