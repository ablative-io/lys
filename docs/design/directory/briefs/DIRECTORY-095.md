---
type: brief
id: DIRECTORY-095
cluster: directory
title: The identity server's warnings reach its log: one subscriber writes warn and above through the say sink
---

# DIRECTORY-095: The identity server's warnings reach its log: one subscriber writes warn and above through the say sink

> **Cluster:** directory
> **Depends on:** DIRECTORY-094
> **Design anchor:**
> - ADR-002 — The token revolver is the first consumer of the handle — The worker asks the broker for its next account instead of walking its own list; the store keeps the set of real accounts behind one handle and the proxy takes the next in turn and logs which one served each call. This replaces the account pool file.
> **Checklist:**
> - C733 — lys-identity-server installs one tracing subscriber at start that says every WARN and ERROR event, with its target and fields, through the say sink into identity.log; info and below are off (DIRECTORY-095 R1).
> **Stories:**
> - S417 (Operator, Installs and runs the standalone identity product) — As an operator, I want every warning the identity server raises written in its log with where it came from, so that a failure the service noticed is never silent.

## Purpose

Building DIRECTORY-094 on 10 October 2026 found that lys-identity-server installs no tracing subscriber. main.rs prints every log line through the `say` sink (println to stdout, which the install keeps as identity.log). So the server's tracing events reach no one: 12 tracing::warn! and 12 tracing::error! call sites in src (counted with grep at 16:2x). Among them is apps_client_credentials.rs's warning that an app client credential whose issue was not kept could not be ended at the broker. Waffles ruled at 16:14 that it is a product defect, not a later row: one subscriber at start that writes warn and above through the say sink into identity.log, with the target and the fields; info and below off by default; one pin; the 094 line unchanged.

## Task

Add a hand-written tracing subscriber to lys-identity-server (no new dependency). It is enabled for WARN and ERROR only. It says each event as one line through the service's `say` sink: its level, its target, its message and every other field as name=value. It ignores spans. main.rs installs it as the global default before the service starts, on the same `say` it gives the service. Pin it with a test that installs it the way main does and finds a warn event's line, and no info event's line.

## Requirements

### R1: Warn and above are said in identity.log with their target and fields

Behavioural. WHEN lys-identity-server starts, THE SYSTEM SHALL install, before the service is built, one global tracing subscriber that is enabled only for events at WARN or ERROR. It says each such event once through the `say` sink every log line goes through, as `<level> <target>: <message> <name>=<value> …`, with the fields in the order the event gives them. Events below WARN SHALL be said nowhere and SHALL cost no formatting. Spans SHALL be accepted and ignored. A field's value SHALL be written as the event recorded it; no event in the server carries a secret, and none may start to.

**Acceptance:**
- A unit test in warn_lines.rs gives the subscriber a warn event with target `lys_identity_server::apps_client_credentials`, the field app="haematite" and a message, under tracing::subscriber::with_default with a capturing say. It finds exactly one line: `WARN lys_identity_server::apps_client_credentials: <message> app=haematite`.
- The same test gives it an info and a debug event and finds no further line.
- An integration test (tests/warn_lines.rs) calls warn_lines::install with a capturing say, the function main.rs calls, then raises tracing::warn! from a spawned thread, and finds its line through the global subscriber. A second install answers an error by name (`log_subscriber_installed`) and does not panic.
- main.rs calls warn_lines::install on its `say` before service_saying, and a refusal to install is printed and stops the server with a failure exit.
- The DIRECTORY-094 pins pass unchanged.

**Files:**
- create: crates/lys-identity-server/src/warn_lines.rs
- create: crates/lys-identity-server/tests/warn_lines.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/main.rs

**Checklist:**
- C733 — lys-identity-server installs one tracing subscriber at start that says every WARN and ERROR event, with its target and fields, through the say sink into identity.log; info and below are off (DIRECTORY-095 R1).

**Stories:**
- S417 (Operator, Installs and runs the standalone identity product) — As an operator, I want every warning the identity server raises written in its log with where it came from, so that a failure the service noticed is never silent.

## Boundaries

- No new dependency: the subscriber is written against the tracing crate the server already uses.
- The DIRECTORY-094 token refusal line is said exactly as it is today, through say directly, not through tracing.
- Info, debug and trace stay off; a setting to turn them on is not this brief's.

## Verification

- cargo clippy -p lys-identity-server --all-targets -- -D warnings; nextest of warn_lines (unit and integration) and the provider tests; after the next Lys install, identity.log is read for `WARN ` lines with a token and secret filter.
