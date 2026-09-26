# Audit receipt: the contract as it stands

This document carries, in one place, the audit receipt every sign-in and
every permission change on the platform leaves: its fields, who signs it,
where the signed record sits, which brief emits it for which operation, and
what the verifier checks. It decides nothing. Every rule below is carried
from where it was already decided, and each is cited there; a change to any
of them is a change at its source, never an edit here. It fixes no encoding,
tag or content type: those are written in
docs/design/identity/IDENTITY-EVENTS.md under DIRECTORY-003's joint review
(A3), and this document lists in section 9 what that review is asked to
carry. Written by DIRECTORY-007 R1 (docs/design/directory/briefs/DIRECTORY-007.md).

## 1. The fields

The shared contract of IDENTITY-001, word for word
(docs/design/identity/briefs/IDENTITY-001.md:57):

> Audit receipts carry a version, stable operation ID, actor, affected identity, operation, payload commitment and resulting log coordinate/checkpoint. Secrets and whole context objects are excluded. Exact signed encoding is reviewed before use, not frozen by this draft.

One field is added to those seven under a lead's ruling: the **test tag**,
which marks a receipt emitted by a development install under a test key
(W3, ADR-026). No other field is added to the receipt without a lead's
ruling.

## 2. Who signs

The service that performed the operation signs the receipt, never the party
the receipt is about: a claim must not be settable by the party being
judged (P8, CN13). The service attests the authenticated human actor and
their provenance; it never claims a person signed bytes with a key they do
not hold (docs/design/identity/briefs/IDENTITY-001.md:45).

A verifier is given the service's public key. It never takes a key carried
in the receipt as its authority (CN13).

## 3. Where the record sits

The signed event is the appended leaf, appended in the order the operations
happened. The receipt is returned beside it, carrying the log coordinate
the append yielded and the payload commitment, and is never itself appended
(docs/design/identity/briefs/IDENTITY-001.md:49,
docs/design/directory/briefs/DIRECTORY-003.md:64-66, CN14).

## 4. Who emits it, per operation

One row per operation, each naming exactly one emitting brief or row
(A12):

| Operation | Emitter | Notes |
|---|---|---|
| An identity moving between lifecycle states (activate, suspend, reinstate, retire) | DIRECTORY-003 R5 | docs/design/directory/briefs/DIRECTORY-003.md:122 |
| A person or agent being registered, and a display-profile change | DIRECTORY-003 R1 | A12 |
| A provider being linked to a person, and unlinked from one | DIRECTORY-004 ID001_LINK_AUDIT | Through its outbox, received by DIRECTORY-003 R4's receiver (docs/design/directory/briefs/DIRECTORY-004.md:41, A12) |
| A grant being given or taken away | DIRECTORY-006 GRANT_AUDIT | Its payload waits on the grant representation that is OPEN for Tom (docs/design/directory/DESIGN.md:58, A6); the common receipt commits only to the seven fields and treats the payload commitment as a commitment over whatever representation is settled (docs/design/directory/briefs/DIRECTORY-006.md:100) |
| A sign-in the directory service records | DIRECTORY-007 R4 | After Rauthy authenticated the person (A4, A11) |
| A sign-in the directory service refuses | DIRECTORY-007 R4 | After Rauthy authenticated the person; see section 5 (A4, A11) |
| An agent being provisioned | None: no row and no receipt of its own | Provisioned is a view over grants and a handle, not a transition (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:26-28), so it shows as the agent's register transition and then each grant given to it (A5) |

This list names the operations that must emit a receipt, not the only ones.
IDENTITY-001's rule makes one signed event both the change and its audit
record (docs/design/identity/briefs/IDENTITY-001.md:44), so any later
operation that changes a directory record leaves one too (A12). The
emitting briefs keep emitting their receipts inside their own walls; this
document replaces none of them (A1).

## 5. The step-1 sign-in boundary

In step 1 the directory service refuses exactly one kind of authenticated
sign-in: an issuer-subject Rauthy authenticated that is bound to no
identity the administrator registered. That refusal leaves the refusal
receipt (A11).

A suspended or retired identity produces no state-based refusal receipt,
because in step 1 state is recorded and not enforced (CN11,
docs/design/directory/briefs/DIRECTORY-003.md:122). Suspension semantics
stay OPEN for Tom. The step-2 refusals arrive with enforcement (A11).

