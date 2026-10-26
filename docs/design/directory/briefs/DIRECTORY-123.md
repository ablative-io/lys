---
type: brief
id: DIRECTORY-123
cluster: directory
title: Waiting on you is what needs the person, decided where it stands, at the top of Home: a check is due only after its period, an agent's checks are kept in one act, and Home fits the window
---

# DIRECTORY-123: Waiting on you is what needs the person, decided where it stands, at the top of Home: a check is due only after its period, an agent's checks are kept in one act, and Home fits the window

> **Cluster:** directory
> **Depends on:** DIRECTORY-101, DIRECTORY-104
> **Checklist:**
> - C856 — A grant is due a check only when a review period has passed since its last check (its giving, or its last keep); keeping a grant takes it off Waiting on you until the next period. DIRECTORY-123/R1.
> - C857 — A reviewer keeps every due grant an agent holds in one recorded act, naming the grants they saw; refusals by name. DIRECTORY-123/R2.
> - C858 — Waiting on you draws one row per agent with grants due a check and decides app actions where they stand, so every line is decided on Home and the count is the rows. DIRECTORY-123/R3.
> - C859 — Home leads with Waiting on you, full width, with no table that scrolls inside the page and no sideways scroll; alarms take one line each when firing and one line when none. DIRECTORY-123/R4.
> - C860 — The rail's Waiting on you opens Home's Waiting on you, whose count is the rail's count; the Requests and Checks pages stay one press from the section. DIRECTORY-123/R5.
> - C861 — Standing bars hold for DIRECTORY-123, with Tom's 26 October reading replayed: hundreds of grants, one keep per agent, the count falls to what is left. DIRECTORY-123/R6.
> **Stories:**
> - S491 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a grant to come back for a check only when its period has passed since I last kept it, so that what waits on me is what needs me, not every grant I ever gave.
> - S492 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer of an agent holding many grants, I want to keep all of its access in one act after reading the list, so that a hundred checks are not a hundred presses.
> - S493 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the person things wait on, I want every line of Waiting on you to be decided where it stands, so that a number never sends me to a page with nothing on it.
> - S494 (Newcomer to the screens, Learning what a screen shows) — As a person opening Lys, I want Home to lead with what waits on me and fit my window, so that I never scroll a table inside the page or sideways to find it.
> - S495 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a person who sees a count beside Waiting on you, I want pressing it to show exactly those things, so that the count and what I see can never disagree.
> - S496 (Directory administrator, Resolves a refused act) — As a directory administrator, I want Tom's 26 October reading replayed as a test, so that it is proven that hundreds of standing grants never again read as hundreds of things to do.

## Purpose

Tom, 26 October 2026 about 17:50, at Lys: "it's saying that there's like 500 and something things that are waiting on me, but then there's nothing there. Like I can't actually do anything, and the dashboard is really fucking stupid... the dashboard should have... the waiting on me section... there's this table up the top that scrolls, and then there's like alarms that take up the entire width, and then there's this long thing of stuff that goes after it." Read in the code at 9f432e2a: GET /waiting counts four kinds (requests, agent drafts, app actions, checks due), and the rail's count opens #/requests, which lists requests alone. A check is due for every standing grant an agent answering to the person holds (reviews_api.rs review_view: stands() and nothing else), keeping one records a decision and never takes it off the list, and there is no review period anywhere in Lys. So every live grant Tom's agents hold is a permanent line, Keep cannot lower the count, the rail sends him to a page that shows none of it, and an app action or a check only offers Open to another page. On 15 October he said the dashboard needed a rethink; this is that rethink, in his words: Home is where Waiting on you lives.

## Task

Make a check due only after a review period since its last check, keep an agent's due grants in one act, decide every Waiting on you line where it stands, lead Home with it at full width with no inner or sideways scroll and alarms as lines, and make the rail open it.

## Requirements

### R1: A grant is due a check only when its review period has passed since its last check

