import type { AgentView, CannotGiveAnswer, MeView, PeopleView, ReceiptAnswer } from '../src/generated';
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

/**
 * A grant as the service answers it: none of these is revoked and each is held
 * by an active identity on a chain admission admits, so each stands and its
 * effective end is its own end.
 */
const grant = (g: Partial<Grant> & Pick<Grant, 'id' | 'holder' | 'relation' | 'actions' | 'pass_on' | 'source'>): Grant => {
  const span = g.window ?? { starts_at: at(27, 9), ends_at: null };
  return {
    issuer: ADA, responsible: ADA, resource: { kind: 'project', id: 'identity' }, window: span,
    model_version: 1, operation: 'op-' + hex(200), revoked: false, revoked_at: null, revoked_revision: null,
    last_use: { seen: false, recorded: 0, source: 'reported' },
    standing: { stands: true }, effective_ends_at: span.ends_at, ...g,
  };
};

export const GRANTS: Grant[] = [
  grant({ id: ROOT_G, holder: ADA, relation: 'owner', actions: ['edit', 'grant', 'view'], pass_on: { kind: 'to', actions: ['edit', 'view'], recipients: ['agent'] }, source: null, window: { starts_at: at(27, 9), ends_at: at(27, 10) } }),
  grant({ id: LEDGER_G, holder: ADA, relation: 'viewer', actions: ['view'], pass_on: { kind: 'use_only' }, source: null, resource: { kind: 'project', id: 'ledger' } }),
  grant({ id: SCRIBE_G, holder: SCRIBE, relation: 'viewer', actions: ['view'], pass_on: { kind: 'use_only' }, source: ROOT_G, window: { starts_at: at(27, 9), ends_at: at(4, 10) }, last_use: { seen: true, at: at(27, 9), route: 'tool', use_event: 5, recorded: 1, source: 'reported' } }),
];

/**
 * GET /grants/cannot-give for Ada giving from her root grant to the scribe, as
 * the service answers it: her use-only ledger grant, and her sign-in identity.
 * Her root grant covers every relation of the model, so no relation is listed.
 */
export const CANNOT_GIVE_ROOT_SCRIBE: CannotGiveAnswer = {
  source: ROOT_G,
  recipient: SCRIBE,
  items: [
    { subject: 'grant', grant: LEDGER_G, reason: 'use_only', source: false },
    { subject: 'sign_in_identity', reason: 'sign_in_identity', source: false },
  ],
};

/** GET /grants/model: the relations the service's model defines. */
export const MODEL: GrantModel & { withheld_from_agents: string[] } = {
  action_sentences: { view: 'View this resource', edit: 'Edit this resource', grant: 'Give access to this resource' }, version: 3,
  relations: { editor: ['edit', 'view'], owner: ['edit', 'grant', 'view'], viewer: ['view'], 'only.edit': ['edit'] },
  withheld_from_agents: ['grant'],
};

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

/** /grants/reach: the /grants/who answers above, every resource and action at once. */
export const reach = (body: unknown): Answer => {
  const b = body as { resources: { kind: string; id: string; actions: string[] }[] };
  const resources = b.resources.map(({ kind, id, actions }) => {
    const byHolder = new Map<string, string[]>();
    for (const action of actions) {
      const page = who({ route: 'browser', resource: { kind, id }, action, page_size: 100, after: null }).body as { holders: { holder: string }[] };
      for (const { holder } of page.holders) byHolder.set(holder, [...(byHolder.get(holder) ?? []), action]);
    }
    return { kind, id, holders: [...byHolder].map(([holder, held]) => ({ holder, actions: held })) };
  });
  return ok({ revision: 7, resources });
};

export type Answer = { status: number; body: unknown };
/** A route's answer, or a function of the posted body for routes that are asked. */
export type Route = Answer | ((body: unknown) => Answer);
export const ok = (body: unknown): Answer => ({ status: 200, body });
export const refused = (status: number, refusal: string, reason: string): Answer => ({ status, body: { refusal, reason } });

/** The commit the stubbed service says it was built from. */
export const BUILD = '0123456789abcdef0123456789abcdef01234567';

