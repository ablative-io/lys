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
> - C497 — Every grant change's receipt is read back from GET /grant-receipts/{index} with the grant log's signed head and an inclusion proof, and one function verifies it against a pinned key (DIRECTORY-058 R3).
> - C498 — Every sign-in Lys admits and every sign-in it refuses, for a registered identity or an unregistered issuer-subject claim, is a change in the directory log with a receipt, carrying no credential (DIRECTORY-058 R4).
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

Behavioural. The lys-identity crate gains verify_receipt_answer(answer, service_key, origin), where answer is a ReceiptAnswer struct of the receipt, the signed message, the signed note and the inclusion proof. The caller builds it from the route's JSON, since lys-identity has no serde dependency, and Receipt is read through its accessors (receipt.rs lines 48 to 78). The note is read in lys-core, whose parse_note (crates/lys-core/src/checkpoint/note.rs line 218) is private and refuses an empty signature block (line 232). The file note.rs gains a public note_origin that does not call parse_note. It splits the note at its last blank line as parse_note does, answers the origin, the first line of the body, and counts the signature lines, answering zero for an empty block. The function verify_receipt_answer first reads note_origin. A note with no signature line is refused checkpoint_unsigned. A body whose origin is not the pinned origin is refused checkpoint_origin. A note note_origin cannot split is refused checkpoint_signature. It then verifies the note under the pinned key, whose name is the origin, with verify_checkpoint (note.rs line 203, through verify_note at line 170), which answers every fault alike, and so a fault here is refused checkpoint_signature. The checks of verify_receipt (crates/lys-identity/src/receipt.rs lines 93 to 140) today all answer ReceiptInvalid. They are split so that each has its own refusal. The message's own check (verify_event, line 100) keeps its present refusals, SignatureInvalid (signer.rs line 154), EventMalformed, SignerMismatch and EventNotCanonical, passed through unchanged, as error.rs lines 4 to 6 require. The version check (line 102) answers receipt_version. The event and payload checks (lines 107 and 112) answer receipt_mismatch. The leaf hash check (line 117) answers leaf_hash_mismatch. The index check against the tree size (line 123) answers checkpoint_behind_receipt. The proof check (line 135) answers inclusion_proof. The function verify_receipt keeps its signature. Each refusal is a variant of IdentityError in crates/lys-identity/src/error.rs. ReceiptInvalid stays for its other use, a missing receipt in crates/lys-identity/src/link_audit.rs line 40. A forged tree around a genuine event, with a true proof against its own root, is refused checkpoint_unsigned or checkpoint_signature.

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

### R3: Every grant change answers a receipt the grant log proves, read back by route

Behavioural. A grant change's receipt is answered today only in the answer to the write that made it (GrantReceipt, crates/lys-identity/src/grants/receipt.rs line 21, answered at crates/lys-identity-server/src/grants/handlers.rs line 133), no route reads it back, and verify_grant_receipt (receipt.rs) takes an unsigned tree size and root, the gap R1 closes for the directory log. The grant events are leaves of their own log, appended at crates/lys-identity/src/grants/recovery.rs line 161 by GrantLedger over the same Ledger the directory uses (crates/lys-identity/src/restart.rs). GrantLedger gains origin(), answered by the Ledger::origin R1 adds, from its own store. Grants, which holds the service key today (crates/lys-identity/src/grants/authority.rs line 105), makes a signed head of the grant log with lys_core::checkpoint::sign_note under a note key whose name is the grant log's origin, at open for the head it opens at, at each committed append, and at each reconcile that adopts leaves, and at no read. It holds the signed head as one value with its tree size and root, answers it by signed_head(), and counts the heads it has made by heads_made(), as R1 does for the directory. A sign that fails after open marks the grants broken as a refused leaf does, and every later grant call is refused LogUnavailable until a restart. GET /grant-receipts/{index} answers the receipt of the grant leaf at index, the signed event, the signed head with tree_size and root beside it, and an inclusion proof against that head, from GrantLedger's entry() and inclusion_proof() (recovery.rs). The route admits a caller exactly as GET /grants/{id} admits a reader of the grant the leaf changed (crates/lys-identity-server/src/grants.rs line 163), and refuses any other caller by the same name, so it shows no grant the caller could not already read. The lys-identity crate gains verify_grant_receipt_answer(answer, service_key, origin), which reads the note with R2's note_origin and verify_checkpoint and answers R2's refusals for the note and the proof (checkpoint_unsigned, checkpoint_origin, checkpoint_signature, checkpoint_behind_receipt, inclusion_proof), and verify_grant_receipt's own checks for the receipt, each with its own named refusal. The function verify_grant_receipt keeps its signature. The grant log's origin is its store's, never a machine default and never the directory log's.

