---
type: brief
id: DIRECTORY-046
cluster: directory
title: Identity server reads without repeated work: reviews, requests, issuance key, served screens, record reads and the screens' calls
---

# DIRECTORY-046: Identity server reads without repeated work: reviews, requests, issuance key, served screens, record reads and the screens' calls

> **Cluster:** directory
> **Blocked by:** DIRECTORY-043 landed on main (identity server hot paths; it edits lys-identity/src/directory.rs and the directory write path): the build starts only when `git log --oneline origin/main --grep=DIRECTORY-043` names its last requirement's commit.
> **Design anchor:**
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C348 — GET /reviews filters by viewer first and finds each latest decision by index (DIRECTORY-046 R1).
> - C349 — A request is found by id, not by scanning every request (DIRECTORY-046 R2).
> - C350 — Certificate issue uses the service key the server already holds (DIRECTORY-046 R3).
> - C351 — Served screens never block an async worker and index.html is read once (DIRECTORY-046 R4).
> - C352 — A record is serialised without cloning it (DIRECTORY-046 R5).
> - C353 — The grants screens ask for reach in one concurrent batch (DIRECTORY-046 R6).
> - C354 — The screens choose the right route first and fetch independent reads together (DIRECTORY-046 R7).
> **Stories:**
> - S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

## Purpose

Six per-request paths in the identity server and two in its screens do work that is thrown away or repeated: whole-lineage walks before a cheap filter, linear scans over every request ever asked, a key file re-read per certificate, blocking file reads inside async handlers, a whole-record clone to serialise, and screens making serial or refused calls.

## Task

Remove the repeated work named in each requirement, keeping every answer identical, each saving proved by a counting test.

## Requirements

### R1: GET /reviews filters by viewer first and finds each latest decision by index

Behavioural. crates/lys-identity-server/src/reviews_api.rs (about line 207) runs stands() on every grant before the viewer filter and finds each grant's latest decision by a reverse linear scan (last_for). Apply the viewer filter first, then stands() only on what remains; keep the latest decision per grant in a map built once per read (or maintained on write).

**Acceptance:**
- A counting test with G grants of which V are the viewer's shows V stands() walks, not G, and one pass over decisions; the answer equals today's.

**Files:**
- modify: crates/lys-identity-server/src/reviews_api.rs

**Checklist:**
- C348 — GET /reviews filters by viewer first and finds each latest decision by index (DIRECTORY-046 R1).

**Stories:**
- S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

### R2: A request is found by id, not by scanning every request

Behavioural. crates/lys-identity-server/src/requests_store.rs request(id) (about line 305) scans every request ever asked, and ask, intend and decide call it on every write. Keep an id index beside the list, maintained on the same writes, loaded once on open.

**Acceptance:**
- A counting test with N stored requests shows request(id) touches one entry; ask, intend and decide answers are unchanged.

**Files:**
- modify: crates/lys-identity-server/src/requests_store.rs

**Checklist:**
- C349 — A request is found by id, not by scanning every request (DIRECTORY-046 R2).

**Stories:**
- S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

### R3: Certificate issue uses the service key the server already holds

Behavioural. crates/lys-identity-server/src/certificates_issue.rs (about line 162) calls load_service_key on every issue, while routes.rs already holds the same key. Pass the held key into issuance; read the file never per request.

**Acceptance:**
- A test issues several certificates and shows no key-file read after start; certificates are byte-identical.

**Files:**
- modify: crates/lys-identity-server/src/certificates_issue.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C350 — Certificate issue uses the service key the server already holds (DIRECTORY-046 R3).

**Stories:**
- S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

### R4: Served screens never block an async worker and index.html is read once

Behavioural. crates/lys-identity-server/src/surface.rs (about line 50) does std::fs::read inside the async handler for every page and asset and re-reads index.html per navigation. Read the verified screens package once at start (it is verified and immutable while the server runs), serve from memory, and keep any remaining file I/O off the async workers.

**Acceptance:**
- A test serves index.html and an asset many times and counts one read of each; bytes and headers are unchanged; grep shows no std::fs call in an async fn in surface.rs.

**Files:**
- modify: crates/lys-identity-server/src/surface.rs

**Checklist:**
- C351 — Served screens never block an async worker and index.html is read once (DIRECTORY-046 R4).

**Stories:**
- S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

### R5: A record is serialised without cloning it

Behavioural. crates/lys-identity/src/directory.rs Directory::record (about line 136) clones the whole Record, event history included, so GET /identities/{id} can serialise it. Serialise from a borrow (a read guard or a closure given &Record), never a copy.

**Acceptance:**
- A test shows the route's JSON is byte-identical and the Record type is not cloned on the path (a counting Clone seam or a borrow-only signature).

**Files:**
- modify: crates/lys-identity/src/directory.rs

**Checklist:**
- C352 — A record is serialised without cloning it (DIRECTORY-046 R5).

**Stories:**
- S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

### R6: The grants screens ask for reach in one concurrent batch

Behavioural. surface/identity/src/features/grants/check.ts reachMap (about line 75) awaits resources x actions x pages /grants/who calls one after another before the Access, Graph and people reach views render. Issue them concurrently with a bounded pool, or through one batched read if the server offers it by then, with the same result map.

**Acceptance:**
- A vitest with a counting fetch shows the calls are in flight together (peak concurrency above one) and the rendered reach is unchanged.

**Files:**
- modify: surface/identity/src/features/grants/check.ts

**Checklist:**
- C353 — The grants screens ask for reach in one concurrent batch (DIRECTORY-046 R6).

**Stories:**
- S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

### R7: The screens choose the right route first and fetch independent reads together

Behavioural. surface/identity/src/api.ts widest() (about line 63) sends every non-admin people or agent read to the admin route first and takes a refusal; several screens fetch /me and then /people one after the other. Choose the route from the signed-in person's role already known from /me, and fetch independent reads together.

**Acceptance:**
- A vitest with a counting fetch shows no refused admin call for a non-admin and /me with /people in flight together; screens render the same.

**Files:**
- modify: surface/identity/src/api.ts

**Checklist:**
- C354 — The screens choose the right route first and fetch independent reads together (DIRECTORY-046 R7).

**Stories:**
- S148 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As someone using the identity screens and API all day, I want every read to do its work once, so that the service stays light while it runs beside everything else.

## Boundaries

- SHALL NOT change any answer, stored byte, hash, signature or wire shape: every change is the same result with less work, and a test pins the result before and after.
- SHALL NOT add a timeout, deadline, sleep, poll interval, cache expiry by clock, #[allow], #[ignore] or any bypass.
- Every file stays under 500 lines of code and ast-grep stays at zero hits.
- A counting test proves each saving (calls, reads, clones, scans or round trips counted), never a wall-clock timing.
- Where a line number has moved, the function named is the target; the dev record names where it now is.

## Verification

- The full Lys gate and, where the screens change, the surface checks exit 0 at the card's head, measured by the card round.
- Each requirement's counting test is red on the base and green at the head.
