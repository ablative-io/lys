---
type: brief
id: DIRECTORY-104
cluster: directory
title: What waits on a person carries what deciding it needs, and the dashboard decides in its rows: Tom's reading of 18 October merged on the one waiting read
---

# DIRECTORY-104: What waits on a person carries what deciding it needs, and the dashboard decides in its rows: Tom's reading of 18 October merged on the one waiting read

> **Cluster:** directory
> **Depends on:** DIRECTORY-101
> **Design anchor:**
> - ADR-109 — The server judges a grant's standing and effective end over its chain; the screens render them and walk nothing — GrantView carries each grant's effective standing ({stands: true}, or {stands: false, refusal, grant, reason} read through the service's sight rules, so a withheld refusal names no grant or identity the caller may not see) and its effective end, both worked out on the server by the same judgement a check makes, and every screen renders those two members and walks no chain, compares no window with the clock and reads no holder's lifecycle state to decide standing. Rejected: keeping the client chain walk beside the server's judgement, and a screen-side derivation of the effective end.
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C770 — GET /waiting's request entries carry the request as GET /requests answers it and its agent_draft entries carry the draft as GET /drafts answers it, so one read serves the heading's number and the row's decision; product_draft and review entries carry neither.
> - C771 — The dashboard decides in its rows from the one waiting read: Approve access and Decline request in a request's row, Approve and Decline in a draft's row, Open on an app action or a check due, a refused part on its own line, and the word Decide nowhere; the rail says Dashboard, the page has no heading of its own, the stop symbol stands alone, and the week's figure is said once with the person's own limit under it.
> **Stories:**
> - S441 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I approve or decline a request or a draft from the dashboard row it stands in, without opening another page, and the number in the heading is the number of rows I see.
> - S442 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I open Lys and the rail says Dashboard, the page says it nowhere else, and the week's figure is said once with my own limit under it, so nothing on the front page repeats itself.

## Purpose

On 18 October 2026 from 21:04 to 21:48 Tom read the dashboard on the thing itself and said what it is: the rail says Dashboard and the page carries no heading of its own; the Decide button is gone and Approve and Decline stand in the row; the budgets are on the dashboard, the week's figure said once with the person's own limit under it; the stop symbol stands alone; nothing repeats. Lane lane/surface-06 holds that reading at f5f8539e, three commits on ca1228b0, with the three dashboards A, B and C switchable on the page for him to decide between. Meanwhile DIRECTORY-101 R1 landed on main at 5e88f3f0: GET /waiting, one oldest-first list of everything waiting on the person, read by the rail's count and Home's Waiting on you in place of four reads. The two meet in four files (Dashboard.tsx, WaitingOnYou.tsx, dashboard-waiting.test.tsx, tests/dashboard.test.tsx) and they pull apart: the branch decides in the row from the request and the draft themselves, which the lists answer; the route answers one sentence per entry and a Decide link. Reading both lists again beside /waiting would make the count and the rows come from different answers and give the page three reads where it has one (ADR-112), and it would be done twice when the route carries the record. So the route carries it: a request entry carries the request as GET /requests answers it, a draft entry the draft as GET /drafts answers it, and the dashboard decides each row from the entry it drew, nothing read twice and nothing decided on another page.

## Task

Make GET /waiting's request and agent_draft entries carry the request and the draft their own lists answer, then merge lane/surface-06 (f5f8539e) into main as one merge commit, resolving the four files both sides changed so the dashboard reads what waits through GET /waiting alone and decides each row from the entry it drew, with Tom's reading of 18 October kept exactly as the branch pinned it. Out of scope: which of A, B and C the dashboard becomes (Tom decides on the page), who may decide anything, and what GET /requests and GET /drafts answer.

## Requirements

### R1: GET /waiting carries what deciding needs

Behavioural. A request entry of GET /waiting (crates/lys-identity-server/src/waiting_api.rs, requests()) carries one more member, request: the request exactly as GET /requests answers it, the RequestView of requests_views.rs that requests_api::listed already returned to build the entry. An agent_draft entry carries draft: the draft exactly as GET /drafts answers it, the view drafts_list::listed already returned. A product_draft or review entry carries neither member: those are decided on their own pages. Nothing else about the route changes: the order, the refused parts in place, the signed-in caller only. The schema the route publishes through types() carries the two optional members, and surface/identity/src/generated/index.ts carries them the way the repository keeps that file (read its head and its last commits before touching it, and change it the same way). In surface/identity/src/features/dashboard/waiting.ts, WaitingEntry gains request?: AccessRequest and draft?: Draft, typed from the requests and drafts contracts, and opens() answers the word Open for every kind, never Decide.

**Acceptance:**
- Red first: a test in crates/lys-identity-server/src/waiting_api_tests.rs reads GET /waiting and GET /requests for the same caller and asserts the request entry's request member equals, member for member, that request in the list's answer; the same for an agent_draft entry's draft member against GET /drafts; and asserts a product_draft entry and a review entry carry neither member
- The openapi contract test that reads the published schema is green with the two optional members, and surface/identity/src/generated/index.ts carries them
- gate -stages nextest -test 'package(lys-identity-server)' green, the counts in the report

**Files:**
- modify: crates/lys-identity-server/src/waiting_api.rs
- modify: crates/lys-identity-server/src/waiting_api_tests.rs
- modify: surface/identity/src/generated/index.ts
- modify: surface/identity/src/features/dashboard/waiting.ts

**Checklist:**
- C770 — GET /waiting's request entries carry the request as GET /requests answers it and its agent_draft entries carry the draft as GET /drafts answers it, so one read serves the heading's number and the row's decision; product_draft and review entries carry neither.

**Stories:**
- S441 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I approve or decline a request or a draft from the dashboard row it stands in, without opening another page, and the number in the heading is the number of rows I see.

### R2: The dashboard decides in its rows from the one waiting read: Tom's reading of 18 October, merged

Behavioural. Merge origin/lane/surface-06 (f5f8539e) into main as one merge commit whose first parent is main and whose second is the branch, taking every file the branch changed and main did not exactly as the branch has it (the files this requirement creates and the Spend, StopEverything, Drafts, Palette, help, rail and test files it modifies), and resolving the four files both sides changed so that: (a) the page reads what waits through GET /waiting alone, as main does at ed8b4706: Ready carries waiting: WaitingLine[] | Refused and the branch's budgets, never requests or drafts, and the dashboard makes no read of /requests or /drafts; (b) WaitingOnYou takes waiting, me and changed, and draws each line from the entry it is: a request entry's row is its about and the branch's DecisionForm built from entry.request (the acts Approve access and Decline request, describe as the branch's sourceWords); an agent_draft entry's row is its about and DraftDecide built from entry.draft; a product_draft or review entry's row keeps its Open link to its page; a refused part is its Unread line with Open; a request or draft entry that carries no record (an older server) says so in its row in words and offers Open to its page, never a Decide link and never nothing; the word Decide appears nowhere on the dashboard; (c) the number in the heading is waitingCount(lines), shown only when no part is refused, and the empty line stands at zero; (d) DashboardOptions' Waiting tile and dashboard-cherie.fixture.ts draw from the new Ready, the fixture's waiting being WaitingLine[] whose draft entry carries its draft; (e) Tom's reading stays exactly as the branch pinned it in surface/identity/tests/dashboard.test.tsx: the rail's first entry is Dashboard, the page has no header and the word Dashboard appears once, Approve and Decline stand in the row, the stop symbol stands alone, the person's weekly limit is under the agents' total. WaitingOnYou.tsx stays under 500 lines (ADR-111): if the row kinds crowd it, the rows move to a sibling module named for them.