/** Every route the screens read, answered as the service answers them. */
export const SERVICE: Record<string, Route> = {
  '/directory/people': ok(DIRECTORY),
  '/people': ok(OWN),
  ['/directory/agents/' + SCRIBE]: ok(SCRIBE_VIEW),
  ['/directory/agents/' + REVIEWER]: ok(REVIEWER_VIEW),
  '/receipts/4': ok(RECEIPTS[4]),
  '/receipts/5': ok(RECEIPTS[5]),
  '/me': ok(ME),
  '/me/account': ok({ email: 'ada@example.test', enabled: true }),
  '/roles': ok({ roles: [] }),
  '/teams': ok({ teams: [] }),
  '/runtime/live': ok({ sessions: [], unanswered: [] }),
  '/requests': ok({ requests: [] }),
  '/reviews': ok({ scope: 'personal', due: [], unanswered: [], revision: 0, judged_at: 1790000000 }),
  '/runtime/found': ok({ sessions: [] }),
  '/runtime/sessions': ok({ sessions: [] }),
  '/service-accounts': ok({ scope: 'personal', service_accounts: [] }),
  ['/agents/' + SCRIBE + '/runtime/sessions']: ok({ sessions: [] }),
  ['/agents/' + REVIEWER + '/runtime/sessions']: ok({ sessions: [] }),
  '/authority': ok({ authority: 'Step 1 of the directory has one administrator.', build: BUILD }),
  '/grants': ok({ grants: GRANTS, revision: 7 }),
  '/apps': ok({ apps: [] }),
  '/grants/model': ok(MODEL),
  'POST /grants/why': why,
  'POST /grants/who': who,
  'POST /grants/reach': reach,
  'POST /grants': (body) => ok({ operation: (body as { operation: string }).operation, grant: 'grant-' + hex(34), index: 3, receipt: { caller: ADA } }),
  [`/grants/cannot-give?route=browser&source=${ROOT_G}&recipient=${SCRIBE}`]: ok(CANNOT_GIVE_ROOT_SCRIBE),
};

// The second test person. Ada above is the first; Bea here is signed in through
// her own session, holds her own grants and answers for her own agent, so a test
// can switch the signed-in person and prove the personal data changed with it.
// Nothing above is reused for her: every id, grant and end date is her own.

/** Bea, active and signed in, with her one active agent. */
export const BEA_PERSON = {
  id: BEA,
  display_name: 'Bea (test person)',
  state: 'active' as const,
  agents: [{ id: REVIEWER, display_name: 'Reviewer', state: 'active' as const }],
};

/** The directory as an administrator sees it with Bea active: both people. */
export const BEA_DIRECTORY: PeopleView = { scope: 'directory', people: [DIRECTORY.people[0], BEA_PERSON] };

/** Bea's own scope: herself and her agent, never Ada. */
export const BEA_OWN: PeopleView = { scope: 'personal', people: [BEA_PERSON] };

/** GET /me for Bea's session: her own person and her own sign-in account. */
export const BEA_ME: MeView = {
  person: { id: BEA, display_name: 'Bea (test person)', state: 'active' },
  signed_in: { provider: ISSUER, subject: 'bea' },
  sign_in_identities: [{ provider: ISSUER, subject: 'bea' }],
  service_accounts: [],
};

export const BEA_ROOT_G = 'grant-' + hex(41);
export const BEA_REVIEWER_G = 'grant-' + hex(42);

/** 15 November 2026: Bea's inherited end, chosen to differ from Ada's 27 October. */
export const BEA_ENDS = at(15, 11);

/** Bea's grants: her root on project:ledger, and what her agent holds under it. */
export const BEA_GRANTS: Grant[] = [
  grant({
    id: BEA_ROOT_G, issuer: BEA, holder: BEA, responsible: BEA, relation: 'editor', actions: ['edit', 'view'],
    pass_on: { kind: 'to', actions: ['view'], recipients: ['agent'] }, source: null,
    resource: { kind: 'project', id: 'ledger' }, window: { starts_at: at(27, 9), ends_at: BEA_ENDS },
  }),
  grant({
    id: BEA_REVIEWER_G, issuer: BEA, holder: REVIEWER, responsible: BEA, relation: 'viewer', actions: ['view'],
    pass_on: { kind: 'use_only' }, source: BEA_ROOT_G,
    resource: { kind: 'project', id: 'ledger' }, window: { starts_at: at(27, 9), ends_at: BEA_ENDS },
  }),
];

/** Every route answered for Bea's session: her /me, her scope, her grants. */
export const BEA_SERVICE: Record<string, Route> = {
  ...SERVICE,
  '/me': ok(BEA_ME),
  '/people': ok(BEA_OWN),
  '/directory/people': ok(BEA_DIRECTORY),
  '/grants': ok({ grants: BEA_GRANTS, revision: 9 }),
};
