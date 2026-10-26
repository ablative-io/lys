---
type: brief
id: HOME-040
cluster: home
title: Every request Lys serves ends in an answer or a close: no lock across another service, no clock on a peer, a stop ends on its exit, and what Lys waits on is listed with a cancel
---

# HOME-040: Every request Lys serves ends in an answer or a close: no lock across another service, no clock on a peer, a stop ends on its exit, and what Lys waits on is listed with a cancel

> **Cluster:** home
> **Depends on:** DIRECTORY-122
> **Checklist:**
> - C862 — No lock is held across a call to another service: an email change, a letter, a setup step or a start waits on its peer without holding what another person's act needs. HOME-040/R1.
> - C863 — No call Lys makes to another service ends on a clock: each ends on the peer's answer, the peer's close or a transport error, with kernel keepalive on every outbound socket. HOME-040/R2.
> - C864 — A stop ends on the process's exit, and a person who will not wait ends it by an act; no stop escalates on a timer. HOME-040/R3.
> - C865 — A caller who closes ends what was being done for them: no task, upstream call or lock outlives the request that started it. HOME-040/R4.
> - C866 — Every outbound call in flight is listed with whom it waits on and since when, and an administrator can cancel it by name. HOME-040/R5.
> - C867 — Standing bars hold for HOME-040, with a quiet peer replayed for mail, the runner, SpiceDB, the issuer and the model upstream. HOME-040/R6.
> **Stories:**
> - S497 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a stalled mail server to stall only the letter it was sending, so that nobody's sign-in waits on it.
> - S498 (Estate operator, Runs Lys behind every agent and session) — As an estate operator, I want every call Lys makes to end on its peer's answer or its peer's close, so that a failure always names itself and nothing is left saying it never started.
> - S499 (Seat operator, Starts an agent session from what Lys records for it) — As a seat operator stopping a session, I want the stop to end when the process exits and to end it myself if I choose, so that no timer decides for me.
> - S500 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose request was dropped, I want everything Lys was doing for me to end with it, so that a closed request never leaves work or a lock behind.
> - S501 (Estate operator, Runs Lys behind every agent and session) — As an estate operator, I want to see what Lys is waiting on and for how long, and cancel it, so that a quiet peer is something I can see and end.
> - S502 (Developer, Works on lys-home's code beside the record module) — As a developer, I want a peer that accepts and never answers replayed against every outbound call, so that it is proven a quiet peer stalls only its own act.

## Purpose

Promised to Flapjack on 26 October 2026 as the Lys side of liminal SDK-024 and Cambium's CORE-092 rulings: every request ends in an answer or a close. A survey of the Lys server at 2a23bbb2 found three kinds of wait that break it. Locks held across another service: accounts_own.rs holds email_changes and the account lock across two SMTP sends, so a mail server that accepts and goes quiet stalls that person's sign-in (session_admission takes the same account lock); mail.rs send_once holds one record lock for all mail across the SMTP send, so one quiet send stalls every letter; setup.rs and setup_password.rs hold setup_lock across issuer calls; start/run.rs holds the computer's start lock across the runner. Clocks taken as failure or readiness, which Tom ruled out on 15 October: runner_client/session.rs ends a Status, Liveness or Owned ask after 5 s; runner_client/hub.rs counts a machine present for 2 s after its last contact (HANDOFF); stop_api/exit.rs gives an end 10 s (END_BUDGET); seats_monitor.rs escalates a stop after 30 s (STOP_WAIT); spicedb_http.rs bounds connect, write and read at 5 s; seat_import_monitor.rs asks for 10 s (ASKED_FOR); and the issuer, sign-in provider, message edge and app manifest clients carry their own timeouts. Waits with nothing to end them: model_proxy_route.rs forwards every model call on reqwest::Client::new() with no keepalive, so an upstream whose link goes half-open leaves the call open for ever. Liminal settled the same question in SDK-024: a wait ends on bytes, the peer's close or a transport error, the kernel's keepalive finds a half-open link, and a caller who will not wait closes. Cambium's CORE-092 ruling for a stuck far service is elapsed time shown with a cancel, the cancel or a disconnect being the event. This brief brings Lys to the same rule.

## Task

Release every lock before a call to another service, end every outbound wait on its peer's answer, close or transport error with kernel keepalive beneath it, end stops on exit or a person's act, end all work for a request when its caller closes, and list what Lys is waiting on with a cancel.

## Requirements

### R1: No lock is held across a call to another service

THE SYSTEM SHALL hold no lock, async or sync, across a call to another service (SMTP, the issuer, SpiceDB, a runner, an app, the model upstream). Where an act must happen once, it SHALL record its claim under the lock, release the lock, make the call, and record the outcome under the lock again; a second act for the same subject while the first is in flight waits on that subject's own event (its outcome recorded), never on a lock shared with other subjects. In particular: an email change (accounts_own.rs) records its pending codes and releases the account lock and email_changes before either letter is sent, and a failed send is answered by name with the pending change withdrawn under the lock; send_once (mail.rs) claims its operation, sends outside the record lock, and records the letter; a setup step (setup.rs, setup_password.rs) and a start (start/run.rs) hold their turn per subject (the founding account; the agent on its computer) and the turn is taken and released around the record, not the call. Lock order stays account, then directory, then foundings.

**Acceptance:**
- Red first (crates/lys-identity-server/tests/quiet_peer.rs): with an SMTP listener that accepts and never answers, an email change for person A stays open while person A signs in with Google and person B changes their own email through a second, answering mail transport; both complete. Closing A's request ends A's change with the pending codes withdrawn.
- Red first: two letters for different operations while one SMTP send is quiet: the second is sent and recorded; the same operation asked twice while in flight is sent once.

**Files:**
- create: crates/lys-identity-server/tests/quiet_peer.rs
- modify: crates/lys-identity-server/src/accounts_own.rs
- modify: crates/lys-identity-server/src/mail.rs
- modify: crates/lys-identity-server/src/setup.rs
- modify: crates/lys-identity-server/src/setup_password.rs
- modify: crates/lys-identity-server/src/start/run.rs

**Checklist:**
- C862 — No lock is held across a call to another service: an email change, a letter, a setup step or a start waits on its peer without holding what another person's act needs. HOME-040/R1.

**Stories:**
- S497 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a stalled mail server to stall only the letter it was sending, so that nobody's sign-in waits on it.

### R2: No call to another service ends on a clock

THE SYSTEM SHALL end every wait on another service on the peer's answer, the peer's close, or a transport error, and on nothing else: the 5 s runner ask bound (runner_client/session.rs), the 2 s HANDOFF presence window (runner_client/hub.rs), WAIT in spicedb_http.rs, ASKED_FOR in seat_import_monitor.rs, and the request and connect timeouts of the issuer, OIDC, sign-in provider, message edge and app manifest clients (issuer_admin.rs, oidc.rs, sign_in_providers.rs, message_edges.rs, apps_connect_manifest.rs) are removed. Every outbound socket, the model proxy's included, SHALL carry the keepalive liminal's SDK-023 R3 sets (SO_KEEPALIVE on, idle 30 s, interval 10 s, 5 probes), so a half-open link is learned from the kernel and named as the transport error it is. A machine is present while its runner holds a bridge open; the runner opens its next bridge before it answers on the last, so presence has no gap to bridge with a clock, and absence is that connection's close. The one bound kept is a connect's own: a peer that accepts the socket and never speaks its protocol's first word is not that service (a wrong address or port), it is named so in those words, as SDK-024 keeps OPEN_DEADLINE; each such bound is listed by name in the handback with the first word it waits for.

**Acceptance:**
- Red first (tests/quiet_peer.rs): a runner, a SpiceDB gateway, an issuer and a model upstream that each accept and go quiet after their first word hold the request open and answer nothing else; closing the peer's socket ends the request with that peer's refusal by name, read by its status and body.
- ast-grep rule rules/ast-grep/no-clock-on-a-peer.yml refuses tokio::time::timeout, timeout_at, sleep and a reqwest timeout or connect_timeout in crates/lys-identity-server/src outside the listed connect bounds and the schedules and goals due times, and the gate runs it.

**Files:**
- create: rules/ast-grep/no-clock-on-a-peer.yml
- modify: crates/lys-identity-server/src/runner_client/session.rs
- modify: crates/lys-identity-server/src/runner_client/hub.rs
- modify: crates/lys-identity-server/src/spicedb_http.rs
- modify: crates/lys-identity-server/src/seat_import_monitor.rs
- modify: crates/lys-identity-server/src/issuer_admin.rs
- modify: crates/lys-identity-server/src/oidc.rs
- modify: crates/lys-identity-server/src/sign_in_providers.rs
- modify: crates/lys-identity-server/src/message_edges.rs
- modify: crates/lys-identity-server/src/apps_connect_manifest.rs
- modify: crates/lys-identity-server/src/model_proxy_route.rs
- modify: crates/lys-runner/src/dial/transport.rs
- modify: crates/lys-identity-server/tests/quiet_peer.rs

**Checklist:**
- C863 — No call Lys makes to another service ends on a clock: each ends on the peer's answer, the peer's close or a transport error, with kernel keepalive on every outbound socket. HOME-040/R2.

**Stories:**
- S498 (Estate operator, Runs Lys behind every agent and session) — As an estate operator, I want every call Lys makes to end on its peer's answer or its peer's close, so that a failure always names itself and nothing is left saying it never started.

### R3: A stop ends on the exit or on a person's act

WHEN a stop is asked THE SYSTEM SHALL answer it when the runner reports the process's exit, read as the exit event (the runner's reap), with no END_BUDGET (stop_api/exit.rs) and no STOP_WAIT escalation (seats_monitor.rs). While the stop waits, the session's page shows the stop as waiting since the instant it was asked, with an End now act; End now asks the runner's End act by the person's word, and is recorded as theirs. A runner whose connection closes while a stop waits ends the stop with runner_unreachable by name, never with a guess about the process.