THE SYSTEM SHALL count a grant due for its reviewer when it stands (unrevoked, within its window) and at least the review period has passed since its last check, where its last check is its last keep or, never kept, the start of its window: giving a grant is its first check. The review period is the written configuration key [reviews] period_days, a whole number of days of at least 1; the installer and the upgrade write it as 30 where it is absent, and the service refuses to start with it missing or malformed, by name. Reason for 30: one calendar month is the cadence a person can hold in mind for access they gave, and a grant given for one task is checked before a second month runs on unseen. GET /reviews and GET /waiting read the same rule from one function, so they never disagree. A kept grant leaves both at once and returns when its period passes.

**Acceptance:**
- Red first (crates/lys-identity-server/src/waiting_api_tests.rs and tests/reviews.rs): a grant given now is not due; a grant whose window began 31 days ago and was never kept is due; keeping it removes it from GET /reviews and GET /waiting in the same answer; it is due again only once 30 days have passed since the keep; the clock is the service's test clock, never a sleep.
- A configuration without [reviews] period_days refuses to start, naming the key; the installer test reads 30 written into a fresh configuration and an upgraded one.

**Files:**
- modify: crates/lys-identity-server/src/reviews_api.rs
- modify: crates/lys-identity-server/src/reviews_state.rs
- modify: crates/lys-identity-server/src/waiting_api.rs
- modify: crates/lys-identity-server/src/waiting_api_tests.rs
- modify: crates/lys-identity-server/src/config.rs
- modify: crates/lys-identity-server/tests/reviews.rs

**Checklist:**
- C856 — A grant is due a check only when a review period has passed since its last check (its giving, or its last keep); keeping a grant takes it off Waiting on you until the next period. DIRECTORY-123/R1.

**Stories:**
- S491 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer, I want a grant to come back for a check only when its period has passed since I last kept it, so that what waits on me is what needs me, not every grant I ever gave.

### R2: A reviewer keeps every due grant an agent holds in one recorded act

WHEN a reviewer calls POST /reviews/agents/{agent}/keep with an operation and the ids of the due grants they were shown THE SYSTEM SHALL keep exactly those grants for that reviewer as one durable write, one decision per grant naming the reviewer, and answer the kept grants with their decisions; asked again with the same operation it answers the same without writing. It SHALL refuse the whole act, keeping none, with GrantNotDue naming any grant that is not due for this reviewer, GrantNotVisible for a grant they may not see, AgentMismatch for a grant the agent does not hold, ReviewerOnly for a caller who is not the reviewer, and ReviewReused for an operation already used for a different list. A grant due after the list was read is not kept: the act keeps what was seen, never more. The single-grant keep stands as it is.

