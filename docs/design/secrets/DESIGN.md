---
type: design
cluster: secrets
title: Secrets broker: agents hold handles, never credentials
---

# Secrets broker: agents hold handles, never credentials

> **Cluster:** secrets

## Intention

Every identity, person or agent, uses credentials through a short-lived handle the door swaps for the real credential, so no credential ever reaches an agent and revoking is instant.

## Problem

Credentials live in files on each machine and in each worker's environment. An agent that holds a key cannot have it taken back, usage cannot be attributed in one place, and resting an account means copying a file to every machine.

## Solution

A broker in Rust inside the door: an encrypted store of real credentials, handles bound to identities, a proxy that checks SpiceDB, swaps the handle, forwards the call and writes one audit line, rotation across several accounts under one handle, OAuth refresh at the proxy, sealed records tagged in SpiceDB, and leases counted by uses, time window and spend. The token revolver is its first consumer.

## Principles

- **P1** — A handle, never a credential: the credential never leaves the server.
- **P2** — Revoking is dropping the handle; what was already handed to a process needs its own revocation story, stated per case.
- **P3** — Every use writes one audit line naming the seat, the handle, the real account and the time.
- **P4** — A key is used through the proxy, never read; only memories are read, in the smallest piece asked for.
- **P5** — Every grant traces back to the person who authorised it.
- **P6** — Permission to use never implies permission to lend. Pass-on rights are affirmative and recipient-specific; a missing prohibition is not a grant. Server-verified ownership is the affirmative may-lend route in docs/design/identity/CONFORMANCE.md at commit 1353c22 row 7.3; a display label is not proof of ownership.
- **P7** — Secret visibility, usage, delegation and revocation follow the same current human-rooted authority at every server seam, independent of how the caller reaches it.

## Decisions

- ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
- ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-019 — The secrets broker lives in lys, as crates/lys-secrets; Cambium consumes it and is never its home (amends ADR-001) — The broker lives in the lys repository as the crate crates/lys-secrets, part of the standalone identity platform, built in Rust. A person installs Lys to hold secrets; Cambium reaches the broker through the door as a consumer and is never its home. Rejected: the broker inside the cambium door that ADR-001 and SECRETS-002 name, which would make Cambium a runtime a person must install to keep a credential.
- ADR-020 — A lease is a broker record under the audit log, not a signed format — The lease is a broker record under the audit log: it counts uses, time and spend against a grant and is read only by the broker, so nothing a stranger verifies changes. The limit on a lease's time is the window of the lys-identity grant it counts against (DIRECTORY-006 R1): the lease's own not_after never extends past it. Rejected: a signed lease format, a new version beside lys/delegation/v1 with its own adversarial review; and reading the window as one the signed delegation keeps, which v1 cannot carry.
- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-095 — Ownership of a secret alone confers no revoke over the leases derived from it — The person acted for under a lease revokes it, and ownership of the secret the lease was issued from confers no revoke, over the owner revoking every credential derived from the secret, because who may revoke is a policy choice that ownership alone does not confer. Rejected: the owner revokes every credential derived from the secret.
- ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
- ADR-124 — A signing key is used inside the secrets broker, which signs typed members under a purpose's domain and never gives the key out — A secret may be sealed as a signing key with one purpose from a closed list. The broker signs for a live, admitted handle, builds the bytes to sign itself from typed members with the same function the verifier uses, and answers a COSE_Sign1 signature and the public key only.

## Goals

- An implementation brief, SECRETS-002, covers every part of the temporary key model in the statement with numbered requirements and acceptance criteria.
- Every checklist item and user story of the broker is covered by a SECRETS-002 requirement.
- The token revolver can take its next account from the broker instead of its own list.
- Every checklist item and user story of the broker is covered by a SECRETS-002 or SECRETS-003 requirement.
- An implementation brief, SECRETS-003, carries a baseline row, a contract row with its adversarial review, and code rows that deliver SECRETS-002's R1 to R9 in crates/lys-secrets, each code row with counted acceptance legs, an estimate in hours, and DIRECTORY-002, DIRECTORY-003 and DIRECTORY-006 R1 to R4 as blockers, each with a check a stranger can run.
- Every checklist item and user story of the broker is covered by a requirement of a SECRETS brief.

