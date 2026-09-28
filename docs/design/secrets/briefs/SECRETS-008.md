---
type: brief
id: SECRETS-008
cluster: secrets
title: A service issues a handle bound to a public key it names, for a person's agent
---

# SECRETS-008: A service issues a handle bound to a public key it names, for a person's agent

> **Cluster:** secrets
> **Depends on:** SECRETS-006, DIRECTORY-059
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-125 — An agent session's holder key is held by the runner, and a session proves itself by peer credentials and ancestry — The runner makes one holder key per session in memory. A start becomes key, then issue, then spawn. The harness and lys mcp ask the runner for single presentations over its socket. The session is proved by peer credentials plus the process ancestry reaching the session's own root pid, checked on every connection, on macOS and Linux both, through a named maintained dependency and with no unsafe in Lys code.
> **Checklist:**
> - C416 — The service lys-identity-server derives a handle under one the agent holds, bound to a public key it names, only when the holder acts for the person it acts on behalf of (SECRETS-008 R1).
> - C417 — Every issue through the route is on the audit record, and its handle token is in no log or audit line (SECRETS-008 R2).
> **Stories:**
> - S167 (Person, Grants an agent provisioned under them access to an account) — As a person starting an agent, I want its session's handles issued to a key only its runner holds, so that the agent can use every handle it is launched with and no key is ever handed to it.

## Purpose

A broker handle is usable only with presentations signed by the holder key bound when it is issued (crates/lys-secrets/src/broker/admit.rs lines 77 to 87). Today only the lys-secrets Issue command issues a handle, loading the holder key from a file (crates/lys-secrets/src/bin/lys-secrets/main.rs lines 136 to 146). So an agent started by Lys is launched with handle ids and no key that can present for them. ADR-125 gives each agent session one holder key, held by the runner in memory. This card gives the broker the route that issues a handle bound to that key's public key, asked for by lys-identity-server on behalf of the person who starts the agent.

## Task

Add POST /_lys/handles/issue. It admits only a screen service acting on behalf of a person, through the existing on-behalf admission. It derives one handle under a handle the agent already holds, bound to a named Ed25519 public key, with every use, the end and the spend left above it, after checking that the holder acts for the person acted for. It answers the handle id and token, and the derive is on the audit record as every derive is.

## Requirements

### R1: The issue route derives a session handle under a held handle

