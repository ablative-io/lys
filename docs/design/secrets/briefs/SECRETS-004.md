---
type: brief
id: SECRETS-004
cluster: secrets
title: Say who may revoke a lease and let its holder relinquish it, and filter the secrets list by scope at the server (CONFORMANCE 7.6 and 7.8)
---

# SECRETS-004: Say who may revoke a lease and let its holder relinquish it, and filter the secrets list by scope at the server (CONFORMANCE 7.6 and 7.8)

> **Cluster:** secrets
> **Depends on:** SECRETS-002
> **Blocked by:** The lead's sign-off on the card before any row is built., R2 to R6 touch crates/lys-secrets, the broker's host under the host decision SECRETS-003 carries on brief/secrets/309540a4: they wait on `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0, and R5 and R6, which build on SECRETS-003 R4's proxy and its revocation states, also wait on `git cat-file -e origin/main:crates/lys-secrets/src/proxy.rs` exits 0; SECRETS-003 lands as one pull request, so R4 arrives with R3., R7 waits on the step-2 SpiceDB brief's evaluator file and on the server's route-building file: `git cat-file -e origin/main:crates/lys-identity-server/src/spicedb/check.rs` exits 0 and `git cat-file -e origin/main:crates/lys-identity-server/src/routes.rs` exits 0. Each check is keyed on the file, because the briefs that land them have no settled id.
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-095 — Ownership of a secret alone confers no revoke over the leases derived from it — The person acted for under a lease revokes it, and ownership of the secret the lease was issued from confers no revoke, over the owner revoking every credential derived from the secret, because who may revoke is a policy choice that ownership alone does not confer. Rejected: the owner revokes every credential derived from the secret.
> **Checklist:**
> - C30 — The person acted for under a lease may revoke it at any time: the revoke stops issuing at once, and GET /leases/{lease_id} reads upstream pending until the system behind confirms, with no timer confirming it (CONFORMANCE 7.6).
> - C31 — A revoke by anyone but the person acted for is refused: with the not-found refusal naming nothing when the caller cannot discover the lease, which only its person acted for and its holder can; for the lease's holder with a refusal naming the lease and relinquish; and with a refusal naming the lease for any other caller the seam lets discover it.
> - C32 — A lease's holder may relinquish it, recorded as a relinquish and not as a revoke, which stops issuing at once and reads upstream pending until the system behind confirms.
> - C33 — Whether a secret's owner may revoke every credential derived from it is recorded as a proposed ADR cited from this cluster's decisions, and no code and no test grants it.
> - C34 — The secrets list takes one scope from organisation, team and mine, applied at the server; with no scope it answers every secret the caller may see; any other scope is refused by name (CONFORMANCE 7.8).
> - C35 — The signed-in person's team ids are read from the group claims on their token, one group per team, through one function.
> - C36 — Every secret visibility and lease revoke check is a call on one seam, answering from the record's scope, team and owner fields until the step-2 SpiceDB evaluator exists, and asking SpiceDB through it once it does.
> - C37 — A revoke by the person acted for, or a relinquish by the holder, of a lease that has already ended is refused by name as already ended, carrying how, when and by whom it first ended, and records nothing; the first end record stays the only one (CONFORMANCE 7.6).
> - C38 — The secrets list is a person's view: an agent that asks for it is refused by name, the refusal naming the agent and saying that agents reach secrets only through their virtual credentials, with no list, not even an empty one, and nothing recorded (CONFORMANCE 7.8).
> **Stories:**
> - S19 (Person, Grants an agent provisioned under them access to an account) — As the person an agent acts for under a lease, I want to revoke the lease and see issuing stopped at once and the confirmation from the system behind shown pending until it comes, so that I know what has stopped and what is still unconfirmed.
> - S20 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent holding a lease, I want to give it back by a relinquish recorded as my own act, so that ending my own access is never recorded as a revoke nobody asked for.
> - S21 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person looking at the secrets list, I want to choose organisation, team or mine and have the server answer only what I may see within that scope, so that no filter ever shows me a secret I may not see.

## Purpose

Amends SECRETS-002 for two rows of docs/design/identity/CONFORMANCE.md: row 7.6, who may revoke a lease, in its test part, and row 7.8, the scope filter on the secrets list. The person acted for revokes, the holder relinquishes, and everyone else is refused without learning what they may not see; the secrets list answers one scope from a closed set of three, applied at the server. The open part of row 7.6, whether a secret's owner may revoke every credential derived from it, is recorded as proposed ADR-095, and nothing here grants it.

## Task

A new brief that depends on SECRETS-002 and leaves SECRETS-002's documents unedited. Each row names the SECRETS-002 requirement it extends: R6 for scopes and denials that do not leak, R7 for the revoke's two facts, SEC_REVOKE_STATES. SECRETS-002 R8's in-flight cancellation rule stays open and outside this acceptance. Split with SECRETS-002: C16 (scopes enforced in listing) and C18 (local refusal shown apart from unconfirmed upstream action) stay SECRETS-002's; C30 to C38 are this brief's and narrow them for rows 7.6 and 7.8.

Host: the broker's host is crates/lys-secrets, the host decision SECRETS-003 carries on brief/secrets/309540a4. No ADR id is cited for it: main's ledger ends at ADR-018, and the number is taken from main's ledger by whichever brief lands first. SECRETS-003 builds that crate as a library with no HTTP layer, so this brief adds the routes GET /secrets, GET /leases/{lease_id}, POST /leases/{lease_id}/revoke and POST /leases/{lease_id}/relinquish in crates/lys-secrets/src/api/, as a router the serving process mounts; the serving process and its sign-in are not this brief's. A signed-in person reaches a route from the mounting server's authentication; a lease's holder that is an agent, which does not sign in, reaches POST /leases/{lease_id}/relinquish and GET /leases/{lease_id} by the signed presentation SECRETS-003 defines for an agent's calls to the broker, verified against the key registered for the holder's identity, and no new way to authenticate is added. Tests insert the caller and their group claims directly.

Checks: every visibility and revoke check is a call on the one seam of R3, which answers from the record's scope, team and owner fields until the step-2 SpiceDB brief lands its evaluator, and R7 makes the seam ask SpiceDB through a `PermissionCheck` trait declared in crates/lys-secrets, which crates/lys-identity-server implements with the step-2 brief's one evaluator in crates/lys-identity-server/src/spicedb/check.rs and passes in where the routes are built, and through nothing else; crates/lys-secrets depends on no server crate. SECRETS-003's proxy use checks on a handle are not visibility or revoke checks, and this brief leaves them as they are. R5 and R6 build on SECRETS-003 R4: a lease's end moves through the revocation states R4 defines for SEC_REVOKE_STATES, with no second upstream state, and a use is measured at R4's proxy. Holding to DIRECTORY-002 R4 means the narrow thing: no SpiceDB permission check, relationship write or schema write is added under crates/lys/src/identity/.

In: the owner-revoke question as proposed ADR-095 (R1); team ids from the group claims through one function (R2); the one seam (R3); the scoped list (R4); the revoke and its refusals, and the refusal of an already ended lease (R5); the relinquish (R6); the refusal of the secrets list to an agent (R4); the seam asking SpiceDB (R7). Out, named as neighbours: the lease screen with its Revoke control, and the secrets list screen with its scope control of All, Organisation, Team and Mine. Out: the in-flight rule, upstream-grant withdrawal, emergency stop, a team model in the directory, and any grant to a secret's owner.

## Requirements

### R1: Record who may revoke beyond the person acted for as proposed ADR-095, and grant nothing under it

Extends SECRETS-002 R7, for the open part of CONFORMANCE row 7.6. The decision ledger carries ADR-095, status proposed, scope secrets: the person acted for under a lease revokes it, and ownership of the secret the lease was issued from confers no revoke; the rejected alternative is that the owner revokes every credential derived from the secret. The secrets design cites ADR-095 in its decisions by id. WHILE ADR-095 is proposed, THE SYSTEM SHALL keep the set of who may revoke a lease beyond the person acted for empty. THE SYSTEM SHALL NOT carry any code path, route or test that grants a secret's owner a revoke of a lease they are not acted for under, and this brief SHALL NOT mark ADR-095 decided or edit any other ledger entry.

**Acceptance:**
- docs/design/decisions.json holds exactly 1 entry with id ADR-095, its status reads `proposed`, and its decision names the owner revoking every credential derived from the secret as the rejected alternative.
- The decisions array of docs/design/secrets/design.json contains `ADR-095`.
- Every ledger entry from ADR-001 to ADR-018 is byte-identical to its text before this brief: `git diff` of docs/design/decisions.json shows added lines only, apart from the ledger's `updated` field.

**Files:**
- modify: docs/design/decisions.json
- modify: docs/design/secrets/design.json

**Checklist:**
- C33 — Whether a secret's owner may revoke every credential derived from it is recorded as a proposed ADR cited from this cluster's decisions, and no code and no test grants it.

### R2: Read the signed-in person's team ids from the group claims on their token, through one function

Extends SECRETS-002 R6. Blocked by: `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0. THE SYSTEM SHALL read the team ids of the signed-in person from the identity provider's group claims on their token (ADR-009), one group per team, through the one function `team_ids` in crates/lys-secrets/src/teams.rs, which takes the claims as its argument so that tests inject them. A directory brief that models teams replaces this source inside `team_ids` and nowhere else. THE SYSTEM SHALL NOT read the group claims anywhere but `team_ids`, SHALL NOT take a team id from a request's path, query or body, and SHALL NOT read an agent's team membership for any list: agents never use the secrets list, so an agent's team membership changes no list.

