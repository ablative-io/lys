# Permissions across the stack: Lys, Cambium, aion, haematite, liminal

Waffles, 6 October 2026, on Tom's word of 11:3x the same day: "implement permissions across the stack
(aion, cambium, haematite, and liminal which will essentially replace argus++) ... think really carefully
about how that's designed including how we best support its maintenance and growth in the short and long
term ... something that wasn't a complete nightmare to operate." This page is the design the briefs hang
off: one in Lys (the authority), one in each product. Every claim about today's code names its file.

## 1. What is true today

- Lys holds the identities: person, agent, service account, and an approved app's connector
  (lys-identity id.rs:222-230). A computer with a runner joins with a one-use code and its own key
  (network_join.rs) but is not an identity and holds no grant.
- A Lys grant names an issuer, a holder, the responsible person, ONE resource (`kind:id`), a relation,
  a set of actions, whether it may be passed on, its source and a time window (grants/types.rs:300-324).
  There is no pattern resource and no "needs a draft" or "needs two approvals" on a grant.
- Every approved app has a versioned permission schema in Lys: kinds, relations, actions, and the
  parents a kind may be placed in, so relations held on a parent flow to the child; a change that would
  strand a standing grant is refused by name (apps_schema_api.rs head). This is the model each product
  will register itself in. It already exists and is the right place.
- Drafts are their own signed records: created, approved, refused, corrected, with a target
  `{kind, id, action}` (drafts_api.rs). Nothing today says which acts need one.
- Products ask Lys `POST /grants/check/batch` and `GET /grants/which`. Lys answers questions; it issues
  nothing a product can verify by itself. Lys's OIDC id token carries `iss, sub, aud, iat, exp, auth_time`
  and no rights (provider/endpoints.rs:480-500).
- Cambium has its own participants, passes and roles (none enforced: any signed-in caller may invite;
  a new invited person sees every channel; PERMISSIONS-PROPOSAL-2026-10-05.md). aion has namespaces as
  its tenancy ("a namespace you do not hold is answered as not-found"). haematite has databases and a
  Studio; liminal has a shared token or Ed25519 wire passes that cambium-door mints. Argus registers
  clients and says plainly "registration grants no additional authority": it has no authority model.

## 2. The decisions

D1. **One authority.** Lys holds every identity and every grant. No product keeps a role table of its
own that is not derived from Lys. A product's permission model is DATA registered in Lys (its app
schema), not code in Lys: adding a product, a kind or an action is a schema version in that product's
repository, never a Lys release.

D2. **Two ways to decide, chosen per route, never per request.** Every product route is classified
once, in a committed matrix in that product's repository, as exactly one of:
- *hot*: decided from the caller's pass alone, offline, no call to Lys (read, post, publish, subscribe,
  a worker taking a task);
- *deliberate*: decided by asking Lys live (`/grants/check/batch`) with the pass as the caller, so a
  revocation is immediate (invite, retire a seat, configure, delete, deploy a document, cancel a run,
  stop an agent, send keys to a session, open a federation link, restore a backup).
A deliberate route NEVER falls back to the pass when Lys cannot be asked: it refuses with "Lys could not
be asked" and the act is not done. A hot route never asks Lys. The gate of every product fails when a
route is missing from the matrix (section 5).

