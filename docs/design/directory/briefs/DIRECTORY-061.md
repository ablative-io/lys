---
type: brief
id: DIRECTORY-061
cluster: directory
title: A SpiceDB call ends on its answer or when its caller leaves, never on a clock
---

# DIRECTORY-061: A SpiceDB call ends on its answer or when its caller leaves, never on a clock

> **Cluster:** directory
> **Depends on:** DIRECTORY-048, DIRECTORY-050
> **Design anchor:**
> - ADR-127 — A SpiceDB wait ends on its answer, its close or its caller, and grants sections run off the async workers — Keep RelationshipStore synchronous. Run each grants section under spawn_blocking inside a cancel scope owned by the handler's future. post_json registers its live stream with the scope, and dropping the handler's future cancels the scope, which shuts the stream down so the blocking read returns. A section that gets the lock after its request left returns before calling SpiceDB. WAIT and the three socket timeouts go with no clock in their place. Making the SpiceDB store and the grant authority async with a tokio Mutex was weighed and not taken, because it changes the lys-identity core trait and every grant act for the same result.
> **Checklist:**
> - C422 — No SpiceDB connect, write or read in Lys waits on a clock, and a cancelled call returns at once (DIRECTORY-061 R1).
> - C423 — Every grants section runs off the async workers, and a request that leaves ends its SpiceDB wait and lets the grants lock go (DIRECTORY-061 R2).
> - C424 — A section that takes the grants lock after its request left calls no SpiceDB (DIRECTORY-061 R2).
> **Stories:**
> - S169 (Person waiting on a permission check, Uses a Lys screen or route that checks a grant) — As a person whose request is waiting on the permission service, I want my request to end when I leave it, and nobody else's request held behind mine.

## Purpose

The one production clock left in the Lys crates is WAIT in crates/lys-identity-server/src/spicedb_http.rs (line 9), a five-second bound on each SpiceDB connect, write and read (lines 29 to 31). Tom's rule is that no wait in Lys ends on a clock. Taking WAIT away alone would be worse than keeping it, because post_json is a blocking std TcpStream call made by SpiceDb::call (crates/lys-identity-server/src/spicedb.rs line 185) inside with_grants (crates/lys-identity-server/src/grants.rs lines 130 to 154), which holds the std Mutex over GrantState (routes.rs, AppState.grants, locked at grants.rs line 141) on a tokio worker. A connected SpiceDB that never answers would then hold that lock and that worker for as long as the process lives. ADR-127 rules how the wait ends instead. It ends on SpiceDB's answer, on SpiceDB closing the connection, or on the request that asked for it going away.

## Task

Take WAIT and the three timeouts out of post_json. Make every SpiceDB exchange wait in poll(2) on its socket and on its request's wake pipe, with no timeout, so a request that leaves ends the connect, the write or the read. Run every grants section off the async workers inside that request's cancel scope, and check the scope as soon as the grants lock is taken.

## Requirements

### R1: post_json waits on SpiceDB or on its request, never on a clock

Behavioural. A new module crates/lys-identity-server/src/spicedb_cancel.rs holds Cancel, a flag with a wake pipe, and a scope that sets the current thread's Cancel for the length of a closure. Cancel::cancel sets the flag and writes one byte to the pipe. The SpiceDB endpoint is a socket address literal, and a configuration naming a host name is refused at load with the reason 'the SpiceDB endpoint must be an address, so no name lookup is waited on', so post_json does no name lookup. post_json in spicedb_http.rs opens a non-blocking socket and makes every wait, the connect, each write and each read, in poll(2) over the socket and the current scope's wake pipe with no timeout. The pipe becoming readable ends the call with the error text 'the request that asked left before SpiceDB answered', and the socket is closed. A call made after its scope is cancelled returns that error without opening a socket. SpiceDb::call maps that error to the GrantError it maps an unreachable SpiceDB to today, so each section takes the path it takes today when a SpiceDB call fails. With no scope on the thread, as at start (SpiceDb::open, spicedb.rs line 137), post_json polls the socket alone, and a SpiceDB that neither answers nor closes shows as the start still at its named step. The poll and pipe come from the workspace's nix.

**Acceptance:**
- A test holds a listener that accepts and never answers, calls post_json to it inside a scope, and cancels the scope from another thread, and post_json returns the error naming that the request left.
- In that test the listener's accepted socket reads end of file after the cancel.
- A test fills a listener's accept backlog so a further connect stays pending, calls post_json to it inside a scope, and cancels the scope, and post_json returns the error naming that the request left.
- A test that cancels the scope before calling post_json sees it return that error, and the listener has accepted no connection.
- A test calls post_json with no scope to a listener that answers 200 with a JSON body, and gets that status and body.
- A post_json to a loopback port that refuses returns an error that names the address.
- A configuration whose SpiceDB endpoint is localhost:58443 is refused at load with the reason naming that the endpoint must be an address.
- grep -nE 'Duration|timeout' crates/lys-identity-server/src/spicedb_http.rs prints nothing.

