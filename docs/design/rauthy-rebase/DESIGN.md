---
type: design
cluster: rauthy-rebase
title: Rauthy release rebase: the fork and the pin onto the hardened upstream release
---

# Rauthy release rebase: the fork and the pin onto the hardened upstream release

> **Cluster:** rauthy-rebase

## Intention

The identity line's sign-in authority runs on a Rauthy fork we maintain (ADR-009), and real people may only sign in once that fork stands on an upstream release that carries the authentication hardening the baseline audit found missing. When this cluster is done, moving the fork onto such a release is a measured act: the trigger is a set of recorded facts, the fork's history reads as upstream's tag with our commits on top by name, every earlier lys pin still clones, and the standalone install is proved to come up on data the previous build wrote.

The work should feel safe to run unattended. Nothing that others depend on moves until the thing it moves to is green, every force-push names what it expects to replace, and every failure stops the card by name and leaves the fork where it was. The card gathers the evidence the decision to open real sign-in will need; it does not take that decision.

## Problem

The fork's ablative branch and the lys vendor/rauthy pin sit on upstream v0.36.2 (dd61ac3c84d6b238108dc8438b53043b5177a662). The baseline audit (docs/design/identity/RAUTHY-BASELINE.md) found that this release does not recheck account status at code exchange, does not bind a code to its exact redirect URI, and lets a logged-out session be revived; upstream fixed these in c44f7cac (#1696) and 989f9ff9 (#1728), which are on upstream main but in no release above v0.36.2. The named blocker IDENTITY-001-UPSTREAM-AUTH-STATE therefore holds IDENTITY-001 rows 06 and 07 (real-person sign-in), and IDENTITY-001 names the rebase that clears its condition as the separate brief IDENTITY-002. Upstream has no test for any of the three cases, so a release carrying the commits could still regress them unseen. And the rebase itself is a force-push of a branch whose exact commits earlier lys commits pin: done carelessly it strands those gitlinks, or leaves ablative on a head that does not build.

## Solution

One card, RAUTHYREBASE-001 (the IDENTITY-002 of IDENTITY-001 and ADR-009), written ahead of its trigger and run as named steps in dependency order. trigger-check reads upstream's non-pre-release tags above v0.36.2, tests each for the two hardening commits by ancestry, records that the two commits add no guard test, and copies the accepted v0.36.2 exception lines of the deployment report by one recorded command, grep -n -F 'accepted v0.36.2 exception'. trigger-guards writes the three guard tests as one fork commit, places it on each candidate tag under its own ref rauthy-rebase/<tag commit>-guards, and reads there the logs of the fork gate test command, just test-postgres and just test-hiqlite from the commit's justfile, each starting its test backend before cargo test; the newest tag whose guards pass is the base. migration-numbers compares upstream's and the fork's postgres and hiqlite migration numbers and stops on a shared one. rebase carries the fork commits, when any exist, onto the tag unaltered with the guard commit on top, on the candidate branch rauthy-rebase/<tag commit>, while the -guards ref stays until the pin move has landed; candidate-build and candidate-suite prove that head green on that branch, in both cases. Only then are the previous ablative head kept under a pre-rebase tag and ablative moved by lease (ADR-019); any later failure puts ablative back on the tagged head. pin-move, pin-clone and earlier-clone move the lys gitlink and prove it with DIRECTORY-002's ID001_PIN_CLONE and a recursive clone of the lys start commit. install brings a disposable PostgreSQL database (ADR-005) up under the previously pinned build, then under the rebuilt image, and signs in the identity the old build wrote. records writes the rebase report and replaces the copied exception lines in the deployment report; once the lys gate has passed and the pin move has landed, guards-ref-removal writes the gate result into the card's landing record as evidence for IDENTITY-001-UPSTREAM-AUTH-STATE (ADR-003) and removes the -guards ref.

The lys pull request changes four paths: the vendor/rauthy gitlink, the Rauthy entry of deploy/identity/versions.json, the deployment report and the new rebase report. The guard-test file and any fork commit live in the fork's repository and are named with their owner, never listed as lys files (ADR-009). The card needs DIRECTORY-002 landed, because ID001_PIN_CLONE, deploy/identity/versions.json and the deployment report are its; it does not wait for DIRECTORY-004, whose provider-link commit, if it exists when the trigger arrives, is one of the fork commits the rebase carries.

## Principles

- **P1** — The trigger is recorded facts, each the output of the command that measured it, never a judgement.
- **P2** — Nothing others depend on moves before the head it moves to is green; ablative is never left on a failing head.
- **P3** — Every force-push names the head it expects to replace, and a refused lease stops the card by name.
- **P4** — A conflict or a shared migration number stops the card; our commits and our migrations are never rewritten or renumbered to fit.
- **P5** — Existing data is a copy the card made itself; the kept development database is never touched.
- **P6** — The card records evidence for the decision to open real sign-in and never takes that decision.

## Decisions

- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
- ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
- ADR-019 — The fork's ablative branch moves only from a green candidate, with its previous head kept under a tag — Before any force-push of ablative, its previous head is pushed under a tag naming it (pre-rebase-<old base tag>-<short sha>). The rebased head is proved first on a candidate branch rauthy-rebase/<new upstream commit>; ablative is force-pushed with --force-with-lease naming the tagged previous head only after the fork build and full suite pass there. A failure on the candidate moves nothing and keeps the candidate branch; a later failure in the same card resets ablative to the tagged head with --force-with-lease naming the rebased head, verified by ls-remote, and stops by name. Before the rebased build, the postgres and hiqlite migration numbers of both sides are listed, and a number held by both stops the card naming both files and the upstream commit; it is never renumbered to fit. Rejected: force-pushing the rebased head straight to ablative and proving it there, force-pushing without keeping the previous head reachable, and renumbering a colliding fork migration.

