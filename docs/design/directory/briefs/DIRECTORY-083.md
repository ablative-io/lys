---
type: brief
id: DIRECTORY-083
cluster: directory
title: `lys identity stop` reaches the service, and the service records the console's stop
---

# DIRECTORY-083: `lys identity stop` reaches the service, and the service records the console's stop

> **Cluster:** directory
> **Design anchor:**
> - ADR-117 — Lys runs the agents it starts: its own background terminal, or a runner another tool provides — Lys runs the agents it starts. A runner is the part that holds an agent's terminal: Lys ships one of its own, a background process on each machine that runs each started agent in its own pseudo-terminal, and accepts any other runner that speaks the same small, published runner protocol. Through a runner, Lys starts, types into, presses keys in, reads, waits on, resizes, compacts and stops a session, rotates it across its accounts when it reports a usage limit, and wakes it with a message, each act under the caller's grants and recorded. Lys depends on no particular runner: its own is the default, another is chosen per machine.
> **Checklist:**
> - C504 — `lys identity stop` and the service take the console stop's route, signing domain, signature header and body from one definition in lys-runner, and the CLI keeps no copy of any of them (DIRECTORY-083 R1).
> - C505 — A console stop signed by the service's own key, posted through the service's real router, pulls the cord and records the console's claim as who, the reason and the service's own time (DIRECTORY-083 R2, R3).
> - C506 — A console stop with no signature, another key's signature, a body changed after signing or a session cookie beside the signature is refused by name, and the cord is not pulled (DIRECTORY-083 R2, R3).
> - C507 — A console stop sent again is answered as the same pull and stops nothing twice; sent after a release it is refused cord_reused (DIRECTORY-083 R2, R3).
> - C508 — A console stop is admitted while an upgrade can still be put back, and every start after it is refused everything_stopped in words naming the console's person (DIRECTORY-083 R2, R3).
> **Stories:**
> - S267 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want `lys identity stop` at this computer to stop every agent on every computer and to record who stopped them, when and why, even when nobody is signed in to the screens and while an upgrade can still be put back, so that one command at the machine is always enough to stop everything.

## Purpose

`lys identity stop` never records its stop on the service. stop.rs ask_service (lines 253 to 295) signs a console request with the service key under "lys-identity/console-stop/v1" and posts it to /runtime/stop-everything/console, or /api/runtime/stop-everything/console behind the screens (console_endpoint, lines 233 to 250). The service has no such route: cord_api.rs routes() (lines 72 to 75) serves only GET and POST /runtime/stop-everything and POST /runtime/stop-everything/release, both for a signed-in administrator, and nothing on the service verifies the console signature. So the stop is answered 404: runners on other computers are not reached, no reason is recorded and no start is held back. Commit 5a493f36 (4 October) named the gap ("lys identity stop calls a console route the service does not have yet"), and Waffles found it again in the BOX 17 window on 6 October (27e6d948). No test joins the CLI's request to the service's router; that join is the first test to write. The operator token cannot stand in: a service install keeps none (install.rs operator_token), and operator.rs refuses it while an upgrade is reversible.

## Task

Give the console stop one definition that both sides use, serve it on the service, admit it on the service key's signature alone, and record it as a pull like any other. Write the join test first. Out of scope: releasing the cord from the console (release stays with a signed-in administrator), the screens, and what a pull does once admitted.

## Requirements

### R1: One definition of the console stop, used by both sides

Ubiquitous. THE SYSTEM SHALL define the console stop once, in lys-runner console_stop.rs, which both lys and lys-identity-server already depend on: the route /runtime/stop-everything/console, the signing domain "lys-identity/console-stop/v1", the signature header "x-lys-console-signature", the body {operation, by, reason, kill} with unknown members refused, and the bytes signed, the domain, a newline and the body exactly as sent. stop.rs SHALL build its request from it and keep no route, domain, header or body shape of its own; it still chooses the /api prefix from the installed surface_dir as it does today. The body carries no nonce and no time: the operation id is the replay guard, since the cord store spends each operation once for good (cord_store.rs lines 451 to 466), and the service records its own time, as for a signed-in pull.

