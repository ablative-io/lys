---
type: brief
id: SECRETS-001
cluster: secrets
title: Write the secrets broker implementation brief
---

# SECRETS-001: Write the secrets broker implementation brief

> **Cluster:** secrets
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> **Checklist:**
> - C1 — SECRETS-002 is a design-system brief with one numbered requirement, carrying acceptance criteria and file paths, for each part of the temporary key model: the handle and the proxy swap with its audit line; rotation under one handle; the token revolver as the first consumer; OAuth refresh at the proxy; the seat's own login put into its environment at spawn; sealed knowledge tagged in SpiceDB; the three revocation cases, including the cancellation rule for calls in flight; and leases counted by uses, time window and spend.
> - C2 — Every checklist item and user story the broker's implementation needs is recorded in this cluster and covered by a SECRETS-002 requirement.
> - C3 — The rendered markdown of this cluster matches its JSON.
> **Stories:**
> - S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time.

## Purpose

Agents get a short-lived, limited handle to a credential, never the credential itself, and the token revolver is the first user. This brief produces the implementation brief for that broker, so its rows can be reviewed and built one at a time. Documents only.

## Task

Read docs/design/identity/STATEMENT-2026-09-22.md, the sections 'Secrets: the temporary key model' through 'What revoking reaches', 'Two rules from Tom, 13:55', 'Lifecycle, budgets and leases: the discussion of 14:00' and 'Where it lives: lys', and the decisions ADR-001 to ADR-004 in docs/design/decisions.json. Read the source the design inventory names before assigning any file: the lys crates and the door repository. Write SECRETS-002 in the design-system brief form, add the broker's checklist items and user stories to this cluster, and add a structure row to design.json for every path SECRETS-002 names. Every path written in a document of this cluster is relative to the repository root, even though the session starts in docs/.

## Requirements

### R1: Author the implementation brief SECRETS-002

THE SYSTEM SHALL have a brief docs/design/secrets/briefs/SECRETS-002.json, valid against the design-system brief schema, with one requirement per part of the temporary key model named in C1. Each requirement SHALL state its behaviour in the statement's own terms, list acceptance criteria that can be checked against code and tests, and name the files it creates, modifies or deletes. The repository that owns each file SHALL be grounded in the statement and in the source the inventory names, cited by path and line, and never assumed: the statement places the store and proxy in the door ('Secrets: the temporary key model') and the handle and sealed-record formats in lys ('Where it lives: lys': lys sealed envelopes and the delegation format, applied). A file owned by lys is listed in the requirement's files and gains a structure row. A file owned by the door repository is named in the requirement's spec with its owner and citation and is NOT listed in files and gains NO structure row, because no root naming the door repository is set for this round; it moves into files when that root is set. A requirement whose owner the sources do not settle is recorded as open. Where the statement leaves a point open (the delegation schema; whether revoking a login token at the provider fails the seat's next call; the proxy-handle login path), SECRETS-002 SHALL record it as open and SHALL NOT settle it. design.json SHALL gain a structure row, with brief SECRETS-002, for every path SECRETS-002 names.

**Acceptance:**
- docs/design/secrets/briefs/SECRETS-002.json exists and validate.py reports it valid.
- SECRETS-002 has a requirement for each of the eight parts named in C1, each with at least one acceptance criterion and at least one file path.
- Every file SECRETS-002 names carries its owning repository with a citation to the statement or the source; lys-owned files are in files and structure, door-owned files are in the spec text only, and no structure row or files entry in this cluster carries a root token.
- The cancellation rule for calls already admitted by the proxy is a requirement of its own with its own acceptance criteria.
- Each point the statement leaves open appears in SECRETS-002 as open and is not decided there.
- check-coverage.py reports every SECRETS-002 path present in design.json structure.
- No file in the cluster contains a credential, token or key value.

**Files:**
- create: docs/design/secrets/briefs/SECRETS-002.json
- create: docs/design/secrets/briefs/SECRETS-002.md
- modify: docs/design/secrets/design.json
- modify: docs/design/secrets/DESIGN.md

**Checklist:**
- C1 — SECRETS-002 is a design-system brief with one numbered requirement, carrying acceptance criteria and file paths, for each part of the temporary key model: the handle and the proxy swap with its audit line; rotation under one handle; the token revolver as the first consumer; OAuth refresh at the proxy; the seat's own login put into its environment at spawn; sealed knowledge tagged in SpiceDB; the three revocation cases, including the cancellation rule for calls in flight; and leases counted by uses, time window and spend.

**Stories:**
- S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time.

### R2: Record the broker's checklist items and stories and cover them

THE SYSTEM SHALL add to checklist.json a section of the broker's implementation items and to stories.json the personas and stories of the people and agents who use the broker (a person granting an agent access, an agent using a handle, an operator resting an account, a reviewer reading the audit). Every item and story added SHALL be named by at least one SECRETS-002 requirement, and the rendered markdown SHALL match the JSON.

**Acceptance:**
- check-coverage.py on the cluster exits 0: no item or story is unassigned and no brief names an unknown id.
- render-cluster.py on the cluster leaves the rendered markdown unchanged after the commit.
- The stories include the token revolver asking for its next account.

**Files:**
- modify: docs/design/secrets/checklist.json
- modify: docs/design/secrets/CHECKLIST.md
- modify: docs/design/secrets/stories.json
- modify: docs/design/secrets/USER-STORIES.md

**Checklist:**
- C2 — Every checklist item and user story the broker's implementation needs is recorded in this cluster and covered by a SECRETS-002 requirement.
- C3 — The rendered markdown of this cluster matches its JSON.

**Stories:**
- S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time.

## Boundaries

- Change no code: only paths under docs/design/secrets/ are created or modified.
- A credential, token or key never passes through an agent: no credential value is written, read or quoted.
- The statement is the authority: nothing it leaves open is decided here.
- Manifold is never a structural part: the engine that starts or ends a seat is whichever one runs the agent.
- No row of SECRETS-002 is dispatched until Waffles has reviewed it.

## Verification

- From docs/: python3 $DS2_METHOD/scripts/validate.py design/secrets exits 0.
- From docs/: python3 $DS2_METHOD/scripts/check-coverage.py design/secrets exits 0.
- From docs/: python3 $DS2_METHOD/scripts/render-cluster.py design/secrets exits 0 and git status shows no further change.