**Acceptance:**
- Red first: surface/identity/src/features/dashboard/dashboard-waiting.test.tsx, in main's shape (lines as GET /waiting answers them), pins that a request line carrying request draws Approve access and Decline request in its row, a draft line carrying draft draws Approve and Decline, a product_draft line and a review line draw Open, a refused part is its own line, a request line carrying no record says so and offers Open, and no row on the page reads Decide
- surface/identity/tests/dashboard.test.tsx keeps every assertion the branch added (no .page-header, Dashboard as the rail's first entry and title, the acts by aria-label in the row, the person's budgets read, no /usage or agent or team budgets read) and asserts the page's reads name /waiting and never /requests or /drafts
- surface/identity/src/features/dashboard/dashboard-options.test.tsx is green against the new Ready and the fixture
- gate -stages tsc,vitest -dir surface/identity green: tsc 0 findings, every vitest file green, the counts in the report
- git log -1 --format=%P on the merge commit names main's head then f5f8539e, and git log --oneline main lists f5f8539e, b74ea2c2 and d810c461

**Files:**
- create: surface/identity/src/features/dashboard/DashboardOptions.tsx
- create: surface/identity/src/features/dashboard/dashboard-cherie.fixture.ts
- create: surface/identity/src/features/dashboard/dashboard-options.css
- create: surface/identity/src/features/dashboard/dashboard-options.test.tsx
- create: surface/identity/src/features/drafts/DraftDecide.tsx
- modify: surface/identity/src/features/dashboard/Dashboard.tsx
- modify: surface/identity/src/features/dashboard/WaitingOnYou.tsx
- modify: surface/identity/src/features/dashboard/dashboard-waiting.test.tsx
- modify: surface/identity/src/features/dashboard/Spend.tsx
- modify: surface/identity/src/features/dashboard/StopEverything.tsx
- modify: surface/identity/src/features/dashboard/dashboard.css
- modify: surface/identity/src/features/drafts/Drafts.tsx
- modify: surface/identity/src/shell/Palette.tsx
- modify: surface/identity/src/shell/help/pages.ts
- modify: surface/identity/src/shell/railItems.ts
- modify: surface/identity/tests/dashboard.test.tsx
- modify: surface/identity/tests/agent-run.test.tsx
- modify: surface/identity/tests/goto.test.tsx
- modify: surface/identity/tests/help.test.tsx
- modify: surface/identity/tests/shell-places.test.tsx
- modify: surface/identity/tests/sign-in-landing.test.tsx
- modify: surface/identity/tests/signed-out-walk.test.tsx
- modify: surface/identity/src/features/dashboard/waiting.ts
- modify: surface/identity/src/features/dashboard/dashboard-touch.test.tsx