D3. **The pass is Lys's access token with a rights claim.** Lys's OIDC flow is the one sign-in for people
AND agents (an agent proves itself with its key where a person uses Google; the token is the same shape).
The access token for an app carries, for that app only (`aud`): the holder, its kind, the responsible
person, and a list of rights `{resource: kind, id-or-prefix, actions, mode, grant}`. Short-lived
(minutes, a setting; refreshed by the product's client library). Signed by Lys with the key its
`/.well-known` publishes. Products verify offline. A revoked grant stops at the next refresh for hot
routes and at once for deliberate routes. Liminal's existing wire pass remains, minted by cambium-door
from the holder's Lys rights, until liminal verifies Lys's token directly (its brief says when).

D4. **A grant carries its mode.** `outright` (the act happens), `by_draft` (the product writes the act as
a Lys draft, answers "held for approval" with the draft id, and does it after one approval), `by_two`
(the same with two distinct approvers). One mode per grant; "Archie may stop seats, by draft" is one
grant. This is Tom's rule to Hermes in Lys terms: "you can do whatever your grant allows without it having
to be a draft first", with the grant saying what needs a draft or two approvals. A by_draft or by_two
right never applies to a hot route (the matrix forbids it; Lys refuses to issue such a grant on a hot
action, which the schema marks).

D5. **Draft execution is pull, not push.** Lys calls no product back. Each product has one draft runner
that reads Lys for approved drafts of its own kinds, does them, and records done or refused-on-execution
in Lys. A draft nobody executes is visible on the Lys dashboard ("drafts to approve", "requests to
decide"), never silent.

D6. **Resources are hierarchical, never patterns.** A grant on a parent reaches the children placed in
it (Lys placements, today). Liminal subjects `orders.created` are placed under `orders`; a workspace's
places under the workspace; a namespace's runs under the namespace; a database's collections under the
database. A private place is a child with `restricted = true` in its placement: the parent's relations
do not flow; only an explicit grant on it reaches it. No wildcard ever enters SpiceDB.

D7. **Roles are named bundles in the product's schema.** `cambium.workspace.administrator = {read,
post, edit, delete, configure, invite, seat_retire, huddle_moderate}`; `aion.namespace.operator = {...}`.
A grant may name a role; the pass expands it to actions at issue. A product deploy that widens a role is
a schema change by the app's connector, which waits for the administrator on the Apps screen (today's
rule), so a widening is reviewed before anyone holds it.

D8. **A fifth identity: the machine.** A computer or device (a runner's machine, a liminal node) becomes
a grant holder, answering to the administrator who issued its join code as its responsible person. Its
key pair is the one `POST /runner/join` already records. Federation links hold export/import grants on
subject prefixes with the machine as holder. Until this exists, no product designs around a device
holding rights (Hermes, 10:06).

D9. **One client library, `lys-pass`, in the Lys repository**, consumed by every product by pinned
commit: token fetch and refresh, offline verification against Lys's published key, rights evaluation
(hierarchy and prefix), the deliberate-route check, the draft hand-off and runner, the one refusal shape
(`{refused: {resource, action, needed: grant, request: <Lys URL>}}`), and the conformance fixtures
(section 5). Four products implementing the same thing four times is where the drift and the nightmare
would come from.

D10. **Availability is a deployment property, stated, not a silent fallback.** A product keeps honouring
an unexpired pass for hot routes when Lys is unreachable and refuses every deliberate route; past the
pass lifetime everybody is refused by name. That is written in each product's design page and shown in
its status screen ("Lys last answered at ..."). The owner always has the Lys sign-in itself (Google) and
Lys's administrator role is held by at least one person; that is the break-glass, and it is recorded.

D11. **Every act under a grant is receipted with the grant id**, in the product's own log and, for
deliberate acts, in Lys (grant receipts exist). "Who did this and under what" is answerable from Lys for
every deliberate act and from the product for every hot act.

## 3. Argus++ becomes liminal services under the same rules

What Argus does today (context watch, seats start/stop, budgets, alarms, launches, queue, session keys,
prompts) becomes liminal services with subjects (`seats.<name>`, `sessions.<id>.keys`, `budgets.<name>`,
`alarms`, `launches`). Rights on those subjects are Lys grants like any other. Sending keys to a session
is acting as that agent: its default mode in the schema is `by_draft`, and `outright` is held only via
the administrator role. Lys remains the policy (budgets, goals, stops, "Stop all" already live in Lys);
liminal carries the bus; nothing in liminal decides authority on its own.

## 4. What stays small so it stays operable

- One screen to see who can do what: Lys Access (who, reach, requests, drafts). Products link to it from
  every refusal.
- One matrix per product, in its repository, readable by a person: route, classification, resource kind,
  action, default mode. The gate holds it to the code.
- One library. One refusal shape. One way to request access (Lys requests).
- Roles for people to think in; actions for the code to check. Nobody assembles thirty actions by hand.
- Nothing is derived at a product's start-up from local files; a product with no Lys is a product with
  nobody signed in, said on its screen.

## 5. Conformance: how it does not rot

- `lys-pass` ships fixtures every product's gate runs: a pass with the wrong audience, an expired pass, a
  pass whose key is not Lys's, a right on a parent reaching a child, a restricted child not reached, a
  by_draft right refused on a hot route, the refusal shape byte for byte, the deliberate route refused when
  Lys is unreachable.
- Each product's gate: every route in the router appears in the matrix (a route not named fails the
  gate); for every matrix row a test proves refusal by name without the right and admission with it;
  the schema file the product registers equals the one in its repository (no drift between what Lys holds
  and what the code assumes).
- Lys's gate: a schema version that strands a grant is refused (exists); a grant with a by_draft mode on
  a hot action is refused; the token's rights claim round-trips through `lys-pass`.

## 6. Order of work (each is a brief; the ids are given in the briefs themselves)

1. Lys: grant mode; rights in the access token; `lys-pass`; hot/deliberate marking on schema actions;
   restricted placements; roles in the schema. (ACCESS-001 to ACCESS-004 in this repository, cluster docs/design/access.)
2. Lys: the machine identity (ACCESS-005).
3. Cambium: sign in through Lys (081 installed first); permissions by Lys rights with the matrix; invite,
   seat and configure as deliberate routes; places as placements; the surface defects found on 6 October.
4. aion: namespaces as resources; start/cancel/signal/deploy deliberate; reads hot; workers as machines or
   service accounts.
5. haematite: databases as resources; Studio as an approved app; observer, writer, administrator roles;
   restore deliberate.
6. liminal: subjects as placed resources; publish/subscribe/call/serve hot from the pass; links as
   machine grants; the Argus services; cambium-door minting the wire pass from Lys rights until liminal
   verifies the token itself.

## 7. Not decided here (Tom's to steer, each named in its brief)

- The pass lifetime (I propose ten minutes, refreshed at half-life).
- Whether a person may hold `by_two` rights at all, or only agents and connectors.
- Whether haematite's Studio sits behind Lys as an approved app on its own origin or stays on a
  development proxy (nothing in Lys rules the same-origin pattern today).
