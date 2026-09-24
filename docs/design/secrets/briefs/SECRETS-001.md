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

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 met: SECRETS-002.json exists, and validate.py prints 'design/secrets/briefs/SECRETS-002.json: OK [brief.schema.json]'. Row 2 met: C1's eight parts map to R1 to R9, with the revocation part split into R7 (the three cases) and R8 (in-flight cancellation). Each requirement has 3 to 7 acceptance criteria. R1, R6 and R9 list lys files in files. R2, R3, R4, R5, R7 and R8 name only door paths (for example crates/cambium-door/src/http/secrets_proxy.rs and crates/cambium-store/src/traits/secrets.rs), in their spec text, as R1's own rule requires. Row 3 met: every path has an owner and a citation to the statement by line (for example :25, :27, :82) or to the source. Lys paths are in files and structure. Door paths are named in the spec with the door repository (cambium, read at cbcd2cc9d) and cited against crates/cambium-store/src/traits/mod.rs, crates/cambium-door/src/http/router.rs and crates/cambium-door/src/http/agent_seat.rs:13-15. No files entry or structure row carries a root token. Row 4 met: the cancellation rule is R8, with three acceptance criteria of its own. Row 5 met: the delegation schema is recorded OPEN in R1 and R9. Provider revocation of a login token is OPEN in R5 and R7. The proxy-handle login path is OPEN in R5. Points the source raised are also recorded open: the seat/speaks-for role has no consumer (crates/lys-core/src/delegation/artifact.rs:242-254); v1 has no expiry (crates/lys-core/src/delegation/mod.rs:218-221); sealed-envelope/v1 seals with an empty AAD (crates/lys-core/src/seal/sealed_envelope.rs:180-183, docs/design/WIRE-FORMATS.md:18); the audit line's signer (statement :80 against agent_seat.rs:13-15) and its schema (:187); the file for the revocation fold; the revolver's worker-side owner; and the cancellation rule itself until its review. Row 6 met: check-coverage.py exits 0 with no structure failure. Row 7 met: the cluster holds no credential values, and a grep for common token shapes found nothing.
- Deviation: (1) Six requirements (R2, R3, R4, R5, R7, R8) have empty files arrays. They touch only door-owned files, which R1's spec says must be named in the spec text and not in files until a door root is set. Their 'at least one file path' is the door path in their spec. (2) validate.py exits 1 for the cluster because of design.json's existing `gate` field, which is not mine to change. Every document of this round validates OK. (3) The worker side of the token revolver lives in manifold (crates/manifold-node/src/seat/launcher.rs:31-40 at 3df5ac5f64). The design inventory names no engine repository, so I read it only to show that its owner is open.
- Files changed:
  - created: `docs/design/secrets/briefs/SECRETS-002.json` — The implementation brief: nine requirements R1 to R9 in EARS form. Each carries acceptance criteria, owners with citations and open points. Blocked by Waffles' review, SpiceDB beside the door, the door root not being set, the delegation schema, and the lys-core release that freezes lys/delegation/v1.
  - created: `docs/design/secrets/briefs/SECRETS-002.md` — The brief rendered from its JSON by render-cluster.py.
  - modified: `docs/design/secrets/design.json` — Gains three structure rows with brief SECRETS-002, one per lys path the brief lists in files: crates/lys-core/src/delegation/mod.rs, docs/design/WIRE-FORMATS.md, crates/lys-core/src/seal/mod.rs. The existing gate field is untouched.
  - modified: `docs/design/secrets/DESIGN.md` — Re-rendered with the three new structure rows.
- Checklist delivery:
  - [x] C1 — SECRETS-002 is a design-system brief with one numbered requirement, carrying acceptance criteria and file paths, for each part of the temporary key model: the handle and the proxy swap with its audit line; rotation under one handle; the token revolver as the first consumer; OAuth refresh at the proxy; the seat's own login put into its environment at spawn; sealed knowledge tagged in SpiceDB; the three revocation cases, including the cancellation rule for calls in flight; and leases counted by uses, time window and spend. — R1 to R9 cover the eight parts. The cancellation rule is R8, on its own.
- Story delivery:
  - [x] S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time. — Nine numbered requirements with acceptance criteria and blockers, so a reviewer can take them one row at a time; CN2 is repeated in blocked_by and boundaries.

**Review (recorded):**