**Checklist:**
- C771 — The dashboard decides in its rows from the one waiting read: Approve access and Decline request in a request's row, Approve and Decline in a draft's row, Open on an app action or a check due, a refused part on its own line, and the word Decide nowhere; the rail says Dashboard, the page has no heading of its own, the stop symbol stands alone, and the week's figure is said once with the person's own limit under it.

**Stories:**
- S442 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I open Lys and the rail says Dashboard, the page says it nowhere else, and the week's figure is said once with my own limit under it, so nothing on the front page repeats itself.

## Boundaries

- Who may decide a request or a draft is unchanged, and GET /requests and GET /drafts answer exactly what they answer today; the entry carries the record, it does not re-decide it
- No read is added to the dashboard beyond those at ed8b4706 and the branch's one read of the person's budgets; /requests and /drafts are never read by the dashboard
- The option strip and Cherie's fixture stay dev-build only, as the branch has them: which of A, B and C the dashboard becomes is Tom's decision on the page, not this brief's
- No timer, no fixed wait, no poll and no deadline anywhere, in code or test
- Nothing in lys-pass, nothing installed, the installed Lys untouched
- The merge is one merge commit with both parents, never a squash and never a rebase of the branch; the branch's three commits stay in main's history
- Every file touched stays under 500 lines (ADR-111)

## Verification

- gate -stages nextest -test 'package(lys-identity-server)'
- gate -stages tsc,vitest -dir surface/identity
- gate -stages clippy, then the whole declared gate
- git log -1 --format=%P on the merge commit, and git log --oneline main | grep -c 'f5f8539e\|b74ea2c2\|d810c461' answering 3
- python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory read for FAIL lines
