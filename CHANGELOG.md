# Changelog

All notable changes to `lys-core` and `lys` are recorded here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions
follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) with the 0.x
caveat that minor bumps may break.

**This file describes only what ships.** Design drafts, planned formats, and
work in progress are deliberately absent — a release record that mentions
unreleased things cannot be used to tell what a published version contains.

## [Unreleased]

### Added — `lys-log-store` (new crate)

- **Durable, append-only, write-once storage for a log**, so `lys-core` stays
  free of I/O. Three pieces: the `LeafStore` trait stating what a log needs from
  storage, `FileLeafStore` providing it from a directory of leaf files, and
  `Log<S>` maintaining the RFC 6962 tree and the open-time integrity check over
  any backend.

  **The trait exposes no `fork`, no `merge`, no `delete`, no `truncate` and no
  way to rewrite a stored leaf.** For an append-only transparency log these are
  not missing features: the single property such a log provides is that its
  operator cannot hold two histories and show each observer whichever suits, and
  two branches of one log are exactly two histories. A storage API offering
  `fork` hands that attack back as a supported operation.

  A write must land at the next free index or be refused, which makes index reuse
  and skipped positions *reported* errors — `LeafAlreadyWritten` and
  `LeafWouldLeaveGap` — rather than an overwrite or a silent hole. A reported gap
  is attributable to a specific refused write; a silent one is indistinguishable
  from an entry nobody ever offered.

  Writes are fsynced, **file and containing directory both**, before returning
  `Ok`. On POSIX a newly created or renamed file is not durable until its parent
  directory's entry is, so syncing only the file would make durable-on-return
  mean "probably".

  **The pin refuses a second root at a size it already holds**
  (`PinRootChanged`). Pinning is the one operation on a store permitted to
  replace stored data, so it is the one place equivocation can enter, and a
  monotonic size check does not stop it: a different root at an *equal* size does
  not move the size backwards. An append-only tree has exactly one root per size,
  so accepting two would record that the log's history is two different things —
  the property the rest of this crate exists to make unrepresentable. Re-pinning
  the identical `(tree_size, root)` remains permitted, because a no-op must not
  be an error, and that idempotent repeat is the door the check stands in.

### Added — `lys-core` (behind the off-by-default `unstable-anchor` feature)

- **`merkle::root_from_consistency_path`** — RFC 6962 §2.1.4.2, deriving the
  *newer* root rather than comparing two roots the caller already holds. A
  verifier supplying both roots can only confirm a pair it already knew; deriving
  is what lets a receipt tell it where the log got to, and is what RFC 9942's
  detached payload is for. The older root is a required argument and is checked
  against the path's own reconstruction, so a proof descending from a different
  history is refused rather than producing a root a signature would appear to
  endorse. Swept against `ct-merkle` over every size pair, and cross-checked
  against the RFC's *recursive* `SUBPROOF` transcribed in Go.

- **`receipt::ConsistencyReceipt`** with `sign_consistency_receipt`,
  `verify_consistency_receipt` and `verify_consistency_receipt_bytes` —
  `lys/consistency-receipt/v1`, RFC 9942 `vdp` type `-2`. Verification takes the
  older root as a **required** argument; there is deliberately no overload
  without it, because a verifier that accepted the anchor's word for both
  endpoints would be checking a claim the anchor could make about any pair of
  trees it liked.

  **The content type differs from the inclusion receipt's as a security
  property, not a naming convention.** Both kinds sign a bare 32-byte root with a
  detached payload, so a shared type would give them byte-identical signing
  preimages and an inclusion receipt could be re-labelled as a consistency one
  with its signature still valid. The proof type `-1` versus `-2` lives in the
  unprotected header and is free to rewrite, so it cannot be the discriminator.

  **Equal sizes are refused at issuance and at verification.** An equal-size
  proof is true but carries no derivation: the newer root would be the caller's
  own argument handed back, collapsing the signature check into "has this anchor
  ever signed this 32-byte value?" over an attacker-chosen value.

### Changed — `lys`

- The `lys log` commands now run on `lys-log-store`; the log directory layout,
  the write-once rules and the integrity routine moved there rather than being
  duplicated. **The on-disk layout is byte-identical** — the format marker,
  filename width and `state.json` shape are unchanged, so existing log
  directories open exactly as before.
- Interrupted appends are still repaired at open and still reported, but the
  library now *returns* the fact and the CLI prints it, instead of the library
  writing to stderr on its caller's behalf.

## [0.2.0] — 2026-07-30

### Added — `lys-core`

- **Certificate issuance over a key the holder already controls**, via PKCS#10
  certificate-signing requests: `ca::create_certificate_request`,
  `ca::verify_certificate_request` → `VerifiedRequest`, and
  `CertificateAuthority::issue_certificate_for_request`. The request's
  self-signature is the proof of possession, and it is verified before issuance.

  This is the difference between a certificate that binds a key its holder
  demonstrably controls and one that binds a key the authority generated and
  discarded. The attack it prevents targets the *authority*: without it, a
  requester can present someone else's public key under their own name, and every
  statement that person signs afterwards verifies against a certificate naming the
  requester. See the `ca::request` module docs.