**Acceptance:**
- A delegation's receipt read from GET /grant-receipts/{index} has the same fields as the delegation's own answer.
- A revocation's receipt read from GET /grant-receipts/{index} has the same fields as the revocation's own answer.
- The answer's note verifies under the service's public key with lys_core::checkpoint::verify_checkpoint.
- The note's key name is the grant log's origin.
- The note's body equals the tree_size and root beside it in the answer.
- After a grant change and before any request, the held signed head is of the grant log's new tree.
- Ten grant receipt reads after a grant change leave heads_made() where the change left it.
- A caller who cannot read the grant is refused by the name GET /grants/{id} answers it.
- The refused answer names nothing of the grant.
- The function verify_grant_receipt_answer accepts the true answer.
- The forged tree around a genuine grant event is refused checkpoint_unsigned or checkpoint_signature.
- A changed proof is refused inclusion_proof.
- A note of a tree smaller than the receipt's index needs is refused checkpoint_behind_receipt.
- A receipt that names another grant is refused by its own name.

**Files:**
- create: crates/lys-identity-server/src/grant_receipts_api.rs
- create: crates/lys-identity-server/tests/grant_receipts_signed.rs
- create: crates/lys-identity/src/grants/receipt_answer.rs
- create: crates/lys-identity/src/grants/receipt_answer_tests.rs
- modify: crates/lys-identity/src/grants/recovery.rs
- modify: crates/lys-identity/src/grants/authority.rs
- modify: crates/lys-identity/src/grants/receipt.rs
- modify: crates/lys-identity/src/grants/mod.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C497 — Every grant change's receipt is read back from GET /grant-receipts/{index} with the grant log's signed head and an inclusion proof, and one function verifies it against a pinned key (DIRECTORY-058 R3).