**Acceptance:**
- Red first (crates/lys-identity-server/tests/reviews.rs): an agent with 120 due grants is kept in one call; its review store gains 120 decisions in one write (the store's write count read before and after); each refusal by name keeps nothing; a repeated operation writes nothing; a grant given between the read and the keep stays due.

**Files:**
- modify: crates/lys-identity-server/src/reviews_api.rs
- modify: crates/lys-identity-server/src/reviews_store.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-identity-server/src/error.rs
- modify: crates/lys-identity-server/tests/reviews.rs
- modify: surface/identity/src/generated/index.ts

**Checklist:**
- C857 — A reviewer keeps every due grant an agent holds in one recorded act, naming the grants they saw; refusals by name. DIRECTORY-123/R2.

**Stories:**
- S492 (Reviewer, Reviews a brief before any of its rows is dispatched) — As a reviewer of an agent holding many grants, I want to keep all of its access in one act after reading the list, so that a hundred checks are not a hundred presses.

### R3: Every Waiting on you line is decided where it stands, with one row per agent for its checks

GET /waiting SHALL answer one review entry per agent with grants due for the caller, carrying the agent, the due grants (id, relation, resource in words, last check) and their count, in place of one entry per grant, and every app action entry SHALL carry its draft as GET /product-drafts answers it. Waiting on you (WaitingOnYou.tsx) SHALL draw a review row as "Check the access <agent> holds: <n> grants" with the grants listed beneath on request, a Keep all act (R2, sending the ids shown) and an Open act to the agent's access page where any grant is withdrawn; and SHALL draw an app action row with its approve and refuse acts in the row, through the existing /product-drafts/{id}/approve and /refuse. Every act is an Act. The count in the heading is the number of rows, the rail's count is the same number, and a part that could not be read stays its own line, as today.

**Acceptance:**
- Red first (crates/lys-identity-server/src/waiting_api_tests.rs): an agent with three due grants is one entry with three grants; two agents are two entries; an app action entry carries its draft.
- Red first (surface/identity/src/features/dashboard/dashboard-waiting.test.tsx): a review row shows the agent and count, Keep all posts the ids shown and the row leaves on success; an app action row approves and refuses in place; no row's only act is Open.

**Files:**
- modify: crates/lys-identity-server/src/waiting_api.rs
- modify: crates/lys-identity-server/src/waiting_api_tests.rs
- modify: surface/identity/src/features/dashboard/WaitingOnYou.tsx
- modify: surface/identity/src/features/dashboard/waiting.ts
- modify: surface/identity/src/features/dashboard/dashboard-waiting.test.tsx
- modify: surface/identity/src/features/drafts/ProductDrafts.tsx
- modify: surface/identity/src/features/reviews/KeepGrant.tsx
- modify: surface/identity/src/generated/index.ts

**Checklist:**
- C858 — Waiting on you draws one row per agent with grants due a check and decides app actions where they stand, so every line is decided on Home and the count is the rows. DIRECTORY-123/R3.

**Stories:**
- S493 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As the person things wait on, I want every line of Waiting on you to be decided where it stands, so that a number never sends me to a page with nothing on it.

### R4: Home leads with Waiting on you, fits the window, and draws alarms as lines

Home (Dashboard.tsx, dashboard.css) SHALL draw, in order: the stop line as today; Waiting on you, the full width of the page; alarms, one line each while firing and a single "No alarms" line when none fire, never a full-width panel of tiles; then Exceptions and Spend side by side where the window is wide enough, one above the other where it is not. Home SHALL NOT draw the agents table: the tree beside the terminal lists agents, and Stop everything stays on the stop line. No section scrolls inside the page; the page scrolls as one. At any window from 360 pixels wide up, nothing on Home scrolls sideways. Getting started stays above Waiting on you while it shows.

**Acceptance:**
- Red first (surface/identity/tests/dashboard-layout.test.tsx): the sections come in the order above; no element on Home carries overflow scrolling of its own; the agents table is absent and Stop everything is on the stop line; alarms render as lines, with the none line when none fire.
- Red first (surface/identity/tests/dashboard-alarms.test.tsx): a firing alarm is one line with its act; none firing is one line.

**Files:**
- modify: surface/identity/src/features/dashboard/Dashboard.tsx
- modify: surface/identity/src/features/dashboard/dashboard.css
- modify: surface/identity/src/features/dashboard/Alarms.tsx
- modify: surface/identity/tests/dashboard-layout.test.tsx
- modify: surface/identity/tests/dashboard-alarms.test.tsx
- modify: surface/identity/tests/dashboard.test.tsx

**Checklist:**
- C859 — Home leads with Waiting on you, full width, with no table that scrolls inside the page and no sideways scroll; alarms take one line each when firing and one line when none. DIRECTORY-123/R4.

**Stories:**
- S494 (Newcomer to the screens, Learning what a screen shows) — As a person opening Lys, I want Home to lead with what waits on me and fit my window, so that I never scroll a table inside the page or sideways to find it.

### R5: The rail's Waiting on you opens Home's Waiting on you, and the pages it gathers stay one press away

The rail's Waiting on you (railItems.ts, Rail.tsx) SHALL open Home with its Waiting on you section in view and focused, and its count SHALL be the section's count from the one GET /waiting read. The section's foot SHALL carry links to Requests (#/requests), Checks (#/reviews) and Drafts (#/access/drafts), so every list it gathers is one press away; those pages are unchanged.

**Acceptance:**
- Red first (surface/identity/tests/rail.test.tsx): pressing Waiting on you goes to Home with the section focused; the rail count equals the section's heading count for the same answer; the section's foot links reach the three pages.

**Files:**
- modify: surface/identity/src/shell/railItems.ts
- modify: surface/identity/src/shell/Rail.tsx
- modify: surface/identity/tests/rail.test.tsx
- modify: surface/identity/src/features/dashboard/WaitingOnYou.tsx

**Checklist:**
- C860 — The rail's Waiting on you opens Home's Waiting on you, whose count is the rail's count; the Requests and Checks pages stay one press from the section. DIRECTORY-123/R5.

**Stories:**
- S495 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a person who sees a count beside Waiting on you, I want pressing it to show exactly those things, so that the count and what I see can never disagree.

### R6: Standing bars hold for DIRECTORY-123, with Tom's reading replayed

THE SYSTEM SHALL hold every standing bar on the exact final commit: cargo fmt, clippy pedantic -D warnings with all targets, cargo nextest run (never cargo test), cargo test --doc, ast-grep, the file-length gate, tsc and vitest for surface/identity, and scripts/design/gate.sh with the rendered markdown equal to its JSON. One test replays Tom's reading through the public routes: a person with six agents holding 540 standing grants, all older than the period, reads 6 review rows and a count of 6, never 540; Keep all on each leaves a count of 0; a request and an app action added after are 2 rows, each decided in its row. No sleep, timeout taken as readiness or poll is added. No test is deleted, weakened or marked serial.

**Acceptance:**
- The replay test (crates/lys-identity-server/tests/reviews.rs, a module named tom_26_october) fails on 9f432e2a and passes on the final commit, both runs' lines kept in the handback.
- Every gate leg green on the final commit, each judged by its parsed output lines, never by its exit code.

**Checklist:**
- C861 — Standing bars hold for DIRECTORY-123, with Tom's 26 October reading replayed: hundreds of grants, one keep per agent, the count falls to what is left. DIRECTORY-123/R6.

**Stories:**
- S496 (Directory administrator, Resolves a refused act) — As a directory administrator, I want Tom's 26 October reading replayed as a test, so that it is proven that hundreds of standing grants never again read as hundreds of things to do.

## Boundaries

- SHALL NOT keep or withdraw any live grant by hand, script or raw route: Tom's checks are kept by Tom through R2's act after install.
- SHALL NOT change what a grant permits, its window, or its revocation: only when it is due a check.
- SHALL NOT remove the Requests, Checks or Drafts pages, or the single-grant keep.
- SHALL NOT add a sleep, a timeout taken as readiness, or a poll anywhere; the review clock is the service's clock, read when asked.
- SHALL NOT draw a bare button for an act: every act is an Act.

## Verification

- cargo nextest run -p lys-identity-server -E 'binary(reviews) | test(waiting_api)' alone, read by its PASS and FAIL lines; vitest on surface/identity for dashboard-waiting, dashboard-layout, dashboard-alarms, dashboard and rail, read by its summary line; then the full gate.
- Red first: every new test, the replay included, is run before the code and its failing lines kept in the handback.
- Walked on the installed Lys as a test person with agents and old grants: the count on the rail and on Home agree; Keep all lowers it by one row; an app action is approved in its row; Home is read at 1440 and at 390 pixels wide with nothing scrolling sideways and no inner scroll. Each step's screen is read, not assumed.
- Hospital line: If this code were used in a hospital, the worst credible failure is a charge nurse whose queue shows five hundred items that cannot be opened, who stops looking at it, and misses the one real request to grant an agent access to a ward's records, and it could harm about every patient on that ward while the access stands unchecked.
