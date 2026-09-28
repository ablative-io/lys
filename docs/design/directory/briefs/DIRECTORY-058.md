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
> - C403 — The receipts route answers its checkpoint as a note signed by the service key, signed at open, at each committed append and at each settle that adopts leaves, under the log's own origin (DIRECTORY-058 R1).
> - C404 — One function verifies a receipt answer against a pinned key with a named refusal for each failure; a forged tree around a genuine event is refused (DIRECTORY-058 R2).
> **Stories:**
> - S162 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a reader of a receipt, I want the checkpoint it is proved against to be signed by the service, so that a proof of inclusion tells me the act is in the service's log and not in a tree somebody built around it.

## Purpose

GET /receipts/{index} answers a receipt, the current checkpoint and an inclusion proof of the leaf in it. The checkpoint is two unsigned fields, tree_size and root. Anyone who can answer in the service's place can build a tree of their own around one genuine signed event, give its root as the checkpoint and a true proof against it, and the reader cannot tell. The reader today can trust the event's signature and nothing about the log. The sign-in provider's link-audit sender reads this route to acknowledge its observations, and states this gap in its own tests.

## Task

Answer the checkpoint as a signed note under the service key, in the form lys-core already signs and verifies (checkpoint::note, CheckpointBody), beside the two fields that are there now. Give readers one function that verifies the note, the inclusion proof and the receipt together.

## Requirements

### R1: The receipts route answers a signed checkpoint

Behavioural. The answer of GET /receipts/{index} gains checkpoint.note, the C2SP signed note of the log's signed head (origin, tree size, root) signed with the service key by lys_core::checkpoint::sign_note, under a note key whose name is the origin. The origin is the log's own origin, which the store holds from its creation (config.rs line 38, log_origin, passed at routes.rs line 190). The issuer at config.rs line 42 is the sign-in provider's address and is not used. Directory cannot reach the origin today, so EventLog (crates/lys-identity/src/log.rs line 90) gains origin(), answered by a new Ledger::origin (crates/lys-identity/src/restart.rs line 83) from its store, which already answers it (restart.rs line 181, the LeafStore trait at crates/lys-log-store/src/store.rs line 126). The key stays where it is held today, inside Directory (crates/lys-identity/src/directory.rs line 43). The signed head is made by one function of Directory, called at open for the head it opens at, at each committed append (directory.rs line 209), and inside settle whenever reconcile adopts leaves, after they are applied and before its snapshot (directory.rs line 131). The function settle runs for appends (directory.rs lines 222 to 224 and 240) and for reads (directory.rs lines 105 to 107, 136 to 138 and 142 to 144, reached by the receipt route at receipts_api.rs line 36). It always runs under a mutable borrow of Directory, which is the writer thread after DIRECTORY-043, so every sign stays on the one path that changes the directory. A settle that adopts nothing makes no signed head. So every head the directory answers is signed before any answer reads it, whether a write, a retry answered from retry() (directory.rs lines 241 to 242) or a read moved it. Directory holds the signed head and answers it by signed_head(), which returns the held note with its tree size and root as one value. The receipt route and tests/signed_head_settle.rs read it only through signed_head(). The function sign_note (lys-core note.rs lines 94 to 139) refuses only a note name or origin it cannot encode, and open signs under the same origin before it answers anything. If a sign fails after open all the same, the directory is marked broken exactly as a refused leaf marks it (directory.rs lines 122 to 128). An append that had committed still answers its receipt, and every later call is refused LogUnavailable, answered 503 by error_status.rs line 15, until a restart, whose open signs again. The signed head, the tree size and the root are one value in the directory's state, replaced together, so every read answers all three from one state. That holds after DIRECTORY-043 moves writes onto a writer thread, because the signed head is made on the write path and published with the head. Directory counts the signed heads it has made, and answers the count by heads_made(), so a test can see that a read makes none. The receipt route takes tree_size and root from the held signed head in place of log.head() (receipts_api.rs line 44). The signed head always equals the log's head once settle has run, so the inclusion proof is built against the current tree as today, and no proof at an older size is needed. The members tree_size and root stay in the answer and equal the note's body. GET /checkpoint answers the same signed note alone. The words signed head are used for it, because the ledger's periodic checkpoints are a different thing. DIRECTORY-050 R4 also edits receipts_api.rs, and this card builds after DIRECTORY-050 lands, so the card round reads every receipts_api.rs cite again at that head. The uncertain store of the tests lives in its own directory, crates/lys-identity/tests/uncertain_support/mod.rs, as revocation_support does, so tests/support/mod.rs is unchanged.

**Acceptance:**
- The note verifies under the service's public key with lys_core::checkpoint::verify_checkpoint.
- The note's body equals the tree_size and root beside it in the answer.
- The note's key name is the log's origin.
- After an append and before any request, the directory's held signed head is of the new tree.
- Ten reads after an append leave heads_made() where the append left it.
- A read after an append answers a note of the larger tree.
- A service opened over an existing log answers a note of the head it opened at.
- GET /checkpoint answers the same note as the receipt answer read at the same head.
- An append whose outcome was uncertain, resolved inside the append, leaves a signed head of the adopted tree, measured with the uncertain store of tests/uncertain_support/mod.rs.
- A settle reached from a read that adopts leaves raises heads_made() by exactly one.
- On a quiet directory, a receipt request whose read settle adopts the leaf answers a note of a tree that holds it, with no append and no restart.
- A client's retry of an append whose own settle could not read back, answered from retry(), leaves a signed head whose tree holds the retried leaf.

