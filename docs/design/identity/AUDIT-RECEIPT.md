# Audit receipt: the contract as it stands

This document carries the audit receipt every sign-in and every permission change leaves,
as DIRECTORY-007 R1 writes it (docs/design/directory/briefs/DIRECTORY-007.md). It records
rulings already made and cites where each was made; it makes no new decision, fixes no
encoding, tag or content type, and decides nothing the design records as OPEN for Tom. The
signed encoding, the tag and the content type stay with
docs/design/identity/IDENTITY-EVENTS.md under DIRECTORY-003's joint review (A3).

## 1. The contract and its one ruled addition

The receipt's shape is IDENTITY-001's shared contract, carried word for word from
docs/design/identity/briefs/IDENTITY-001.md:57:

> Audit receipts carry a version, stable operation ID, actor, affected identity, operation, payload commitment and resulting log coordinate/checkpoint. Secrets and whole context objects are excluded. Exact signed encoding is reviewed before use, not frozen by this draft.

One field is added to those seven under a lead's ruling: the test tag (W3, ADR-026), which
marks a receipt signed with a test key on a development install (section 7). No other field
is added to the receipt without a lead's ruling.

## 2. The signer rule

The service that performed the operation signs the receipt, never the party the receipt is
about: a claim must not be settable by the party being judged (P8, CN13). The service attests
the authenticated actor and their provenance; it never claims a person signed bytes with a
key they do not hold (docs/design/identity/briefs/IDENTITY-001.md:45).

A verifier is given the service's public key. It never takes a key carried in the receipt as
its authority (CN13).

## 3. The leaf rule

The signed event is the appended leaf, appended in the order the operations happened. The
receipt is returned beside it, carrying the log coordinate the append yielded and the payload
commitment, and is never itself appended (docs/design/identity/briefs/IDENTITY-001.md:49,
docs/design/directory/briefs/DIRECTORY-003.md:64-66, CN14).

## 4. Who emits the receipt for each operation

Each operation has exactly one emitting brief or row. DIRECTORY-007 defines the common
receipt and its verifier and replaces nothing: the emitters named here keep emitting inside
their own walls (A1).

| Operation | Emitter | Note |
|---|---|---|
| An identity moving between lifecycle states | DIRECTORY-003 R5 | activate, suspend, reinstate, retire (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:32-44) |
| A person or agent being registered, and a display-profile change | DIRECTORY-003 R1 | A12 |
| A provider being linked to a person and unlinked from one | DIRECTORY-004 ID001_LINK_AUDIT | through its outbox and DIRECTORY-003 R4's receiver (A12) |
| A grant being given or taken away | DIRECTORY-006 GRANT_AUDIT | the payload waits on the grant representation that is OPEN for Tom (docs/design/directory/DESIGN.md:58, A6); the common receipt commits only to the seven fields and treats the payload commitment as a commitment over whatever representation is settled |
| A sign-in the directory service records | DIRECTORY-007 R4 | after Rauthy authenticated the person (A4, A11) |
| A sign-in the directory service refuses | DIRECTORY-007 R4 | after Rauthy authenticated the person; section 5 (A4, A11) |
| An agent being provisioned | none: no row and no receipt of its own | provisioned is a view over grants and a handle, not a transition (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:26-28), so it shows as the agent's register transition and then each grant given to it (A5) |

This list names operations that must emit a receipt, not the only ones. IDENTITY-001's rule
makes one signed event both the change and its audit record
(docs/design/identity/briefs/IDENTITY-001.md:44, A12), so any further change the directory
commits leaves its receipt by the same rule.

## 5. The step-1 sign-in boundary

In step 1 the directory service refuses exactly one kind of authenticated sign-in: an
issuer-subject Rauthy authenticated that is bound to no identity the administrator
registered. That refusal leaves the refusal receipt.

A suspended or retired identity produces no state-based refusal receipt, because in step 1
state is recorded and not enforced (CN11, docs/design/directory/briefs/DIRECTORY-003.md:122).
Suspension semantics stay OPEN for Tom, and the step-2 refusals arrive with enforcement (A11).