**Acceptance:**
- A test passes claims whose `groups` claim holds `team_a` and `team_b`: `team_ids` returns exactly the 2 ids `team_a` and `team_b`.
- A test passes claims whose `groups` claim is an empty array: `team_ids` returns 0 ids.
- `rg -n 'pub fn team_ids\(' crates/lys-secrets/src` prints exactly 1 line, in crates/lys-secrets/src/teams.rs.

**Files:**
- create: crates/lys-secrets/src/teams.rs
- create: crates/lys-secrets/src/teams_tests.rs
- modify: crates/lys-secrets/src/lib.rs

**Checklist:**
- C35 — The signed-in person's team ids are read from the group claims on their token, one group per team, through one function.

**Stories:**
- S21 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person looking at the secrets list, I want to choose organisation, team or mine and have the server answer only what I may see within that scope, so that no filter ever shows me a secret I may not see.

### R3: Answer every visibility and revoke check through one seam, from the record's scope, team and owner fields

Extends SECRETS-002 R6 and R7. Blocked by: `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0. Each secret record in the store of crates/lys-secrets/src/store.rs, the entry that already carries its owning identity, carries its scope, one of `personal`, `team` and `organisation`, and its team id when its scope is `team`; crates/lys-secrets/src/secret.rs keeps only the redacting type for credential bytes, and that type gains no field; secret.rs gains only the declaration of its test module. Each lease record in crates/lys-secrets/src/lease.rs carries its holder, its person acted for, and the secret it was issued from. THE SYSTEM SHALL answer every secret visibility check, every lease discovery check and every lease revoke check through the one seam in crates/lys-secrets/src/access.rs, which answers from those fields until R7 makes it ask SpiceDB: a caller may see a secret whose scope is `organisation`, a secret whose scope is `team` and whose team is one of the caller's team ids from R2, and a secret whose scope is `personal` and whose owner is the caller; a caller may discover a lease only when they are its person acted for or its holder, and seeing the secret it was issued from does not make a lease discoverable; a caller may revoke a lease only when they are its person acted for. The seam relates to SECRETS-003's proxy this way: the proxy's use checks on a handle stay where SECRETS-003 puts them; every secret visibility, lease discovery and lease revoke check this brief adds is a call on access.rs and on nothing else, and R7 makes access.rs answer through the `PermissionCheck` trait R7 declares in crates/lys-secrets, which crates/lys-identity-server implements with the step-2 evaluator in crates/lys-identity-server/src/spicedb/check.rs. THE SYSTEM SHALL NOT make a visibility, discovery or revoke check anywhere but through this seam, SHALL NOT answer a discovery or revoke check from the secret's owner field or from the secret's visibility (ADR-095), SHALL NOT add a field to the redacting type of secret.rs, and SHALL NOT add any SpiceDB permission check, relationship write or schema write under crates/lys/src/identity/ (DIRECTORY-002 R4).

**Acceptance:**
- A test in crates/lys-secrets/src/access_tests.rs builds its own fixture in that file: person_a in team_a and person_b in team_b, each by one group in injected group claims; secrets a_personal (scope personal, owner person_a), a_team (scope team, team team_a, owner person_a), a_org (scope organisation, owner person_a), b_personal (scope personal, owner person_b), b_team (scope team, team team_b, owner person_b) and b_org (scope organisation, owner person_b). It asks the see check for person_a over the 6 secrets: exactly 4 are answered yes, a_personal, a_team, a_org and b_org, and the test asserts it asked 6 times.
- A test in crates/lys-secrets/src/access_tests.rs, over the same in-file fixture and a lease issued from b_org, acted for person_a and held by agent_a, asks the discover check and the revoke check for the 4 callers person_a, agent_a, person_b (the secret's owner, who sees b_org) and person_c (a person in neither team_a nor team_b): the discover check answers yes for exactly 2, person_a and agent_a; the revoke check answers yes for exactly 1, person_a; and the test asserts it asked 8 times.
- A test in crates/lys-secrets/src/secret_tests.rs destructures the redacting type of crates/lys-secrets/src/secret.rs with a pattern that names every field that type declares at the commit where SECRETS-003 lands it on origin/main, and no rest pattern `..`, so the crate's tests compile only while that type's fields are exactly SECRETS-003's; `cargo test -p lys-secrets --all-features secret_fields_unchanged` reports exactly 1 passed.
- A test in crates/lys-secrets/src/access_tests.rs builds the store's secret record of crates/lys-secrets/src/store.rs for b_team with its scope field set to `team` and its team field set to team_b, reads both fields back, and asserts they are `team` and team_b.
- The diff of this brief's commits lists 0 paths under crates/lys/src/identity/.

**Files:**
- create: crates/lys-secrets/src/access.rs
- create: crates/lys-secrets/src/access_tests.rs
- create: crates/lys-secrets/src/secret_tests.rs
- modify: crates/lys-secrets/src/store.rs
- modify: crates/lys-secrets/src/lease.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/src/secret.rs

**Checklist:**
- C36 — Every secret visibility and lease revoke check is a call on one seam, answering from the record's scope, team and owner fields until the step-2 SpiceDB evaluator exists, and asking SpiceDB through it once it does.

### R4: Answer the secrets list for one scope from organisation, team and mine, applied at the server, to people only (CONFORMANCE 7.8)

Extends SECRETS-002 R6. Blocked by: `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0. WHEN a signed-in person asks GET /secrets with the query parameter `scope` set to one of `organisation`, `team` and `mine`, THE SYSTEM SHALL apply the scope at the server and answer only the secrets the caller may see through R3's seam within it: `mine` is the secrets whose scope is `personal` and whose owner is the caller; `team` is the secrets whose scope is `team` and whose team is one the caller belongs to by R2; `organisation` is the secrets whose scope is `organisation`. Each value is keyed on the secret's own scope. WHEN the request carries no `scope`, THE SYSTEM SHALL answer every secret the caller may see; that list is not a fourth value. IF `scope` carries any other value, THEN THE SYSTEM SHALL refuse with HTTP 400 and the error `unknown_scope` naming the value given, and answer no secret. IF the caller of GET /secrets is an agent, with or without a scope, THEN THE SYSTEM SHALL refuse with HTTP 403 and the error `agent_uses_virtual_credentials`, naming the agent and saying that agents reach secrets only through their virtual credentials, record nothing, and answer no list, not even an empty one; a person is answered as above. THE SYSTEM SHALL NOT answer a secret the caller may not see under any scope or under none, SHALL NOT widen `team` to team secrets of a team the caller does not belong to, SHALL NOT accept a fourth value for the unscoped list, and SHALL NOT let an agent's team membership change any list.