- `merkle::root_from_inclusion_path` — reconstructs the root an RFC 6962 §2.1.1
  inclusion path implies, rather than checking a path against a root you already
  hold. Cross-checked against `ct-merkle` at every size from 1 to 33 and every
  leaf index, and against the RFC's own recursive definition in Go.
- `TrustError::ReceiptVerification` and `TrustError::BundleVerification` — one
  non-oracle failure value per artifact class, instead of reusing another class's.

### Added — `lys-core`, behind the off-by-default `unstable-anchor` feature

- `receipt` — the `lys/anchor-receipt/v1` tagged `COSE_Sign1` artifact (RFC 9942
  verifiable data structures), with a detached Merkle-root payload.
- `bundle` — the `lys/verification-bundle/v1` container and its verifier.

  **Both formats are drafts and exempt from semantic versioning.** They are
  feature-gated precisely so that publishing this release does not freeze them:
  three specification bugs have already been found in them by implementation, and
  no production anchor exists to issue under them. Nothing is lost by the default
  being off — until an anchor exists, nothing can emit a receipt or a bundle, so
  nothing needs to verify one.

### Added — `lys` CLI

- `lys ca request` and `lys ca issue --request` — the holder and authority sides
  of proof-of-possession issuance.
- `lys ca issue --validity <30m|12h|7d>` — validity windows finer than a day.
  `lys-core` always accepted a `Duration`; only the CLI could not ask for one.
  `--validity-days` is retained and unchanged.
- `lys verify --cert <pem> --issuer-public-key <hex> [--at <rfc3339>]` — verifies
  an attestation *and* the certificate vouching for its signer, requiring the
  attestation's signer to be the key the certificate certifies. Answers "did this
  named subject make this statement?" rather than leaving two hex strings to be
  compared by eye.
- `lys log status` — read-only log state.
- `lys key inspect --ssh` and `--allowed-signers` — OpenSSH public-key and
  `allowed_signers` lines, so an identity works with existing SSH tooling.
- A global `--json` flag, honoured by every subcommand.

### Changed

- **`TrustError` is now `#[non_exhaustive]`.** This is the breaking change in this
  release: an exhaustive `match` on it no longer compiles without a wildcard arm.
  It is deliberate and one-time — from here, every new variant is additive, so a
  new artifact class can have an honest error of its own without a semver break.
  The alternative was reusing an unrelated variant, which makes errors mean less
  with every addition.
- `repository` metadata now points at `github.com/ablative-io/lys`. (0.1.0's
  published metadata carries the pre-move URL permanently.)

### Fixed

- **`lys open` no longer leaves recovered plaintext world-readable.** The 0600
  mode was applied on creation, so writing to a path that already existed kept
  the existing, looser permissions. Now tightened unconditionally.
- The X25519 scalar derived from an Ed25519 seed is held in `Zeroizing`, so it is
  wiped rather than left in freed memory.

### Security

Both entries under **Fixed** are security-relevant and affect 0.1.0: the
plaintext-permissions bug is a local disclosure, and the unzeroized scalar is
key material outliving its use. Both are fixed here.

### Internal

- Go interop gates for both anchor formats: receipts byte-identical against
  `veraison/go-cose` across 152 tree shapes, and bundles verdict-identical across
  23 cases against an independent Go verifier (`sumdb/note` for signed notes,
  go-cose for signatures, RFC 6962's recursive structure for roots).
- The gate command set is now `--all-features`; see `CLAUDE.md`. Without it, 81
  tests compile out with the `unstable-anchor` feature.

## [0.1.0] — 2026-07-30

First release: the extraction and hardening of `meridian-trust` into a
standalone crate plus CLI.

### Added — `lys-core`

- `keys` — Ed25519 identities from raw 32-byte seeds, with `Zeroizing` seed
  buffers and redaction pinned by test rather than assumed.
- `attestation` — the `lys/attestation/v2` tagged `COSE_Sign1` artifact over a
  payload hash and timestamp, with a canonical-strict verifier that rejects
  non-canonical CBOR and unprotected-header smuggling that vanilla COSE accepts.
- `ca` — Ed25519-rooted X.509 issuance and verification, with capability claims
  carried verbatim as a non-critical extension under the IANA-assigned lys arc
  `1.3.6.1.4.1.66364`.
- `merkle` — RFC 6962 append-only trees, inclusion and consistency proofs.
- `checkpoint` — C2SP `tlog-checkpoint` bodies and signed notes, cross-checked
  against Go's `sumdb/note`.
- `tlog` — self-contained inclusion and consistency proof artifacts a third party
  can verify with nothing but the artifact and a verifier key.
- `seal` — sealed envelopes (X25519 + HKDF-SHA256 + AES-256-GCM) bound to a
  sender attestation.
- `error` — one `TrustError` for the crate, with causeless non-oracle variants for
  every verification surface.

### Added — `lys` CLI

`key generate|inspect`, `attest`, `verify`, `ca issue|verify`,
`log init|append|checkpoint|prove|verify`, `seal`, `open`, and the read-only
`inspect` viewers, each printing an `UNVERIFIED` banner naming the command that
does verify.

[0.2.0]: https://github.com/ablative-io/lys/releases/tag/v0.2.0
[0.1.0]: https://github.com/ablative-io/lys/releases/tag/v0.1.0