**Acceptance:**
- Red first (crates/lys-identity-server/src/stop_api/exit_tests.rs): a process that ignores its stop keeps the stop waiting with no answer and no End sent, until End now is asked, whose record names the person; a runner that closes ends the stop with runner_unreachable.

**Files:**
- create: crates/lys-identity-server/src/stop_api/exit_tests.rs
- modify: crates/lys-identity-server/src/stop_api/exit.rs
- modify: crates/lys-identity-server/src/seats_monitor.rs
- modify: crates/lys-identity-server/src/stop_api.rs

**Checklist:**
- C864 — A stop ends on the process's exit, and a person who will not wait ends it by an act; no stop escalates on a timer. HOME-040/R3.

**Stories:**
- S499 (Seat operator, Starts an agent session from what Lys records for it) — As a seat operator stopping a session, I want the stop to end when the process exits and to end it myself if I choose, so that no timer decides for me.

### R4: A caller who closes ends what was being done for them

WHEN the caller of a request closes its connection THE SYSTEM SHALL end every piece of work started for that request: the handler's future is dropped, a blocking task it spawned is told to leave by the same drop (as LeaveOnDrop does for the runner ask), an upstream model call is closed, and every claim made under R1 is withdrawn or settled under its lock. No task started for a request outlives it, and no work started for a request is detached from it.