A first sign-in in step 1 registers nobody: in step 1 the only caller that
may register a person is the configured administrator
(docs/design/directory/briefs/DIRECTORY-003.md:39). So a first sign-in
leaves a sign-in receipt, or the refusal receipt when its issuer-subject is
bound to no registered identity, and the administrator's register act
leaves the register receipt. A7's one register event on first sign-in is
ADR-011's proposal for step 2's self-registration
(docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:34) and applies only
when a brief brings that; main corrected A7 this way for step 1, and A8
stands where it corrects A7.

Rauthy's own refusals, a bad password or a failed provider callback, are
out of scope. Rauthy's native events are asynchronous and severity-filtered
(docs/design/identity/briefs/IDENTITY-001.md:46), and a signing key inside
Rauthy is a fork change under ADR-009 that no brief of this repository
makes (A4).

## 6. Receipts are for changes only

A permission check that is answered and changes nothing is not a receipt
and never enters the signed receipt log. When step 2 brings enforcement,
allows and refusals go to a separate access record that step 2 defines:
refusals always, allows decided with step 2. This keeps high-volume checks
out of the signed receipt log (W1, ADR-024, CN15).

## 7. Test receipts

A development install emits test receipts, signed with a test key and
carrying the test tag, inside IDENTITY-001's development isolation
(docs/design/identity/briefs/IDENTITY-001.md:50). It never performs the act
docs/design/lys-anchor/DECISIONS.md:335 reserves, emitting a receipt
outside a test; that act, with publishing lys-anchor and generating a
production key (docs/design/lys-anchor/DECISIONS.md:333-335), stays Tom's.

The verifier refuses a test-tagged receipt by name unless it is given the
test key explicitly, and its refusal tells the caller to supply the test
key. A real key never verifies a test receipt, so a test receipt never
passes as real (W3, ADR-026, CN16).

## 8. What the verifier checks, in order

As DIRECTORY-007 R2 (the crate crates/lys-receipt) and R3 (the command
`lys log verify receipt`) build it (A9, CN17). The verifier runs offline,
given only the receipt, the log and the service's public key (W2,
ADR-025); it reads the log and never writes to it.

1. **Shape.** A receipt that is not JSON, lacks a field, or carries a
   version other than the one the verifier is written against is refused
   as malformed, naming what was malformed.
2. **Test tag.** A test-tagged receipt consults only the test key; with no
   test key given it is refused before any cryptography with
   `test-tagged receipt: supply the test key` (the CLI adds
   `with --test-key`). An untagged receipt consults only the verifier key
   and never the test key.
3. **Signature** over the receipt's signed bytes against the consulted
   key's Ed25519 public key.
4. **Coordinate**: it lies within the log's extent.
5. **Commitment**: the named commitment hash of the leaf bytes at that
   coordinate equals the receipt's payload commitment.

Refusal classes, keeping the published discipline of
crates/lys/src/commands/log/verify.rs:7-14:

| Class | Covers | Message |
|---|---|---|
| Malformed receipt | Step 1; the CLI also names a malformed key string on its own | Names the malformed part |
| Needs the test key | Step 2, a test-tagged receipt with no test key | `test-tagged receipt: supply the test key` |
| Verification failed | Every failure of steps 3, 4 and 5 | `receipt verification failed: invalid receipt, signature, coordinate or leaf` |

Every cryptographic or structural failure collapses to the one
verification-failed message, so tamper classes are indistinguishable from
the output. The pre-crypto, actionable refusals stay named on their own.
The verifier enumerates no operation vocabulary: it reads the operation as
a string of IDENTITY-EVENTS.md's vocabulary (A12). Its success report is
the log's origin, the leaf index, the operation, the operation ID, the
affected identity and whether the receipt is a test receipt.

## 9. What IDENTITY-EVENTS.md must carry before the code rows start

These are inputs to DIRECTORY-003's joint review of
docs/design/identity/IDENTITY-EVENTS.md, not edits to it (A3, ADR-025).
DIRECTORY-007 R2 to R5 start only when that file is on main carrying:

- the envelope version field the verifier is written against;
- the typed payloads;
- the named commitment hash: SHA-256 named explicitly, never confused with
  a BLAKE3 content address (docs/design/identity/briefs/IDENTITY-001.md:49);
- the test tag;
- the sign-in and sign-in-refusal operations in the vocabulary, with the
  affected identity empty and the issuer and subject in the typed payload
  for a refusal of an unbound subject;
- the recorded review.

## 10. Release

A lys release carrying the verifier is a separate, deliberate act after
IDENTITY-EVENTS.md's version is ratified and recorded, never before
(CLAUDE.md:78-80). Landing on main publishes nothing (A10, CN18).
