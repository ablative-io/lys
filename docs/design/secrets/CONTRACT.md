# SECRETS-003 R2 — the secrets broker's contract

This is the contract the broker in crates/lys-secrets is built to (ADR-019). It states behaviour as prose a test can disagree with. Each clause starts a line with its id; each names, in the clause-to-leg table at the end, the counted leg of SECRETS-003 R3 to R8 that fires for it. The cancellation rule's outcomes become binding when the lead accepts this row (SECRETS-002 R8). No code row starts before the lead accepts this document after its adversarial review (docs/design/secrets/reports/SECRETS-003-adversarial-review.md), which is written by a reviewer who did not write this document.

It rests on the baseline recorded from source in docs/design/secrets/BASELINE.md. lys-core and every published lys format are unchanged: the presentation, the lease, the sealed-record binding and the audit line are broker-internal shapes.

Every refusal below is a named reason whose message names the act that answers it.

## Invariant

K1: A credential never passes through the agent. The broker holds every credential in its store and hands a seat a handle; the proxy swaps the handle for the credential on the broker's side of the call, so no credential byte reaches the seat's side of any exchange, and none reaches a log line, an error, a Debug or Display output or an audit line. Every credential byte, raw handle byte and key byte lives in the one redacting type of crates/lys-secrets/src/secret.rs, whose buffer is `Zeroizing` and whose Debug and Display print no byte of it. The one named exception is the seat's own login at spawn (SECRETS-002 R5): the broker answers the engine that starts a seat with that seat's login, which reaches the seat's process, and records which account's login went to which seat.

## Key custody

K2: The broker holds exactly two keys. The store key is an X25519 static secret: every store entry is sealed to its public half with lys/sealed-envelope/v1 through lys-core unchanged. The audit key is an Ed25519 signing key: it signs every audit line. A key's id is the SHA-256 fingerprint of that key's public half; a key id is never a key byte.

K3: Each key is kept in its own key file whose location the operator supplies when the broker starts. A key file lies outside the store directory and outside the log directory. The broker refuses to start with a key file inside either, by the name KeyFileMisplaced (act: move the key file outside the store and log directories and supply its new location at start). No key byte is written into the store directory or the log directory.

K4: The broker unlocks the store by reading the store key file at start and holding the key only in `Zeroizing` memory inside the redacting type. When no store key is supplied, the broker refuses to open the store by the name StoreKeyMissing (act: supply the store key file at start) and reads no entry.

K5: The store key is rotated by: generating a new store key; opening every entry with the old key and resealing it to the new one, which advances each entry's sequence (K20); appending exactly one audit line that records the rotation and names the old and the new key id and no key byte; and only then removing the old key file. Until that audit line is appended, the old key is current and the resealed entries are not read; from the moment it is appended, the new key is current and every entry opens under it. A store opened afterwards with the old key is refused by the name StoreKeyRetired (act: use the current key), and no entry is read under the old key.

## Handle

K6: A handle is generated as at least 32 bytes from the operating system's secure random source, because an unsalted SHA-256 of a short handle can be brute-forced. It is bound to exactly one identity when it is issued, and it is issued only under a grant that traces to a person; issuance under a grant that traces to no person is refused by the name NoPersonRoot (act: have a person grant the access).

K7: A handle's id is generated separately from the same secure random source and is never derived from the handle. The handle id is what every record and audit line names.

K8: The store keeps only the handle's SHA-256 digest, never the raw handle. A presented handle is hashed and compared digest to digest in constant time. A presented handle whose digest matches no issued handle is refused by the name HandleUnknown (act: present a handle the broker issued); one bound to another identity by HandleWrongIdentity (act: present the handle from the identity it was issued to); one that has been dropped by HandleDropped (act: ask the grant's owner for a new handle).

