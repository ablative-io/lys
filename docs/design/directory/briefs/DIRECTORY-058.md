---
type: brief
id: DIRECTORY-058
cluster: directory
title: A receipt's checkpoint is signed by the service, so inclusion in it proves the act is in the log
---

# DIRECTORY-058: A receipt's checkpoint is signed by the service, so inclusion in it proves the act is in the log

> **Cluster:** directory
> **Checklist:**
> - C403 — The receipts route answers its checkpoint as a note signed by the service key, signed once per append, with its origin read from configuration (DIRECTORY-058 R1).
> - C404 — One function verifies a receipt answer against a pinned key with a named refusal for each failure; a forged tree around a genuine event is refused (DIRECTORY-058 R2).
> **Stories:**
> - S162 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a reader of a receipt, I want the checkpoint it is proved against to be signed by the service, so that a proof of inclusion tells me the act is in the service's log and not in a tree somebody built around it.

## Purpose

GET /receipts/{index} answers a receipt, the current checkpoint and an inclusion proof of the leaf in it. The checkpoint is two unsigned fields, tree_size and root. Anyone who can answer in the service's place can build a tree of their own around one genuine signed event, give its root as the checkpoint and a true proof against it, and the reader cannot tell. The reader today can trust the event's signature and nothing about the log. The sign-in provider's link-audit sender reads this route to acknowledge its observations, and states this gap in its own tests.

## Task

Answer the checkpoint as a signed note under the service key, in the form lys-core already signs and verifies (checkpoint::note, CheckpointBody), beside the two fields that are there now. Give readers one function that verifies the note, the inclusion proof and the receipt together.

## Requirements

### R1: The receipts route answers a signed checkpoint

Behavioural. The answer of GET /receipts/{index} gains checkpoint.note: the C2SP signed note of the checkpoint body (origin, tree size, root) signed with the service key by lys_core::checkpoint::sign_note. The origin is the deployment's issuer address, read from the service's configuration, never a default. tree_size and root stay in the answer and equal the note's body. The note is signed when the checkpoint is made, not on each request: the log's head moves only on an append, so one signature serves every read until the next append. GET /checkpoint answers the same signed note alone.

**Acceptance:**
- The note verifies under the service's public key with lys_core::checkpoint::verify_checkpoint, and its body equals the tree_size and root beside it.
- Two reads with no append between them answer byte for byte the same note; a read after an append answers a note of the larger tree.
- A service whose configuration names no issuer is refused at start by name, not at the first read.

**Files:**
- create: crates/lys-identity-server/src/checkpoint_api.rs
- create: crates/lys-identity-server/tests/receipts_signed.rs
- modify: crates/lys-identity-server/src/receipts_api.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C403 — The receipts route answers its checkpoint as a note signed by the service key, signed once per append, with its origin read from configuration (DIRECTORY-058 R1).

**Stories:**
- S162 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a reader of a receipt, I want the checkpoint it is proved against to be signed by the service, so that a proof of inclusion tells me the act is in the service's log and not in a tree somebody built around it.

### R2: One function verifies a receipt against a pinned key

Behavioural. lys-identity gains verify_receipt_answer(answer, service_key, origin): it verifies the note under the pinned key and origin, the event's signature under the same key, that the receipt's fields equal the signed event's, and the inclusion proof of the event's leaf against the note's root at the note's tree size. Each failure is its own named refusal: checkpoint_unsigned, checkpoint_signature, checkpoint_origin, checkpoint_behind_receipt (the note's tree is smaller than the receipt's index needs), event_signature, receipt_mismatch, inclusion_proof. A forged tree around a genuine event, with a true proof against its own root, is refused checkpoint_unsigned or checkpoint_signature.

**Acceptance:**
- One test per named refusal produces it from a well-formed answer changed in that one respect.
- The forged tree around a genuine event is refused, and the same answer with the true signed note is accepted.
- An answer whose note is of a later, larger tree than the receipt's own is accepted when the proof is against the note's root.

**Files:**
- create: crates/lys-identity/src/receipt_answer.rs
- create: crates/lys-identity/src/receipt_answer_tests.rs
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C404 — One function verifies a receipt answer against a pinned key with a named refusal for each failure; a forged tree around a genuine event is refused (DIRECTORY-058 R2).

**Stories:**
- S162 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a reader of a receipt, I want the checkpoint it is proved against to be signed by the service, so that a proof of inclusion tells me the act is in the service's log and not in a tree somebody built around it.

## Boundaries

- SHALL NOT remove or rename tree_size and root in the answer; readers that use them keep working.
- SHALL NOT sign on each request or hold the service key anywhere but where it is held today.
- SHALL NOT take the origin or any other field from a machine default.
- SHALL NOT print or log a key value on any path.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass; a wait ends on an event.
- SHALL NOT add a silent fallback: every failure is a named refusal.

## Verification

- The full Lys gate and ast-grep scan exit 0 at the card's head, measured by the card round.
- On a scratch install: read a receipt, verify it with verify_receipt_answer under the key read from /service-key, then present the forged tree and see it refused.
