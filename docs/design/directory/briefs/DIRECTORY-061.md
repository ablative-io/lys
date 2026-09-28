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
> - ADR-127 — A SpiceDB wait ends on its answer, its close or its caller, and grants sections run off the async workers — Keep RelationshipStore synchronous. Run each grants section under spawn_blocking inside a cancel scope owned by the handler's future. Every SpiceDB exchange uses a non-blocking socket and waits in poll(2) on that socket and the scope's wake pipe, with no timeout, and dropping the handler's future cancels the scope, which wakes the poll so the call returns. A section that gets the grants lock after its request left returns before it opens GrantState or calls SpiceDB. The SpiceDB endpoint must be a socket address, so no name lookup is ever waited on. WAIT and the three socket timeouts go with no clock in their place. Making the SpiceDB store and the grant authority async with a tokio Mutex was weighed and not taken, because it changes the lys-identity core trait and every grant act for the same result.
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

Behavioural. A new module crates/lys-identity-server/src/spicedb_cancel.rs holds Cancel, a flag with a wake pipe, and a scope that sets the current thread's Cancel for the length of a closure. Cancel::cancel swaps the flag and, only on the first cancel, writes one byte to the pipe, whose write end is non-blocking, so a repeated or concurrent cancel and the guard's drop never block and never fail. The scope puts back the thread's earlier Cancel when it returns and when it unwinds. The SpiceDB endpoint is a socket address literal, and a configuration naming a host name is refused at load with the reason 'the SpiceDB endpoint must be an address, so no name lookup is waited on', so post_json does no name lookup. post_json in spicedb_http.rs opens a non-blocking socket and makes every wait, the connect, each write and each read, in poll(2) over the socket and the current scope's wake pipe with no timeout. The pipe becoming readable ends the call with the error text 'the request that asked left before SpiceDB answered', and the socket is closed. A call made after its scope is cancelled returns that error without opening a socket. SpiceDb::call maps that error to the GrantError it maps an unreachable SpiceDB to today, so each section takes the path it takes today when a SpiceDB call fails. With no scope on the thread, as at start (SpiceDb::open, spicedb.rs line 137), post_json polls the socket alone, and a SpiceDB that neither answers nor closes shows as the start still at its named step. The poll, the pipe and the non-blocking socket come from rustix, which the workspace already pins at 1.1. This card adds its event, pipe and net features in the root Cargo.toml and takes rustix.workspace into crates/lys-identity-server/Cargo.toml, so Cargo.lock gains no new crate.

**Acceptance:**
- A test holds a listener that accepts and never answers, calls post_json to it inside a scope, and cancels the scope from another thread, and post_json returns the error naming that the request left.
- In that test the listener's accepted socket reads end of file after the cancel.
- A test fills a listener's accept backlog, starts post_json to it inside a scope, and sees the socket's connect still in progress (poll of the socket not yet writable) before it cancels the scope, and post_json then returns the error naming that the request left.
- A test cancels one scope three times from two threads and drops its guard, and no call blocks or fails.
- A test runs post_json inside a nested scope and, after the inner scope returns, finds the outer scope's Cancel in place on the thread.
- A test that cancels the scope before calling post_json sees it return that error, and the listener has accepted no connection.
- A test calls post_json with no scope to a listener that answers 200 with a JSON body, and gets that status and body.
- A post_json to a loopback port that refuses returns an error that names the address.
- A configuration whose SpiceDB endpoint is localhost:58443 is refused at load with the reason naming that the endpoint must be an address.
- grep -nE 'Duration|timeout' crates/lys-identity-server/src/spicedb_http.rs prints nothing.
- cargo tree -p lys-identity-server shows rustix and no new crate beyond what Cargo.lock held at the card's base.

**Files:**
- create: crates/lys-identity-server/src/config_tests.rs
- create: crates/lys-identity-server/src/spicedb_cancel.rs
- create: crates/lys-identity-server/src/spicedb_cancel_tests.rs
- modify: Cargo.lock
- modify: Cargo.toml
- modify: crates/lys-identity-server/Cargo.toml
- modify: crates/lys-identity-server/src/config.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/spicedb.rs
- modify: crates/lys-identity-server/src/spicedb_http.rs

**Checklist:**
- C422 — No SpiceDB connect, write or read in Lys waits on a clock, and a cancelled call returns at once (DIRECTORY-061 R1).

