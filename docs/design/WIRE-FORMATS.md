# Wire formats — the frozen contracts and the proposals awaiting ratification

> **Why this document exists.** Once a signature is produced or a leaf is logged under a format, that format is frozen — changing it breaks every historical verification (see [CLAUDE.md](../../CLAUDE.md#wire-formats-are-forever)). Every wire contract therefore passes through an explicit gate: **proposed → ratified → frozen**. Nothing ships under a proposed format. Ratification is a deliberate human decision, recorded in the decision log at the bottom of this file.
>
> The governing rule applies with full force here: a stranger must be able to verify every lys artifact with standard, non-vendor tooling. Where the verification world has converged on a format, we adopt it byte-for-byte; we do not invent.

Evidence base: a July 2026 survey of RFC 6962, the C2SP specs (tlog-checkpoint, signed-note, tlog-tiles, static-ct-api), Tessera, Rekor v2 / the Sigstore bundle, RFC 9942 (COSE Receipts) + RFC 9943 (SCITT), and the Rust verifier ecosystem (ct-merkle, Cloudflare's `signed_note`/`tlog_tiles`, coset, sigstore-rs). Load-bearing citations inline below.

---

## 1. Frozen today

These are live contracts. Evolving any of them means a new `v2` artifact alongside, never a mutation.

| Contract | Format | Where defined |
|---|---|---|
| Attestation `lys/attestation/v2` (the at-0.1.0 contract) | Tagged COSE_Sign1 (RFC 9052): protected `{1: -8 (EdDSA), 3: "application/vnd.lys.attestation.v2+cbor", 4: raw 32-byte Ed25519 signer key}`, empty unprotected map, payload = deterministic CBOR map `{1: 32-byte SHA-256 payload hash, 2: unix-ms timestamp}`, Ed25519 over `Sig_structure` with empty external_aad; verifier is canonical-encoding-strict | `lys-core/src/attestation/` |
| Sealed envelope `lys/sealed-envelope/v1` | JSON (serde shape of `SealedEnvelope`); X25519 ephemeral + HKDF-SHA256 (info `lys-sealed-envelope/v1`) + AES-256-GCM | `lys-core/src/seal/` |
| Merkle leaf encoding — typed (`Serialize`) path | postcard serialization of the leaf type; leaf hash = RFC 6962 `SHA-256(0x00 ‖ postcard-bytes)` | `lys-core/src/merkle/` (`AppendOnlyTree<L: Serialize>`) |
| Merkle leaf encoding — raw path (transparency log; every `lys log` artifact) | leaf **file bytes verbatim** — no postcard framing, no length prefix; leaf hash = RFC 6962 `SHA-256(0x00 ‖ raw-file-bytes)`, reproducible as `(printf '\x00'; cat leaf-file) \| shasum -a 256` | `lys-core/src/merkle/` (`RawLeaf`, `raw_leaf_hash`, `verify_inclusion_raw`) |
| Certificates | X.509 (rcgen-issued, Ed25519), capability claims in an extension under `LYS_OID_ARC` | `lys-core/src/ca/` |

The two leaf encodings are separate frozen contracts that never mix within one tree (the `RawLeaf` marker type makes mixing unrepresentable). They diverge on the wire: for the leaf bytes `leaf-0`, the raw path hashes `SHA-256(0x00 ‖ "leaf-0")` while the postcard path hashes `SHA-256(0x00 ‖ 0x06 ‖ "leaf-0")` (postcard's length prefix) — sentinel tests in `merkle/tree_tests.rs` and `merkle/proof_tests.rs` pin the divergence. A third-party verifier of `lys log` artifacts MUST use the raw path.

Note: the earlier `lys/attestation/v1` JSON artifact never shipped — nothing durable was signed under it, and it was removed rather than frozen (decision D4). The COSE_Sign1 artifact above is the only attestation format; like every row here it freezes at 0.1.0.

---

## 2. IMPLEMENTED (D1) — signed tree root: C2SP checkpoint (signed note)

**The artifact `lys log` emits for "here is the state of the log, signed" is a [C2SP tlog-checkpoint](https://github.com/C2SP/C2SP/blob/main/tlog-checkpoint.md) wrapped in the [C2SP signed-note](https://github.com/C2SP/C2SP/blob/main/signed-note.md) envelope, signed with Ed25519.**

This is the single most convergent format in the entire transparency ecosystem: Tessera emits it, Rekor v2 emits it, static-CT/Sunlight emit it, the public witness network consumes it, and the Sigstore bundle embeds the note text verbatim. Go reference tooling (`sumdb/note`), Cloudflare's Rust crates (`signed_note`, `tlog_tiles`), and every sigstore verifier consume it today. It is Ed25519 + SHA-256 + RFC 6962 — exactly the lys primitive lineup.

### 2.1 Checkpoint body (exact)

Three newline-terminated lines, then optional extension lines:

```
<origin>
<tree size, ASCII decimal, no leading zeros>
<standard base64 (RFC 4648 §4, with padding) of the 32-byte RFC 6962 root hash>
```

- **Origin** is the log's unique identity. Operator-chosen, SHOULD be a schema-less URL (`example.com/lys/prod-01`). The CLI takes `--origin`; it is not defaulted, because two logs sharing an origin is a security defect.
- **There is no timestamp line, and we do not invent one.** The base checkpoint format has none; Rekor v2 outsources signed time to a separate timestamp authority. If a lys deployment needs signed time over a root, that is an attestation over the checkpoint bytes — a composition of two existing artifacts, not a format change.
- Extension lines: none in v1. Verifiers must tolerate their presence (per spec); we emit none.

### 2.2 Signed-note envelope (exact, byte-trap inventory)

The note is: body (ending `\n`), one blank line, then one signature line per signer:

```
— <keyname> <base64(4-byte key ID ‖ Ed25519 signature)>
```

Byte-exact rules that Go/Rust verifiers enforce silently — each one gets a test:

1. The dash is **U+2014 em dash**, not `--`.
2. The signature covers the body **including its trailing `\n`**, excluding the blank line and signature lines.
3. Ed25519 signature type byte is `0x01`; signature per RFC 8032 over the note text.
4. Key ID = first 4 bytes of `SHA-256(keyname ‖ 0x0A ‖ 0x01 ‖ pubkey)`.
5. Verifier key text form is `<name>+<hex keyid>+<base64(0x01 ‖ pubkey)>` — `lys key inspect` should learn to print this so any lys identity is usable as a note verifier key.

**Conformance obligation:** round-trip test vectors against the Go `sumdb/note` reference implementation, not merely against our own code or Rust peers. (Cloudflare's `signed_note`/`tlog_tiles` crates are young 0.2.0; the *format* is what's rock-solid.)

### 2.3 What we explicitly rejected

- **CT v1 STH (`TreeHeadSignature`)**: RFC 6962 §2.1.4 permits only ECDSA P-256 and RSA-PKCS1v1.5 — **there is no Ed25519 codepoint**, so an Ed25519 STH is nonstandard by construction. Legacy direction; Rekor v2 removed the API. Dead end.
- **Custom JSON `{root, size, sig}`**: verifiable by nobody but us. Violates the one rule.
- **Bare COSE_Sign1 over the root**: workable but precedent-free — the COSE-world artifact is the *receipt* (§4.1), not a standalone signed root.

---

## 3. IMPLEMENTED (D2) — inclusion & consistency proofs: self-contained JSON objects

**The artifacts `lys log prove` emits are JSON objects carrying the RFC 6962 proof triple plus the relevant checkpoint(s) embedded verbatim** — the Sigstore-bundle `InclusionProof` pattern, which is the surviving precedent for proofs persisted as files rather than served from an online API.

### 3.1 Inclusion proof

```json
{
  "format": "lys/log-inclusion-proof/v1",
  "tree_size": 1234,
  "leaf_index": 42,
  "hashes": ["<base64>", "<base64>", "..."],
  "checkpoint": "<the full signed-note text, verbatim, including trailing newline>"
}
```

### 3.2 Consistency proof

```json
{
  "format": "lys/log-consistency-proof/v1",
  "tree_size_1": 1000,
  "tree_size_2": 1234,
  "hashes": ["<base64>", "..."],
  "checkpoint_1": "<signed note for the OLD tree, verbatim>",
  "checkpoint_2": "<signed note for the NEW tree, verbatim>"
}
```

### 3.3 Rules

- Hashes: **standard base64 with padding** (matches CT v1 JSON and checkpoint line 3 — nobody in this ecosystem uses hex or base64url for these artifacts).
- Sizes/indices: JSON decimal numbers. lys refuses to emit proofs for trees at or beyond 2^53 leaves (JSON number precision boundary); stated here so the limit is a documented contract, not a surprise.
- The `format` field is a lys addition (self-describing files are worth one field); everything else mirrors the sigstore `InclusionProof` shape so the fields are recognizable to existing verifiers.
- **Redundancy is checked, not trusted:** `tree_size` MUST equal the embedded checkpoint's line-2 value (and root recomputed from the proof MUST equal the checkpoint's line-3 hash); `lys log verify` rejects mismatches. A proof without its checkpoint is unverifiable, which is why the checkpoint rides inside the artifact.
- The leaf itself is NOT in the artifact. The verifier holds the leaf bytes (the thing being proven), computes the RFC 6962 **raw-path** leaf hash `SHA-256(0x00 ‖ raw-file-bytes)` — the raw leaf-encoding row in §1: the file bytes verbatim, with **no postcard framing** — and runs the proof. This matches how every sigstore verifier works and keeps the proof privacy-neutral — a proof file alone reveals no log contents.
- Third-party verifiability today: any RFC 6962 verifier — Go `sumdb/tlog`, Rust `ct-merkle` / `tlog_tiles::check_record`/`check_tree` — after a trivial base64 decode; the shape is hand-checkable with a 20-line script. That is the bar.

### 3.4 Forward-compatibility with RFC 9942 (why this freeze is not a trap)

RFC 9942's registered verifiable-data-structure algorithm `RFC9162_SHA256 = 1` is the same RFC 6962 SHA-256 tree, and its proof CBOR is the same triple — `[tree-size, leaf-index, inclusion-path]` / `[size-1, size-2, consistency-path]`. The later COSE receipt (§4.1) is a **re-encoding of identical semantics, not a new proof system**. Freezing JSON now and the receipt later is two version-1 formats, not a v1→v2 mutation.

---

## 4. DIRECTION — the COSE boundary (ratify direction now; detailed specs follow separately)

### 4.1 Receipts: RFC 9942, at the anchor phase, additive

When `lys-anchor` lands, it issues **RFC 9942 COSE receipts** (COSE_Sign1; protected `vds = 1` i.e. `RFC9162_SHA256`; proofs in the unprotected `vdp` map, inclusion `-1` / consistency `-2`; detached payload = the Merkle root, recomputed by the verifier), slotting into RFC 9943 transparent statements. **Not emitted today**: the RFC is a month old, there is no interop corpus, no Rust implementation verifies the full receipt dance yet, and the COSE algorithm-identifier churn (below) is live. Adding receipts later alongside the JSON artifacts is purely additive (§3.4).

### 4.2 IMPLEMENTED (D4) — attestations: COSE_Sign1, the only attestation artifact

**The attestation artifact is a tagged COSE_Sign1 (RFC 9052), and it is the only attestation format — the JSON serde-shape artifact was deleted before anything durable was signed under it.** Rationale: "every signed artifact lys emits verifies with an off-the-shelf COSE library" is the strongest sentence we can put in front of a stranger, and COSE's `Sig_structure` is the standards-grade form of the domain separation v1 hand-rolled.

Byte-exact contract (every rule gets a test):

1. Outer form: CBOR tag 18 (`0xD2`) over `[protected: bstr, unprotected: {}, payload: bstr, signature: bstr(64)]`, all definite lengths. The tag is required; the unprotected map must be empty.
2. Protected headers, exactly three, canonical order: `1: -8` (EdDSA — the deployed-practice code point: go-cose v1.3.0 and pycose 1.1.0 have no `-19`; revisiting is a v3 matter), `3: "application/vnd.lys.attestation.v2+cbor"` (the v2 domain discriminator, signature-covered), `4:` the raw 32-byte Ed25519 signer key (signature-covered — stronger than v1, whose signer key rode outside the signed bytes).
3. Payload (embedded): deterministic CBOR map `{1: bstr(32) SHA-256 of the attested bytes, 2: int unix-millisecond timestamp (i64, pre-epoch representable)}` — both signature-covered. The artifact never carries the attested bytes; total size is 191–199 bytes.
4. Signature: Ed25519 (strict verification) over `Sig_structure = ["Signature1", protected, h'', payload]` per RFC 9052 §4.4; external_aad is always empty.
5. Deterministic encoding is RFC 8949 §4.2 core deterministic, and the verifier is **canonical-encoding-strict**: an artifact that is not byte-identical to the canonical re-encoding of its parsed fields is rejected even if its signature verifies — this closes unprotected-header smuggling and indefinite-length/reordering malleability, which vanilla COSE verifiers accept.
6. On-disk form: the raw COSE bytes (`.cose`, media type `application/cose`) — no wrapper, so the file verifies with any COSE library directly and drops verbatim into the raw-leaf log path.
7. Cross-form kill: the v1 preimage (`lys/attestation/v1 ‖ timestamp_le ‖ hash`) is rejected by construction (a `Sig_structure` begins `0x84 0x6A "Signature1"`; the four in-repo signing contexts are byte-0 disjoint) and by test.

**Conformance obligation:** round-trip vectors against the vendored Go `veraison/go-cose` reference (byte-identical signing both ways, mutual verification, shared negative controls, plus strictness-delta assertions where lys rejects valid-but-noncanonical artifacts that vanilla COSE accepts) — the D6 standard, gated by `LYS_REQUIRE_GO` in CI.

### 4.3 What deliberately does NOT move to COSE

- **Certificates stay X.509.** Already the interop standard; nothing to buy.
- **Sealed envelopes stay `lys/sealed-envelope/v1`.** Transport encryption between two parties — no third party ever verifies one, so the stranger-verifiability argument doesn't apply.

---

## 5. Decision log

| # | Decision | Status |
|---|---|---|
| D1 | Signed root artifact = C2SP checkpoint in signed-note envelope, Ed25519, no timestamp line, no extension lines in v1 (§2) | **IMPLEMENTED — 2026-07-11, built to spec under the operator's build green-light; formal ratification pending, veto window open until 0.1.0 publishes.** Substance carried in `lys-core/src/checkpoint/` module docs |
| D2 | Proof artifacts = self-contained JSON (`lys/log-inclusion-proof/v1`, `lys/log-consistency-proof/v1`) with embedded verbatim checkpoint(s), standard base64, 2^53 guard (§3) | **IMPLEMENTED — 2026-07-11, built to spec under the operator's build green-light; formal ratification pending, veto window open until 0.1.0 publishes.** Substance carried in `lys-core/src/tlog/` module docs |
| D3 | RFC 9942 COSE receipts deferred to `lys-anchor`, added as a parallel v1 artifact, never replacing D2 (§4.1) | **PROPOSED — awaiting ratification** |
| D4 | Attestation artifact = tagged COSE_Sign1 (`lys/attestation/v2`), alg = EdDSA(-8) after the deployed-practice check, protected content-type + kid domain binding, deterministic CBOR, canonical-strict verifier; the v1 JSON artifact deleted unshipped (§4.2) | **IMPLEMENTED — 2026-07-11, built to spec under the operator's build green-light; formal ratification pending, veto window open until 0.1.0 publishes.** Substance carried in `lys-core/src/attestation/` module docs; conformance evidence in `lys-core/tests/cose_conformance.rs` against vendored `veraison/go-cose` v1.3.0 |
| D5 | Certificates remain X.509; sealed envelopes remain `lys/sealed-envelope/v1` (§4.3) | **PROPOSED — awaiting ratification** |
| D6 | Conformance testing for D1/D2 includes vectors verified against the Go `sumdb/note` reference, not only Rust implementations (§2.2) | **IMPLEMENTED — 2026-07-11.** Evidence: `lys-core/tests/go_conformance.rs` round-trips sign/verify (including a blank-line body and a failed-known-key rejection) against the vendored `golang.org/x/mod/sumdb/note` v0.22.0, byte-identical notes both ways |

Ratified decisions get their status flipped here with a date and the ratifier's name; the sections above then become frozen contracts and move (in substance) into module-level docs alongside the code that implements them. **IMPLEMENTED** is the intermediate state: code is built and tested to the proposed bytes, but nothing durable has been published under them and the formats do not freeze until 0.1.0 — the operator can still amend on review, and ratification remains a recorded human decision, never inferred from a build going green.
