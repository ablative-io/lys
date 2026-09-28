---
type: brief
id: LYSCORE-006
cluster: lys-core
title: Sign and decompress once: the issuer certificate per authority and one key decompression per verify
---

# LYSCORE-006: Sign and decompress once: the issuer certificate per authority and one key decompression per verify

> **Cluster:** lys-core
> **Blocked by:** LYSCORE-002 and LYSCORE-005 landed on main (both edit ca/authority.rs and keys/identity.rs): the build starts only when `git log --oneline origin/main --grep=LYSCORE-005` and `--grep=LYSCORE-002` each name their last requirement's commit.
> **Design anchor:**
> - ADR-112 — Lys hot paths do each piece of work once: no replay, no whole-state read for a sliver, no clone to read, no blocking on an async worker — Work on a request, append or open path is done once and scales with what the caller touches, not with history: lookups by index instead of scans, a checkpoint or cursor instead of a replay, a filtered read instead of the whole set, borrowed data instead of a clone made to read, one fsync per batch instead of per entry, and blocking I/O and std mutexes kept off async workers. Each fix is proved by counting the work done in a test that fails before it, never by a clock.
> **Checklist:**
> - C95 — The issuer certificate is built once per authority (LYSCORE-006 R1).
> - C96 — One key decompression per verify (LYSCORE-006 R2).
> **Stories:**
> - S34 (Third-party verifier, Verifying lys artifacts) — As anyone verifying Lys signatures or issuing certificates at volume, I want each key decompressed and each issuer certificate built once, so that verification and issuance cost what they must and nothing more.

## Purpose

Every certificate the authority issues rebuilds and self-signs its own issuer certificate (an extra Ed25519 signature and DER encode per issue), and every Ed25519 verification in Lys decompresses the public key twice. Both sit under every certificate and every signature check.

## Task

Remove the repeated work named in each requirement, keeping every answer identical, each saving proved by a counting test.

## Requirements

### R1: The issuer certificate is built once per authority

Behavioural. crates/lys-core/src/ca/authority.rs, issue_certificate (about line 252) calls issuer_certificate, which builds and self-signs the issuer certificate on every issue. Build it when the authority is constructed or first used, hold it on the authority, and hand out the held one. The issued certificates are byte-identical to today's.

**Acceptance:**
- A test issues ten certificates from one authority and counts one issuer signature, not ten; each issued certificate's DER equals the one the current code gives for the same inputs.

**Files:**
- modify: crates/lys-core/src/ca/authority.rs

**Checklist:**
- C95 — The issuer certificate is built once per authority (LYSCORE-006 R1).

**Stories:**
- S34 (Third-party verifier, Verifying lys artifacts) — As anyone verifying Lys signatures or issuing certificates at volume, I want each key decompressed and each issuer certificate built once, so that verification and issuance cost what they must and nothing more.

### R2: One key decompression per verify

Behavioural. crates/lys-core/src/keys/identity.rs, Ed25519Identity::verify (about line 139) decompresses the public key in is_usable_ed25519_public_key and again for verify_strict. Decompress once and use the one point for both the usability check and verify_strict (or hold the decompressed VerifyingKey on the identity). Every verdict, including each refusal of a weak or unusable key, stays the same.

**Acceptance:**
- A test over the existing accepted and refused key vectors gives the same verdicts, and a counting seam shows one decompression per verify.

**Files:**
- modify: crates/lys-core/src/keys/identity.rs

**Checklist:**
- C96 — One key decompression per verify (LYSCORE-006 R2).

**Stories:**
- S34 (Third-party verifier, Verifying lys artifacts) — As anyone verifying Lys signatures or issuing certificates at volume, I want each key decompressed and each issuer certificate built once, so that verification and issuance cost what they must and nothing more.

## Boundaries

- SHALL NOT change any answer, stored byte, hash, signature or wire shape: every change is the same result with less work, and a test pins the result before and after.
- SHALL NOT add a timeout, deadline, sleep, poll interval, cache expiry by clock, #[allow], #[ignore] or any bypass.
- Every file stays under 500 lines of code and ast-grep stays at zero hits.
- A counting test proves each saving (calls, reads, clones, scans or round trips counted), never a wall-clock timing.
- Where a line number has moved, the function named is the target; the dev record names where it now is.

## Verification

- The full Lys gate and, where the screens change, the surface checks exit 0 at the card's head, measured by the card round.
- Each requirement's counting test is red on the base and green at the head.