- Alignment: fixed
- Acceptance verdicts:
  - [x] docs/design/secrets/briefs/SECRETS-002.json exists and validate.py reports it valid. — With DS2_METHOD=/Users/tom/Developer/projects/deno_rust/meridian/.meridian/design-system-v2, validate.py design/secrets prints 'design/secrets/briefs/SECRETS-002.json: OK [brief.schema.json]' and 'All 5 document(s) valid.' It exits 0, before and after the harden edits.
  - [x] SECRETS-002 has a requirement for each of the eight parts named in C1, each with at least one acceptance criterion and at least one file path. — The parts map to R1 to R9: handle and proxy (R1), rotation (R2), revolver (R3), OAuth (R4), spawn login (R5), sealed records (R6), revocation (R7, plus R8 for in-flight calls) and leases (R9). Each has 3 to 7 acceptance criteria. R1, R6 and R9 list lys paths in files. R2, R3, R4, R5, R7 and R8 name door paths in their spec text, as SECRETS-001 R1's spec requires, for example crates/cambium-door/src/http/secrets_rotation.rs in R2 and secrets_in_flight.rs in R8.
  - [x] Every file SECRETS-002 names carries its owning repository with a citation to the statement or the source; lys-owned files are in files and structure, door-owned files are in the spec text only, and no structure row or files entry in this cluster carries a root token. — The lys paths delegation/mod.rs, seal/mod.rs and WIRE-FORMATS.md are in files and in design.json structure. Door paths appear only in the spec text, each under the prefix 'Door-owned, named here only (the door repository ... read at cbcd2cc9d)'. I checked cbcd2cc9d in apps/cambium: agent_seat.rs:13-15 says the private key is never persisted, and the store traits' mod.rs has no secrets module. A script check found no files entry beginning with /, $, < or ~ and none containing '..'. The design.json structure paths are all relative to the repository root.
  - [x] The cancellation rule for calls already admitted by the proxy is a requirement of its own with its own acceptance criteria. — SECRETS-002 R8, 'State and enforce the cancellation rule for calls already admitted', has 3 acceptance criteria of its own and cites the statement at :55.
  - [x] Each point the statement leaves open appears in SECRETS-002 as open and is not decided there. — The delegation schema (:21) is OPEN in R1(a) and R9. Provider revocation of a login token (:56) is OPEN in R5(b) and R7(a). The proxy-handle login path (:45) is OPEN in R5(a). The audit event schema (:187) is OPEN in R1(e). After the harden fix, R7 no longer chooses how an engine learns of a revocation.
  - [x] check-coverage.py reports every SECRETS-002 path present in design.json structure. — check-coverage.py design/secrets reports 'Coverage clean: all items covered, briefs consistent.' and exits 0. The 3 lys paths in SECRETS-002's files are structure rows with brief SECRETS-002.
  - [x] No file in the cluster contains a credential, token or key value. — A grep -rE across docs/design/secrets/ found no matches (exit 1). It looked for sk-, ghp_, eyJ, AKIA, BEGIN key blocks, xox tokens and hex runs of 40 or more characters.
- Checklist verified: C1
- Stories verified: S1
- Issues:
  - R8 cited statement :156 for Chippy's release 2, 'demonstrate revocation and recovery after a crash'. That quotation is on line 155; line 156 is release 3.
  - R9 acceptance fixed its one-use race test at 'over 100 repetitions of the race'. That count traces to nothing in the statement, the design or the brief, and a race that is not forced may never fire.
  - R7 acceptance 2 wrote '(the test double engine receives the end request)'. That decides that the door sends the engine an end request. The statement (:56) says only that the engine that runs the seat ends it on its own.
- Fixes:
  - Changed R8's citation to docs/design/identity/STATEMENT-2026-09-22.md:155.
  - Removed the repetition count of 100 from R9's race criterion. Both requests are now held at the use check until both have arrived, so the race fires on every run.
  - Removed the end-request parenthetical from R7 acceptance 2. The criterion now asserts only that the door names the seat and ends no process.
  - Re-rendered SECRETS-002.md with render-cluster.py; a second run was byte-identical.

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

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 met: check-coverage.py exits 0. It reports 'Coverage clean: all items covered, briefs consistent': 14 items and 11 stories, with C4 to C14 and S2 to S11 each named by at least one SECRETS-002 requirement and no unknown ids. Row 2 met: render-cluster.py exits 0, and a second run leaves the rendered markdown byte-identical (same checksum before and after), so the markdown matches the JSON. Row 3 met: S6, of the Token revolver persona, reads 'when a session prints its usage-limit words, I want to ask the broker for my next account instead of walking my own list', and R3 carries it.
- Deviation: (none)
- Files changed:
  - modified: `docs/design/secrets/checklist.json` — Adds the section 'The broker's implementation' with items C4 to C14: one per part, plus C13 (open points stay open) and C14 (no engine dependency).
  - modified: `docs/design/secrets/CHECKLIST.md` — Re-rendered from checklist.json.
  - modified: `docs/design/secrets/stories.json` — Adds personas Person, AI Agent, Token revolver, Engine, Operator and a Reviewer who reads the audit, with stories S2 to S11.
  - modified: `docs/design/secrets/USER-STORIES.md` — Re-rendered from stories.json.
- Checklist delivery:
  - [x] C2 — Every checklist item and user story the broker's implementation needs is recorded in this cluster and covered by a SECRETS-002 requirement. — C4 to C14 and S2 to S11 are recorded and each is covered by a SECRETS-002 requirement; coverage exits 0.
  - [x] C3 — The rendered markdown of this cluster matches its JSON. — Re-rendering changes nothing.
- Story delivery:
  - [x] S1 (Reviewer, Reviews a brief before any of its rows is dispatched) — As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time. — The rendered SECRETS-002.md shows every requirement next to the item and story ids it covers.

**Review (recorded):**

- Alignment: aligned
- Acceptance verdicts:
  - [x] check-coverage.py on the cluster exits 0: no item or story is unassigned and no brief names an unknown id. — check-coverage.py with the ledger's DS2_METHOD reports 14 checklist items, 11 user stories, 2 briefs and 'Coverage clean', and exits 0. C4 to C14 and S2 to S11 are each named by a SECRETS-002 requirement.
  - [x] render-cluster.py on the cluster leaves the rendered markdown unchanged after the commit. — render-cluster.py exits 0. The md5 of every .md file was identical before and after a second run, both before and after the harden edits. git status showed no further change.
  - [x] The stories include the token revolver asking for its next account. — stories.json has the persona 'Token revolver' with S6: 'I want to ask the broker for my next account instead of walking my own list'. SECRETS-002 R3 names S6.
- Checklist verified: C2, C3
- Stories verified: S1

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