Behavioural. POST /_lys/handles/issue is a new route in crates/lys-secrets/src/bin/lys-secrets/serve.rs, handled in the new crates/lys-secrets/src/bin/lys-secrets/issue_route.rs. The route first checks for the lys-service header, and a request without it is refused issue_service_only before any other check. It then admits the caller only through on_behalf (crates/lys-secrets/src/bin/lys-secrets/callers.rs line 104), which becomes pub(crate). Today ServiceWindow::admit refuses a `service:` name as service_person_invalid (crates/lys-secrets/src/service.rs lines 169 to 174). DIRECTORY-059 R1, which this card depends on, admits a `service:` name equal to the signing service's own, and its refusal of every other route lives in access.rs, which this route does not pass through. So the route itself refuses an identity acted for that begins `service:` as issue_service_only, before any issue. The request is JSON holding exactly two members, under and holder_key, and its type denies unknown members. The member under is the id of a handle the agent already holds, the one the launch record names (HandleName.id, crates/lys-identity-server/src/launch_template.rs lines 24 to 31). The member holder_key is a 32-byte Ed25519 public key written as exactly 64 lower-case hex characters. Anything else is refused holder_key_invalid naming the number of characters given. The route calls one new library function, Broker::derive_for_person in the new crates/lys-secrets/src/broker/issuing.rs, with the held handle's id, the key and the person acted for. It derives one handle for the held handle's own identity, bound to the key, as Broker::derive does (crates/lys-secrets/src/broker/lineage.rs lines 219 to 345), with the parent set to the held handle. It takes no presentation of the held handle, because the authority is the person's. So in place of presented and the presentation half of live (crates/lys-secrets/src/broker/admit.rs lines 57 and 93), it finds the held handle by id. Straight after that lookup, before any other check, it checks that the held handle's identity acts for the person acted for, by the broker's existing acts_for (crates/lys-secrets/src/broker/ending.rs lines 528 to 536), which becomes pub(super). The check passes when the identity is the person, or when member_of says it acts on `person/<the person acted for>`. It never compares Permitted::person, because the directory's permit names no person (crates/lys-secrets/src/bin/lys-secrets/spice.rs lines 136 to 137). A held handle whose identity does not act for the person, and an id that names no handle, are both refused issue_not_acting_for naming the id asked for and the person, in the same words, so a service acting for a person learns nothing about handles that are not that person's. Only then does it refuse as live does today when the held handle is dropped, ended or past its window, through one new function standing that live then calls for the same half. Every other check of derive holds unchanged, namely permitted, ancestry_admits, the depth under MAX_DEPTH, recipient_admitted, within_scope, may_use and within_grant. No lend check is asked, because the derived handle's identity is the held handle's own. The shared function takes one argument lend, a bool, and asks the lend check in the place it has today (lineage.rs lines 250 to 264, between the depth check and recipient_admitted) only when lend is true. derive passes true and derive_for_person passes false, so the order of every check derive asks is unchanged. The derived handle takes every use left above it, ends when the held handle ends, and carries the spend left above it as its cap, so it can never outlive or outspend the handle it is cut under. A held handle with no use left is refused BeyondAncestry. Dropping or ending the held handle ends the derived one through the lineage that exists today. The shared checks and the record's writing move out of derive into one private function in issuing.rs that both call, so derive keeps every refusal it has today. Every check comes before the handle id is generated, the audit line of kind issue with the outcome derived from the held handle, the insert and the handles write. The whole derive runs in the one call on the broker that on_broker makes under the broker's lock (serve.rs lines 131 to 139), so no other request runs between the checks and the write. The answer is the derived handle's id and its token in hex, as the Issue command prints them (main.rs lines 160 and 161). The three refusals issue_service_only, holder_key_invalid and issue_not_acting_for are one new enum IssueRefusal in crates/lys-secrets/src/error/issue.rs, re-exported from crates/lys-secrets/src/lib.rs beside the other refusal enums (lines 42 to 45). SecretsError gains one variant Issue holding it by #[from], and its name comes through error/name.rs. The function refused in callers.rs (lines 42 to 73) answers 400 for holder_key_invalid and 403 for the other two. The holder_key is never compared with the held handle's own key, by design, because the authority is the person's and not the held handle's holder. So a service acting for a person can derive under any handle whose identity acts for that person, a handle lent to that person's agent included, and an identity that acts for two people can have a handle derived under it by either. The module issuing.rs is declared by one line `mod issuing;` in crates/lys-secrets/src/broker.rs beside mod held and mod lineage (lines 36 and 39), and broker.rs changes by that line alone.

**Acceptance:**
- A presentation signed by the named key is admitted for a handle the route derived, in crates/lys-secrets/tests/issue_route.rs.
- A presentation signed by any other key is refused PresentationInvalid for that handle.
- The derived handle's parent is the held handle named by under.
- The derived handle's uses are the uses left on the held handle.
- The derived handle's end is the held handle's end.
- After the held handle is dropped, a presentation for the derived handle is refused HandleDropped.
- A request without the lys-service header is refused issue_service_only.
- A screen service acting for `service:` and its own name is refused issue_service_only.
- A holder_key of 62 hex characters is refused holder_key_invalid naming 62.
- A holder_key of 64 characters that are not all lower-case hex is refused holder_key_invalid.
- A request whose under names no handle is refused issue_not_acting_for in the same words as a handle that does not act for the person.
- A request under a held handle with no use left is refused BeyondAncestry.
- A request under a held handle whose holder does not act for the person acted for is refused issue_not_acting_for naming both.
- After a request its test first asserts is refused issue_not_acting_for, handles.json in the broker's root holds no new handle.
- With a permission source whose permit names no person, as the directory's does, a request under a handle whose holder acts for the person acted for is derived.
- A request holding a member other than under and holder_key is refused 400.
- As a guard, the existing tests of derive in crates/lys-secrets/tests/lineage.rs pass unchanged.
- A request under a dropped held handle whose identity does not act for the person acted for is refused issue_not_acting_for, not HandleDropped.