**Acceptance:**
- Red first (tests/quiet_peer.rs): for the runner ask, the model proxy and a letter, the client closes mid-wait; the quiet peer then sees its connection closed, the service's in-flight list (R5) no longer names the call, and the same subject's next act proceeds.

**Files:**
- modify: crates/lys-identity-server/src/model_proxy_route.rs
- modify: crates/lys-identity-server/src/runner_client/session.rs
- modify: crates/lys-identity-server/src/mail.rs
- modify: crates/lys-identity-server/tests/quiet_peer.rs

**Checklist:**
- C865 — A caller who closes ends what was being done for them: no task, upstream call or lock outlives the request that started it. HOME-040/R4.

**Stories:**
- S500 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent whose request was dropped, I want everything Lys was doing for me to end with it, so that a closed request never leaves work or a lock behind.

### R5: What Lys waits on is listed and can be cancelled

THE SYSTEM SHALL keep, in memory, one entry for every outbound call in flight: its service (mail, issuer, SpiceDB, runner, app, model upstream), the peer's address, the act and subject it serves, and the instant it began. GET /waits answers them to an administrator, oldest first, and POST /waits/{id}/cancel ends the named call as its caller's close would (R4), answered by the call's own refusal naming the administrator's cancel. The Health page lists them with the time each has waited, counted on the page from its start, and a Cancel act on each. Nothing is cancelled or flagged for having waited long.