**Stories:**
- S162 (Verifier, Checks a recorded identity change without the operator's cooperation) — As a reader of a receipt, I want the checkpoint it is proved against to be signed by the service, so that a proof of inclusion tells me the act is in the service's log and not in a tree somebody built around it.

### R4: Every sign-in and every refused sign-in is a change in the directory log with a receipt

Behavioural. No sign-in is an act in any log today: the directory's changes (crates/lys-identity/src/event.rs, enum Change) are SetupPerson, RegisterPerson, RegisterAgent, ReportingRegistration, ReportsToChanged, ChangeProfile, BindLogin, Transition, LinkAudit and AgentCall. Change gains SignIn and SignInRefused, each naming the identity, the actor's provenance as the service attests it, and for SignInRefused the refusal's name, and the two join lys/identity-event/v1's change codes. Every session Lys begins starts at session_admission::begin (crates/lys-identity-server/src/session_admission.rs line 58), reached from sign_in_callback.rs line 24, sign_in_upstream.rs line 343, sign_in.rs line 500 and setup.rs line 405. WHEN begin admits a session for an identity, THE SYSTEM SHALL commit one SignIn change naming it, through the directory's one write path inside the same with_directory borrow in which bound_caller admitted it, before the session is begun, and the sign-in's answer SHALL carry the index of its leaf. WHEN begin refuses after the actor has resolved to an identity, at the disabled account (line 78), the account mismatch (line 83) or bound_caller's refusal such as IdentityNotActive, THE SYSTEM SHALL commit one SignInRefused change naming that identity and the refusal's name, whatever the name is. An actor bound to no identity is recorded as DIRECTORY-007 recorded it: begin admits such an actor today, because bound_caller passes NoPerson (session_admission.rs line 30), and that is how the administrator's first-run sign-in and any unregistered subject reach a session. WHEN begin admits a session for an actor bound to no identity, THE SYSTEM SHALL commit one SignIn change, and WHEN begin refuses one, one SignInRefused change with the refusal's name, each carrying the issuer and the subject as the issuer named them and marked unregistered. Such a change binds no identity, creates none, and changes no record of the projection; its leaf names a claim the issuer made, not a person the directory has registered, and the wire format, the projection and every answer that shows it say so by the unregistered mark. A wrong email or password is the issuer's refusal (sign_in.rs line 294), made before begin is reached, and it commits nothing here: the issuer judges and audits it in its own event log, where its brute-force limits also live (SignInThrottled, sign_in.rs line 293), and that log is where the audit of those attempts lives. The directory log records what the directory decided about an actor the issuer admitted, so an actor must pass the issuer before writing a single leaf, and no stranger can grow the signed log at will. THE SYSTEM SHALL count every sign-in the issuer refuses, SignInRefused and SignInThrottled each, from the server's start, and GET /connections SHALL answer the two counts on the sign-in service's row, where an administrator sees the sign-in service. The counts carry no email, address or password. If the commit fails, the sign-in is refused LogUnavailable and no session begins, so no session is ever answered without its record. A read and an answered permission question commit nothing. The changes are signed by the service key as every change is, so R1's signed head and R2's verifier cover them, and their time is the clock the server injects. No change, answer or log line carries a password, token, cookie or authorization code.

**Acceptance:**
- A person's password sign-in leaves exactly one new SignIn leaf naming that person.
- That leaf's receipt verifies with verify_receipt_answer under the service key.
- The sign-in's answer carries the index of that leaf.
- A sign-in through each of the callback, upstream, password and setup entry points leaves exactly one SignIn leaf.
- A suspended person's sign-in is refused by name and leaves exactly one SignInRefused leaf whose reason is that refusal's name.
- That suspended person's refused sign-in leaves no SignIn leaf.
- A disabled account's sign-in leaves exactly one SignInRefused leaf naming the person, with reason SignInRefused.
- A wrong password refused by the issuer leaves no leaf.
- Two wrong passwords and one throttled sign-in raise the sign-in service's row in GET /connections to two refused and one throttled.
- The counts answered carry no email, address or password the test used.
- A sign-in admitted for an actor bound to no identity leaves exactly one SignIn leaf marked unregistered, carrying the issuer and the subject the issuer named.
- A sign-in refused for an actor bound to no identity leaves exactly one SignInRefused leaf marked unregistered, carrying the issuer, the subject and the refusal's name.
- An unregistered leaf binds no identity, and the directory's record count and every record are unchanged by it.
- No answer, screen or log line shows an unregistered leaf as a registered person.
- Three reads and three answered permission questions after a sign-in leave the leaf count unchanged.
- A directory whose append fails refuses the sign-in LogUnavailable and begins no session.
- No SignIn or SignInRefused leaf's bytes contain the password, the token, the cookie or the authorization code the test used.

**Files:**
- create: crates/lys-identity/src/directory_sign_in.rs
- create: crates/lys-identity-server/tests/sign_in_receipts.rs
- modify: crates/lys-identity/src/event.rs
- modify: crates/lys-identity/src/encoding.rs
- modify: crates/lys-identity/src/projection.rs
- modify: crates/lys-identity/src/lib.rs
- modify: crates/lys-identity-server/src/session_admission.rs
- modify: docs/design/WIRE-FORMATS.md
- modify: crates/lys-identity-server/src/sign_in.rs
- modify: crates/lys-identity-server/src/connections_api.rs
- modify: surface/identity/src/features/connections/Connections.tsx

**Checklist:**
- C498 — Every sign-in Lys admits and every sign-in it refuses, for a registered identity or an unregistered issuer-subject claim, is a change in the directory log with a receipt, carrying no credential (DIRECTORY-058 R4).

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

## Amendments

### Amendment 1: R3 grant receipts read back and proved under a signed head; R4 every sign-in and refused sign-in is a change in the directory log

- **Date:** 2026-10-06
- **By:** Archie

DIRECTORY-007 is superseded (Waffles, 3b1e9e4c); what it held that must not be lost is that every sign-in and every permission change is an act in a log with a receipt. Neither 050 nor 058 named them: event.rs has no sign-in change, and a grant receipt is answered only in its write's answer with no route and no signed head (Archie, 6faa3c05). R3 and R4 carry them here, on R1's signed head and R2's verifier. Ruled by Waffles at 5e8e80b3.

### Amendment 2: R4 records a sign-in by an actor bound to no identity, marked unregistered, binding and creating nothing

- **Date:** 2026-10-06
- **By:** Archie

Waffles ruled at f1211552 that R4 takes DIRECTORY-007's way: the issuer and the subject as the issuer named them, marked unregistered, binding no identity, creating none, never a credential. begin admits an unbound actor today (session_admission.rs line 30), so the admitted case is recorded as well as the refused one. A wrong password stays the issuer's refusal, audited in the issuer's log and counted by Lys on the sign-in service's row, never a directory leaf (Waffles, e6a9d94a).