**Acceptance:**
- CONFORMANCE 7.8: with the fixture of tests/support: person_a in team_a and person_b in team_b, each by one group in the injected group claims on their token; secrets a_personal (scope personal, owner person_a), a_team (scope team, team team_a, owner person_a), a_org (scope organisation, owner person_a), b_personal (scope personal, owner person_b), b_team (scope team, team team_b, owner person_b) and b_org (scope organisation, owner person_b), GET /secrets?scope=mine as person_a answers exactly 1 secret, a_personal; GET /secrets?scope=team answers exactly 1 secret, a_team; GET /secrets?scope=organisation answers exactly 2 secrets, a_org and b_org.
- CONFORMANCE 7.8: with the same fixture, GET /secrets with no scope as person_a answers exactly 4 secrets, a_personal, a_team, a_org and b_org, and b_personal and b_team are absent from it and from each of the 3 scoped lists.
- CONFORMANCE 7.8: GET /secrets?scope=all as person_a answers HTTP 400 with error `unknown_scope` naming `all` and 0 secrets, and GET /secrets?scope=user answers HTTP 400 with error `unknown_scope` naming `user` and 0 secrets.
- CONFORMANCE 7.8: giving agent_a, held for person_a, the group `team_b` in its injected claims leaves each of person_a's 4 lists with the same members as before.
- CONFORMANCE 7.8: agent_a calls GET /secrets with no scope, and then with scope=team: each answers HTTP 403 with error `agent_uses_virtual_credentials`, the body names agent_a and says that agents reach secrets only through their virtual credentials, the body carries no list field, the audit log's line count is the same after both calls as before them, and in the same test GET /secrets with no scope as person_a answers exactly 4 secrets, a_personal, a_team, a_org and b_org.

