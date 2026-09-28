# The audit receipt

This document writes down the audit receipt as it already stands, who emits one for each
operation, and what docs/design/identity/IDENTITY-EVENTS.md must carry before the receipt's
code is built. It is DIRECTORY-007 R1 (docs/design/directory/briefs/DIRECTORY-007.md). It decides
nothing new: every rule below is cited to where it was settled, and it fixes no encoding, tag or
content type. Those are IDENTITY-EVENTS.md's, under DIRECTORY-003's joint review (A3).

## 1. The contract

The shared contract of IDENTITY-001, word for word (docs/design/identity/briefs/IDENTITY-001.md:57):

> Audit receipts carry a version, stable operation ID, actor, affected identity, operation, payload commitment and resulting log coordinate/checkpoint. Secrets and whole context objects are excluded. Exact signed encoding is reviewed before use, not frozen by this draft.

One field is added to those seven under a lead's ruling: the test tag (W3), which marks a receipt
signed with a test key inside development isolation (section 7). No other field is added to the
receipt without a lead's ruling.

## 2. The signer

The service that performed the operation signs the receipt's event, never the party the receipt is
about (P8, CN13). A claim must not be settable by the party being judged, so a person's or an
agent's own key never signs an audit record about them. A verifier is given the service's public
key out of band and never takes a key carried in the receipt as its authority: a key the receipt
names can only be compared against the key the verifier was given, never trusted in its place.

## 3. The leaf

The signed event is the appended leaf, appended in the order the operations happened. The receipt
is returned beside it, carrying the log coordinate the append yielded and the payload commitment,
and is never itself appended (docs/design/identity/briefs/IDENTITY-001.md:49,
docs/design/directory/briefs/DIRECTORY-003.md:64-66, CN14). The coordinate cannot be inside the
leaf, because the leaf's bytes are fixed before the log gives it a place.

## 4. Who emits a receipt for each operation

Each row names exactly one emitting brief or row. The list names the operations that must emit a
receipt, not the only ones that may: IDENTITY-001's rule makes one signed event both the change and
its audit record (docs/design/identity/briefs/IDENTITY-001.md:44, A12), so every signed directory
change leaves one, whether or not it is listed here.

| Operation | Emitter | Note |
| --- | --- | --- |
| An identity moving between lifecycle states | DIRECTORY-003 R5 | Activate, suspend, reinstate, retire; the state is recorded, not enforced, in step 1. |
| A person or agent being registered, and a display-profile change | DIRECTORY-003 R1 | A12. |
| A provider being linked to a person, and unlinked from one | DIRECTORY-004 ID001_LINK_AUDIT | Through its outbox and DIRECTORY-003 R4's receiver (A12). |
| A grant being given or taken away | DIRECTORY-006 GRANT_AUDIT | Its payload waits on the grant representation that is OPEN for Tom (docs/design/directory/DESIGN.md:58, A6). The common receipt commits only to the seven fields and treats the payload commitment as a commitment over whatever representation is settled. |
| A sign-in the directory service records | DIRECTORY-007 R4 | After Rauthy authenticated the person (A4, A11). |
| A sign-in the directory service refuses | DIRECTORY-007 R4 | After Rauthy authenticated the person (A4, A11); section 5. |
| An agent being provisioned | None | No row and no receipt of its own: provisioned is a view over grants and a handle, not a transition (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:26-28), so it shows as the agent's register transition and then each grant given to it (A5). |

## 5. The step-1 sign-in boundary

In step 1 the directory service refuses exactly one kind of authenticated sign-in: an
issuer-subject Rauthy authenticated that is bound to no identity the administrator registered. That
refusal leaves the refusal receipt.

A suspended or retired identity produces no state-based refusal receipt in step 1, because state is
recorded and not enforced there (CN11, docs/design/directory/briefs/DIRECTORY-003.md:122).
Suspension semantics stay OPEN for Tom, and the step-2 refusals arrive with enforcement (A11).

A first sign-in in step 1 registers nobody: the only caller that may register a person is the
configured administrator (docs/design/directory/briefs/DIRECTORY-003.md:39). So a first sign-in
leaves a sign-in receipt, or the refusal receipt when its issuer-subject is bound to no registered
identity, and the administrator's register act leaves the register receipt. This is how
main corrected A7 (A8): A7's one register event on first sign-in is ADR-011's proposal for step 2's
self-registration (docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:34) and applies only when a
brief brings that.

Rauthy's own refusals, a bad password or a failed provider callback, are out of scope. Rauthy's
native events are asynchronous and severity-filtered (docs/design/identity/briefs/IDENTITY-001.md:44),
and a signing key inside Rauthy is a fork change under ADR-009 that no brief of this repository
makes (A4).

## 6. Changes only

A permission check that is answered and changes nothing is not a receipt and never enters the
signed receipt log (W1, CN15, ADR-024). When step 2 brings enforcement, allows and refusals go to a
separate access record that step 2 defines: refusals always, allows decided with step 2. This is
written down so that nobody later puts high-volume checks into the signed receipt log.

## 7. Test receipts

A development install emits test receipts, signed with a test key and carrying the test tag, inside
IDENTITY-001's development isolation, and never performs the act
docs/design/lys-anchor/DECISIONS.md:335 reserves (W3, CN16, ADR-026). The verifier refuses a
test-tagged receipt by name unless it is given the test key explicitly, and says to supply the test
key. A real key never verifies a test receipt, and a test key never verifies a receipt as real, so a
test receipt never passes as real. Harness tests stay as well.

## 8. The verifier's order and refusal classes

The verifier is offline and is given only the log and the service's public key (W2, ADR-025). It
keeps the published discipline of crates/lys/src/commands/log/verify.rs (A9, CN17), as DIRECTORY-007
R2 and R3 build it, checking in this order:

1. Where the receipt carries the test tag, only the test key is consulted; if none was given the
   receipt is refused by name before any cryptography, with `test-tagged receipt: supply the test key`
   (the CLI adds `with --test-key`). Where it carries no test tag, only the verifier key is consulted
   and never the test key.
2. The signature, against the consulted key's Ed25519 public key.
3. The coordinate lies within the log's extent.
4. The named commitment hash of the record at that coordinate equals the receipt's payload
   commitment.

Every failure of steps 2 to 4 collapses to one refusal class for the receipt artifact,
`receipt verification failed: invalid receipt, signature, coordinate or leaf`, so tamper classes are
indistinguishable in the output. The pre-crypto, actionable refusals stay named on their own:
malformed JSON (a malformed receipt, naming what was malformed), a malformed key string, and a
test-tagged receipt presented without the test key. The verifier enumerates no operation vocabulary:
it reads the operation as a string of IDENTITY-EVENTS.md's vocabulary (A12).

## 9. What IDENTITY-EVENTS.md must carry before R2 to R5 start

These are inputs to DIRECTORY-003's joint review, not edits by this brief (A3, ADR-025):

- the envelope version field the verifier is written against;
- the typed payloads;
- the named commitment hash: SHA-256, named explicitly, never confused with a BLAKE3 content
  address (docs/design/identity/briefs/IDENTITY-001.md:49);
- the test tag;
- the sign-in and sign-in-refusal operations in the vocabulary, with the affected identity empty and
  the issuer and subject in the typed payload for a refusal of an unbound subject;
- the recorded review.

## 10. Release

A lys release carrying the verifier is a separate, deliberate act after IDENTITY-EVENTS.md's version
is ratified and recorded, never before (A10, CN18, CLAUDE.md:78-80). Landing on main publishes
nothing.