**Files:**
- create: crates/lys-identity-server/src/checkpoint_api.rs
- create: crates/lys-identity-server/tests/receipts_signed.rs
- create: crates/lys-identity/tests/uncertain_support/mod.rs
- create: crates/lys-identity/tests/signed_head_settle.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/receipts_api.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity/src/directory.rs
- modify: crates/lys-identity/src/log.rs
- modify: crates/lys-identity/src/restart.rs

**Checklist:**
- C403 — The receipts route answers its checkpoint as a note signed by the service key, signed at open, at each committed append and at each settle that adopts leaves, under the log's own origin (DIRECTORY-058 R1).

**Stories:**
- S162 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a reader of a receipt, I want the checkpoint it is proved against to be signed by the service, so that a proof of inclusion tells me the act is in the service's log and not in a tree somebody built around it.

### R2: One function verifies a receipt against a pinned key

Behavioural. The lys-identity crate gains verify_receipt_answer(answer, service_key, origin), where answer is a ReceiptAnswer struct of the receipt, the signed message, the signed note and the inclusion proof. The caller builds it from the route's JSON, since lys-identity has no serde dependency, and Receipt is read through its accessors (receipt.rs lines 48 to 78). The note is read in lys-core, whose parse_note (crates/lys-core/src/checkpoint/note.rs line 218) is private and refuses an empty signature block (line 232). The file note.rs gains a public note_origin that does not call parse_note. It splits the note at its last blank line as parse_note does, answers the origin, the first line of the body, and counts the signature lines, answering zero for an empty block. The function verify_receipt_answer first reads note_origin. A note with no signature line is refused checkpoint_unsigned. A body whose origin is not the pinned origin is refused checkpoint_origin. A note note_origin cannot split is refused checkpoint_signature. It then verifies the note under the pinned key, whose name is the origin, with verify_checkpoint (note.rs line 203, through verify_note at line 170), which answers every fault alike, and so a fault here is refused checkpoint_signature. The checks of verify_receipt (crates/lys-identity/src/receipt.rs lines 93 to 140) today all answer ReceiptInvalid. They are split so that each has its own refusal. The message's own check (verify_event, line 100) keeps its present refusals, SignatureInvalid (signer.rs line 154), EventMalformed, SignerMismatch, EventTooLarge and EventNotCanonical, passed through unchanged, as error.rs lines 4 to 6 require. The version check (line 102) answers receipt_version. The event and payload checks (lines 107 and 112) answer receipt_mismatch. The leaf hash check (line 117) answers leaf_hash_mismatch. The index check against the tree size (line 123) answers checkpoint_behind_receipt. The proof check (line 135) answers inclusion_proof. The function verify_receipt keeps its signature. Each refusal is a variant of IdentityError in crates/lys-identity/src/error.rs. ReceiptInvalid stays for its other use, a missing receipt in crates/lys-identity/src/link_audit.rs line 40. A forged tree around a genuine event, with a true proof against its own root, is refused checkpoint_unsigned or checkpoint_signature.

**Acceptance:**
- The function note_origin answers zero signature lines for a note whose signature block is empty.
- A well-formed answer whose note has no signature line is refused checkpoint_unsigned.
- A well-formed answer whose note names another origin is refused checkpoint_origin.
- A well-formed answer whose note is signed by another key is refused checkpoint_signature.
- A well-formed answer whose message signature is changed is refused SignatureInvalid.
- A well-formed answer whose receipt version is changed is refused receipt_version.
- A well-formed answer whose receipt names another event is refused receipt_mismatch.
- A well-formed answer whose leaf hash is changed is refused leaf_hash_mismatch.
- A well-formed answer whose note is of a tree smaller than the receipt's index needs is refused checkpoint_behind_receipt.
- A well-formed answer whose proof is changed is refused inclusion_proof.
- The forged tree around a genuine event is refused.
- The same answer with the true signed note is accepted.
- An answer whose note is of a later, larger tree than the receipt's own is accepted when the proof is against the note's root.
- The link audit's missing receipt still answers ReceiptInvalid.

**Files:**
- create: crates/lys-identity/src/receipt_answer.rs
- create: crates/lys-identity/src/receipt_answer_tests.rs
- modify: crates/lys-identity/src/error.rs
- modify: crates/lys-identity/src/lib.rs
- modify: crates/lys-identity/src/receipt.rs
- modify: crates/lys-core/src/checkpoint/note.rs
- modify: crates/lys-core/src/checkpoint/note_tests.rs

**Checklist:**
- C404 — One function verifies a receipt answer against a pinned key with a named refusal for each failure; a forged tree around a genuine event is refused (DIRECTORY-058 R2).

**Stories:**
- S162 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a reader of a receipt, I want the checkpoint it is proved against to be signed by the service, so that a proof of inclusion tells me the act is in the service's log and not in a tree somebody built around it.

## Boundaries

- SHALL NOT remove or rename tree_size and root in the answer, so readers that use them keep working.
- SHALL NOT sign on a read whose settle adopts nothing, or hold the service key anywhere but inside Directory, where it is held today.
- SHALL NOT take the origin or any other field from a machine default.
- SHALL NOT print or log a key value on any path.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore] or any bypass. A wait ends on an event.
- SHALL NOT add a silent fallback. Every failure is a named refusal.

## Verification

- The full Lys gate and ast-grep scan exit 0 at the card's head, measured by the card round.
- On a scratch install, read a receipt, verify it with verify_receipt_answer under the key read from /service-key, then present the forged tree and see it refused.