**Acceptance:**
- stop.rs names no console route, domain, header or body member of its own.
- The service decodes exactly the body stop.rs sends, and refuses an unknown member by name.
- The bytes the CLI signs and the bytes the service verifies come from one function.

**Files:**
- create: crates/lys-runner/src/console_stop.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys/src/identity/stop.rs

**Checklist:**
- C504 — `lys identity stop` and the service take the console stop's route, signing domain, signature header and body from one definition in lys-runner, and the CLI keeps no copy of any of them (DIRECTORY-083 R1).

**Stories:**
- S267 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want `lys identity stop` at this computer to stop every agent on every computer and to record who stopped them, when and why, even when nobody is signed in to the screens and while an upgrade can still be put back, so that one command at the machine is always enough to stop everything.

### R2: The service admits the console stop on its own key's signature and records it

Event-driven. WHEN a POST reaches the console route, THE SYSTEM SHALL verify the signature header over the signed bytes with the public half of the service's own event key, the key routes.rs loads from event_key_file, which the installer points at the same keys/service.key the CLI signs with (server_config.rs line 105). A missing, malformed or wrong signature, a body changed after signing, or a session cookie beside the signature is refused by name before any pull, and the cord is not pulled. The route is admitted on the signature alone: no session, no operator token, and admitted while an upgrade is still reversible. signed_first.rs SHALL treat the signature header as a credential its route judges over the body, under the unverified body limit, and lys-openapi SHALL name it as its own security scheme. WHEN admitted, THE SYSTEM SHALL pull the cord through the same cord_pull.rs work as a signed-in pull, recording `by` as "console", `by_name` as the console's claim from the body ("<user> at the console of this computer"), the reason, kill and the service's own time, so every refused start names that person. The claim is recorded as the console's word, never looked up in the directory. Handles are ended for a console pull without a session: cord_pull.rs and stop_api.rs ask the broker through secrets_api ask_as on behalf of each agent's owner as the directory names it, and a broker refusal is recorded in handles_refused by name, never skipped. WHEN the same signed body comes again, THE SYSTEM SHALL answer the pull it already made and stop nothing twice; after a release it is refused cord_reused.

**Acceptance:**
- A console stop signed by the service key pulls the cord, and the pull records by "console", the claim, the reason and kill.
- A missing, malformed or other key's signature is refused by name and pulls nothing.
- A body changed after signing is refused by name and pulls nothing.
- A session cookie beside the console signature is refused by name.
- The console stop is admitted while an upgrade is reversible.
- A start after a console stop is refused everything_stopped, naming the console's person and reason.
- A console pull ends each running agent's handles, or names the broker's refusal.
- The same signed body sent again answers the same pull; after a release it is refused cord_reused.

**Files:**
- modify: crates/lys-identity-server/src/cord_api.rs
- modify: crates/lys-identity-server/src/cord_pull.rs
- modify: crates/lys-identity-server/src/cord_store.rs
- modify: crates/lys-identity-server/src/error_cord.rs
- modify: crates/lys-identity-server/src/stop_api.rs
- modify: crates/lys-identity-server/src/signed_first.rs
- modify: crates/lys-identity-server/src/openapi_table.rs
- modify: crates/lys-openapi/src/lib.rs

**Checklist:**
- C505 — A console stop signed by the service's own key, posted through the service's real router, pulls the cord and records the console's claim as who, the reason and the service's own time (DIRECTORY-083 R2, R3).
- C506 — A console stop with no signature, another key's signature, a body changed after signing or a session cookie beside the signature is refused by name, and the cord is not pulled (DIRECTORY-083 R2, R3).
- C507 — A console stop sent again is answered as the same pull and stops nothing twice; sent after a release it is refused cord_reused (DIRECTORY-083 R2, R3).
- C508 — A console stop is admitted while an upgrade can still be put back, and every start after it is refused everything_stopped in words naming the console's person (DIRECTORY-083 R2, R3).

**Stories:**
- S267 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want `lys identity stop` at this computer to stop every agent on every computer and to record who stopped them, when and why, even when nobody is signed in to the screens and while an upgrade can still be put back, so that one command at the machine is always enough to stop everything.

