# Certificate revocation: an appended leaf, a folded live set, a history that still verifies

The contract of DIRECTORY-013 (docs/design/directory/briefs/DIRECTORY-013.json). It builds DP26
(docs/design/lys-anchor/DECISIONS.md:400-425) for certificates under ADR-039. The code that
implements it lives in crates/lys-identity/src/revocation/, the consumer crate DIRECTORY-003
creates, so revocation stays a consumer-side non-goal of lys-core and no lys-core code and no
published wire format changes.

The formats below are drafts. They are signed only in tests until the repository's
constructed-attack review of this contract has its verdict on the card (section 13).

## 1. The premise

Revoking a certificate is one leaf appended to an append-only certificate log through
lys-log-store's `LeafStore` at its extent, and the live set is folded from the log.

Nothing deletes, rewrites, truncates, forks or merges a leaf. The `LeafStore` trait
(crates/lys-log-store/src/store.rs) is unchanged: it keeps its six methods, `origin`, `extent`,
`leaf`, `put_leaf`, `pinned` and `pin`, and `put_leaf` refuses any index but the extent. A
revocation can therefore only be a leaf written at the store's extent, through `Log::append`.

## 2. The unit

A certificate carries one capability claim, so the revocable unit and the claim unit are the
same unit, as DP26 rules. Revoking a claim is revoking its certificate and issuing a new one
without it.

No leaf revokes part of a certificate. A certificate carrying several claim extensions is
revoked whole and is never treated as partially revoked. There is no claim selector in any leaf.

## 3. The leaves of a certificate log

There is one certificate log per issuing authority. Every leaf begins with the ASCII domain tag
`lys-identity/certificate-log/v1` and one zero byte (32 bytes together), then one kind byte.

| Kind | Leaf | Body after the kind byte | Length |
|---|---|---|---|
| 0x01 | issuance leaf | the certificate's DER, which runs to the end of the leaf with no length prefix | 33 bytes plus the DER, at least one DER byte |
| 0x02 | revocation leaf | the 32-byte SHA-256 of the revoked certificate's DER, then a 64-byte Ed25519 signature (section 4) | exactly 129 bytes |
| 0x03 | attestation entry | the 32-byte SHA-256 of the certificate's DER, then the attestation's COSE bytes, which run to the end of the leaf with no length prefix | 65 bytes plus the COSE bytes, at least one COSE byte |

A certificate is named by the SHA-256 of its DER, computed from the bytes supplied, never from a
serial or a name the issuer chose.

Malformed, and refused `certificate_leaf_malformed` naming the reason, with no partial leaf
returned: any other tag, a missing zero byte, any other kind byte, a revocation leaf of any other
length than 129 bytes, an issuance leaf with no DER byte, and an attestation entry with no COSE
byte after its hash.

## 4. The signed bytes of a revocation

The revocation signature is an Ed25519 signature over, in order and with no length prefix
anywhere:

1. the ASCII domain tag `lys-identity/certificate-revocation/v1`;
2. one zero byte;
3. the 32-byte SHA-256 of the log's origin, taken over the origin's UTF-8 bytes;
4. the 32-byte certificate hash.

Binding the origin's hash means a revocation signed for one log does not verify in another; the
distinct domain tag means these bytes are never the signed bytes of any other lys format.

## 5. The signer

The rule is that only the key of the authority that issued the certificate signs its revocation.

A responsible person (ADR-003) or an administrator revokes by asking the directory, which asks
the authority to append. That request path is a later unit, so until it lands the only way to
revoke is a caller that holds the issuing authority identity. The append layer signs with the
issuing authority identity it is given and does not decide who asked.

The fold refuses a revocation leaf whose signature does not verify under the issuing
authority's key, whatever other key signed it, as `revocation_not_signed_by_issuer`, and that
leaf revokes nothing.

## 6. Permanence

Permanence means no leaf reinstates a revoked certificate. An issuance leaf naming a certificate the fold already
holds revoked, a verbatim replay of its earlier issuance leaf included, is a reinstatement leaf:
it is refused by name as `certificate_reinstatement_refused`, and the certificate stays revoked.
The way back is a new certificate.

This refuses the last-wins hazard documented at `crates/lys-core/src/delegation/artifact.rs:222-241` (DIRECTORY-013 cites the block as artifact.rs:370-373, its position before the file was reorganised):
where a fold lets the later leaf win, a verbatim copy of a public leaf appended after its
revocation makes the revoked state current again with no key at all. This fold never lets a
later leaf win over a revocation, so a replay is refused rather than repeated.

