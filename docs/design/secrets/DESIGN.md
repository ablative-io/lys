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

## Goals

- An implementation brief, SECRETS-002, covers every part of the temporary key model in the statement with numbered requirements and acceptance criteria.
- Every checklist item and user story of the broker is covered by a SECRETS-002 requirement.
- The token revolver can take its next account from the broker instead of its own list.

## Non-Goals

- OpenBao or another external secrets engine — Tom's model is built in Rust inside the door; an external engine is only reconsidered if credentials minted on demand are needed.
- The seat login through a proxy handle and base URL — Not tested with a subscription login; the statement keeps it open to prove later and nothing depends on it.
- The delegation schema — The statement says it is not settled.

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