**Files:**
- create: crates/lys-secrets/src/secret_list.rs
- create: crates/lys-secrets/src/api/mod.rs
- create: crates/lys-secrets/src/api/secrets.rs
- create: crates/lys-secrets/tests/secret_list.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/Cargo.toml
- modify: Cargo.lock
- modify: crates/lys-secrets/tests/support/mod.rs

**Checklist:**
- C34 — The secrets list takes one scope from organisation, team and mine, applied at the server; with no scope it answers every secret the caller may see; any other scope is refused by name (CONFORMANCE 7.8).
- C38 — The secrets list is a person's view: an agent that asks for it is refused by name, the refusal naming the agent and saying that agents reach secrets only through their virtual credentials, with no list, not even an empty one, and nothing recorded (CONFORMANCE 7.8).

**Stories:**
- S21 (Account holder, Uses and deliberately delegates scoped access without exposing credentials) — As a person looking at the secrets list, I want to choose organisation, team or mine and have the server answer only what I may see within that scope, so that no filter ever shows me a secret I may not see.

### R5: Revoke a lease as the person acted for, and refuse the revoke to everyone else (CONFORMANCE 7.6)

Extends SECRETS-002 R7: the two facts of a revoke are SEC_REVOKE_STATES of SECRETS-002 R7, and SECRETS-002 R8's in-flight cancellation rule stays open and outside this acceptance. Extends SECRETS-002 R6 for the refusal that names nothing. Builds on SECRETS-003 R4: its proxy and the revocation states SEC_REVOKE_STATES names, which R4 defines. Blocked by: `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0 and `git cat-file -e origin/main:crates/lys-secrets/src/proxy.rs` exits 0. WHEN the person acted for under a lease calls POST /leases/{lease_id}/revoke, at any time, THE SYSTEM SHALL stop issuing under the lease at once, record the lease's end as a revoke by that caller, and show upstream pending until the system behind confirms separately; the revoke ends the lease by calling the one pub(crate) function `end_with_upstream_pending` in crates/lys-secrets/src/revoke.rs, which stops issuing, records the end with its act and its caller, and moves the lease's upstream into SECRETS-003 R4's revocation state for upstream unconfirmed, appending one audit line for the transition as R4 does, and is the one function any act that ends a lease with upstream pending calls. GET /leases/{lease_id} SHALL then read ended_by `revoke`, issuing `stopped` and upstream `pending`, and upstream `confirmed` only once the system behind's acknowledgement is delivered, by the one function `deliver_upstream_ack` in crates/lys-secrets/src/revoke.rs, which moves it into R4's revocation state for upstream confirmed and appends one audit line for that transition; `pending` and `confirmed` are the words GET /leases/{lease_id} reads for those two R4 states. A holder who is also the person acted for under that lease revokes as the person acted for. IF the caller cannot discover the lease through R3's seam, which every caller but its person acted for and its holder cannot, THEN THE SYSTEM SHALL refuse with HTTP 404 and the error `lease_not_found`, naming nothing, and SHALL answer GET /leases/{lease_id} the same way. IF the caller is the lease's person acted for and the lease has already ended, by a revoke or the end of its time window, THEN THE SYSTEM SHALL refuse with HTTP 409 and the error `lease_already_ended`, carrying the way it ended (`revoke` or `expired`), the instant it ended and, for a revoke, who ended it, all read from the first end record, and SHALL record nothing, so that the first end record stays the only one. IF the caller is the lease's holder and not its person acted for, THEN THE SYSTEM SHALL refuse with HTTP 403 and the error `holder_relinquishes`, naming the lease and naming relinquish as the holder's act. IF the caller can discover the lease through R3's seam, is not its holder, and the seam answers no to the revoke, THEN THE SYSTEM SHALL refuse with HTTP 403 and the error `revoke_not_permitted`, naming the lease. The upstream state is the field `upstream` of the lease record `Lease` that SECRETS-003 defines in crates/lys-secrets/src/lease.rs, the record its CONTRACT.md `Lease` section describes; its public path is `lys_secrets::Lease`, its re-export from crates/lys-secrets/src/lib.rs, and since SECRETS-003's brief names no such re-export, this row adds that `pub use` to lib.rs. The field `upstream` holds SECRETS-003 R4's revocation state and no second pending or confirmed state: revoke.rs declares no state values of its own. The field is not public outside the crate, so code outside the crate that writes it through `lys_secrets::Lease` fails to compile with E0616 and with no other error, the path `lys_secrets::Lease` being public; its type is a wrapper declared in crates/lys-secrets/src/revoke.rs whose one inner field, holding R4's state, is private to revoke.rs, so no code outside revoke.rs writes the state. `end_with_upstream_pending` is the only writer of upstream unconfirmed (`pending`) and `deliver_upstream_ack` is the only writer of upstream confirmed (`confirmed`). A use of a lease is measured at SECRETS-003 R4's proxy, whose upstream test double counts the requests forwarded to it, as R4's own tests do. THE SYSTEM SHALL NOT keep a second upstream pending or confirmed state beside R4's revocation states, SHALL NOT change upstream `pending` to `confirmed` anywhere but in `deliver_upstream_ack`, so that no timer and no other operation changes it, SHALL NOT let a refused revoke stop issuing, record an end or send any request to the system behind, SHALL NOT record a relinquish the caller did not ask for, SHALL NOT write a second end record for a lease that has already ended, and SHALL NOT let a secret's owner discover or revoke a lease as owner (ADR-095). R6's 9-step test on the revoked lease L1 measures this row's clause that no timer and no other operation changes upstream `pending` to `confirmed`, and R6 names C30 and S19 for it. The refusals cover only the direct act of revoking a lease: withdrawal of an upstream grant under ADR-003 and SEC_REVOKE_CHAIN, and emergency stop under CONFORMANCE row 3.4, end a lease as their own acts and are not changed.

