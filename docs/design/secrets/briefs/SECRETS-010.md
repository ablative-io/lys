---
type: brief
id: SECRETS-010
cluster: secrets
title: A resident agent renews its own handle by presenting it, within its grant
---

# SECRETS-010: A resident agent renews its own handle by presenting it, within its grant

> **Cluster:** secrets
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> **Checklist:**
> - C422 — A holder renews its live handle within its grant, and the old one ends in the same act (SECRETS-010 R1).
> **Stories:**
> - S169 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent that runs all day, I want to renew my handle by presenting it, within my grant, so that I keep working without a person issuing again and without ever seeing the credential.

## Purpose

A resident service such as Tom's work watcher holds a handle that ends after its uses or its minutes (`lys-secrets issue`, defaults 10 and 60, cli.rs 86-89), and nothing lets it renew. The broker can derive (lineage.rs 231), but no route serves it, and a derived handle cannot outlive the one above it. SECRETS-008's issue route is for a screen service acting for a person, not for a holder itself. Today a person must issue again by hand each time (Archie, 7 October, 9ab721d4).

## Task

Add the renewal route (R1). Out of scope: changing what a grant allows; SECRETS-008.

## Requirements

### R1: A holder renews its own live handle by presenting it

Behavioural. POST /_lys/handles/renew admits a holder by its live handle's token and a presentation signed by that handle's key over this request, as a proxied call is admitted. It takes the uses and the end the holder asks for. The broker issues one successor handle on the same secret and the same holder key. Its end is refused as LeaseBeyondGrant when past the window of the grant the holder uses the secret under, and its spend cap is the old handle's. The old handle ends in the same act. The issue, the end and the audit record are one durable write. A handle that is ended, dropped or revoked cannot be renewed, and neither can one whose grant is gone; each is refused by name. The new token is answered once.

**Acceptance:**
- Renew, then a call on the old token is refused as ended and one on the new token passes.
- Asking past the grant's window is refused LeaseBeyondGrant, and the old handle still works.
- A crash between issue and end leaves either both as before or the successor live and the old ended.

**Files:**
- create: crates/lys-secrets/src/bin/lys-secrets/renew.rs
- create: crates/lys-secrets/src/bin/lys-secrets/renew_tests.rs
- modify: crates/lys-secrets/src/bin/lys-secrets/router.rs
- modify: crates/lys-secrets/src/broker/handles.rs
- modify: crates/lys-secrets/src/broker/handles_tests.rs

**Checklist:**
- C422 — A holder renews its live handle within its grant, and the old one ends in the same act (SECRETS-010 R1).

**Stories:**
- S169 (AI Agent, Uses a handle for its outbound calls and reads its sealed records) — As an agent that runs all day, I want to renew my handle by presenting it, within my grant, so that I keep working without a person issuing again and without ever seeing the credential.

## Boundaries

- SHALL NOT let a renewal outlive the holder's grant or change its secret or key.
- SHALL NOT renew an ended, dropped or revoked handle.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], unsafe code or any bypass.

## Verification

- The design gate, parsed: every cluster clean, no FAIL, every file valid.
- The full Lys gate on Dean, read from its parsed output: fmt, clippy pedantic in both configurations, ast-grep, nextest and cargo test --doc, 0 failed.
- Each requirement's red at main quoted in the handback.
