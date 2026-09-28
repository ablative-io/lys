# Certificate revocation: the contract

This document is the contract for revoking a lys certificate. It builds DP26
(`docs/design/lys-anchor/DECISIONS.md:400-425`), ruled and not built, for certificates. The
code it describes lives in `crates/lys-identity/src/revocation/`, the consumer crate, so
revocation stays a consumer-side non-goal of lys-core (ADR-039). Nothing here changes
lys-core, lys-log-store or any published wire format tag.

## 1. The premise

Revoking a certificate is one leaf appended to an append-only certificate log through
lys-log-store's `LeafStore`, written at the store's extent, and the live set of certificates
is folded from that log. Nothing deletes, rewrites, truncates, forks or merges a leaf. The
`LeafStore` trait is unchanged: its six methods (`origin`, `extent`, `leaf`, `put_leaf`,
`pinned`, `pin`) and its contract stay as written in `crates/lys-log-store/src/store.rs`, and
`put_leaf` still refuses any index but the extent. A revocation can therefore only be a leaf
written at the extent.

## 2. The unit

A certificate carries one capability claim, so the revocable unit and the claim unit are the
same unit, as DP26 rules. Revoking a claim is revoking its certificate and issuing a new one
without it. No leaf revokes part of a certificate. A certificate carrying several claim
extensions is revoked whole and is never treated as partially revoked.

## 3. The leaves of a certificate log

There is one certificate log per issuing authority. Every leaf begins with the ASCII domain
tag `lys-identity/certificate-log/v1` and one zero byte, then one kind byte, then the body:

| Kind | Leaf | Body after the kind byte | Length |
|---|---|---|---|
| 0x01 | issuance | the certificate's DER, running to the end of the leaf, no length prefix | at least one DER byte |
| 0x02 | revocation | the 32-byte SHA-256 of the revoked certificate's DER, then the 64-byte Ed25519 signature of section 4 | exactly 129 bytes |
| 0x03 | attestation entry | the 32-byte SHA-256 of the certificate's DER, then the attestation's COSE bytes, running to the end of the leaf, no length prefix | at least one COSE byte after the hash |

The revocation leaf's 129 bytes are the 31-byte tag, the zero byte, the kind byte, the 32-byte
hash and the 64-byte signature.

A leaf is malformed (`certificate_leaf_malformed`) when it carries any other tag or any other
kind, when it is a revocation leaf of any length but 129, when it is an issuance leaf with no
DER byte, or when it is an attestation entry with no COSE byte after its hash.

## 4. The signed bytes of a revocation

The Ed25519 signature in a revocation leaf is over these bytes, in order:

1. the ASCII domain tag `lys-identity/certificate-revocation/v1`;
2. one zero byte;
3. the 32-byte SHA-256 of the log's origin, taken over the origin's UTF-8 bytes;
4. the 32-byte SHA-256 of the revoked certificate's DER.

Binding the origin means a revocation signed for one log does not verify in another; binding
the certificate hash means it revokes that certificate and no other.

## 5. The signer

The rule is that only the key of the authority that issued the certificate signs its revocation. A person
responsible for the certificate under ADR-003, or an administrator, revokes by asking the
directory, which asks the issuing authority to append. That request path is a later unit, so
until it lands the only way to revoke is a caller that holds the issuing authority identity.
The fold refuses a revocation leaf whose signature does not verify under the issuing
authority's key (`revocation_not_signed_by_issuer`), whatever other key signed it.

## 6. Permanence

The rule is that no leaf reinstates a revoked certificate. An issuance leaf naming a certificate the fold
already holds revoked, a verbatim replay of its own issuance leaf included, is a reinstatement
leaf. It is refused by name (`certificate_reinstatement_refused`), and the certificate stays
revoked. The way back is a new certificate.

The last-wins hazard documented at `crates/lys-core/src/delegation/artifact.rs:222-241` (DIRECTORY-013
cites the block as artifact.rs:370-373, its position before the file was reorganised), where
a verbatim copy of an earlier leaf made a revoked key current again under last-wins by log
position, is named here and refused: the fold never lets a later issuance leaf undo an
earlier revocation.

## 7. A revocation names an issued certificate

The rule is that a revocation always names a certificate the log already holds. A revocation leaf whose
certificate has no earlier valid issuance leaf is refused as `revocation_before_issuance` and
revokes nothing. A later issuance leaf of that certificate is a valid issuance and not a
reinstatement, because nothing was revoked.

## 8. The fold

The fold is a pass over every leaf from index 0 to the store's extent.

An issuance leaf is valid when its DER verifies under the issuing authority's public key with
lys-core's `verify_certificate_chain_at`, unchanged, called at the certificate's own notBefore
instant read from that DER. The check is then of the issuer's signature, and it reads no
clock. lys-core's `verify_certificate_chain` reads the wall clock and is not used. The
validity window at the caller's instant is the verification call's check, not the fold's.

The log carries no instant and the fold never invents one. A certificate used before its
window begins is refused at the moment of use by the claim verifier, as `claim_not_yet_valid`
on the typed-claim card (hhAN8h77), and not by the fold.

A certificate whose subject common name equals the issuing authority's lowercase hex public
key fails `verify_certificate_chain_at`'s self-signed screen. That is a known false positive,
recorded by DIRECTORY-013 at `crates/lys-core/src/ca/authority.rs:335-341` and living, since
lys-core's files were split, in `crates/lys-core/src/ca/authority/verify.rs` (the screen at lines
78-84, its rustdoc naming the false positive), so its issuance leaf records
`certificate_chain_invalid`. The typed-claim card's issuance refuses such a subject by name,
and the false positive is answered by its own lys-core card, not by this contract.