A first sign-in in step 1 registers nobody: the only caller that may register a person is the
configured administrator (docs/design/directory/briefs/DIRECTORY-003.md:39). So a first
sign-in leaves a sign-in receipt, or the refusal receipt when its issuer-subject is bound to
no registered identity, and the administrator's register act leaves the register receipt.
Here main corrected A7: A7's one register event on first sign-in is ADR-011's proposal for
step 2's self-registration (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:34) and
applies only when a brief brings that (A8).

Rauthy's own refusals, a bad password or a failed provider callback, are out of scope.
Rauthy's native events are asynchronous and severity-filtered
(docs/design/identity/briefs/IDENTITY-001.md:44), and a signing key inside Rauthy is a fork
change under ADR-009 that no brief of this repository makes (A4).

## 6. Receipts are for changes only

A permission check that is answered and changes nothing is not a receipt and never enters
the signed receipt log. When step 2 brings enforcement, allows and refusals go to a separate
access record that step 2 defines: refusals always, allows decided with step 2. Nobody later
puts high-volume checks into the signed receipt log (W1, ADR-024, CN15).

## 7. Test receipts

A development install emits test receipts, signed with a test key and carrying the test tag,
inside IDENTITY-001's development isolation, and never performs the act
docs/design/lys-anchor/DECISIONS.md:335 reserves (W3, ADR-026, CN16). The verifier refuses a
test-tagged receipt by name unless it is given the test key explicitly, telling the caller to
supply the test key, and a real key never verifies a test receipt, so a test receipt never
passes as real. Harness tests stay as well.

## 8. The verifier's order and refusal classes

The verifier is the lys CLI's `lys log verify receipt`, offline, given only the log
directory, the receipt and the service's public key (W2, ADR-025). Its core is the library
crates/lys-receipt (DIRECTORY-007 R2); the subcommand is DIRECTORY-007 R3. It checks in this
order:

1. **Test tag.** A test-tagged receipt consults only the test key; if none was given it is
   refused, before any cryptography, as `test-tagged receipt: supply the test key` (the CLI
   adds `with --test-key`). An untagged receipt consults only the verifier key and never the
   test key.
2. **Signature** over the receipt's signed bytes, against the consulted key.
3. **Coordinate** lies within the log's extent.
4. **Commitment**: the named commitment hash of the leaf bytes at that coordinate equals the
   receipt's payload commitment.

Every failure of 2, 3 or 4 collapses to one refusal class for the receipt artifact,
`receipt verification failed: invalid receipt, signature, coordinate or leaf`, so tamper
classes are indistinguishable in the output, the discipline of
crates/lys/src/commands/log/verify.rs:7-14 (A9, CN17). The pre-crypto, actionable refusals
stay named on their own: malformed JSON or a missing field or a foreign version (malformed
receipt), a malformed key string, and a test-tagged receipt presented without the test key.

The verifier reads the log and never writes to it, takes no key a receipt carries as its
authority, and enumerates no operation vocabulary: the operation is read as a string of
IDENTITY-EVENTS.md's vocabulary (A12, CN13).

## 9. What IDENTITY-EVENTS.md must carry before the code rows start

These are inputs to DIRECTORY-003's joint review, not edits to that file; DIRECTORY-007 R2 to
R5 start only when docs/design/identity/IDENTITY-EVENTS.md is on main carrying them (A3,
ADR-025):

- the envelope version field the verifier is written against;
- the typed payloads;
- the named commitment hash: SHA-256 named explicitly, never confused with a BLAKE3 content
  address (docs/design/identity/briefs/IDENTITY-001.md:49);
- the test tag;
- the sign-in and sign-in-refusal operations in the vocabulary, with the affected identity
  empty and the issuer and subject in the typed payload for a refusal of an unbound subject;
- the recorded review.

## 10. The release rule

A lys release carrying the verifier is a separate, deliberate act after IDENTITY-EVENTS.md's
version is ratified and recorded, never before, because publishing a crate that exposes a
format freezes it (CLAUDE.md:78-80). Landing on main publishes nothing (A10, CN18).