## Non-Goals

- OpenBao or another external secrets engine — Tom's model is built in Rust inside the door; an external engine is only reconsidered if credentials minted on demand are needed.
- The seat login through a proxy handle and base URL — Not tested with a subscription login; the statement keeps it open to prove later and nothing depends on it.
- The delegation schema — The statement says it is not settled.
- A signed lease format, a new version beside lys/delegation/v1 — ADR-020: the lease is a broker record read only by the broker; a lease a stranger verifies offline is a new brief with its own adversarial review.
- A release and demonstration row for the broker — SECRETS-003's words list its rows, and a release row is not among them; it would come from a later roadmap decision.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/secrets/briefs/SECRETS-002.json` | the secrets broker implementation brief | SECRETS-001 |
| `docs/design/secrets/briefs/SECRETS-002.md` | its rendered markdown | SECRETS-001 |
| `docs/design/secrets/design.json` | this design; the structure gains a row for every path SECRETS-002 names | SECRETS-001 |
| `docs/design/secrets/DESIGN.md` | rendered design | SECRETS-001 |
| `docs/design/secrets/checklist.json` | checklist; gains the broker implementation items | SECRETS-001 |
| `docs/design/secrets/CHECKLIST.md` | rendered checklist | SECRETS-001 |
| `docs/design/secrets/stories.json` | stories; gains the broker implementation stories | SECRETS-001 |
| `docs/design/secrets/USER-STORIES.md` | rendered stories | SECRETS-001 |
| `crates/lys-core/src/delegation/mod.rs` | lys/delegation/v1; its docs record the handle and the lease's time window as applications of the delegation format (SECRETS-002 R1, R9) | SECRETS-002 |
| `docs/design/WIRE-FORMATS.md` | the lys wire-format register; records the handle, sealed-record and lease formats before any is signed (SECRETS-002 R1, R6, R9) | SECRETS-002 |
| `crates/lys-core/src/seal/mod.rs` | sealed envelopes; its docs record the sealed record as an application of lys/sealed-envelope/v1 (SECRETS-002 R6) | SECRETS-002 |
| `docs/design/secrets/briefs/SECRETS-003.json` | the broker build brief: baseline, contract and counted code rows in crates/lys-secrets | SECRETS-003 |
| `docs/design/secrets/briefs/SECRETS-003.md` | its rendered markdown | SECRETS-003 |
| `docs/design/secrets/BASELINE.md` | SECRETS-003 R1: the revolver call site, the pool file's consumers, every credential path into a seat classed, and what lys/delegation/v1 and lys/sealed-envelope/v1 can carry, by file and line | SECRETS-003 |
| `docs/design/secrets/CONTRACT.md` | SECRETS-003 R2: the invariant, the key custody, the handle, the lease, the cancellation rule, the SpiceDB relations, the sealed-record binding and the audit line | SECRETS-003 |
| `docs/design/secrets/reports/SECRETS-003-adversarial-review.md` | SECRETS-003 R2: every attack tried against the contract and the clause defeating each | SECRETS-003 |
| `Cargo.toml` | the workspace manifest; gains crates/lys-secrets as a member (SECRETS-003 R3) | SECRETS-003 |
| `Cargo.lock` | the workspace lockfile; gains the HTTP routing dependencies of crates/lys-secrets (SECRETS-004 R4) and the lys-secrets dependency of crates/lys-identity-server (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-secrets/Cargo.toml` | the broker crate's manifest, created by SECRETS-003; gains the HTTP routing dependencies (SECRETS-004 R4) | SECRETS-003 |
| `crates/lys-secrets/src/lib.rs` | the broker crate's module declarations, created by SECRETS-003; declares the SECRETS-004 modules (SECRETS-004 R3 to R7) and re-exports the lease record as lys_secrets::Lease (SECRETS-004 R5) | SECRETS-003 |
| `crates/lys-secrets/src/error.rs` | the broker's error type; no credential byte in any variant | SECRETS-003 |
| `crates/lys-secrets/src/secret.rs` | the redacting type for credential bytes, created by SECRETS-003; its fields unchanged, it gains only the declaration of its test module (SECRETS-004 R3) | SECRETS-003 |
| `crates/lys-secrets/src/store.rs` | the encrypted store, created by SECRETS-003; each secret record, the entry that carries its owning identity, gains its scope and its team (SECRETS-004 R3) | SECRETS-003 |
| `crates/lys-secrets/src/key_rotation.rs` | store-key rotation: every entry resealed to the new key, the old key refused by name, one audit line naming both key ids | SECRETS-003 |
| `crates/lys-secrets/src/handle.rs` | handles bound to one identity, issued, resolved and dropped | SECRETS-003 |
| `crates/lys-secrets/src/lease.rs` | the lease record Lease, created by SECRETS-003; each lease carries its holder, its person acted for, its source secret and its end (SECRETS-004 R3), and its upstream field, holding SECRETS-003 R4's revocation state and not public outside the crate (SECRETS-004 R5) | SECRETS-003 |
| `crates/lys-secrets/src/audit.rs` | audit lines appended through lys-log-store | SECRETS-003 |
| `crates/lys-secrets/schema/secrets.zed` | the SpiceDB relations, created by SECRETS-003; gains the relations the seam asks (SECRETS-004 R7) | SECRETS-003 |
| `crates/lys-secrets/src/proxy.rs` | the check, swap, forward and audit line | SECRETS-003 |
| `crates/lys-secrets/src/rotation.rs` | the next real account in turn under one handle | SECRETS-003 |
| `crates/lys-secrets/src/in_flight.rs` | the register of admitted calls and the cancellation rule applied at a drop | SECRETS-003 |
| `crates/lys-secrets/src/oauth.rs` | OAuth refresh at the proxy | SECRETS-003 |
| `crates/lys-secrets/src/spawn_login.rs` | the seat's own login answered at spawn and the record of which went to which seat | SECRETS-003 |
| `crates/lys-secrets/src/sealed.rs` | sealed records read by name under a relation check; keys refused for reading | SECRETS-003 |
| `crates/lys-secrets/src/next_account.rs` | the revolver's ask for its next account | SECRETS-003 |
| `crates/lys-secrets/tests/store_leases.rs` | R3's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/secret_scope.rs` | SEC3_SECRET_SCOPE: the counting test that keeps every secret byte in the redacting type | SECRETS-003 |
| `crates/lys-secrets/tests/proxy.rs` | R4's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/oauth.rs` | R5's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/spawn_login.rs` | R6's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/sealed.rs` | R7's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/next_account.rs` | R8's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/support/mod.rs` | shared test doubles, created by SECRETS-003; gains the two-people, two-teams fixture and the injected group claims (SECRETS-004 R4 to R7) | SECRETS-003 |
| `docs/design/secrets/briefs/SECRETS-004.json` | the brief amending SECRETS-002 for CONFORMANCE rows 7.6 and 7.8: who may revoke a lease, the relinquish, and the scoped secrets list | SECRETS-004 |
| `docs/design/secrets/briefs/SECRETS-004.md` | its rendered markdown | SECRETS-004 |
| `docs/design/decisions.json` | the project decision ledger; gains proposed ADR-095, ownership alone confers no revoke (SECRETS-004 R1) | SECRETS-004 |
| `crates/lys-secrets/src/teams.rs` | team_ids: the signed-in person's team ids from the group claims on their token, the one place they are read (SECRETS-004 R2) | SECRETS-004 |
| `crates/lys-secrets/src/teams_tests.rs` | team_ids tests with injected claims (SECRETS-004 R2) | SECRETS-004 |
| `crates/lys-secrets/src/access.rs` | the one seam every secret visibility, lease discovery and lease revoke check calls; answers from the record's fields, then through the PermissionCheck it is given (SECRETS-004 R3, R7) | SECRETS-004 |
| `crates/lys-secrets/src/access_tests.rs` | seam tests answering from the record's fields (SECRETS-004 R3) | SECRETS-004 |
| `crates/lys-secrets/src/permission.rs` | PermissionCheck, the trait the seam answers through once SpiceDB is asked; declared in lys-secrets so it depends on no server crate (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-identity-server/src/spicedb/check.rs` | the step-2 SpiceDB evaluator, created by the step-2 SpiceDB brief whose id is not settled; implements PermissionCheck and is passed in where the routes are built (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-identity-server/Cargo.toml` | the identity server's manifest, landed by the directory briefs; gains the dependency on lys-secrets, so the server depends on the library and never the reverse (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-identity-server/src/routes.rs` | where the identity server builds its routes, landed by the directory briefs; passes the PermissionCheck implementation to the secrets routes (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-secrets/src/secret_tests.rs` | the test that destructures the redacting type with no rest pattern, so it compiles only while its fields are SECRETS-003's (SECRETS-004 R3) | SECRETS-004 |
| `crates/lys-secrets/src/secret_list.rs` | the scoped secrets list: organisation, team, mine, or no scope (SECRETS-004 R4) | SECRETS-004 |
| `crates/lys-secrets/src/api/mod.rs` | the broker's HTTP routes, module declarations only (SECRETS-004 R4) | SECRETS-004 |
| `crates/lys-secrets/src/api/secrets.rs` | GET /secrets with its scope parameter, and its refusal to an agent (SECRETS-004 R4) | SECRETS-004 |
| `crates/lys-secrets/src/api/leases.rs` | GET /leases/{lease_id}, POST /leases/{lease_id}/revoke and POST /leases/{lease_id}/relinquish (SECRETS-004 R5, R6) | SECRETS-004 |
| `crates/lys-secrets/src/revoke.rs` | the revoke by the person acted for, its refusals and the refusal of an already ended lease, end_with_upstream_pending, the one function that ends a lease with upstream pending for revoke and relinquish, and deliver_upstream_ack, the one writer of confirmed, both moving the lease through SECRETS-003 R4's revocation states with no second state of their own (SECRETS-004 R5, R6) | SECRETS-004 |
| `crates/lys-secrets/src/relinquish.rs` | the holder's relinquish, recorded as its own act (SECRETS-004 R6) | SECRETS-004 |
| `crates/lys-secrets/tests/secret_list.rs` | the CONFORMANCE 7.8 legs (SECRETS-004 R4) | SECRETS-004 |
| `crates/lys-secrets/tests/lease_revoke.rs` | the CONFORMANCE 7.6 revoke legs (SECRETS-004 R5) and the revoke leg against an always-no PermissionCheck double (SECRETS-004 R7) | SECRETS-004 |
| `crates/lys-secrets/tests/lease_relinquish.rs` | the CONFORMANCE 7.6 relinquish legs (SECRETS-004 R6) | SECRETS-004 |
| `crates/lys-secrets/src/broker/admit.rs` | touched by SECRETS-005 R1: A presented token is found by a hashed lookup | SECRETS-005 |
| `crates/lys-secrets/src/broker/using.rs` | touched by SECRETS-005 R1: A presented token is found by a hashed lookup | SECRETS-005 |
| `crates/lys-secrets/src/broker/ending.rs` | touched by SECRETS-005 R2: Lineage and endings are indexed | SECRETS-005 |
| `crates/lys-secrets/src/bin/lys-secrets/serve.rs` | touched by SECRETS-005 R3: Broker work never blocks an async worker, and the permission check runs outside the lock | SECRETS-005 |
| `crates/lys-secrets/src/bin/lys-secrets/view.rs` | touched by SECRETS-005 R4: Routes and services are loaded once | SECRETS-005 |
| `crates/lys-secrets/src/bin/lys-secrets/callers.rs` | touched by SECRETS-005 R4: Routes and services are loaded once | SECRETS-005 |
| `crates/lys-secrets/src/bin/lys-secrets/held.rs` | touched by SECRETS-005 R6: Views ask each permission once per request | SECRETS-005 |
| `crates/lys-secrets/src/broker/scope.rs` | touched by SECRETS-005 R6: Views ask each permission once per request | SECRETS-005 |
| `crates/lys-secrets/src/files.rs` | touched by SECRETS-005 R6: Views ask each permission once per request | SECRETS-005 |
| `crates/lys-secrets/src/broker/signing.rs` | R1: A signing key is a secret with one named purpose | SECRETS-006 |
| `crates/lys-secrets/src/broker/signing_tests.rs` | R1: A signing key is a secret with one named purpose | SECRETS-006 |
| `crates/lys-secrets/src/broker.rs` | R1: A signing key is a secret with one named purpose | SECRETS-006 |
| `crates/lys-secrets/src/bin/lys-secrets/sign.rs` | R2: The broker builds what it signs and returns the signature only | SECRETS-006 |
| `crates/lys-secrets/tests/signing.rs` | R2: The broker builds what it signs and returns the signature only | SECRETS-006 |
| `crates/lys-secrets/src/bin/lys-secrets/main.rs` | R2: The broker builds what it signs and returns the signature only | SECRETS-006 |
| `crates/lys-secrets/tests/log_window.rs` | R1: The log command pages from the tail | SECRETS-007 |
| `crates/lys-secrets/src/bin/lys-secrets/args.rs` | R1: The log command pages from the tail | SECRETS-007 |
| `crates/lys-secrets/src/broker/rotation.rs` | R2: The whole-log read is an audit, by name | SECRETS-007 |
| `crates/lys-secrets/src/bin/lys-secrets-demo.rs` | R2: The whole-log read is an audit, by name | SECRETS-007 |
| `crates/lys-secrets/src/error/signing.rs` | the signing refusals | SECRETS-006 |
| `crates/lys-secrets/src/error/name.rs` | refusal names | SECRETS-006 |
| `crates/lys-secrets/src/bin/lys-secrets/cli.rs` | the broker's subcommands | SECRETS-006 |
| `crates/lys-secrets/src/bin/lys-secrets/oauth_proxy.rs` | the OAuth proxy route | SECRETS-006 |
| `crates/lys-secrets/src/broker/spawn.rs` | the login hand-over at spawn | SECRETS-006 |
| `crates/lys-secrets/src/broker/records.rs` | the sealed record read | SECRETS-006 |
| `crates/lys-secrets/src/bin/lys-secrets/signature_route.rs` | the signature route | SECRETS-006 |
| `crates/lys-core/src/agent_request.rs` | the bytes of a signed agent request, shared by signer and verifier | SECRETS-006 |
| `crates/lys-core/src/lib.rs` | lys-core's module declarations | SECRETS-006 |
| `crates/lys-core/src/keys/identity.rs` | the Ed25519 identity and its seed constructor | SECRETS-006 |
| `crates/lys-identity-server/src/agent_signature.rs` | the agent request signature check | SECRETS-006 |
| `crates/lys-secrets/tests/support/served.rs` | the served broker of the tests | SECRETS-006 |
| `crates/lys-secrets/tests/support/leases.rs` | the leases fixture with its clock | SECRETS-006 |

## Inventory

- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority: 'Secrets: the temporary key model' through 'What revoking reaches', 'Two rules from Tom, 13:55', 'Lifecycle, budgets and leases' and 'Where it lives: lys'
- `docs/design/decisions.json` — the project decision ledger this cluster anchors to
- `crates/` — the five lys crates (lys, lys-core, lys-anchor, lys-anchor-cli, lys-log-store): sealed envelopes, the delegation format, the log. The door repository, where the statement places the store and the proxy, is the cambium checkout, apps/cambium in the ablative estate on this Mac; it is read, never written, by this brief
- `docs/design/identity/CONFORMANCE.md` — Committed behaviour source: docs/design/identity/CONFORMANCE.md at commit 1353c22. Bind the SEC_* acceptance IDs to its rows before dispatch; mock-up sample data is not enforcement evidence.

## Constraints

- **CN1** — No credential, token or key value is ever written, read or quoted in any document of this cluster.
- **CN2** — No row of SECRETS-002 is dispatched before Waffles has reviewed it.
- **CN3** — Every path written in a document of this cluster is relative to the repository root, whatever directory a session starts in; a command runs from its own tree and spells its paths from there.
- **CN4** — Resolve standalone broker-host ownership before dispatch. Earlier SECRETS-002 Cambium path proposals and empty door-owned file walls are not authority to make Cambium a required runtime service; ADR-004 remains binding.
- **CN5** — No row of SECRETS-003 is dispatched before the lead has reviewed it; no code row starts before the baseline row and the contract row are accepted.