**Stories:**
- S169 (Person waiting on a permission check, Uses a Lys screen or route that checks a grant) — As a person whose request is waiting on the permission service, I want my request to end when I leave it, and nobody else's request held behind mine.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Row 1 (never-answering listener, cancel from another thread, LEFT) is met by spicedb_cancel_tests.rs:114. post_json runs inside a scope on its own thread, the test reads the whole request, cancels from a second spawned thread, and asserts Err(LEFT); the mechanism is spicedb_http.rs:128-146 (wait returns Ended::Left when the pipe is readable) and :62. Row 2 (the accepted socket reads EOF) is met by the same test at the last assert, read == 0; the socket is closed because the TcpStream built at spicedb_http.rs:74 is dropped on return. Row 3 (full backlog, connect still in progress, then LEFT) is met with a deviation by spicedb_cancel_tests.rs:133. The listener is made with backlog 1 and filled with 16 non-blocking probes, then post_json starts in a scope. A probe started after it is shown not writable by a no-timeout poll beside an already-readable pipe (writable_now, :92), the call is shown unfinished (is_finished), and after the cancel it returns LEFT. Row 4 (three cancels from two threads plus the guard drop) is met by :160, which also asserts through FIONREAD that exactly one byte is in the pipe (spicedb_cancel.rs:70-77). Row 5 (nested scope) is met by :184, which asserts Arc::ptr_eq with the outer Cancel after the inner scope returns and again after an inner scope unwinds through resume_unwind, and that no Cancel is left after the outer scope (spicedb_cancel.rs:104-118). Row 6 (cancel before the call) is met by :207: LEFT, and a non-blocking accept gets WouldBlock (spicedb_http.rs:47-50 returns before any socket). Row 7 (no scope, 200 with JSON) is met by :223, which asserts Answer{200, BODY} and that the bearer was sent. Row 8 (refused port names the address) is met by :240, where the error starts with '<addr> could not be reached' (spicedb_http.rs:63). Row 9 (localhost:58443 refused at load) is met by config_tests.rs:44 through config.rs:163 and spicedb_address. Row 10 (grep -nE 'Duration|timeout' on spicedb_http.rs) prints nothing; I ran the grep and the doc comments say 'given no time bound'. Row 11 (cargo tree shows rustix and no new crate) is not observed, because cargo is forbidden to me here. The lock evidence is that Cargo.lock's only change is the line '+ "rustix"' in lys-identity-server's dependency list, with rustix 1.1.4 its single existing entry. The spec's rule that SpiceDb::call maps LEFT to the same GrantError as an unreachable SpiceDB already holds without a change: SpiceDb::call is at spicedb_scope.rs:232 and its raw (:208-211) maps every post_json error through unavailable to GrantError::PermissionEngineUnavailable. With no scope, as at start, wake is None and only the socket is polled (spicedb_http.rs:59, :130-133).
- Deviation: (1) Row 3 observes a probe socket, not post_json's own. post_json's socket is private to the call, so the test shows the connect is held by showing that a probe connected after post_json started is still not writable (poll with no timeout beside a ready pipe) and that post_json has not returned, then cancels. It cannot prove that post_json was inside its connect wait at the moment of the cancel rather than not yet started; either way the cancel ends it with LEFT. (2) spicedb.rs is in the file list but is not modified: SpiceDb::call and raw are in spicedb_scope.rs (not spicedb.rs line 185 as the brief says), and raw already maps every post_json error, LEFT included, to PermissionEngineUnavailable. (3) Row 11 was not run, because the brief forbids cargo commands; the Cargo.lock diff is given instead. (4) The refusal is checked in Config::load (through a new pub Config::spicedb_address), not in Config::validate. The shared test harness (tests/identity_contract/src/harness.rs:388, outside the wall) calls validate(), and tests/connections.rs:62 configures 'does-not-exist.invalid:8089', so a validate-time refusal would break a test outside the wall. The brief says 'refused at load', which this matches.
- Files changed:
  - created: `crates/lys-identity-server/src/spicedb_cancel.rs` — Cancel (AtomicBool flag + CLOEXEC pipe with a non-blocking write end; only the first cancel writes one byte, and a failed write is logged, never returned), Leaving (the guard that cancels on drop), scope (sets the thread's Cancel and puts back the earlier one on return and on unwind through a Drop guard), current, still_asked, LEFT, and the async judged used by R2.
  - created: `crates/lys-identity-server/src/spicedb_cancel_tests.rs` — Unit tests for cancel ending a never-answered call and closing its socket, an in-progress connect behind a full backlog, repeated and concurrent cancels plus the guard drop, nested scope restore on return and on unwind, a cancel before the call opening no socket, a call with no scope reading 200 and its JSON, a refused port naming the address, and still_asked.
  - modified: `crates/lys-identity-server/src/spicedb_http.rs` — post_json parses the endpoint as a SocketAddr (no name lookup) and returns LEFT without opening a socket if its scope is already cancelled. It opens a non-blocking CLOEXEC socket through rustix, and waits in poll(2) with no timeout on the socket and, when a scope is set, its wake pipe for the connect, each write and each read. A readable pipe ends the call with LEFT and the socket is closed on return. WAIT, Duration and the timeouts are gone.
  - modified: `crates/lys-identity-server/src/config.rs` — Config::load calls the new Config::spicedb_address, which refuses a non-address endpoint with 'the SpiceDB endpoint must be an address, so no name lookup is waited on'. It also declares config_tests.
  - created: `crates/lys-identity-server/src/config_tests.rs` — Checks that localhost:58443 is refused at load with the reason and the endpoint named, and that 127.0.0.1 and [::1] endpoints load (two cases counted).
  - modified: `crates/lys-identity-server/src/lib.rs` — Declares mod spicedb_cancel.
  - modified: `Cargo.toml` — Workspace rustix gains the event, net and pipe features beside fs and process.
  - modified: `crates/lys-identity-server/Cargo.toml` — Adds rustix.workspace = true.
  - modified: `Cargo.lock` — lys-identity-server's dependency list gains "rustix" (the existing 1.1.4). No new [[package]] entry.
- Checklist delivery:
  - [x] C422 — No SpiceDB connect, write or read in Lys waits on a clock, and a cancelled call returns at once (DIRECTORY-061 R1). — No connect, write or read in post_json waits on a clock (spicedb_http.rs:104-146), and a cancelled call returns LEFT at once, before any socket (spicedb_http.rs:47-50). The row 3 and row 11 caveats are under deviation.
- Story delivery:
  - [ ] S169 (Person waiting on a permission check, Uses a Lys screen or route that checks a grant) — As a person whose request is waiting on the permission service, I want my request to end when I leave it, and nobody else's request held behind mine. — R1's part holds: a request's SpiceDB wait ends when it leaves. But the story also needs R2, and R2 is blocked: the runner operator check (runner_sessions.rs:116) still runs its grants section on the async worker, outside any cancel scope.

### R2: Every grants section runs off the async workers and ends when its request leaves

Behavioural. A new async function judged in spicedb_cancel.rs takes the Arc of AppState and a Send closure over Judged, makes a Cancel, and runs with_grants inside that Cancel's scope under tokio::task::spawn_blocking. It holds a guard in its own future whose drop calls Cancel::cancel. with_grants reads the flag immediately after it takes the grants lock, before the first-use open of GrantState (grant_setup.open, grants.rs line 145) and before act, and returns at once when it is set, so a request that left calls no SpiceDB. Every async handler in crates/lys-identity-server/src that calls with_grants on the tree this card builds on calls judged instead, capturing owned values rather than borrowed ones. A request waiting on the lock holds a blocking thread until the lock is free and then returns at once if its request has left. The order of each section's own acts is unchanged. The proof that a client closing its connection drops the handler's future is taken through the served transport, a real TCP client against the server as main.rs serves it with axum::serve. If that transport keeps the handler running after the client closes, this card adds what ends it on the connection's close, in crates/lys-identity-server/src/main.rs and a new crates/lys-identity-server/src/serve_close.rs declared in lib.rs. If the served transport already drops the handler, serve_close.rs is not created, and the same row proves it.

**Acceptance:**
- git grep -n 'with_grants(' crates/lys-identity-server/src prints only its definition in grants.rs and its one call in spicedb_cancel.rs.
- A test starts the service as main.rs serves it, with its SpiceDB endpoint at a listener that accepts and never answers, sends a request to a route that checks a grant over a real TCP connection, and closes that connection, and the listener's accepted socket reads end of file.
- In that test a second request to the same route afterwards reaches the listener as a new connection, which shows the grants lock was let go.
- A test queues a request behind a held grants lock while the grant slot is still empty, closes that request's connection, and lets the lock go, and the listener accepts no connection for it.
- grep -rnE 'Duration::|_timeout|timeout\(' crates/lys-identity-server/src/spicedb_cancel.rs prints nothing.

**Files:**
- create: crates/lys-identity-server/tests/spicedb_cancel.rs
- create: crates/lys-identity-server/src/serve_close.rs
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
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C423 — Every grants section runs off the async workers, and a request that leaves ends its SpiceDB wait and lets the grants lock go (DIRECTORY-061 R2).
- C424 — A section that takes the grants lock after its request left calls no SpiceDB (DIRECTORY-061 R2).

**Stories:**
- S169 (Person waiting on a permission check, Uses a Lys screen or route that checks a grant) — As a person whose request is waiting on the permission service, I want my request to end when I leave it, and nobody else's request held behind mine.

#### R2 — Execution record

**Dev (recorded):**

- Status: blocked
- How: Row 1 (git grep -n 'with_grants(' shows only the definition and one call in spicedb_cancel.rs) is not met. crates/lys-identity-server/src/runner_sessions.rs:116 still calls with_grants inside runner_sessions::operator, a sync pub fn. Its callers are in crates/lys-identity-server/src/runner_api.rs: admitted at lines 184-191, the handlers at 202, 225, 247, 265, 286, 319 and 348, and wake at 389. runner_api.rs is outside this requirement's wall, and making operator async requires editing it. Under CN9 I stopped and name it here: a brief revision adding runner_api.rs (operator and admitted become async over Arc<AppState>, and nine call sites gain .await) finishes this row. Also, the grants.rs definition reads 'with_grants<T>(', so this grep never prints the definition, at the base or now. Every other with_grants call in the wall is replaced; spicedb_cancel.rs:147 holds the one call. Row 2 (a real TCP client closes and the never-answering listener's accepted socket reads EOF) is written as tests/spicedb_cancel.rs:156, closing_the_connection_ends_the_spicedb_wait_and_lets_the_grants_lock_go. It sends GET /grants over a raw TcpStream, accepts the section's SpiceDB call and reads its whole request, drops the client, and asserts read == 0. The drop reaches the handler because hyper 1.11.1 errors a busy connection on EOF (proto/h1/conn.rs:487-503, mid_message_detect_eof) and axum 0.8.9 never sets half_close, so the handler future and its Leaving guard are dropped. For that reason serve_close.rs is not created and main.rs is unchanged, as the spec allows. Row 3 (a second request reaches the listener as a new connection) is met in the same test: a second GET /grants gives a new accept on the listener, which is then closed and read to EOF. After the runtime is dropped, a non-blocking accept shows no third call. Row 4 (a request queued behind the held lock, closed, the lock let go, no connection for it) is written as tests/spicedb_cancel.rs:176. Request A holds the directory, apps and grants locks inside its SpiceDB read, with the grant slot still empty. Request B is sent and closed. A GET /grants/model on a later connection is answered, which shows the server is past B. A is then closed and its call reads EOF, the service is stopped and its runtime dropped (tokio's BlockingPool drop waits for every running section), and a non-blocking accept gets WouldBlock. A request that left cannot reach SpiceDB through either guard, still_asked or post_json's own start check, so spicedb_cancel_tests.rs:252 pins still_asked on its own. The ordering from B's close to B's cancel relies on hyper handling B's head and EOF in one poll_loop pass. That is not proven by a signal, so a spurious failure is possible in principle, though a hang is not. Row 5 (grep -rnE 'Duration::|_timeout|timeout\(' spicedb_cancel.rs prints nothing) is met; I ran it. The order of every section's own acts is unchanged: only the call wrapper changed, and claims keeps roles, then profile_version, then grants.
- Deviation: (1) Blocked on crates/lys-identity-server/src/runner_api.rs, outside the wall. runner_sessions::operator stays sync and its section still runs through with_grants on the async worker, so C423 and row 1 are not met until a brief revision admits runner_api.rs (CN9). (2) The boundary 'no code line in grants.rs beyond the flag read' cannot hold together with 'every async handler calls judged'. Each awaited call puts a .await on its own line under rustfmt, so grants.rs gains nine '.await' lines and nine changed call lines. Those pushed grants.rs to 508 code lines against the 500 limit (it was 498 at the base). To stay under the limit without adding code there, I moved the why handler unchanged into grants_batch.rs, which is in the wall and already holds the /grants/check/batch and /grants/which questions; grants.rs is now 485. (3) serve_close.rs and main.rs are not created or changed, because the served transport already drops the handler; the brief allows this. (4) CN1 ('documents only') read literally forbids every code file in this brief. I followed the brief's own requirement file walls instead. (5) The crusher hook reported compile errors in files this card does not touch: grant_contract/views.rs:117 and :208 (UnreportedView and StandingView not ToSchema), provisioning_api.rs:136 (SessionSettings), stop_api.rs:91 (ConfirmedEnd), and routes.rs:181 ('Arc does not implement Copy'). They appeared before this card's module was even declared, they are outside the wall, and they are left alone.
- Files changed:
  - modified: `crates/lys-identity-server/src/spicedb_cancel.rs` — judged(state: Arc<AppState>, act) makes a Cancel, holds Leaving in its own future (dropping the future cancels), and runs scope(&cancel, || with_grants(&state, act)) under tokio::task::spawn_blocking. A JoinError maps to DirectoryUnavailable. still_asked refuses with PermissionEngineUnavailable{LEFT} when the thread's Cancel is set.
  - modified: `crates/lys-identity-server/src/grants.rs` — The one flag read, crate::spicedb_cancel::still_asked()?, sits at line 173, right after the grants lock and before the first-use open and act. The nine handlers call crate::spicedb_cancel::judged(Arc::clone(&state), move |judged| ...).await. The why handler moved to grants_batch.rs, and its route line (137) points there.
  - modified: `crates/lys-identity-server/src/grants_batch.rs` — batch and which call judged. It now also holds the why handler (line 240), moved unchanged from grants.rs apart from judged, and the module doc names POST /grants/why.
  - modified: `crates/lys-identity-server/src/apps_schema_api.rs` — check, change and approve call judged with move closures; the with_grants import is removed.
  - modified: `crates/lys-identity-server/src/certificates_issue.rs` — grants(state: &Arc<AppState>, agent: AgentId, at) is async and calls judged. claims is async and computes roles, then profile_version, then grants in the order they were computed before. issue awaits claims.
  - modified: `crates/lys-identity-server/src/requests_api.rs` — list and ask call judged.
  - modified: `crates/lys-identity-server/src/requests_decide.rs` — approve, settle and decline call judged.
  - modified: `crates/lys-identity-server/src/resources_api.rs` — list calls judged.
  - modified: `crates/lys-identity-server/src/reviews_api.rs` — reviews and keep call judged.
  - created: `crates/lys-identity-server/tests/spicedb_cancel.rs` — Served-transport tests. They start the service through the shared harness (axum::serve, as main.rs does) on a runtime the test owns, with the SpiceDB endpoint at a listener that accepts and never answers, and talk to it over raw std TCP connections.
- Checklist delivery:
  - [ ] C423 — Every grants section runs off the async workers, and a request that leaves ends its SpiceDB wait and lets the grants lock go (DIRECTORY-061 R2). — Every grants section in the wall runs off the async workers in its request's scope, and closing the connection ends the SpiceDB wait and lets the lock go (tests/spicedb_cancel.rs:156). The runner operator section (runner_sessions.rs:116) still runs on the worker until runner_api.rs is admitted.
  - [x] C424 — A section that takes the grants lock after its request left calls no SpiceDB (DIRECTORY-061 R2). — with_grants calls still_asked at grants.rs:173, right after taking the grants lock and before grant_setup.open and act. The served test is tests/spicedb_cancel.rs:176 and the unit test is spicedb_cancel_tests.rs:252.
- Story delivery:
  - [ ] S169 (Person waiting on a permission check, Uses a Lys screen or route that checks a grant) — As a person whose request is waiting on the permission service, I want my request to end when I leave it, and nobody else's request held behind mine. — Met for every grants route in the wall. Not met for the runner operator check until runner_api.rs is admitted and operator becomes async.

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