**Files:**
- create: crates/lys-secrets/src/bin/lys-secrets/issue_route.rs
- create: crates/lys-secrets/src/broker/issuing.rs
- create: crates/lys-secrets/src/error/issue.rs
- create: crates/lys-secrets/tests/issue_route.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/serve.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/main.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/callers.rs
- modify: crates/lys-secrets/src/error.rs
- modify: crates/lys-secrets/src/error/name.rs
- modify: crates/lys-secrets/tests/support/served.rs
- modify: crates/lys-secrets/src/lib.rs
- modify: crates/lys-secrets/src/broker/ending.rs
- modify: crates/lys-secrets/src/broker/lineage.rs
- modify: crates/lys-secrets/src/broker/admit.rs
- modify: crates/lys-secrets/src/broker.rs

**Checklist:**
- C416 — The service lys-identity-server derives a handle under one the agent holds, bound to a public key it names, only when the holder acts for the person it acts on behalf of (SECRETS-008 R1).

**Stories:**
- S167 (Person, Grants an agent provisioned under them access to an account) — As a person starting an agent, I want its session's handles issued to a key only its runner holds, so that the agent can use every handle it is launched with and no key is ever handed to it.

### R2: Every issue is on the audit record and its token in no log

Behavioural. The derive appends the audit line of kind issue, as derive does today, so the route adds no audit line of its own. A refused request appends nothing, as a refused issue does today. The handle token appears only in the route's answer. It is never written to a log line, an audit line or an error's words. The log-line test starts the broker with Served's start_logged (SECRETS-006 R2), which writes the broker's standard error to a file in the served root, and searches that file.

**Acceptance:**
- One issue through the route appears once at GET /_lys/audit, read on behalf of the person acted for, as a line of kind issue naming the identity and the secret.
- A request its test first asserts is refused issue_not_acting_for adds no line to GET /_lys/audit read on behalf of the person acted for.
- The route's answer holds the handle token's hex, and the test searches the log file for that same hex.
- The log file start_logged writes is not empty when the test searches it.
- No log line the broker writes during the test holds the handle token's hex.
- No audit line holds the handle token's hex.

**Files:**
- modify: crates/lys-secrets/tests/issue_route.rs

**Checklist:**
- C417 — Every issue through the route is on the audit record, and its handle token is in no log or audit line (SECRETS-008 R2).

**Stories:**
- S167 (Person, Grants an agent provisioned under them access to an account) — As a person starting an agent, I want its session's handles issued to a key only its runner holds, so that the agent can use every handle it is launched with and no key is ever handed to it.

## Boundaries

- SHALL NOT issue a handle to a key the broker made or holds. The holder key is only ever named by its public key.
- SHALL NOT admit this route for a handle's own presentation or for a caller that is not a screen service acting on behalf of a person.
- SHALL NOT change issue_capped, or any refusal derive answers today.
- SHALL NOT let a derived handle carry more uses, a later end or a larger cap than the handle it is cut under.
- SHALL NOT generate, audit, insert or write a handle before every check has passed.
- SHALL NOT return, log or write a handle token anywhere but the route's answer.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass.
- SHALL NOT add a silent fallback. Every failure is a named refusal.

## Verification

- The full Lys gate and ast-grep scan exit 0 at the card's head, measured by the card round.
- `cargo test -p lys-secrets --test issue_route` exits 0 on Dean's laptop at the card's head.