### R3: The join is tested first, through the service's real router

Behavioural. Before any route code is written, THE SYSTEM's tests SHALL build a console stop with lys-runner's console_stop.rs, sign it with the test service's own event key, and post it at the path stop.rs chooses, with and without the /api prefix, through the service's real router, and see the cord pulled with the claim and reason recorded and a running session stopped. That test is red on the head this brief is written at, and is posted red before the route lands. The refusals, the replay, the release and the reversible upgrade each have their own test in the same file.

**Acceptance:**
- The join test fails on the head this brief is written at, and passes after R1 and R2.
- Each acceptance line of R2 has a test in tests/stop_everything.rs.

**Files:**
- modify: crates/lys-identity-server/tests/stop_everything.rs
- modify: crates/lys-identity-server/tests/openapi.rs

**Checklist:**
- C505 — A console stop signed by the service's own key, posted through the service's real router, pulls the cord and records the console's claim as who, the reason and the service's own time (DIRECTORY-083 R2, R3).
- C506 — A console stop with no signature, another key's signature, a body changed after signing or a session cookie beside the signature is refused by name, and the cord is not pulled (DIRECTORY-083 R2, R3).
- C507 — A console stop sent again is answered as the same pull and stops nothing twice; sent after a release it is refused cord_reused (DIRECTORY-083 R2, R3).
- C508 — A console stop is admitted while an upgrade can still be put back, and every start after it is refused everything_stopped in words naming the console's person (DIRECTORY-083 R2, R3).

**Stories:**
- S267 (Installer of the identity stack, Runs lys identity install and stop on a machine) — As the person who installed Lys, I want `lys identity stop` at this computer to stop every agent on every computer and to record who stopped them, when and why, even when nobody is signed in to the screens and while an upgrade can still be put back, so that one command at the machine is always enough to stop everything.

## Boundaries

- SHALL NOT admit the console route on a session, the operator token or anything but the service key's signature.
- SHALL NOT let the console release the cord.
- SHALL NOT judge the request by a clock, or add a nonce store beside the cord's spent operations.
- SHALL NOT look the console's claim up in the directory or widen what any other route admits.
- SHALL NOT change what a pull does once admitted, or the screens.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], unsafe code or any bypass.

## Verification

- The join test is posted red at the brief's head before the route lands.
- cargo nextest run for lys, lys-runner, lys-openapi and lys-identity-server whole, cargo test --doc, fmt, clippy pedantic in both configurations, ast-grep and the file-length check exit 0 at the head, on Dean.
- After install, `lys identity stop --reason <words>` on the live install records the pull with that reason, read back from GET /runtime/stop-everything, in a window Waffles calls.

## Amendments

### Amendment 1: the openapi route scanner reads the console route from its one wire definition

- **Date:** 2026-10-07
- **By:** Archie

The full workspace nextest at 98f0e035 failed at tests/openapi.rs:161, every_declared_route_has_an_entry_and_every_entry_a_route: the table entry post /runtime/stop-everything/console had no declared route (Brisket a0c5a573). The route is served: all 20 stop_everything tests passed in that run. The cause is the test's scanner. declared_in (tests/openapi.rs 26-63) takes a route's path only from a quoted literal, and cord_api.rs registers the console route as .route(console_stop::PATH, post(console)), as R1 requires. Putting a literal back in production would break R1's single wire definition, so the test changes and production does not. R3 adds tests/openapi.rs. declared_in resolves a first argument of exactly console_stop::PATH to lys_runner::console_stop::PATH, and reads every other route as before. The comparison between routes and table entries, and its planted-route failure, stay unchanged. One unit test, a_shared_console_route_is_read_from_its_wire_definition, pins both the single-line and the wrapped form. This is the ordinary case the scanner must permit. The change is exactly Brisket's proposal (proposed-openapi-scanner.patch), granted by Waffles (dd55e150). The FAIL at 98f0e035 is the red. The green is the full workspace nextest plus the eight non-install legs at the new exact head, on Annabel. A route whose path is any other non-literal is still skipped without a word. That is recorded as a finding for a later box, not changed here.
