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

A broker in Rust in the lys repository, as the crate crates/lys-secrets of the standalone identity platform (ADR-019), which Cambium consumes through the door and never hosts: an encrypted store of real credentials, handles bound to identities, a proxy that checks SpiceDB, swaps the handle, forwards the call and writes one audit line, rotation across several accounts under one handle, OAuth refresh at the proxy, the seat's own login at spawn, sealed records tagged in SpiceDB, and leases counted by uses, time window and spend. A lease is a broker record under the audit log, read only by the broker; the time window a signed delegation carries is never widened by it (ADR-020). The token revolver is its first consumer. SECRETS-002 states what the broker does, requirement by requirement. SECRETS-003 builds it: a documents-only baseline recorded from source, a contract with its adversarial review before any code, and code rows that each deliver named SECRETS-002 requirements with counted acceptance legs, an estimate in hours, and the directory briefs they wait on.

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
- ADR-019 — The secrets broker lives in lys, as crates/lys-secrets; Cambium consumes it and is never its home — The broker lives in the lys repository as the crate crates/lys-secrets, part of the standalone identity platform. A person installs Lys to hold secrets; Cambium reaches the broker through the door as a consumer and is never its home. Rejected: the broker inside the cambium door that SECRETS-002 names, which would make Cambium a runtime a person must install to keep a credential.
- ADR-020 — A lease is a broker record under the audit log, not a signed format — The lease is a broker record under the audit log: it counts uses, time and spend against a delegation and is read only by the broker, so nothing a stranger verifies changes. The signed delegation keeps the time window SECRETS-002 R9 gives it, and the lease's own not_after never widens that window. Rejected: a signed lease format, a new version beside lys/delegation/v1 with its own adversarial review.

## Goals

- An implementation brief, SECRETS-002, covers every part of the temporary key model in the statement with numbered requirements and acceptance criteria.
- Every checklist item and user story of the broker is covered by a SECRETS-002 requirement.
- The token revolver can take its next account from the broker instead of its own list.
- An implementation brief, SECRETS-003, carries a baseline row, a contract row with its adversarial review, and code rows that deliver SECRETS-002's R1 to R9 in crates/lys-secrets, each code row with counted acceptance legs, an estimate in hours, and DIRECTORY-002, DIRECTORY-003 and DIRECTORY-006 as blockers.

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
| `docs/design/secrets/CONTRACT.md` | SECRETS-003 R2: the invariant, the handle, the lease, the cancellation rule, the SpiceDB relations and the audit line fields | SECRETS-003 |
| `docs/design/secrets/reports/SECRETS-003-adversarial-review.md` | SECRETS-003 R2: every attack tried against the contract and the clause defeating each | SECRETS-003 |
| `Cargo.toml` | the workspace manifest; gains crates/lys-secrets as a member (SECRETS-003 R3) |  |
| `Cargo.lock` | the workspace lockfile (SECRETS-003 R3) |  |
| `crates/lys-secrets/Cargo.toml` | the broker crate's manifest | SECRETS-003 |
| `crates/lys-secrets/src/lib.rs` | module declarations and the crate's invariant docs | SECRETS-003 |
| `crates/lys-secrets/src/error.rs` | the broker's error type; no credential byte in any variant | SECRETS-003 |
| `crates/lys-secrets/src/store.rs` | the encrypted store of real credentials | SECRETS-003 |
| `crates/lys-secrets/src/handle.rs` | handles bound to one identity, issued, resolved and dropped | SECRETS-003 |
| `crates/lys-secrets/src/lease.rs` | the lease record: uses, time window, spend, the atomic use step and the retry outcome | SECRETS-003 |
| `crates/lys-secrets/src/audit.rs` | audit lines appended through lys-log-store | SECRETS-003 |
| `crates/lys-secrets/schema/secrets.zed` | the SpiceDB relations for handle use and sealed-record reads | SECRETS-003 |
| `crates/lys-secrets/src/proxy.rs` | the check, swap, forward and audit line | SECRETS-003 |
| `crates/lys-secrets/src/rotation.rs` | the next real account in turn under one handle | SECRETS-003 |
| `crates/lys-secrets/src/in_flight.rs` | the register of admitted calls and the cancellation rule applied at a drop | SECRETS-003 |
| `crates/lys-secrets/src/oauth.rs` | OAuth refresh at the proxy | SECRETS-003 |
| `crates/lys-secrets/src/spawn_login.rs` | the seat's own login answered at spawn and the record of which went to which seat | SECRETS-003 |
| `crates/lys-secrets/src/sealed.rs` | sealed records read by name under a relation check; keys refused for reading | SECRETS-003 |
| `crates/lys-secrets/src/next_account.rs` | the revolver's ask for its next account | SECRETS-003 |
| `crates/lys-secrets/tests/store_leases.rs` | R3's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/proxy.rs` | R4's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/oauth.rs` | R5's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/spawn_login.rs` | R6's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/sealed.rs` | R7's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/next_account.rs` | R8's counted legs | SECRETS-003 |
| `crates/lys-secrets/tests/support/mod.rs` | shared test doubles: the upstream, the provider, SpiceDB, the canary and the kill points | SECRETS-003 |

## Inventory

- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority: 'Secrets: the temporary key model' through 'What revoking reaches', 'Two rules from Tom, 13:55', 'Lifecycle, budgets and leases' and 'Where it lives: lys'
- `docs/design/decisions.json` — the project decision ledger this cluster anchors to
- `crates/` — the five lys crates (lys, lys-core, lys-anchor, lys-anchor-cli, lys-log-store): sealed envelopes, the delegation format, the log. The door repository, where the statement places the store and the proxy, is the cambium checkout, apps/cambium in the ablative estate on this Mac; it is read, never written, by this brief
- `docs/design/identity/CONFORMANCE.md` — Committed behaviour source: docs/design/identity/CONFORMANCE.md at commit 1353c22. Bind the SEC_* acceptance IDs to its rows before dispatch; mock-up sample data is not enforcement evidence.

## Constraints

- **CN1** — No credential, token or key value is ever written, read or quoted in any document of this cluster.
- **CN2** — No row of SECRETS-002 is dispatched before Waffles has reviewed it.
- **CN3** — Every path written in a document of this cluster is relative to the repository root, whatever directory a session starts in; a command runs from its own tree and spells its paths from there.
- **CN4** — Resolve standalone broker-host ownership before dispatch. Earlier SECRETS-002 Cambium path proposals and empty door-owned file walls are not authority to make Cambium a required runtime service; ADR-004 remains binding. Settled by ADR-019: the broker lives in lys as crates/lys-secrets; Cambium consumes it and is never its home.
- **CN5** — No row of SECRETS-003 is dispatched before the lead has reviewed it; no code row starts before the baseline row and the contract row are accepted.