## 7. A revocation always names an issued certificate

The rule is that a revocation always names a certificate the log already holds. A revocation leaf whose
certificate has no earlier valid issuance leaf is refused as `revocation_before_issuance` and
revokes nothing. A later issuance leaf of that certificate is a valid issuance and not a
reinstatement.

## 8. The fold

The fold is one pass over every leaf from index 0 to the store's extent, in order, reading only
through `LeafStore::extent`, `LeafStore::leaf` and `LeafStore::origin`. A fold writes nothing.

An issuance leaf is valid when its DER verifies under the issuing authority's public key with
lys-core's `verify_certificate_chain_at`, unchanged, at the certificate's own notBefore instant
read from that DER, so that the check is of the issuer's signature and reads no clock.
lys-core's `verify_certificate_chain` reads the wall clock and is not used. The validity window
at the caller's instant is the verification call's check (section 9), not the fold's.

The log carries no instant and the fold never invents one. A certificate used before its window
begins is refused at the moment of use by the claim verifier, as `claim_not_yet_valid` on the
typed-claim card (hhAN8h77), and not by the fold.

A certificate whose subject common name equals the issuing authority's lowercase hex public key
fails `verify_certificate_chain_at`'s self-signed screen. That is a known false positive,
recorded by DIRECTORY-013 at `crates/lys-core/src/ca/authority.rs:335-341` and living, since lys-core's files were split, in `crates/lys-core/src/ca/authority/verify.rs` (the rustdoc of the screen names the false positive), so its issuance leaf records
`certificate_chain_invalid`. The typed-claim card's issuance refuses such a subject by name, and
the false positive is answered by its own lys-core card, not by this contract; the fold does not
work around it.

An issuance leaf that is not valid is recorded as `certificate_chain_invalid` at its index,
issues nothing, and does not count as an issuance for `revocation_before_issuance`, for a
reinstatement or for `certificate_not_in_log`.

A second valid revocation of a certificate already revoked, and a second issuance leaf of a live
certificate, change nothing and are not refusals. A revoked certificate stays in the answer as
revoked with its leaf index and is never dropped from it (ADR-008).

The fold answers:

- the folded size, the number of leaves it walked;
- the issued set: each certificate hash with the index of its first valid issuance leaf;
- the revoked set: each certificate hash with the index of its first valid revocation leaf;
- every refused leaf with its index and refusal name.

A leaf the fold cannot read is recorded as `fold_unreadable_leaf` at its index. It blocks every
permit and never revokes or issues anything. A leaf the store fails to read is not a leaf the
fold could not decode: it is `store_read_failed` naming the index, and the fold answers no
partial set.

## 9. N and the tolerance

The verification call takes the size N the caller has evidence of and a tolerance counted in
entries. There is no default for either, and no default instant: every one is a required input
and nothing supplies one on the caller's behalf.

Every answer, permit and refusal alike, carries the size the fold walked. A refusal stands on
the fold's own authority whatever its size: a certificate the fold holds revoked is refused
however short the folded log is. A permit is refused as `fold_stale` when the folded size is
below N by more than the tolerance. The fold may refuse on its own authority and never permits
beyond what it has walked.

The call answers, in this order: `certificate_chain_invalid` when the certificate does not
verify under the issuing authority's key at the caller's instant, before any fold answer;
`certificate_revoked` naming the revocation leaf's index; `fold_unreadable_leaf`; `fold_stale`;
`certificate_not_in_log`; and otherwise live, carrying the folded size and N.

What the fold does NOT claim: a log shown truncated has a tip too, and the fold
detects truncation only against the caller's N (docs/design/lys-anchor/KEY-HISTORY-FOLD-QUESTIONS.md:100-140).
A caller that supplies an N no larger than the truncated log's size is shown a consistent,
shorter log and the fold cannot tell. Suppression is caught by the caller's evidence of the
size, never by the fold alone.

## 10. History

The log's inclusion and consistency proofs and a certificate's issuance record always verify
after its revocation, with lys-core's `verify_inclusion_raw` and `verify_consistency`, unchanged.
Nothing removes, rewrites or hides the issuance leaf or any leaf before the revocation.

