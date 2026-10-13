---
type: brief
id: ACCESS-007
cluster: access
title: A permission decision asks SpiceDB one question over a kept connection, outside the grants lock, at a consistency token
---

# ACCESS-007: A permission decision asks SpiceDB one question over a kept connection, outside the grants lock, at a consistency token

> **Cluster:** access
> **Depends on:** ACCESS-006, DIRECTORY-089
> **Blocked by:** Lands on main after the ca2da3f9 upgrade lane's END (the installer refuses a dirty tree); no implementation starts on the Lys main checkout while a battery runs there., Tom's finding of 13 Oct 2026 21:5x is the source: every decision builds a fresh frame (lys-identity/src/grants/authority.rs check -> frame.rs Frame::read -> permission.rs RelationshipStore::read), which reads every relationship of every kind from SpiceDB (lys-identity-server/src/spicedb.rs read at fullyConsistent) over a new blocking TCP connection per call with Connection: close (spicedb_http.rs post_json), while with_directory_grants (lys-identity-server/src/grants.rs) holds the directory and grants together. Measured 13 Oct 21:46 AEDT: POST /api/grants/check/batch with two checks answered in 0.90 to 1.31 s at grant revision 282; the same route without a credential answers 401 in 0.08 to 0.17 s, so the cost is the decision, not the wire. Measured 13 Oct 22:05 AEDT: twelve concurrent batch checks of one question each, sent together at lys.ablative.com.au, answered one after another from 0.47 s to 5.22 s, about 0.45 s apart: the decisions queue behind the lock.

## Purpose

Tom, 13 Oct 2026 21:5x: a permission check reads the whole permission set from SpiceDB, over a blocking connection, while holding a lock; the usual fix is to ask SpiceDB the one question you need, with its own check call and a consistency token, so the whole set is not downloaded on every decision. Every door route, every pass mint and every pass refresh in the estate waits on this decision path, so its one-second cost is paid on every click, every live-wire pass and every agent renewal, and grows with every grant the estate adds.

## Task

Authoring START 2026-10-13 22:01 AEDT; source base ca2da3f93f45532a3749ff6d62bddb342041d7ee / tree 289c2f3da51c22a53ec01f5fa59d6c5ecd0623a5. Waffles, from Tom's finding. Decisions taken here by Tom's rules, with their reasons: (1) Lys's own grant revision stays the public consistency word (passes, bindings and /api/grants answers keep carrying it); SpiceDB's ZedToken is kept privately beside each write and used only to ask SpiceDB at_least_as_fresh, because products already speak the revision and the lys_mirror marker already asserts a revision is present. (2) The answer keeps carrying grant, path, via and mode, so the per-question read fetches the relationships about the subject and the resource's ancestry rather than SpiceDB's bare boolean check; a bare check would lose the explanation every product shows. (3) No cached frame across revisions: a decision reads at its settled revision; a cache would be a second copy of the grants to keep right.

## Requirements

### R1: One question per decision, not the whole relationship set

THE SYSTEM SHALL answer a permission decision (Authority::check and the exercise path in crates/lys-identity/src/grants/authority.rs, and the batch route in crates/lys-identity-server/src/grants_batch.rs) by reading only the relationships about the asked subject and the asked resource's ancestry at the settled revision, through a new RelationshipStore::read_about(subject, resource) beside the existing read(), and SHALL keep the answer's grant, path, via and mode exactly as today. The whole-set read() SHALL remain only for explain-all and audit listings and SHALL be named there as the whole-set read.

**Acceptance:**
- Red: a scratch SpiceDB scope holding the relationships of 1,000 unrelated subjects and the asked subject's five grants answers the asked decision reading only the asked subject's and the resource ancestry's relationships (the fake RelationshipStore counts rows handed to the frame); today it hands every row.
- Red: a decision's wall time at 10x the relationships of other subjects stays within the noise band of 1x (measured ten runs each, median compared); today it grows with the set.
- Red: every existing grant_explanations, grants_reach, grants_refusals and channel_membership test passes unchanged: the same grant, path, via and mode on the same questions.
- Red: a batch of N checks about one subject and one resource performs one read_about, not N; N checks about N resources perform at most N.

**Files:**
- create: crates/lys-identity/src/grants/frame_about_tests.rs
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/src/grants/frame.rs
- modify: crates/lys-identity/src/grants/permission.rs
- modify: crates/lys-identity/src/grants/mod.rs
- modify: crates/lys-identity-server/src/grants_batch.rs
- modify: crates/lys-identity-server/src/spicedb.rs

### R2: A kept connection to SpiceDB

THE SYSTEM SHALL talk to SpiceDB over one kept client per engine (HTTP/1.1 keep-alive on the configured http_port, with connect, read and write timeouts, reconnecting on a closed or failed connection), replacing the TcpStream opened per call with Connection: close in crates/lys-identity-server/src/spicedb_http.rs. The error SHALL still name the authority in words and SHALL never hold the bearer.