**Files:**
- create: crates/lys-identity-server/src/spicedb_cancel.rs
- create: crates/lys-identity-server/src/spicedb_cancel_tests.rs
- modify: crates/lys-identity-server/src/spicedb_http.rs
- modify: crates/lys-identity-server/src/spicedb.rs
- modify: crates/lys-identity-server/src/config.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/Cargo.toml

**Checklist:**
- C422 — No SpiceDB connect, write or read in Lys waits on a clock, and a cancelled call returns at once (DIRECTORY-061 R1).

**Stories:**
- S169 (Person waiting on a permission check, Uses a Lys screen or route that checks a grant) — As a person whose request is waiting on the permission service, I want my request to end when I leave it, and nobody else's request held behind mine.

### R2: Every grants section runs off the async workers and ends when its request leaves

Behavioural. A new async function judged in spicedb_cancel.rs takes the Arc of AppState and a Send closure over Judged, makes a Cancel, and runs with_grants inside that Cancel's scope under tokio::task::spawn_blocking. It holds a guard in its own future whose drop calls Cancel::cancel. with_grants reads the flag immediately after it takes the grants lock, before the first-use open of GrantState (grant_setup.open, grants.rs line 145) and before act, and returns at once when it is set, so a request that left calls no SpiceDB. Every async handler in crates/lys-identity-server/src that calls with_grants on the tree this card builds on calls judged instead, capturing owned values rather than borrowed ones. A request waiting on the lock holds a blocking thread until the lock is free and then returns at once if its request has left. The order of each section's own acts is unchanged. The proof that a client closing its connection drops the handler's future is taken through the served transport, a real TCP client against the server as main.rs serves it with axum::serve. If that transport keeps the handler running after the client closes, this card adds what ends it on the connection's close, in crates/lys-identity-server/src/main.rs and a new crates/lys-identity-server/src/serve_close.rs, and the same row proves it.

**Acceptance:**
- git grep -n 'with_grants(' crates/lys-identity-server/src prints only its definition in grants.rs and its one call in spicedb_cancel.rs.
- A test starts the service as main.rs serves it, with its SpiceDB endpoint at a listener that accepts and never answers, sends a request to a route that checks a grant over a real TCP connection, and closes that connection, and the listener's accepted socket reads end of file.
- In that test a second request to the same route afterwards reaches the listener as a new connection, which shows the grants lock was let go.
- A test queues a request behind a held grants lock while the grant slot is still empty, closes that request's connection, and lets the lock go, and the listener accepts no connection for it.
- grep -rnE 'Duration::|_timeout|timeout\(' crates/lys-identity-server/src/spicedb_cancel.rs prints nothing.

**Files:**
- create: crates/lys-identity-server/tests/spicedb_cancel.rs
- modify: crates/lys-identity-server/src/spicedb_cancel.rs
- modify: crates/lys-identity-server/src/grants.rs
- modify: crates/lys-identity-server/src/apps_schema_api.rs
- modify: crates/lys-identity-server/src/certificates_issue.rs
- modify: crates/lys-identity-server/src/grants_batch.rs
- modify: crates/lys-identity-server/src/grants_connector.rs
- modify: crates/lys-identity-server/src/requests_api.rs
- modify: crates/lys-identity-server/src/requests_decide.rs
- modify: crates/lys-identity-server/src/resources_api.rs
- modify: crates/lys-identity-server/src/reviews_api.rs
- modify: crates/lys-identity-server/src/runner_sessions.rs
- modify: crates/lys-identity-server/src/main.rs
- modify: crates/lys-identity-server/src/serve_close.rs

**Checklist:**
- C423 — Every grants section runs off the async workers, and a request that leaves ends its SpiceDB wait and lets the grants lock go (DIRECTORY-061 R2).
- C424 — A section that takes the grants lock after its request left calls no SpiceDB (DIRECTORY-061 R2).

**Stories:**
- S169 (Person waiting on a permission check, Uses a Lys screen or route that checks a grant) — As a person whose request is waiting on the permission service, I want my request to end when I leave it, and nobody else's request held behind mine.

## Boundaries

- SHALL NOT add a timeout, deadline, sleep, poll interval or watchdog anywhere. Every wait ends on an answer, a close or its request leaving, and poll(2) is always called with no timeout.
- SHALL NOT change the RelationshipStore trait in crates/lys-identity or make it async.
- SHALL NOT add a code line to crates/lys-identity-server/src/grants.rs beyond the flag read in with_grants. Anything more goes in spicedb_cancel.rs.
- SHALL NOT add unsafe, #[allow], #[ignore], let _ = or a renamed _name binding.
- SHALL NOT change the order of the acts inside any grants section.

## Verification

- The full Lys gate and ast-grep scan exit 0 at the card's head, measured by the card round.
- sh scripts/file-length.sh exits 0 at the card's head.
- `cargo test -p lys-identity-server --test spicedb_cancel` exits 0 on Dean's laptop at the card's head.
- `cargo test -p lys-identity-server --lib spicedb_cancel` exits 0 on Dean's laptop at the card's head.