An issuance leaf that is not valid is recorded as `certificate_chain_invalid` at its index and
issues nothing. It does not count as an issuance for `revocation_before_issuance`, for a
reinstatement or for `certificate_not_in_log`.

The fold answers:

- the folded size, which is the extent it walked;
- the issued set;
- the revoked set, with the index of each revocation leaf;
- every refused leaf, with its index and its refusal name.

A leaf the fold cannot read (`fold_unreadable_leaf`) blocks every permit, and never revokes
or issues anything. A store that fails to answer (`store_read_failed`) likewise answers no
permit.

## 9. N and the tolerance

The verification call takes the size N the caller has evidence of and a tolerance counted in
entries. There is no default for either, no default instant, and no default anywhere in the
call. Every answer carries the size the fold walked.

- A refusal stands on the fold's own authority, whatever the fold's size: a certificate the
  fold holds revoked is refused even from a short log.
- A permit is refused (`fold_stale`) when the folded size is below N by more than the
  tolerance.

What the fold does NOT claim: a log shown truncated has a tip too, and a truncated log folds
cleanly. The fold detects truncation only against the caller's N. A caller that passes an N
it has no evidence for has no protection against a truncated log.

## 10. History

A revoked certificate keeps its history. The log's inclusion and consistency proofs, and the
certificate's issuance record, always verify after its revocation.

An attestation by a revoked certificate's key verifies if and only if its own attestation
entry precedes the revocation leaf. The log order is the evidence of time, so an attestation
made after a compromise cannot be placed before the revocation. An attestation with no entry
before the revocation leaf fails as `attestation_after_revocation`, and the refusal names the
revocation leaf's index.

An attestation by a certificate that is not revoked needs no log entry of its own: the check
verifies its signature and that no revocation leaf in the log covers the certificate, and
answers not revoked. An attestation whose signature does not verify under the certificate's
key fails as `attestation_signature_invalid`, revoked or not.

## 11. The no-log forms

`lys ca verify`, and lys-core's `verify_certificate_chain` and `verify_certificate_chain_at`,
keep their meaning. They keep passing with no log, and they do not check revocation. `lys ca
verify`'s help and lys-identity's documentation say so.

lys-core's published rustdoc of `verify_certificate_chain` does not say so. That is recorded
here as a known gap whose answer is a documentation-only lys-core release, on its own card,
and not done by this contract.

The form that takes the log is a library call in lys-identity. A command line for it is a
separate unpublished binary, on which the published lys binary does not depend.

## 12. The refusals

| Refusal | When it fires | The act that answers it |
|---|---|---|
| certificate_leaf_malformed | A leaf has another tag or kind, a revocation leaf is not 129 bytes, an issuance leaf has no DER byte, or an attestation entry has no COSE byte after its hash. | Find the writer that produced the leaf and fix it; the leaf stays in the log, refused at its index. |
| revocation_not_signed_by_issuer | A revocation leaf's signature does not verify under the issuing authority's key over the signed bytes of section 4. | Revoke through the issuing authority; a revocation signed by any other key revokes nothing. |
| revocation_before_issuance | A revocation leaf names a certificate with no earlier valid issuance leaf in the log. | Append the certificate's issuance leaf, then append the revocation after it. |
| certificate_reinstatement_refused | An issuance leaf names a certificate the fold already holds revoked, a verbatim replay included. | Issue a new certificate; a revoked certificate stays revoked. |
| certificate_revoked | Verification is asked of a certificate the fold holds revoked. | Use a different certificate; request a new one from the issuing authority. |
| certificate_not_in_log | Verification is asked of a certificate with no valid issuance leaf in the log. | Append the certificate's issuance leaf through the issuing authority, or verify against the log that issued it. |
| fold_unreadable_leaf | The fold meets a leaf it cannot read; every permit is blocked. | Repair or replace the store so every leaf up to the extent reads, then fold again. |
| fold_stale | The folded size is below the caller's N by more than the tolerance. | Fold a log that has caught up to N, or pass the N the caller truly has evidence of. |
| attestation_after_revocation | An attestation by a revoked certificate's key has no attestation entry before the revocation leaf; the refusal names that leaf. | Treat the attestation as made after the revocation; it is not evidence. |
| certificate_chain_invalid | An issuance leaf's DER does not verify under the issuing authority's key at its own notBefore, the self-signed screen's false positive included. | Reissue the certificate through the issuing authority with a subject the screen accepts. |
| attestation_signature_invalid | An attestation's signature does not verify under the certificate's key. | Discard the attestation; it was not made by that key. |
| store_read_failed | The `LeafStore` fails to answer the extent or a leaf. | Restore the store's availability and fold again; no permit is given meanwhile. |

## 13. Where this stands

This contract stands beside DIRECTORY-006 R4, the revocation of grants, and does not
discharge it. Certificate revocation and grant revocation are separate folds. DP26 is the
proposal that C25's review reads, and C25 stays open.

The leaf layouts, the signed bytes of a revocation, the domain tags and the refusal table
above are a new wire format, frozen once a durable leaf is signed under them. They need the
repository's constructed-attack review before any leaf is signed outside a test: a forged
revocation, a revocation signed by a key other than the issuing authority's, a replay of an
issuance leaf after a revocation, cross-log and cross-issuer confusion, a revocation placed
before its certificate's issuance, and a leaf the fold cannot read.
