---
type: brief
id: AGENTS-013
cluster: agents
title: Adding an agent is one act: register, turn on, settings, the computer and the first run under one operation with one receipt
---

# AGENTS-013: Adding an agent is one act: register, turn on, settings, the computer and the first run under one operation with one receipt

> **Cluster:** agents
> **Depends on:** AGENTS-012
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> **Checklist:**
> - C770 — POST /agents with run registers, turns on, saves and approves settings, records a new computer or allows on a named one, and starts only with a working folder, under one operation with one receipt naming each part and the first refusal.
> - C771 — AddAgent sends one request and keeps no walk in the browser; the act says Add or Add and run by whether a folder is chosen; a refused part says what was done and the next act.
> **Stories:**
> - S441 (Tom, owner) — As a person adding an agent, I fill one form and press once, and the agent is added, turned on, allowed on the computer I named and started in its folder, or I am told exactly which step was refused and what was done before it.
> - S442 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person on a fresh install, I can name my computer and its folder on the same form that adds my first agent.

## Purpose

The 18 October 2026 surface review's people-and-agents domain (builder 02, lys-review/CONSENSUS.md, Server asks) found that "Add and run" on the surface is a walk of five server acts kept in the browser's session storage (surface/identity/src/features/people/add-and-run.ts at ca1228b0: register, activate, save and approve settings, allow on the computer, start), so a refusal half-way leaves an agent registered but not turned on, or turned on but not allowed anywhere, with the state of the walk in one tab; and that a first agent on a fresh install cannot choose its folder on the Add form, because POST /network/machines/{id}/folders needs a recorded machine and the Lys host's own computer is not recorded until someone records it.

## Task

POST /agents takes an optional run: { settings, computer | new_computer, working_folder? } and does, under one operation id, register, activate, save and approve the settings, record the computer when new_computer names one, allow the agent on it, and start it only when a working folder is named, answering one receipt that names each part done and the first part refused if any; the surface's walk is deleted and AddAgent sends one request.

## Requirements

### R1: One act adds, turns on, allows and starts an agent under one operation

Behavioural. POST /agents accepts, beside today's body, run: { settings, computer: <machine id> | new_computer: { name }, working_folder? }. With run present the server does, in order and under the one operation id it answers: register the agent; activate it; save the settings and approve them; when new_computer is given, record that computer for the caller as POST /network/machines does, then allow the agent on it, else allow it on computer; when working_folder is given, start the agent there, else do not start. The receipt names each part with done: true and the ids it made, and the first part refused with its named refusal, after which no later part runs; what was done before the refusal stays done and is named, so the person can finish from the agent's page. The route's row in openapi_table.rs carries the new body and receipt. The caller's rights for each part are the same as for the separate routes today; nothing new is allowed.

**Acceptance:**
- Red first: a test in crates/lys-identity-server/src/agents_add_run_tests.rs posts one request with new_computer and a working folder and reads a receipt with every part done and an agent that is active, allowed on the new computer and running; a second request whose caller may not start reads the first four parts done and start refused by name, and the agent active and allowed
- Without run the route answers exactly as before (the existing tests are unchanged)

**Files:**
- create: crates/lys-identity-server/src/agents_add_run.rs
- create: crates/lys-identity-server/src/agents_add_run_tests.rs
- modify: crates/lys-identity-server/src/routes_startup.rs
- modify: crates/lys-identity-server/src/openapi_table.rs

**Checklist:**
- C770 — POST /agents with run registers, turns on, saves and approves settings, records a new computer or allows on a named one, and starts only with a working folder, under one operation with one receipt naming each part and the first refusal.

**Stories:**
- S441 (Tom, owner) — As a person adding an agent, I fill one form and press once, and the agent is added, turned on, allowed on the computer I named and started in its folder, or I am told exactly which step was refused and what was done before it.

### R2: The surface sends one request and keeps no walk

Behavioural. surface/identity/src/features/people/AddAgent.tsx sends one POST /agents with run, shows the receipt part by part, and on a refused part says what was done, what was not, and the next act on the agent's page; the act reads "Add and run" when a working folder is chosen and "Add" when not. surface/identity/src/features/people/add-and-run.ts and its session-storage keys (lys.add-and-run.*) are deleted; a kept walk from before this change found in session storage is shown once as "an earlier add was left part-way; open <agent> to finish it" and removed.

**Acceptance:**
- AddAgent's vitest cases show the receipt, the refused-part words, and the two act words; grep of surface/identity/src for lys.add-and-run answers 0 except the one-time notice
- surface/identity/src/features/people/add-and-run.ts no longer exists

**Files:**
- modify: surface/identity/src/features/people/AddAgent.tsx
- delete: surface/identity/src/features/people/add-and-run.ts

**Checklist:**
- C771 — AddAgent sends one request and keeps no walk in the browser; the act says Add or Add and run by whether a folder is chosen; a refused part says what was done and the next act.

**Stories:**
- S442 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person on a fresh install, I can name my computer and its folder on the same form that adds my first agent.

## Boundaries

- SHALL NOT allow with one act anything the separate routes refuse the same caller.
- SHALL NOT start an agent without a working folder.
- SHALL NOT keep the add's progress in the browser.
- SHALL NOT put a fixed wait, deadline or poll anywhere, in code or test.

## Verification

- Red first for every requirement; cargo nextest run only (never cargo test), clippy pedantic -D warnings, cargo fmt; vitest and tsc for the surface files; the design gate's validate.py and check-coverage.py on the cluster parsed for FAIL lines, never by exit code; file length 500 on every file touched.
- Done means: every route and field answers as specified with the named tests green on the Mac, the surface reads each one, the openapi contract test is green, and the lane's END names any file it touched beyond the lists here so the brief is amended in one commit.