**Acceptance:**
- CONFORMANCE 7.6: with lease L1 issued from a_org, acted for person_a and held by agent_a, person_a calls revoke on L1 while the upstream test double of SECRETS-003 R4's proxy withholds its acknowledgement: the next use of L1 through the proxy is refused, the double's count of forwarded use requests is the same as before the revoke, the audit log gains exactly 1 line for the move to upstream unconfirmed, and GET /leases/L1 reads ended_by `revoke`, issuing `stopped` and upstream `pending`; then `deliver_upstream_ack` runs once for L1 and GET /leases/L1 reads upstream `confirmed`, and the audit log gains exactly 1 more line, for the move to upstream confirmed.
- A `compile_fail,E0616` doctest in crates/lys-secrets/src/revoke.rs takes a `&mut lys_secrets::Lease` and assigns its `upstream` field; `lys_secrets::Lease` is the public re-export from crates/lys-secrets/src/lib.rs, so the path resolves and E0616 is the only error the doctest can hit, and the doctest passes under `cargo test --workspace --all-features`.
- CONFORMANCE 7.6: person_b, who sees a_org and is neither L1's person acted for nor its holder, calls revoke on L1: HTTP 404 with error `lease_not_found`, and the body is byte-identical to the answer for a lease id that does not exist, containing neither L1's id nor any secret title, holder or person; the next use of L1 through SECRETS-003 R4's proxy succeeds and the double's count of forwarded use requests rises by exactly 1, and the double receives 0 revoke requests.
- CONFORMANCE 7.6: agent_a, L1's holder, calls revoke on L1: HTTP 403 with error `holder_relinquishes`, and the body names L1 and names `relinquish`; L1's record carries no end, the next use of L1 through SECRETS-003 R4's proxy succeeds and the double's count of forwarded use requests rises by exactly 1, and the double receives 0 revoke requests.
- CONFORMANCE 7.6: person_b, the owner of b_org, calls revoke on lease L2, issued from b_org, acted for person_a and held by agent_a: HTTP 404 with error `lease_not_found` naming nothing, and L2 keeps issuing.
- CONFORMANCE 7.6: on lease L4, issued from a_org, whose holder and person acted for are both person_a, person_a's revoke is recorded as a revoke: GET /leases/L4 reads ended_by `revoke`.
- CONFORMANCE 7.6: person_a calls revoke on L1 a second time after revoking it: HTTP 409 with error `lease_already_ended`, the body carries the way `revoke`, the instant of the first revoke and person_a, L1 carries exactly 1 end record, and the double receives exactly 1 revoke request in all.
- CONFORMANCE 7.6: person_a calls revoke on lease L5, issued from a_org, acted for person_a and held by agent_a, after the injected clock passes the end of L5's time window: HTTP 409 with error `lease_already_ended`, the body carries the way `expired` and the instant L5's window ended, and L5 carries 0 revoke records and 0 relinquish records.