K9: Whoever presents a handle authenticates by signing a presentation: the domain string `lys-secrets/presentation/v1` followed by the handle id, the call's operation id and a timestamp, signed with the Ed25519 key registered for the identity the handle is bound to. The proxy verifies it with lys-core's verify_attestation_by_signer (crates/lys-core/src/attestation/sign.rs:211) against that registered key, and never with verify_attestation, which never compares the signer (crates/lys-core/src/attestation/mod.rs:51-63). Because the payload begins with the domain string and names the call's operation id, a signature the same key made over any other payload does not verify. A handle presented with no signature is refused as PresentationUnsigned (act: sign the presentation with the key registered for the handle's identity); a signature by any other key as PresentationWrongSigner (act: sign the presentation with the key registered for the handle's identity); a presentation whose operation id is not the call's as PresentationReplayed (act: sign a new presentation for this call's operation id); and a presentation whose timestamp is more than 30 seconds from the broker's clock as PresentationStale (act: sign the presentation again with the current time). The same operation id presented again within that limit is a retry and gets the retry outcome (K11), never a second use.

## Lease

K10: A lease is a broker record under the audit log (ADR-020), read only by the broker, counting uses, a time window and spend against one grant. A use is one atomic step: under the lease's single writer, the broker checks the remaining uses, the window and the scope, and appends the use record naming the operation id, in one append; the use counts from that append and not before. Two requests on the last use are serialised by that writer, so exactly one is admitted and the other is refused by the name LeaseExhausted (act: ask the grant's owner for more uses). A handle presented outside its scope is refused by the name OutsideScope (act: ask the grant's owner for a grant that covers this scope); one presented after its window by LeaseWindowClosed (act: ask the grant's owner for a new lease within the grant's window).

K11: A retry whose first attempt's result is uncertain is answered from the log: the same operation id under the same handle and the same identity returns the first outcome recorded for it and counts no second use. The same operation id under a different handle, or under a different identity, is refused by the name OperationIdReused (act: send a new operation id), and counts no use.

K12: A lease with a hard spend cap reserves before work: the reservation is appended only when the settled spend plus every open reservation plus the new reservation is at most the cap; otherwise it is refused by the name SpendCapReached (act: ask the grant's owner to raise the spend cap), and no work starts. Two concurrent sessions therefore cannot both reserve the same remaining allowance.

K13: A reservation is settled after the work by one append recording the spend that passed through the proxy and closing the reservation. A reservation open when the broker stops stays counted at its full amount until it is settled by the cancellation rule's outcome (O3). The broker reports as spend only what passed through the proxy.

K14: A lease's not_after never extends past the window of the lys-identity grant it counts against (DIRECTORY-006 R1), because lys/delegation/v1 carries no not_after (crates/lys-core/src/delegation/mod.rs:218-221). A lease asked for with a not_after later than that window is refused by the name LeaseBeyondGrant (act: set the lease's not_after within the grant's window).

## Cancellation rule

K15: A drop takes effect at the next check. A new call on a dropped handle is refused as HandleDropped at the presentation check, with no restart of the broker, and forwards nothing upstream.

K16: A call admitted before its handle is dropped is not stopped by the drop alone: it receives exactly one of the outcomes below, and one audit line records that outcome with the call's operation id. The same outcomes apply to every admitted act the broker performs on a handle: a proxied call, a use reserved and not yet settled, an OAuth refresh, a spawn answer, a sealed-record read and a next-account answer.

O1: Admitted and already forwarded upstream when the drop lands: the call finishes, its answer is delivered, its reservation is settled with the spend that passed through, and the audit line records `completed_after_drop`. Exercised by SEC3_PROXY_INFLIGHT.
O2: Admitted and not yet forwarded upstream when the drop lands: the call is cancelled at the forward boundary, nothing is sent upstream, its use and reservation are released, and the audit line records `cancelled_at_boundary`; the caller is answered HandleDropped (act: ask the grant's owner for a new handle). Exercised by SEC3_PROXY_INFLIGHT.
O3: Forwarded upstream when the broker stops before the call's outcome is recorded: on restart the broker appends `outcome_unknown` for it, keeps its use counted and its reservation counted at the full amount, and a retry of that operation id returns `outcome_unknown` and is never forwarded again (K11). Exercised by SEC3_PROXY_INFLIGHT and SEC3_PROXY_RECOVERY.

## SpiceDB relations

K17: A handle's use is permitted when SpiceDB answers a permit for the relation `use` between the handle's bound identity and the resource the handle reaches. Any answer that is not a permit is refused by the name PermissionDenied (act: ask the resource's owner for the permission), and nothing is forwarded.

K18: A sealed record's read is permitted when SpiceDB answers a permit for the relation `read` between the asking identity and the record; the record's owner holds it, and the owner grants it to other identities. An identity without it is refused by the name NoRelation (act: ask the record's owner for the relation); after it is removed, by RelationRemoved (act: ask the record's owner to restore the relation). A removal does not claim back what was already read, and the earlier read's audit line stays.

K19: A SpiceDB answer that may be older than a revocation is handled by the permission-freshness check that DIRECTORY-006 R4 lands (SEC_REVOKE_FRESHNESS). This contract does not choose between awaiting a fresh decision and refusing by name: the broker's behaviour is the one the landed DIRECTORY-006 R4 permission check has, and in either case no call reaches upstream on the stale permit.

## Sealed record binding

K20: lys/sealed-envelope/v1 seals with an empty AAD (crates/lys-core/src/seal/sealed_envelope.rs:182 and :254), and lys-core is unchanged, so an entry's associated data is carried as the authenticated prefix of the sealed plaintext: the entry id, the entry's name, its owning identity's id and its sequence, canonically encoded. The sequence is a monotonic integer the store advances on every write of that entry and records in the store's log. On opening, the recovered prefix is compared with the entry being read before any plaintext is returned. A recovered entry id, name or owner that differs is refused by the name EntryBindingMismatch (act: restore the store from its current log), which defeats ciphertext swapped between two entries. A recovered sequence older than the latest the store's log records for that entry is refused by the name EntryRolledBack, naming the entry (act: restore the store from its current log or re-write the entry), which defeats an older ciphertext of the same entry restored in place. Both return no plaintext.

K21: A key-class sealed record is used through the proxy and never read: a read of it is refused by the name KeyNotReadable (act: use the key through the proxy). A memory-class record is read in the smallest piece asked for. An unknown name, and another person's record asked for by its id, are both refused by the name NotFound (act: ask for a record by a name you can discover), so the answer does not reveal whether the record exists.

## Audit line

K22: An audit line carries these fields and no others: the line's kind (issue, use, drop, reservation, settlement, rotation, forward, refresh, spawn_login, sealed_read, next_account, revocation_transition or inflight_outcome); the broker's clock time; the handle id; the bound identity's id; the seat; the operation id; the real account's name, never its value; the lease id and its use count after the line; the reserved or settled spend; the outcome; and, for a rotation line only, the old and the new store key id. No field is a raw handle, a handle's digest, a credential or a key byte, because the transparency log can be read and replayed.

K23: Every audit line is signed by the broker's audit key named in K2, and is appended through lys-log-store. A replay of the log verifies each line's signature against the audit key's public half before it counts the line.

K24: The audit line's leaf schema is the field list of K22, canonically encoded with the fields in that order, an absent field encoded as absent. This shape is broker-internal: whether it becomes a published lys format a stranger verifies offline stays open estate-wide.

K25: The broker appends at these boundaries, each one append, so a process killed before or after any of them reopens to answers equal to a replay of the log: issue, use, drop, reservation, settlement and store-key rotation; a call's admission, its forwarding and its audit line; a refresh and its audit line; a spawn answer and its seat-to-account record; sealing, a read and a relation change; and a next-account ask and its audit line.

## Open points

Each point SECRETS-002 records open is listed here. Points (3), (4) and (9) are settled broker-internally by the clauses named and stay open estate-wide; the other ten stay open and no clause decides them.

(1) The delegation schema (ADR-003). Open; no clause decides it.
(2) Which lys/delegation/v1 subject and role pair a handle would use. Open; no clause decides it.
(3) Whose key signs an audit line. Settled broker-internally by K23 (the broker's audit key, K2); open estate-wide.
(4) The audit line's leaf schema. Settled broker-internally by K22 and K24; open estate-wide.
(5) The worker-side owner of the revolver change. Open; no clause decides it.
(6) Whether the revolver's answer is the login at spawn or a handle through the proxy. Open; no clause decides it.
(7) The proxy-handle login path. Open; no clause decides it.
(8) Whether revoking a login token at the provider fails the seat's next call. Open; no clause decides it.
(9) How a sealed record is bound to its name and owning identity. Settled broker-internally by K20; open estate-wide.
(10) The file that carries the lys revocation fold. Open; no clause decides it.
(11) The shape of a time window carried in a signed delegation, a new version alongside v1. Open; no clause decides it, and K14 takes the window from the lys-identity grant instead.
(12) The permission-freshness mechanism SEC_REVOKE_FRESHNESS proposes and DIRECTORY-006 R4 builds. Open; K19 defers to the landed check.
(13) The identity lifecycle-state policy pending in CONFORMANCE rows 3.2 and 3.4. Open; no clause decides it.

## Clause-to-leg table

| Clause | Counted legs that fire for it |
| --- | --- |
| K1 | SEC3_SECRET_SCOPE, SEC3_STORE_CANARY, SEC3_PROXY_CANARY, SEC3_OAUTH_CANARY, SEC3_SPAWN_CANARY, SEC3_SEALED_CANARY, SEC3_NEXT_CANARY |
| K2 | SEC3_STORE_KEY |
| K3 | SEC3_STORE_KEY |
| K4 | SEC3_STORE_KEY |
| K5 | SEC3_STORE_ROTATION |
| K6 | SEC3_STORE_REFUSALS, SEC3_STORE_CANARY |
| K7 | SEC3_STORE_CANARY |
| K8 | SEC3_STORE_REFUSALS, SEC3_PROXY_REFUSALS |
| K9 | SEC3_PROXY_REFUSALS, SEC3_OAUTH_REFUSALS, SEC3_SPAWN_REFUSALS, SEC3_SEALED_REFUSALS, SEC3_NEXT_REFUSALS |
| K10 | SEC3_STORE_RACE, SEC3_PROXY_RACE, SEC3_OAUTH_LEASE_RACE, SEC3_SPAWN_RACE, SEC3_SEALED_RACE, SEC3_NEXT_RACE |
| K11 | SEC3_STORE_RETRY |
| K12 | SEC3_STORE_REFUSALS, SEC3_STORE_RECOVERY |
| K13 | SEC3_STORE_RECOVERY, SEC3_STORE_INFLIGHT |
| K14 | SEC3_STORE_REFUSALS |
| K15 | SEC3_PROXY_REFUSALS, SEC3_STORE_REFUSALS |
| K16 | SEC3_STORE_INFLIGHT, SEC3_PROXY_INFLIGHT, SEC3_OAUTH_INFLIGHT, SEC3_SPAWN_INFLIGHT, SEC3_SEALED_INFLIGHT, SEC3_NEXT_INFLIGHT |
| K17 | SEC3_PROXY_REFUSALS |
| K18 | SEC3_SEALED_REFUSALS |
| K19 | SEC3_PROXY_REFUSALS |
| K20 | SEC3_STORE_REFUSALS, SEC3_SEALED_REFUSALS |
| K21 | SEC3_SEALED_REFUSALS |
| K22 | SEC3_STORE_CANARY, SEC3_PROXY_RECOVERY |
| K23 | SEC3_STORE_RECOVERY |
| K24 | SEC3_STORE_RECOVERY |
| K25 | SEC3_STORE_RECOVERY, SEC3_PROXY_RECOVERY, SEC3_OAUTH_RECOVERY, SEC3_SPAWN_RECOVERY, SEC3_SEALED_RECOVERY, SEC3_NEXT_RECOVERY |

K19's stale-answer behaviour is also exercised by the SECRETS-002 R7 criterion test R4 carries for SEC_REVOKE_FRESHNESS, whose expected behaviour is the landed DIRECTORY-006 R4 check's.
