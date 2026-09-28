---
type: brief
id: DIRECTORY-058
cluster: directory
title: A receipt's checkpoint is signed by the service, so inclusion in it proves the act is in the log
---

# DIRECTORY-058: A receipt's checkpoint is signed by the service, so inclusion in it proves the act is in the log

> **Cluster:** directory
> **Depends on:** DIRECTORY-050
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

Behavioural. The answer of GET /receipts/{index} gains checkpoint.note, the C2SP signed note of the log's signed head (origin, tree size, root) signed with the service key by lys_core::checkpoint::sign_note. The origin is the log's own origin, which the store holds from its creation (config.rs line 38, log_origin, passed at routes.rs line 190). The issuer at config.rs line 42 is the sign-in provider's address and is not used. The key stays where it is held today, inside Directory (crates/lys-identity/src/directory.rs line 43). Directory makes the signed head when it opens, for the head it opens at, and again on each append it commits (directory.rs line 209), and holds it. The signed head, the tree size and the root are one value in the directory's state, replaced together, so every read answers all three from one state. That holds after DIRECTORY-043 moves writes onto a writer thread, because the signed head is made on the write path and published with the head. The members tree_size and root stay in the answer and equal the note's body. GET /checkpoint answers the same signed note alone. The words signed head are used for it, because the ledger's periodic checkpoints are a different thing.

**Acceptance:**
- The note verifies under the service's public key with lys_core::checkpoint::verify_checkpoint, and its body equals the tree_size and root beside it.
- After an append, and before any request, the directory's held signed head is of the new tree, so it was made on the write path and not on a read.
- A read after an append answers a note of the larger tree, and a service opened over an existing log answers a note of the head it opened at.

**Files:**
- create: crates/lys-identity-server/src/checkpoint_api.rs
- create: crates/lys-identity-server/tests/receipts_signed.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/receipts_api.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity/src/directory.rs

**Checklist:**
- C403 — The receipts route answers its checkpoint as a note signed by the service key, signed once per append, with its origin read from configuration (DIRECTORY-058 R1).

**Stories:**
- S162 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a reader of a receipt, I want the checkpoint it is proved against to be signed by the service, so that a proof of inclusion tells me the act is in the service's log and not in a tree somebody built around it.

### R2: One function verifies a receipt against a pinned key

Behavioural. The lys-identity crate gains verify_receipt_answer(answer, service_key, origin), where answer is a ReceiptAnswer struct of the receipt, the signed message, the signed note and the inclusion proof. The caller builds it from the route's JSON, since lys-identity has no serde dependency, and Receipt is read through its accessors (receipt.rs lines 48 to 78). It reads the origin from the note's body first and refuses a different origin checkpoint_origin. It then verifies the note under the pinned key with verify_checkpoint, which answers an origin fault and a signature fault alike (lys-core note.rs lines 154 to 164), and so here refuses checkpoint_signature. A note that is not signed at all is checkpoint_unsigned. A note whose tree is smaller than the receipt's index needs is checkpoint_behind_receipt. The rest reuses verify_receipt (receipt.rs lines 93 to 140), which today folds every failure into ReceiptInvalid. It is split so that each failure has its own refusal, event_signature, receipt_mismatch and inclusion_proof, and verify_receipt keeps its signature. Each refusal is a variant of IdentityError in crates/lys-identity/src/error.rs. A forged tree around a genuine event, with a true proof against its own root, is refused checkpoint_unsigned or checkpoint_signature.

**Acceptance:**
- One test per named refusal produces it from a well-formed answer changed in that one respect.
- The forged tree around a genuine event is refused, and the same answer with the true signed note is accepted.
- An answer whose note is of a later, larger tree than the receipt's own is accepted when the proof is against the note's root.

**Files:**
- create: crates/lys-identity/src/receipt_answer.rs
- create: crates/lys-identity/src/receipt_answer_tests.rs
- modify: crates/lys-identity/src/error.rs
- modify: crates/lys-identity/src/lib.rs
- modify: crates/lys-identity/src/receipt.rs

**Checklist:**
- C404 — One function verifies a receipt answer against a pinned key with a named refusal for each failure; a forged tree around a genuine event is refused (DIRECTORY-058 R2).

**Stories:**
- S162 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a reader of a receipt, I want the checkpoint it is proved against to be signed by the service, so that a proof of inclusion tells me the act is in the service's log and not in a tree somebody built around it.

## Boundaries

- SHALL NOT remove or rename tree_size and root in the answer, so readers that use them keep working.
- SHALL NOT sign on a read, or hold the service key anywhere but inside Directory, where it is held today.
- SHALL NOT take the origin or any other field from a machine default.
- SHALL NOT print or log a key value on any path.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass. A wait ends on an event.
- SHALL NOT add a silent fallback. Every failure is a named refusal.

## Verification

- The full Lys gate and ast-grep scan exit 0 at the card's head, measured by the card round.
- On a scratch install, read a receipt, verify it with verify_receipt_answer under the key read from /service-key, then present the forged tree and see it refused.