**Files:**
- create: crates/lys-secrets/src/revoke.rs
- create: crates/lys-secrets/src/api/leases.rs
- create: crates/lys-secrets/tests/lease_revoke.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/src/api/mod.rs
- modify: crates/lys-secrets/src/lease.rs
- modify: crates/lys-secrets/tests/support/mod.rs

**Checklist:**
- C30 — The person acted for under a lease may revoke it at any time: the revoke stops issuing at once, and GET /leases/{lease_id} reads upstream pending until the system behind confirms, with no timer confirming it (CONFORMANCE 7.6).
- C31 — A revoke by anyone but the person acted for is refused: with the not-found refusal naming nothing when the caller cannot discover the lease, which only its person acted for and its holder can; for the lease's holder with a refusal naming the lease and relinquish; and with a refusal naming the lease for any other caller the seam lets discover it.
- C37 — A revoke by the person acted for, or a relinquish by the holder, of a lease that has already ended is refused by name as already ended, carrying how, when and by whom it first ended, and records nothing; the first end record stays the only one (CONFORMANCE 7.6).

**Stories:**
- S19 (Person, Grants an agent provisioned under them access to an account) — As the person an agent acts for under a lease, I want to revoke the lease and see issuing stopped at once and the confirmation from the system behind shown pending until it comes, so that I know what has stopped and what is still unconfirmed.

### R6: Let a lease's holder relinquish it, recorded as its own act (CONFORMANCE 7.6)