An attestation by a revoked certificate's key verifies if and only if its own attestation entry,
naming the certificate's hash with exactly those COSE bytes, precedes the revocation leaf. The
log order is the evidence of time, so an attestation made after a compromise cannot be placed
before the revocation. An attestation with no entry before the revocation leaf, an entry after
it included, fails as `attestation_after_revocation` and the refusal names the revocation leaf.
A fold holding a leaf it could not read answers `fold_unreadable_leaf` before any verified
answer, because the unread leaf might be an earlier revocation.

An attestation by a certificate that is not revoked needs no log entry of its own: the check
verifies its signature under the certificate's subject key (`attestation_signature_invalid`
when it does not) and that no revocation leaf in the log covers the certificate, and answers not
revoked, subject to the same `fold_unreadable_leaf` and `fold_stale` refusals as section 9.

## 11. The no-log forms

`lys ca verify` and lys-core's `verify_certificate_chain` and `verify_certificate_chain_at` keep
their meaning, keep passing with no log and do not check revocation. A revoked certificate whose
chain and validity window verify still passes them. `lys ca verify`'s help and lys-identity's
documentation say so.

lys-core's published rustdoc of `verify_certificate_chain` does not say so, because a rustdoc
edit is a lys-core source change. That is recorded here as a known gap: a library consumer
reading lys-core's published documentation does not see that verification without a log does
not check revocation. Its answer is a documentation-only lys-core release, its own card, not
done by this contract.

The form that takes the log is a library call in lys-identity. A command line for it is a
separate unpublished binary on which the published lys binary does not depend.

## 12. The refusals

| Refusal | When it fires | The act that answers it |
|---|---|---|
| certificate_leaf_malformed | bytes carry a wrong domain tag, no zero byte, an unknown kind byte, a revocation leaf not exactly 129 bytes long, an issuance leaf with no DER byte, or an attestation entry with no COSE byte after its hash | re-encode the leaf from section 3; nothing partial is returned |
| revocation_not_signed_by_issuer | a revocation leaf's signature does not verify under the issuing authority's key over section 4's bytes for this log's origin | the issuing authority signs and appends a new revocation; the refused leaf revokes nothing |
| revocation_before_issuance | a revocation leaf names a certificate with no valid issuance leaf at a lower index | append the issuance first, then a new revocation; the refused leaf revokes nothing |
| certificate_reinstatement_refused | a valid issuance leaf names a certificate the fold already holds revoked, a verbatim replay included | issue a new certificate; the revoked one stays revoked |
| certificate_revoked | the verification call is asked about a certificate the fold holds revoked; it names the revocation leaf's index and the folded size | use a new certificate; the revocation is permanent |
| certificate_not_in_log | the verification call finds no valid issuance leaf for the certificate | append the certificate's issuance leaf, or verify against the log that issued it |
| fold_unreadable_leaf | a leaf in the walked log does not decode; it names the index and blocks every permit | investigate the log; no permit is given from a fold that holds it |
| fold_stale | the folded size is below the caller's N by more than the caller's tolerance; it names the folded size, N and the tolerance | fold a log at least as long as the caller's evidence, or supply the tolerance the caller accepts |
| attestation_after_revocation | an attestation by a revoked certificate's key has no entry naming it before the revocation leaf; it names the revocation leaf's index | none: the attestation cannot be placed before the revocation |
| certificate_chain_invalid | a certificate does not verify under the issuing authority's key with verify_certificate_chain_at, in the fold at its own notBefore or in the verification call at the caller's instant | issue a valid certificate from the issuing authority; an invalid issuance leaf issues nothing |
| attestation_signature_invalid | an attestation's COSE bytes do not verify under the certificate's subject key for the payload | supply the attestation that certificate's key made |
| store_read_failed | the store fails to read a leaf; it names the index and no partial set is answered | repair the store and fold again |

## 13. Where this stands

This contract stands beside DIRECTORY-006 R4 and does not discharge it. Certificate revocation
and grant revocation are separate folds, and no grant, grant revocation or permission decision is
built or changed here. As R4 never lets reinstatement resurrect a revoked grant, no leaf
reinstates a revoked certificate; as R4 never permits from a stale state, the calls here never
permit beyond the tolerance the caller gives.

DP26 is the proposal C25's review reads, and C25 stays open: this contract does not close that
review and does not edit C25.

The formats above, `lys-identity/certificate-log/v1` and
`lys-identity/certificate-revocation/v1`, need the repository's constructed-attack review before
any leaf is signed outside a test. No lys-core code, no lys-log-store code and no published tag
changes under this contract.