**Acceptance:**
- Red first (crates/lys-identity-server/src/waits_api_tests.rs and surface/identity tests): a quiet SMTP send is listed with its start; cancel ends it and the act that waited answers MailNotSent naming the cancel; a non-administrator is refused by name.

**Files:**
- create: crates/lys-identity-server/src/waits_api.rs
- create: crates/lys-identity-server/src/waits_api_tests.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: surface/identity/src/generated/index.ts

**Checklist:**
- C866 — Every outbound call in flight is listed with whom it waits on and since when, and an administrator can cancel it by name. HOME-040/R5.

**Stories:**
- S501 (Estate operator, Runs Lys behind every agent and session) — As an estate operator, I want to see what Lys is waiting on and for how long, and cancel it, so that a quiet peer is something I can see and end.

### R6: Standing bars, with a quiet peer replayed

THE SYSTEM SHALL hold every standing bar on the exact final commit: cargo fmt, clippy pedantic -D warnings with all targets, cargo nextest run (never cargo test), cargo test --doc, ast-grep with no-clock-on-a-peer, the file-length gate, tsc and vitest for surface/identity, and scripts/design/gate.sh with the rendered markdown equal to its JSON. tests/quiet_peer.rs replays one peer that accepts and never answers for each of mail, the runner, SpiceDB, the issuer and the model upstream, and each test ends on an event (a close, an answer, a recorded outcome), never a sleep or a timeout. No test is deleted, weakened or marked serial.

**Acceptance:**
- Every quiet-peer test fails on 2a23bbb2 for the reason R1 to R4 name and passes on the final commit, both runs' lines kept in the handback.
- Every gate leg green on the final commit, each judged by its parsed output lines, never by its exit code.

**Files:**
- modify: crates/lys-identity-server/tests/quiet_peer.rs

**Checklist:**
- C867 — Standing bars hold for HOME-040, with a quiet peer replayed for mail, the runner, SpiceDB, the issuer and the model upstream. HOME-040/R6.

**Stories:**
- S502 (Developer, Works on lys-home's code beside the record module) — As a developer, I want a peer that accepts and never answers replayed against every outbound call, so that it is proven a quiet peer stalls only its own act.

## Boundaries

- SHALL NOT add a sleep, a timeout, a retry on a clock or a poll; schedules and goals keep waking at their due instants, which are the events they were asked for.
- SHALL NOT end, cancel or flag a call for having waited long: only its peer, its caller or a person ends it.
- SHALL NOT change what any act decides or records, only how it waits.
- SHALL NOT change the lock order account, directory, foundings.
- SHALL NOT draw a bare button for an act: every act is an Act.

## Verification

- cargo nextest run -p lys-identity-server -E 'binary(quiet_peer) | test(exit_tests) | test(waits_api)' alone, read by its PASS and FAIL lines; then the full gate.
- Red first: every new test is run before the code and its failing lines kept in the handback.
- Walked on the installed Lys with the mail transport pointed at a listener that never answers: an email change waits and is listed on Health; another person signs in meanwhile; Cancel ends the change by name. Each step's screen is read, not assumed.
- Hospital line: If this code were used in a hospital, the worst credible failure is a mail server that stops answering and, through a held lock, stops every clinician on the ward from signing in at shift change, and it could harm about every patient on that ward while sign-in is stalled.