## Goals

- docs/design/rauthy-rebase holds design.json, checklist.json, stories.json, their rendered markdown and briefs/RAUTHYREBASE-001.json with its rendered markdown, and sh scripts/design/gate.sh exits 0.
- Every checklist item and user story of this cluster is covered by a RAUTHYREBASE-001 requirement, and check-coverage.py reports the cluster clean.
- Roadmap row RM-003 links RAUTHYREBASE-001, its trigger text matches the ruled trigger, and its provenance is unchanged.
- When the card runs, git log <tag commit>..ablative lists our fork commits by subject with exactly one guard-test commit on top, and vendor/rauthy names that top commit.
- When the card runs, ID001_PIN_CLONE passes on the lys pull request's branch and the test identity the previous build wrote signs in under the rebased build.

## Non-Goals

- Lifting IDENTITY-001-UPSTREAM-AUTH-STATE and opening real-person sign-in on IDENTITY-001 rows 06 and 07 — Whether real people may sign in is the human authority's recorded act (ADR-003); this card records the gate result as its evidence.
- Moving the kept development data onto the new Rauthy release — Its own act, after DIRECTORY-002's backup and restore has landed, and it backs up first.
- Renumbering DIRECTORY-004's migrations after upstream's highest — Ruled on DIRECTORY-004's own brief, not in this card.
- A second pin check beside ID001_PIN_CLONE — DIRECTORY-002 owns the pin check; the Row 01 gate is ID001_PIN_CLONE.
- An upstream contribution, a cherry-pick or a nightly base — ADR-009: release-tag rebases only; no upstream contribution is made as part of this card.
- Creating the fork's own brief and chain in ablative-io/rauthy — The fork's chain is the fork's; this card is blocked until it exists.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `vendor/rauthy` | the Rauthy fork submodule; its gitlink moves to the guard-test commit on top of the rebased ablative head |  |
| `deploy/identity/versions.json` | pinned releases and digests of the standalone install; the Rauthy entry's image field takes the rebuilt image's value | DIRECTORY-002 |
| `docs/design/identity/reports/IDENTITY-001-deployment.md` | the deployment report; its v0.36.2 exception lines are replaced by the new release and its advisory check | DIRECTORY-002 |
| `docs/design/identity/reports/IDENTITY-002-rebase.md` | the rebase report: trigger facts, migration lists, fork commits, every step's command and result, the installed hash and the evidence handed on | RAUTHYREBASE-001 |

## Inventory

- `vendor/rauthy` — Gitlink dd61ac3c84d6b238108dc8438b53043b5177a662 (upstream v0.36.2) on branch ablative of ablative-io/rauthy, tracked by .gitmodules; not initialised in a fresh clone. ablative holds 0 fork commits; the fork's main mirrors upstream at 989f9ff9a2e18a9a3195084a86541c14f890a9e2.
- `docs/design/identity/RAUTHY-BASELINE.md` — Row 01 baseline: line 11 defines IDENTITY-002 and its three guard cases, line 31 names IDENTITY-001-UPSTREAM-AUTH-STATE, line 39 names the fork's maintenance owner and what each update records; the table at lines 21-29 names the source paths the guards exercise. Read, never rewritten.
- `docs/design/identity/briefs/IDENTITY-001.json` — external_dependencies IDENTITY-002 (lines 462-473): 4 hours, blocks rows 06 and 07; line 148 defines ID001_PIN_CLONE. Read, never rewritten.
- `docs/design/directory/briefs/DIRECTORY-002.json` — Owns ID001_PIN_CLONE, deploy/identity/versions.json and docs/design/identity/reports/IDENTITY-001-deployment.md; not landed, so none of the three exists on lys main yet.
- `docs/design/directory/briefs/DIRECTORY-004.json` — The provider-link fork commit and its tests ID001_LINK_PAIR, ID001_LINK_REFUSAL, ID001_LINK_MIGRATION and ID001_LINK_AUDIT under the fork's tests/identity_links/; blocked on a fork-owned brief that does not exist.
- `docs/design/roadmap.json` — RM-003 is this work's one row: status idea, cluster rauthy-rebase, links.briefs empty, trigger text predating the ruling.
- `crates/lys/tests` — Four test files and no identity_*.rs; ID001_PIN_CLONE arrives here with DIRECTORY-002.
- `scripts/design/gate.sh` — The design leg: validates, coverage-checks and render-compares every directory holding a design.json.

## Constraints

- **CN1** — Nothing in the products Cambium and Aion changes.
- **CN2** — No production node is touched and no production key is generated; publishing lys-anchor, generating a production key and emitting a receipt outside a test stay the project owner's acts.
- **CN3** — No upstream contribution is made; the fork's main is never pushed to or reset; no upstream commit is altered, squashed into or cherry-picked.
- **CN4** — A fork commit is never rewritten to fit a conflict, and a migration is never renumbered to fit a shared number; either stops the card by name.
- **CN5** — The kept development database is never connected to, copied or migrated by this card.
- **CN6** — IDENTITY-001-UPSTREAM-AUTH-STATE is not lifted and IDENTITY-001 rows 06 and 07 are not opened by this card.
- **CN7** — No second pin check is built; ID001_PIN_CLONE is the pin gate.
- **CN8** — IDENTITY-001's record and docs/design/identity/RAUTHY-BASELINE.md are read, never rewritten.
- **CN9** — A file of the fork's repository is named with its owner and never listed as a file of a lys brief (ADR-009).
