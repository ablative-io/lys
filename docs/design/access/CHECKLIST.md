# Access — Checklist

## Grant mode and drafts

- [ ] **C601** — A grant carries one mode: outright, by_draft or by_two; the codec, events and snapshots version for it and old bytes stay readable (ACCESS-001 R1).
- [ ] **C602** — An app schema marks each action hot or deliberate; Lys refuses a by_draft or by_two grant on a hot action by name (ACCESS-001 R2).
- [ ] **C603** — A product records a held act as a draft naming the grant, the target and the request digest; one approval executes by_draft, two distinct approvers execute by_two; the product reads approved drafts of its kinds and records done or refused-on-execution (ACCESS-001 R3).
- [ ] **C604** — The Access screens show a grant's mode, the drafts held for a product, and who approved each (ACCESS-001 R4).

## The pass

- [ ] **C605** — The access token for an app carries a rights claim: holder, kind, responsible person, and for that audience only, each right's resource, actions, mode and grant id, roles expanded (ACCESS-002 R1).
- [ ] **C606** — The pass lifetime is a setting with a stated default; refresh issues a new pass from live grants; a revoked grant is absent from the next pass (ACCESS-002 R2).
- [ ] **C607** — The key that signs passes is published with its id and rotates without invalidating passes already issued within their lifetime (ACCESS-002 R3).

## lys-pass

- [ ] **C608** — lys-pass verifies a pass offline against the published key, audience and time, and evaluates a right by placement reach and restricted children (ACCESS-003 R1).
- [ ] **C609** — lys-pass asks Lys for a deliberate route and refuses "Lys could not be asked" when it cannot, never falling back to the pass; hands a by_draft act to Lys and runs approved drafts (ACCESS-003 R2).
- [ ] **C610** — The one refusal shape names resource, action, the grant needed and the Lys request URL; the conformance fixtures ship in the crate and every product gate runs them (ACCESS-003 R3).

## Roles and reach

- [ ] **C611** — An app schema names roles as action bundles; a grant may name a role and the pass expands it; a change that widens a role by an app waits for the administrator (ACCESS-004 R1).
- [ ] **C612** — A placement may be restricted: the parent's relations do not flow to it and only an explicit grant reaches it (ACCESS-004 R2).

## Machines

- [ ] **C613** — A machine is the fifth identity and grant holder, made at join, answering to the administrator who issued its code; old bytes stay readable (ACCESS-005 R1).
- [ ] **C614** — The Network screen shows machines as holders and their grants; a machine never holds a kept responsibility (ACCESS-005 R2).

## Organisation-scoped admission and roots (ACCESS-009)

- [ ] **C800** — POST /people/admit and POST /people are judged by organisation.admit on the caller's own organisation through the grants engine; a body naming another organisation is refused organisation_not_own before any write; routes::administrator is not consulted.
- [ ] **C801** — judge_root's root authority is a function of the resource: organisation.grant on the caller's organisation for organisation-owned kinds, the install's person for install-level kinds; every root records its organisation; a resource rooted under another organisation is refused resource_owned_elsewhere while any grant on it lives.
- [ ] **C802** — A delegation to a recipient outside the issuer's organisation, directly or through an agent's, service account's, connector's or machine's owner, is refused recipient_outside_organisation; revocation keeps revocation.rs:71's rule.
- [ ] **C803** — People and Access show the caller's organisation and only its records, from the partitioned routes without client filtering, with the add-person and root acts shown by GET /authority's organisation.admit and organisation.grant; no screen branches on the install's own organisation.

## Revocation reaches hot routes through lys-pass (ACCESS-010)

- [ ] **C804** — lys-pass's RevocationView applies the change stream's baseline, changes and Ready in revision order, is not Ready on a gap, reset, unready or lost connection, resumes from its last revision, and keeps a revocation only within one pass lifetime (ACCESS-010 R1).
- [ ] **C805** — evaluate_with refuses a right with a revoked grant (grant_revoked) and every right of a holder cut off at or after the pass's iat (holder_cut_off) when the view is Ready, and decides from the pass alone, saying so, when it is not (ACCESS-010 R2).
- [ ] **C806** — Stopping an agent, suspending or retiring an identity and revoking an agent's passes each write a holder cut-off to the grant log, carried as a holder_cut_off frame and in the baseline (ACCESS-010 R3).
- [ ] **C807** — D3 says a revoke reaches every product when the change stream delivers it, and that before Ready a hot route is decided from an unexpired pass, with no number of seconds as the bound (ACCESS-010 R4).

## Requests that outlive the model (ACCESS-011)

- [ ] **C839** — GET /requests answers every request as it stands: one whose relation the current model does not define for its kind is listed with no actions and standing relation_unknown, refused on approve by name, declinable, and shown on the page as one that cannot be granted; no single request ever refuses the queue (ACCESS-011 R1 to R3).

## Schema decisions reach the app (ACCESS-012)

- [ ] **C873** — lys-identity's change stream offers a read following schemas (schemas, decided) and the frames schema_standing and schema_decided (outcome approved, declined or applied); ChangeFrame and ChangesRequest are non_exhaustive with constructors; a read that does not follow schemas is the same bytes as in 0.3.2; lys-identity is 0.4.0 (ACCESS-012 R1).
- [ ] **C874** — POST /grants/changes answers an app following schemas with its own schema standing on a first read and each later decision on its schema in order from its cursor, wakes a waiting read on the decision's signal, resets a cursor past the app's decisions, and refuses the administrator by name; the decisions are indexed once, never replayed per read (ACCESS-012 R2).
- [ ] **C875** — lys-identity 0.4.0 is published and tagged the day its gate is green, and liminal's lead is told the same minute (ACCESS-012 R3).
