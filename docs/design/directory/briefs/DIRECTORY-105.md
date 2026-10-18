---
type: brief
id: DIRECTORY-105
cluster: directory
title: The canvas widgets decide in their rows too: Decide nowhere on the surface
---

# DIRECTORY-105: The canvas widgets decide in their rows too: Decide nowhere on the surface

> **Cluster:** directory
> **Depends on:** DIRECTORY-104
> **Design anchor:**
> - ADR-109 — The server judges a grant's standing and effective end over its chain; the screens render them and walk nothing — GrantView carries each grant's effective standing ({stands: true}, or {stands: false, refusal, grant, reason} read through the service's sight rules, so a withheld refusal names no grant or identity the caller may not see) and its effective end, both worked out on the server by the same judgement a check makes, and every screen renders those two members and walks no chain, compares no window with the clock and reads no holder's lifecycle state to decide standing. Rejected: keeping the client chain walk beside the server's judgement, and a screen-side derivation of the effective end.
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C772 — The canvas's requests and drafts widgets decide in their rows with the dashboard's own forms (Approve access and Decline request on a request the person may decide, the approvers' names on one they may not, Approve and Decline on a draft), and no label on the surface reads Decide.
> **Stories:**
> - S443 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I approve or decline a request or a draft in the canvas window I placed for them, the same way I do on the dashboard, without being sent to another page.

## Purpose

Tom's reading of 18 October 2026 took the Decide button off the dashboard: a request or a draft is approved or declined in the row it stands in, never on another page. DIRECTORY-104 landed that on the dashboard's own list, and found on its way out that the canvas's widgets, which draw the same requests and drafts in a window a person places, still end every row in a Decide link to another page (Widgets.tsx lines 50 and 65). The same thing on two screens is decided two ways. This brief makes the widgets decide where they stand with the same forms the dashboard uses, so the word Decide is gone from the surface.

## Task

Give requestsPart and draftsPart the person and a changed callback, draw DecisionForm and DraftDecide in their rows exactly as WaitingOnYou's Row does, move sourceWords to waiting.ts so both import it, pass the canvas's person and re-read through CanvasWidgets.tsx, pin it red first in canvas-parts.test.tsx, and prove the word Decide appears as no label anywhere under surface/identity/src.

## Requirements

### R1: The requests and drafts widgets decide in their rows, the way the dashboard's rows do

Behavioural. In surface/identity/src/features/dashboard/Widgets.tsx, requestsPart takes requests, me and changed and draws each waiting request's row as the dashboard's row draws a request entry (WaitingOnYou.tsx, Row): the asker, what for and why, then, where the person may decide it (can_decide, else the approvers naming them), DecisionForm from features/requests/DecisionForm.tsx built from the request itself (person = me, canIssueRoot = request.can_issue_root === true, changed, describe = sourceWords), so the acts Approve access and Decline request stand in the row; where the person may not decide it, the approvers' names as today. draftsPart takes drafts, me and changed and draws each draft's row with DraftDecide from features/drafts/DraftDecide.tsx (draft, person = me, changed), so Approve and Decline stand in the row. sourceWords moves out of WaitingOnYou.tsx into surface/identity/src/features/dashboard/waiting.ts (exported) and both WaitingOnYou.tsx and Widgets.tsx import it, so the dashboard and the canvas say a source grant the same way. surface/identity/src/features/runtime/CanvasWidgets.tsx passes board.me and a changed that re-reads the board the canvas holds (the read in surface/identity/src/features/dashboard/board.ts), the way the canvas already re-reads it; where the canvas has no such re-read, one re-read of that board on changed is added and the report names it. No link or button on the dashboard or the canvas reads Decide: the dashboard's requests page is still reached from the widget's own heading link (#/requests, #/access/drafts), never from a row.

**Acceptance:**
- Red first, in surface/identity/src/features/dashboard/canvas-parts.test.tsx: a waiting request the person may decide draws, in its row, the acts by aria-label Approve access and Decline request and no link whose text is Decide; a waiting request the person may not decide draws the approvers' names and no act; a draft's row draws Approve and Decline by aria-label and no link whose text is Decide; the counts in the headings are unchanged
- surface/identity/tests/canvas-widgets.test.tsx stays green against the new props, its assertions kept and extended only where a widget's row changed
- grep -rn '>Decide<' surface/identity/src answers nothing, and the report says so with the command
- gate -stages tsc,vitest -dir surface/identity: tsc 0 findings, every vitest file green, the counts in the report

**Files:**
- modify: surface/identity/src/features/dashboard/Widgets.tsx
- modify: surface/identity/src/features/dashboard/WaitingOnYou.tsx
- modify: surface/identity/src/features/dashboard/waiting.ts
- modify: surface/identity/src/features/dashboard/canvas-parts.test.tsx
- modify: surface/identity/src/features/runtime/CanvasWidgets.tsx
- modify: surface/identity/tests/canvas-widgets.test.tsx

**Checklist:**
- C772 — The canvas's requests and drafts widgets decide in their rows with the dashboard's own forms (Approve access and Decline request on a request the person may decide, the approvers' names on one they may not, Approve and Decline on a draft), and no label on the surface reads Decide.

**Stories:**
- S443 (Responsible person, Relies on an agent's certificate staying ended once it is revoked) — As a responsible person, I approve or decline a request or a draft in the canvas window I placed for them, the same way I do on the dashboard, without being sent to another page.

## Boundaries

- Who may decide a request or a draft is unchanged; the widget draws the form the requests and drafts pages already use, it decides nothing itself
- No new read: the canvas reads the board it reads today; changed re-reads it, nothing polls
- No timer, no fixed wait, no poll and no deadline anywhere, in code or test
- The dashboard's own list (WaitingOnYou.tsx) changes only by importing sourceWords from waiting.ts; its tests are untouched and green
- Nothing in lys-pass, nothing installed, the installed Lys untouched
- Every file touched stays under 500 code lines (ADR-111, by scripts/file-length.sh)

## Verification

- gate -stages tsc,vitest -dir surface/identity
- grep -rn '>Decide<' surface/identity/src answering nothing
- python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory read for FAIL lines