Extends SECRETS-002 R7. Builds on SECRETS-003 R4, as R5 does. Blocked by: `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0 and `git cat-file -e origin/main:crates/lys-secrets/src/proxy.rs` exits 0. WHEN a lease's holder calls POST /leases/{lease_id}/relinquish, THE SYSTEM SHALL stop issuing under the lease at once, record the lease's end as a relinquish by the holder, a separate act from a revoke, by calling R5's `end_with_upstream_pending` in crates/lys-secrets/src/revoke.rs, and show upstream pending until the system behind confirms through R5's `deliver_upstream_ack`, moving through SECRETS-003 R4's revocation states exactly as a revoke does, since the credential at the system behind exists whoever ended the lease; GET /leases/{lease_id} SHALL then read ended_by `relinquish`, issuing `stopped`, and upstream `pending` until confirmed and `confirmed` after. IF the caller cannot discover the lease through R3's seam, which every caller but its person acted for and its holder cannot, THEN THE SYSTEM SHALL refuse with HTTP 404 and the error `lease_not_found`, naming nothing. IF the caller is the lease's holder and the lease has already ended, by a revoke, a relinquish or the end of its time window, THEN THE SYSTEM SHALL refuse with R5's HTTP 409 and error `lease_already_ended`, carrying the same way, instant and, for a revoke or a relinquish, who ended it, read from the first end record, and SHALL record nothing. IF the caller can discover the lease and is not its holder, THEN THE SYSTEM SHALL refuse with HTTP 403 and the error `relinquish_not_permitted`, naming the lease. IF the lease's person acted for calls revoke on a lease that has ended by a relinquish, THEN THE SYSTEM SHALL refuse with R5's HTTP 409 and error `lease_already_ended`, carrying the way `relinquish`, the instant of the relinquish and the holder who relinquished it, read from the first end record, and SHALL record nothing. THE SYSTEM SHALL NOT record a relinquish as a revoke, SHALL NOT write a second end record for a lease that has already ended, SHALL NOT let a timer change upstream `pending` to `confirmed`, and SHALL NOT let a refused relinquish stop issuing or record an end.

**Acceptance:**
- CONFORMANCE 7.6: agent_a, L1's holder, relinquishes L1 while the upstream test double of SECRETS-003 R4's proxy withholds its acknowledgement: the next use of L1 through the proxy is refused with the double's count of forwarded use requests unchanged, and GET /leases/L1 reads, in this order, ended_by `relinquish`, issuing `stopped`, and upstream `pending`, then upstream `confirmed` after the double delivers its acknowledgement once.
- CONFORMANCE 7.6: with lease L3 issued from a_org, acted for person_a and held by agent_a, relinquished by agent_a and the acknowledgement withheld, a test drives every other operation this brief exposes on L3 in turn, as 9 steps in this order: (1) a use of L3, (2) GET /leases/L3, (3) a second POST /leases/L3/relinquish by agent_a, (4) POST /leases/L3/revoke by person_a, (5) GET /secrets with no scope, (6) GET /secrets?scope=organisation, (7) GET /secrets?scope=team, (8) GET /secrets?scope=mine, (9) the injected clock advanced past the end of L3's own time window; after each of those 9 steps GET /leases/L3 reads upstream `pending`, and the test asserts it took 9 readings, one for each step. Then `deliver_upstream_ack` runs once for L3 and GET /leases/L3 reads upstream `confirmed`.
- CONFORMANCE 7.6: after agent_a's relinquish, L1's end record names the act `relinquish` and the caller agent_a, and L1 carries 0 revoke records.
- CONFORMANCE 7.6: person_a, L1's person acted for and not its holder, calls relinquish on L1: HTTP 403 with error `relinquish_not_permitted` naming L1, and L1 keeps issuing.
- CONFORMANCE 7.6: person_b, who sees a_org and is neither L1's person acted for nor its holder, calls relinquish on L1: HTTP 404 with error `lease_not_found` naming nothing, and L1 keeps issuing.
- CONFORMANCE 7.6: agent_a calls relinquish on L1 after person_a revoked it: HTTP 409 with error `lease_already_ended`, the body carries the way `revoke`, the instant of the revoke and person_a, and L1 carries exactly 1 end record, naming the act `revoke`.
- CONFORMANCE 7.6: agent_a calls relinquish on L3 a second time after relinquishing it: HTTP 409 with error `lease_already_ended`, the body carries the way `relinquish`, the instant of the first relinquish and agent_a, and L3 carries exactly 1 end record.
- CONFORMANCE 7.6: with L1 revoked by person_a and the acknowledgement still withheld, a test drives every other operation this brief exposes on L1 in turn, as 9 steps in this order: (1) a use of L1, (2) GET /leases/L1, (3) a second POST /leases/L1/revoke by person_a, (4) POST /leases/L1/relinquish by agent_a, (5) GET /secrets with no scope, (6) GET /secrets?scope=organisation, (7) GET /secrets?scope=team, (8) GET /secrets?scope=mine, (9) the injected clock advanced past the end of L1's own time window; after each of those 9 steps GET /leases/L1 reads upstream `pending`, and the test asserts it took 9 readings, one for each step. Then `deliver_upstream_ack` runs once for L1 and GET /leases/L1 reads upstream `confirmed`.
- With the 9-step test on the revoked lease L1 and the 9-step test on the relinquished lease L3 above, and R5's `compile_fail,E0616` doctest on the `upstream` field of `lys_secrets::Lease` passing under `cargo test --workspace --all-features`, upstream reads `confirmed` only after `deliver_upstream_ack` runs, on a revoked lease and on a relinquished one.
- CONFORMANCE 7.6: person_a calls revoke on lease L3 after agent_a relinquished it: HTTP 409 with error `lease_already_ended`, the body carries the way `relinquish`, the instant of the relinquish and agent_a, and L3 carries exactly 1 end record, naming the act `relinquish`.

**Files:**
- create: crates/lys-secrets/src/relinquish.rs
- create: crates/lys-secrets/tests/lease_relinquish.rs
- modify: crates/lys-secrets/src/api/leases.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/tests/support/mod.rs

**Checklist:**
- C30 — The person acted for under a lease may revoke it at any time: the revoke stops issuing at once, and GET /leases/{lease_id} reads upstream pending until the system behind confirms, with no timer confirming it (CONFORMANCE 7.6).
- C32 — A lease's holder may relinquish it, recorded as a relinquish and not as a revoke, which stops issuing at once and reads upstream pending until the system behind confirms.
- C37 — A revoke by the person acted for, or a relinquish by the holder, of a lease that has already ended is refused by name as already ended, carrying how, when and by whom it first ended, and records nothing; the first end record stays the only one (CONFORMANCE 7.6).

**Stories:**
- S19 (Person, Grants an agent provisioned under them access to an account) — As the person an agent acts for under a lease, I want to revoke the lease and see issuing stopped at once and the confirmation from the system behind shown pending until it comes, so that I know what has stopped and what is still unconfirmed.
- S20 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent holding a lease, I want to give it back by a relinquish recorded as my own act, so that ending my own access is never recorded as a revoke nobody asked for.

### R7: Make the seam ask SpiceDB through the step-2 evaluator

Extends SECRETS-002 R6 and R7. Blocked by: `git cat-file -e origin/main:crates/lys-identity-server/src/spicedb/check.rs` exits 0 and `git cat-file -e origin/main:crates/lys-identity-server/src/routes.rs` exits 0, each keyed on the file and never on a brief id or commit text; and R3 landed. crates/lys-secrets declares, in crates/lys-secrets/src/permission.rs, a trait `PermissionCheck` with one method, which answers whether an identity may do an action on a secret or on a lease issued from it; crates/lys-secrets depends on no server crate, so crates/lys-secrets/Cargo.toml is not changed by this row; crates/lys-identity-server/Cargo.toml gains the dependency on lys-secrets, and Cargo.lock changes with it. crates/lys-identity-server implements `PermissionCheck` in crates/lys-identity-server/src/spicedb/check.rs with the step-2 brief's one evaluator in that file (ADR-001), and passes that implementation in where the routes of R4 are built, in crates/lys-identity-server/src/routes.rs. WHEN the step-2 SpiceDB evaluator exists, THE SYSTEM SHALL answer every check of R3's seam in crates/lys-secrets/src/access.rs by a call on the `PermissionCheck` it was given, with the relations the seam asks written in crates/lys-secrets/schema/secrets.zed. IF the `PermissionCheck` returns an error, THEN THE SYSTEM SHALL answer the request with HTTP 503 and the error `permission_engine_unavailable`, and answer no secret and change no lease. THE SYSTEM SHALL NOT answer any check from the record's scope, team and owner fields once this row lands, SHALL NOT fall back to those fields when the `PermissionCheck` returns an error, SHALL NOT call SpiceDB other than through the step-2 evaluator in check.rs, SHALL NOT add a second evaluator, SHALL NOT add a dependency from crates/lys-secrets on crates/lys-identity-server, and SHALL NOT add any check under crates/lys/src/identity/ (DIRECTORY-002 R4).

**Acceptance:**
- A test with a SpiceDB test double that implements `PermissionCheck` and answers no for person_a on a_org, which the record's fields allow: GET /secrets?scope=organisation as person_a answers exactly 1 secret, b_org, and the double records at least 1 check naming a_org.
- A test with the double answering yes to person_a's discovery of L1 and no to person_a's revoke of L1: HTTP 403 with error `revoke_not_permitted` and a body naming L1, and L1 keeps issuing.
- A test in crates/lys-secrets/tests/lease_revoke.rs substitutes a `PermissionCheck` double that answers no to every check and counts its calls, then calls POST /leases/L1/revoke as person_a: the double records exactly 1 check, the revoke answers HTTP 404 with error `lease_not_found` naming nothing, and the next use of L1 succeeds.
- A test with the double returning an error for every check: GET /secrets with no scope as person_a answers HTTP 503 with error `permission_engine_unavailable`, and the body contains 0 secrets.
- `cargo tree -p lys-secrets --all-features -e normal` prints 0 lines containing `lys-identity-server`.
- The diff of this row lists 0 paths under crates/lys/src/identity/.

**Files:**
- create: crates/lys-secrets/src/permission.rs
- modify: crates/lys-secrets/src/access.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/schema/secrets.zed
- modify: crates/lys-secrets/tests/support/mod.rs
- modify: crates/lys-secrets/tests/lease_revoke.rs
- modify: crates/lys-identity-server/src/spicedb/check.rs
- modify: crates/lys-identity-server/Cargo.toml
- modify: Cargo.lock
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C36 — Every secret visibility and lease revoke check is a call on one seam, answering from the record's scope, team and owner fields until the step-2 SpiceDB evaluator exists, and asking SpiceDB through it once it does.

## Boundaries

- No row is built before the lead signs this brief off on its card.
- The host is not rewritten in the secrets design. Its Intention reads "Every identity, person or agent, uses credentials through a short-lived handle the door swaps for the real credential, so no credential ever reaches an agent and revoking is instant." and its Solution reads "A broker in Rust inside the door: an encrypted store of real credentials, handles bound to identities, a proxy that checks SpiceDB, swaps the handle, forwards the call and writes one audit line, rotation across several accounts under one handle, OAuth refresh at the proxy, sealed records tagged in SpiceDB, and leases counted by uses, time window and spend." Both place the broker in the door, while this brief builds in crates/lys-secrets on the host decision SECRETS-003 carries: that conflict is an open finding against SECRETS-003, not against this brief. The two sentences stand as main's text and change only with SECRETS-003's own documents. R2 to R6 depend on SECRETS-003 landing, checked from a clone of lys by `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exiting 0, and R5 and R6 also by `git cat-file -e origin/main:crates/lys-secrets/src/proxy.rs` exiting 0.
- Only the server's answers are built. The lease screen with its Revoke control, and the secrets list screen with its scope control of All, Organisation, Team and Mine, are neighbours outside this brief. The rule the neighbour is held to, with no requirement and no test here: the browser shows what the server answered and nothing else.
- No code and no test grants a secret's owner a revoke of a lease they are not acted for under while ADR-095 is proposed; the set of who may revoke beyond the person acted for stays empty.
- The refusals cover only the direct act of revoking a lease: withdrawal of an upstream grant (ADR-003, SEC_REVOKE_CHAIN) and emergency stop (CONFORMANCE row 3.4) end a lease as their own acts and are not changed.
- SECRETS-002 R8's in-flight cancellation rule stays open; no row decides it.
- SECRETS-002's JSON and rendered markdown are not edited.
- No SpiceDB permission check, relationship write or schema write is added under crates/lys/src/identity/ (DIRECTORY-002 R4); every visibility and revoke check is a call on R3's seam.
- Team ids come only from the group claims `team_ids` reads; no team model is built here.
- The unscoped list is the list with no scope, never a fourth scope value.
- No cambium file is edited, and nothing makes Cambium or any engine a required runtime (ADR-004).
- The token revolver is not changed (ADR-002).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, each clean, and the test run reports 0 failed and 0 ignored.
- From the repository root: sh scripts/design/gate.sh exits 0.
- Before R2 to R6 are built, `git cat-file -e origin/main:crates/lys-secrets/Cargo.toml` exits 0; before R5 and R6 are built, `git cat-file -e origin/main:crates/lys-secrets/src/proxy.rs` exits 0 as well; before R7 is built, `git cat-file -e origin/main:crates/lys-identity-server/src/spicedb/check.rs` exits 0 and `git cat-file -e origin/main:crates/lys-identity-server/src/routes.rs` exits 0.
- `rg -c 'conformance_7_6' crates/lys-secrets/tests/lease_revoke.rs crates/lys-secrets/tests/lease_relinquish.rs` prints at least 1 for each file, and `rg -c 'conformance_7_8' crates/lys-secrets/tests/secret_list.rs` prints at least 1.
- `git diff --name-only` over this brief's commits lists no path under crates/lys/src/identity/, and neither docs/design/secrets/briefs/SECRETS-002.json nor docs/design/secrets/briefs/SECRETS-002.md.