**Acceptance:**
- Red: two consecutive calls to a fake SpiceDB answer over one accepted TCP connection (the fake counts accepts); today each call is a new accept.
- Red: a fake that closes the connection after answering is followed by a successful call on a fresh connection, with no error surfaced to the decision.
- Red: an authority that does not answer within the wait is refused in words naming the authority, and the bearer appears in no error string (the existing spicedb_key_tests shape).
- Red: a fake that answers what is not HTTP is refused by name as today.

**Files:**
- create: crates/lys-identity-server/src/spicedb_http_kept_tests.rs
- modify: crates/lys-identity-server/src/spicedb_http.rs
- modify: crates/lys-identity-server/src/spicedb.rs

### R3: No SpiceDB round trip under the grants lock

THE SYSTEM SHALL read from SpiceDB outside the lock that with_directory_grants and with_grants (crates/lys-identity-server/src/grants.rs) hold over the directory and the grants: settle the revision, release, read about the subject at that revision, then take the lock only to judge and to bind. A revision that moved between the read and the judgement SHALL be re-read once and otherwise refused by the existing BindingRevisionMoved name, never answered from the older read.

**Acceptance:**
- Red: with a fake SpiceDB that delays one answer by two seconds, a second unrelated decision and a pass refresh (provider by_refresh -> rights_claim::pass) each complete in under the delay, measured concurrently; today both wait behind the lock.
- Red: a grant written between the read and the judgement makes the decision re-read once and answer at the new revision; a second move refuses BindingRevisionMoved as today.
- Red: the existing projection_live and change_stream tests pass unchanged.

**Files:**
- create: crates/lys-identity-server/src/grants_lock_tests.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity/src/grants/authority.rs

### R4: Consistency by token, not fullyConsistent on every read

THE SYSTEM SHALL keep the ZedToken SpiceDB returns (writtenAt) beside each relationship write in crates/lys-identity-server/src/spicedb.rs, paired with the Lys grant revision the write carries, and SHALL ask every read_about and check with consistency at_least_as_fresh that token for the settled revision instead of fullyConsistent (today at spicedb.rs lines 204 and 252). The lys_mirror revision marker SHALL remain the assertion that SpiceDB holds the settled revision; a read whose answer lacks the marker SHALL be refused by name as today. Lys's grant revision SHALL remain the only consistency word products see.

**Acceptance:**
- Red: a write records the token it was answered with; the next read about that revision sends at_least_as_fresh with that token (the fake asserts the request body); today the body says fullyConsistent.
- Red: a read at a revision whose token is not kept (after a restart) falls back to fullyConsistent once, keeps the token the answer carries, and the following read uses it.
- Red: the marker refusal is unchanged: a scope missing the settled revision's marker refuses by its existing name.

**Files:**
- modify: crates/lys-identity-server/src/spicedb.rs
- modify: crates/lys-identity-server/src/spicedb_calls_tests.rs

### R5: The cost is measured on the installed server and in the gate

THE SYSTEM SHALL carry a timing test that fails when a single decision's time grows with the number of other subjects' relationships (R1's noise-band red), and the install readback SHALL measure POST /api/grants/check/batch with the door's own two questions at lys.ablative.com.au and at 127.0.0.1:8490, before and after, and state both times.

**Acceptance:**
- Red: the timing test is in the nextest run of crates/lys-identity (not ignored, not serial) and fails on the source base.
- Readback: the two batch times before and after the install are stated beside the 13 Oct figures (0.90 to 1.31 s public, revision 282).

**Files:**
- modify: crates/lys-identity/src/grants/frame_about_tests.rs

## Boundaries

- SHALL NOT keep a cached frame across revisions, a product-local permission cache or any second copy of the grants; a decision reads at its settled revision every time.
- SHALL NOT answer a decision from a read older than the settled revision asked (at_least), and SHALL NOT weaken the lys_mirror marker assertion.
- SHALL NOT change the shape of any /api/grants answer, pass, binding or refusal name; grant, path, via and mode are kept.
- SHALL NOT add a configuration key the program does not write into the file itself with a backup and a log line (Tom, 12 Oct 2026).
- SHALL NOT move to gRPC in this brief; the kept HTTP/1.1 client is the step; gRPC on grpc_port is a later brief if the measure asks for it.
- SHALL NOT be built or tested until every line is written (the 3 Oct rule); one battery, one install.

## Verification

- sh scripts/design/gate.sh at the committed source; parse the validate, coverage and rendered-comparison lines, never the exit code alone.
- cargo nextest run -p lys-identity -p lys-identity-server at the head, counts stated per crate; cargo test --doc; clippy pedantic -D warnings on the touched crates; cargo fmt; file-length gate 500 lines (ADR-111) on every touched file.
- The R1 noise-band timing red fails on the source base and passes at the head, ten runs each, medians stated.
- The install readback states check/batch times before and after at both addresses (R5).
- If this code were used in a hospital, the worst credible failure is a permission answered from an older revision than asked, letting a revoked person read a patient's room until the next decision; it could harm everyone in that workspace, so the marker assertion and the at_least refusal are kept and tested.
