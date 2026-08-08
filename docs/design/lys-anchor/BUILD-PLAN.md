# `lys-anchor` — implementation plan

> **This is a plan, not a specification and not a record of anything built.** No code
> exists for it, nothing has been signed under any format it names, and it freezes
> nothing — a format is frozen by publishing a crate that exposes it or by signing a
> durable artifact under its tag, neither of which this document does.
>
> **Derived from DP1–DP22** as recorded in [DECISIONS.md](DECISIONS.md), the two
> strawmen ([STRAWMAN.md](STRAWMAN.md), [STRAWMAN-SESSION.md](STRAWMAN-SESSION.md)),
> the draft wire contracts in [WIRE-DRAFTS.md](WIRE-DRAFTS.md), the frozen contracts in
> [../WIRE-FORMATS.md](../WIRE-FORMATS.md) §1–§3, and the shipped code in
> `crates/lys-core/` and `crates/lys-log-store/`. Every claim about existing code
> carries a `file:line`, so any of them can be checked rather than believed.
>
> **§9 is the point of writing this down.** The items there are disputes to be had
> *before* the code exists, not review comments to be raised after. Two of the findings
> in §1 are marked **OPEN AND BLOCKING** and are addressed to the operator: the
> increments they gate must not start until they are ruled on.

---

## 0. What was read, and what was not

**Read in full:** `CLAUDE.md`; `docs/design/lys-anchor/DECISIONS.md`; `STRAWMAN.md`;
`STRAWMAN-SESSION.md`; `WIRE-DRAFTS.md`; `docs/design/WIRE-FORMATS.md`;
`docs/DESIGN.md`; `docs/ROADMAP.md`; `crates/lys-log-store/src/store.rs`, `log.rs`,
`lib.rs`; `crates/lys-core/src/lib.rs` and every `mod.rs`; the workspace and all three
crate `Cargo.toml` files.

**Read in part (targeted):** `lys-core/src/receipt/{sign,artifact,consistency,encoding}.rs`
(signatures, structs, module docs, refusal branches); `checkpoint/{note,body,verifier_key}.rs`
(signatures plus the key-ID construction); `tlog/{build,artifact}.rs` (signatures and the
private helpers); `bundle/{artifact,verify}.rs` (constants and the `verify_bundle` body);
`merkle/{proof,tree,consistency,reconstruct}.rs` (signatures only);
`lys-log-store/src/file.rs` (module docs and layout constants); `lys/src/main.rs`,
`commands/log/*`; `lys-core/tests/harness/mod.rs`, `tests/consistency_receipt_conformance.rs`;
the test-function list of `receipt/sign_tests.rs`.

**NOT read:** `lys-core/src/{ca,seal,attestation}/` beyond their `mod.rs` and exported
signatures; any `*_tests.rs` body except by grep; the vendored Go under
`tests/{cose,go}-conformance/`; `docs/VISION.md`, `REVIEW-23-07.md`, `RELEASE*.md`,
`PEN-REGISTRATION.md`, `design/lys-core/*`, `design/lys-log-store/BACKENDS.md`;
`crates/lys/src/cli.rs` in full. **The C2SP `tlog-witness`, `tlog-cosignature`,
`tlog-tiles` and `signed-note` specifications were not read** — they are cited in the
strawman but not vendored here, and §4 is written so that no byte of a cosignature
format is asserted from memory.

---

## 1. Findings first — conflicts, ambiguities, and one objection

These come before the design because three of them change what gets built, and two of
them stop work until they are ruled on.

### F1 — 🔴 OPEN AND BLOCKING (increment 6b) — DP9's certificate gate and DP13's domain-agnosticism cannot both be a shipped default

**Addressed to the operator. Increment 6b must not start until this is ruled on.**

DP9 (`DECISIONS.md:166`) rules the write path "certificate-gated, now that G1 makes a
certificate mean something… deliberately *after* the receipt and bundle work." That
precondition has been met: `lys-core/src/receipt/` and `lys-core/src/bundle/` both exist.
So DP9 is due.

DP13 (`DECISIONS.md:179-187`), nine days later, rules in the operator's own words:
*"completely general and able to be wired into anything and used by anything to sign
anything."*

A certificate gate means a producer must hold a lys certificate chaining to a CA the
anchor recognises. That is not "anything". **These two cannot both be satisfied by a
shipped default, and this plan does not resolve it.**

*One possible resolution, offered as input to the ruling and NOT as a settled position:*
admission could be an `AdmissionPolicy` trait in `lys-anchor`, with the anchor core
knowing nothing about certificates and the deployment choosing a policy — DP9 satisfied
as "the cert-gated policy exists and is what our hosted instance runs", DP13 satisfied as
"the anchor itself imposes nothing." That shape has a cost worth naming before it is
chosen: there would be **no default policy value**, so construction must force the caller
to name one.

**What is being asked:** does DP13 supersede DP9, narrow it to "the policy our instance
runs", or leave it intact as a property of every lys anchor? Increments 1–6a and 7–11 do
not depend on the answer. 6b does, entirely.

### F2 — 🔴 OPEN AND BLOCKING (increment 12) — "the witness API ships in v1" does not say whether it means the contract or a transport

**Addressed to the operator. Increment 12 must not start, and must not be assumed absent,
until this is ruled on.**

DP16 (`DECISIONS.md:227-231`) binds the witness API to v1 "even if zero witnesses run at
first — retrofitting it later is painful, and until it exists every claim made is weaker
than it sounds." DP5 (`DECISIONS.md:165`) rules no lookup endpoint on day one because
"serving lookups over a network turns decode-success into a parsing oracle."

Two readings, and the plan's shape differs between them:

- **"API" = the contract.** The Rust surface, the request/response types, the refusal
  semantics, the per-origin state model. Then v1 is a library plus a filesystem binary,
  and §5 argues that case at length.
- **"API" = an HTTP endpoint a third-party witness can call.** Then a transport crate is
  not increment 12-or-later but a v1 deliverable, and the oracle discipline in §5.2
  becomes load-bearing immediately.

**This plan is written for the first reading and does not settle the question.** §5.3
gives the argument for it; the argument is not a ruling and is not treated as one.

**What is being asked:** does DP16 require an HTTP witness endpoint in v1?

### F3 — the strawman's *reason* for "the parent must NOT verify the child" is factually wrong; DP18's reason is not, and the two are compatible

`STRAWMAN-SESSION.md:129-151` takes the position that a parent stores what it is handed
and does not check the child's internal consistency, with the reason: *"verification would
require the parent to hold the child's entire tree."*

**That reason does not survive contact with RFC 6962.** Checking that a new checkpoint is
consistent with the previous one the parent already holds requires exactly three things,
all of which the parent either already has or is handed: the old root (O(1) state the
parent recorded itself), the new root, and a consistency proof.
`merkle::verify_consistency(old_root, new_root, proof)` takes three arguments and no tree
(`lys-core/src/merkle/proof.rs:251`, exported at `merkle/mod.rs:50`). The parent holds one
`(size, root)` pair per origin. It never holds the child's tree.

**DP18 rules the same conclusion on entirely different reasoning, and that reasoning is
sound.** In the operator's words: *"I don't know how a witness would verify something that
they weren't there for — so they really is witnessing the time, and what was recorded."*
That is an epistemic limit, not a cost argument, and it is not touched by the demolition
above. A witness cannot speak to a stretch of log it never saw, however cheap the
arithmetic would be.

**The two are compatible, and the reason is worth stating precisely, because it is the
line the design has to hold:**

> Checking a new checkpoint against the `(size, root)` **the witness itself previously
> recorded** is comparing two things it personally observed. It stays inside what a
> witness can honestly know. It is *not* auditing the child's log, and it cannot become
> that — the check's inputs are the witness's own memory plus the submission in front of
> it, and there is no form of the call that reaches further back.

What a witness still cannot do, and must never claim, is say anything about the stretch of
log between its own observations, or before its first one. A witness that has seen sizes
100 and 200 knows nothing about what happened at 150 beyond the two roots being
consistent — and "consistent" is a statement about two hashes, not about the entries.

**Recommendation, and it is NOT YET RULED:** record first, then check, then report.

1. **Record unconditionally.** The submitted checkpoint bytes are appended as a leaf
   *before* any check runs. **This is a correction to an earlier draft of this plan, which
   had the anchor refuse an equivocating checkpoint without recording it. That is wrong,
   and DP18 is what exposes it: if equivocation is caught by two witness memories
   disagreeing, then a witness that refuses to record the second, conflicting checkpoint
   has destroyed exactly the evidence the mechanism runs on.** Its own log would then
   contain only one of the two roots, and the contradiction would not be provable from the
   witness's memory at all.
2. **Then check**, against the witness's own prior record, per the compatibility argument
   above.
3. **Then report** — the observation goes to the caller and to the operator, never into a
   signed artifact (§4.4).

**DP18 as written does not decide step 2 either way.** It rules on what a cosignature
*asserts* ("at this time, this anchor presented this root", and nothing about
well-formedness, content truth, or anchor honesty) and on how equivocation is *caught*
(two witness memories disagreeing). It does not say whether a witness may additionally
compare a submission against its own memory as an operational convenience. This plan
recommends that it may, having stated why that stays inside DP18's epistemic limit — and
records the recommendation as unruled.

**§3.2 of the strawman was never ruled on** in any case: it is item 2 of "What I am asking
for from the session" (`STRAWMAN-SESSION.md:213-214`), and DP13–DP17 do not mention it.

### F4 — DP16's "rotation is an append" is not expressible in the frozen-draft bundle

DP16 rules a root identity that only signs key-delegation entries into the log, and an
operational key signing day-to-day checkpoints, so that "the key history *is* log history."

`verify_bundle` takes `anchors: &[NoteVerifierKey]` — one key per link
(`bundle/verify.rs:161-165`) — and the rung check verifies the next link's checkpoint
against *that same key* (`bundle/verify.rs:~218-224`), which makes an anchor's
receipt-signing key and its checkpoint-signing key necessarily the same key (stated as
intentional at `WIRE-DRAFTS.md:756-760`). There is **no slot in the bundle for a
delegation entry or a proof of one**, so a stranger verifying a bundle cannot learn the
operational key from the log; the caller must already know it.

This is not a defect — DP14 says a verifier is always *told* which anchor to trust — but
it means **DP16's verifiability claim is only realised if a separate artifact carries the
delegation and its inclusion proof.** Do not amend the bundle. Design the delegation
*leaf* now (it freezes at leaf 0 of the first real anchor and can never be inserted
afterwards), and defer the key-history *artifact* to its own v1 later. Recorded so the
absence does not read as "nearly done."

### F5 — `NoteVerifierKey` cannot represent a C2SP cosignature key, and `sign_note` cannot emit a second signature line

Three code facts that bound §4:

- `key_id` hardcodes the Ed25519 signed-note algorithm byte `0x01`
  (`checkpoint/note.rs:71`, documented at `note.rs:10` and `note.rs:57-60`), and
  `NoteVerifierKey::from_spec` **rejects any other algorithm byte**
  (`verifier_key.rs:117-119`). The strawman notes a cosignature's key-ID type byte is
  `0x04`, not `0x01` (`STRAWMAN.md:85`). A cosignature verifier key is therefore not
  representable by the shipped type.
- `sign_note` emits "body ‖ blank line ‖ **one** signature line" (`checkpoint/note.rs:94-96`).
  No function appends a signature line to an existing note.
- **Verification of multi-signature notes already works**: `verify_note` iterates the
  parsed signature lines and matches on `(name, key_id)` (`checkpoint/note.rs:170-190`).

Consequence: emitting a C2SP-shaped cosignature line requires new `lys-core` code and a
wire format nobody here has read the spec for. §4 avoids needing it in v1.

### F6 — smaller ones, recorded

- **DP12 vs DP15 is resolved in text, not a conflict.** DP15 (`DECISIONS.md:206-216`)
  states DP12's `anchor.lys.dev/prod-01` was always illustrative.
- **Nothing enforces DP12's URL shape.** `validate_origin`
  (`lys-log-store/src/log.rs:45-49`) delegates to `CheckpointBody::from_root`, which
  enforces only the note-name rules — non-empty, no whitespace, no `'+'`
  (`verifier_key.rs:9-27`). WIRE-FORMATS says SHOULD (`WIRE-FORMATS.md:47`). **The anchor
  library must not add a URL-shape refusal**; it would reject origins existing logs
  already use. Advisory warning at the CLI only.
- **`MAX_LINKS = 32`** (`bundle/artifact.rs:37`) caps verifiable cascade depth at 32. That
  is a design bound on the hydra and belongs in the anchor's docs, not in a discovery.
- **The receipt carries no origin** (`WIRE-DRAFTS.md:608-617`, settled at `:778-785`).
  Combined with DP16's rotation, the `kid` in a receipt is the *operational key at the time
  of issue*, and mapping it to an origin requires the anchor's log. A consequence, not a
  defect; state it in module docs.
- **`chunk_proof_bytes` is private** (`tlog/build.rs:153`), and every caller of
  `sign_receipt` in the repo does its own `.chunks_exact(32)` (`receipt/sign_tests.rs:35`,
  `receipt/consistency_tests.rs:54`, `bundle/verify_tests.rs:59`,
  `tests/receipt_conformance.rs:62`). `lys-anchor` will do the same. **Do not add it to
  `lys-core`** — six lines is not worth a semver-bound public item.
- **DP22 leans on `leaf_index`'s authentication, and that load is already carried.** DP22
  rules no log-position claim inside attestations because the receipt already binds an
  entry to its position — which makes `leaf_index`'s authentication-by-consequence
  load-bearing in a way it was not before. It holds: altering the index changes the
  left/right walk and therefore the reconstructed root, and it is tested
  (`a_tampered_leaf_index_is_refused`, `receipt/sign_tests.rs:182`). Note the contrast with
  `tree_size`, whose authentication is *incomplete* and known to be — sizes 3 and 4 share a
  walk at index 0, pinned as a passing test
  (`tree_size_is_malleable_within_its_walk_equivalence_class`, `receipt/sign_tests.rs:243`;
  the format-level ruling is `WIRE-DRAFTS.md:546-588`). **So DP22 is sound for *position*
  and would not have been sound for *size*.** `lys-anchor` must add no position field
  anywhere — the receipt is where that claim lives, and a second statement of the same fact
  is a second thing that can disagree with itself.

---

## 2. Crate layout

### 2.1 The DP19 boundary — standalone core, federation as an additive surface

DP19 is a hard structural requirement, not a deployment mode: *"it needs to be able to run
standalone as well."* A single anchor, zero witnesses, pinning to no one, must be fully
functional — accepts submissions, appends, publishes checkpoints, issues receipts, with no
peer and no network dependency anywhere in the core path.

**This plan puts the module boundary exactly on that line, and makes it mechanical rather
than a matter of discipline.**

| Surface | Core (standalone) | Federation (additive) |
|---|---|---|
| `Anchor::create` / `Anchor::open` | ✅ | |
| `Anchor::submit` → append → receipt | ✅ | |
| `Anchor::publish_checkpoint` | ✅ | |
| `Anchor::receipt_for`, `inclusion_artifact` | ✅ | |
| `Anchor::status` (incl. `WitnessPosture`) | ✅ | |
| `Signer`, `FileSigner` | ✅ | |
| `AdmissionPolicy` and its policies | ✅ | |
| `lys/anchor-delegation/v1` | ✅ | |
| `witness::` — origin projection, observation, reporting | | ✅ |
| `upward::` — pinning this anchor's checkpoint to a parent | | ✅ |
| Bundle assembly across a cascade | | ✅ |

**The structural guard: `federation` is a Cargo feature and it is OFF by default.**
Nothing in `core` may name anything in `witness` or `upward`, and the compiler enforces
it — if a core path ever reached federation code, the default build would fail to compile.
This is not a new gate to remember: the five mandatory gates already run both a
feature-full and a default build (CLAUDE.md), so the standalone shape is exercised on
every commit by the run that already exists. The default `cargo test --workspace` then
means precisely *"the standalone anchor is complete and green"*, which is the claim DP19
requires.

The silent-coverage-loss objection that CLAUDE.md raises against default-off features
(81 tests compile out without `unstable-anchor`) is answered by the same structure:
`--all-features` is mandatory in the gate set, so federation is never the untested shape.

**Reconciliation with DP14, explicitly.** DP14 (`DECISIONS.md:189-204`) rules that anchors
pin to anchors and that cascading *is* the witness mechanism. DP19 and DP14 are not in
conflict, and the ordering matters:

- Cascading is **optional**. An anchor that pins to nobody is not a degraded anchor; it is
  the base case. Every operation in the core column above completes with no peer in
  existence.
- DP14's "same mechanism, not two" is satisfied *because* federation reuses the core, in
  that direction and not the reverse. `upward::pin` calls `Anchor::submit` on a parent;
  the parent's core `submit` is unchanged and does not know it is being used for a
  cascade. **Federation is a caller of the core, never a layer the core is built
  through.** A design where `submit` consulted a witness, or where publishing a checkpoint
  waited on a peer, would have them backwards, and this plan would call that wrong rather
  than accommodate it.
- **No concrete conflict was found.** The one place they touch is the witness projection
  in §4, and it touches the core only by *reading* the log — it holds no `&mut Anchor` and
  cannot refuse anything the core would have accepted.

### 2.2 The standalone limit, stated plainly, and where the software says it

**A standalone anchor with no witnesses can equivocate undetectably. No local check
catches it.** It can hold two histories and show each observer whichever suits, and
nothing in its own storage, its own pin, or its own signed artifacts detects that. This is
not a gap to be closed later by better local checking — it cannot be closed locally at
all, for the same reason already written into the storage layer: *"an actor able to
rewrite both the leaves and the pin can present a consistent shorter log, and no purely
local check can catch that"* (`lys-log-store/src/store.rs:82-84`). DP16 accepts the same
limit for key compromise and names witnesses as the mitigation (`DECISIONS.md:225-231`).

An operator running standalone must be told this by the software, not by a document they
may never read. **Four places, and none of them is a flag that can be silenced:**

1. **`lys-anchor`'s `lib.rs` module docs** state it as an invariant of the mode, in these
   words, alongside the pointer to what changes it (one external witness).
2. **`Anchor::status()` returns it as a value, not a message.** `AnchorStatus` carries a
   non-optional `posture: WitnessPosture` field; `WitnessPosture::Unwitnessed`'s `Display`
   carries the sentence. A caller cannot obtain a status without obtaining the posture,
   and cannot format the posture without the sentence appearing.
3. **The CLI prints it** on `lys-anchor status` and once at `lys-anchor init`.
4. **The library returns, the CLI prints, and neither may drop it** — the same division,
   for the same reason, as the recovery notice: *"a library that writes to stderr has
   decided for its caller how a repair gets reported"* (`crates/lys/src/commands/log/store.rs:9-14`),
   and `Log::recovered_to` returns the fact rather than logging it (`log.rs:150-156`).

The posture is computed from the anchor's own log — whether it has ever recorded a receipt
from a parent — and not from configuration, so it cannot be set to a comfortable value by
an operator who has not actually arranged a witness.

### 2.3 Where things live, and what does NOT go in `lys-core`

`lys-core` is published (`Cargo.toml:11`, version 0.2.0) and semver-bound. **This plan
adds nothing to `lys-core`.**

| Need | Where it lives | Justification |
|---|---|---|
| Merkle tree, proofs | `lys-core::merkle` — `AppendOnlyTree::prove_inclusion` (`merkle/tree.rs:109`), `prove_consistency` (`:135`), `append_raw` (`:197`) | exists |
| Receipts | `lys-core::receipt`, behind `unstable-anchor` (`lys-core/src/lib.rs:38-39`) | exists |
| Checkpoints / notes | `lys-core::checkpoint` — `sign_note` (`note.rs:94`), `verify_checkpoint` (`note.rs:203`) | exists |
| JSON proof artifacts | `lys-core::tlog` — `build_inclusion_artifact` (`tlog/build.rs:43`) | exists |
| Bundles | `lys-core::bundle`, behind `unstable-anchor` (`lib.rs:30-31`) | exists |
| Durable append-only storage | `lys-log-store` — `LeafStore` (`store.rs:120`), `Log` (`log.rs:58`), `FileLeafStore` | exists; DP10 (`DECISIONS.md:167`) rules files |
| 32-byte proof chunking | `lys-anchor`, six lines | F6 |
| Signer custody trait | `lys-anchor` | unratified boundary; `STRAWMAN-SESSION.md:66-73` |
| Delegation leaf format | `lys-anchor`, draft | freezing it in a published crate is the DP17 trap |
| Admission policy | `lys-anchor` | F1 |
| Witness projection | `lys-anchor`, federation only | per-origin state is not storage's business (`lys-log-store/src/lib.rs:45-50`) |

If any of it later needs to move into `lys-core`, the route is the existing
`unstable-anchor` feature (`lys-core/Cargo.toml:39-55`), already off-by-default and
semver-exempt for exactly this purpose. Not now: a format with no second implementation
and no operator ratification should not sit in the crate that gets published.

### 2.4 Files

Every file is a named file with logic; every `mod.rs` carries only `pub mod` / `pub use` /
docs (CLAUDE.md). Every logic file gets a sibling `*_tests.rs`. Estimated code lines
exclude tests, comments and whitespace; the 500-line rule is the binding constraint and
the splits below keep every file well under it. `[fed]` marks a file compiled only with
the `federation` feature.

```
crates/lys-anchor/
  Cargo.toml                      federation = []  (OFF by default)
  src/
    lib.rs                        docs + pub mod / pub use ONLY
    error.rs                      AnchorError: thiserror, #[non_exhaustive],
                                  non-oracle collapse for anything a stranger
                                  can reach                                     (~120)
    config.rs                     AnchorConfig — origin source, dirs, policy
                                  handle. NO Default impl; origin has no
                                  default value                                 (~90)

    keys/mod.rs
    keys/signer.rs                `Signer` trait (custody boundary) + key type   (~80)
    keys/file_signer.rs           file-backed impl over Ed25519Identity          (~90)
    keys/delegation.rs            build / parse / verify a delegation entry     (~220)
    keys/delegation_encoding.rs   byte-exact COSE_Sign1 encode + decode         (~260)

    anchor/mod.rs
    anchor/open.rs                Anchor::create / Anchor::open; genesis        (~200)
    anchor/submit.rs              submit(): policy → append → receipt           (~180)
    anchor/checkpoint.rs          publish_checkpoint(): the anchor's own note   (~120)
    anchor/prove.rs               receipts + JSON artifacts for an index        (~200)
    anchor/status.rs              AnchorStatus + WitnessPosture (§2.2)          (~110)
    anchor/proof_nodes.rs         InclusionProof bytes → Vec<[u8;32]>            (~50)

    policy/mod.rs
    policy/admission.rs           `AdmissionPolicy` trait + MaxSize, AcceptAll  (~130)
    policy/certificate.rs         DP9 cert-gated policy  [BLOCKED ON F1]        (~220)

    witness/mod.rs                [fed]
    witness/projection.rs         [fed] origin → last observed (size, root),
                                  derived from the log, never authoritative     (~230)
    witness/observe.rs            [fed] record-then-check-then-report (§4.3)    (~240)
    witness/report.rs             [fed] conflict reporting, operator-visible    (~130)

    upward/mod.rs                 [fed]
    upward/pin.rs                 [fed] submit this anchor's checkpoint upward  (~150)

    wire/mod.rs
    wire/submission.rs            Submission / SubmissionOutcome                (~110)
    wire/observation.rs           [fed] Observation / OriginState               (~130)

  src/bin/lys-anchor.rs           binary entry: parse + dispatch, no logic      (~150)
  src/cli/…                       clap types + per-command formatting  [inc. 10]

  tests/
    standalone_is_complete.rs     the whole core path with zero peers
    stranger_verification.rs      the shell + Go stranger gate
    receipt_conformance.rs        go-cose parity over anchor-issued receipts
    cascade.rs                    [fed] two-anchor cascade → verify_bundle
    origin_is_not_a_constant.rs   the lexical gate, with a positive control
    harness/mod.rs                the Go env contract (see §7.10)
```

### 2.5 The binary is separate, and this matters

**`lys-anchor`'s CLI must be its own binary in the `lys-anchor` crate, not new subcommands
on `lys`.**

This is the DP17 trap in a new costume. `lys` is published (`crates/lys/Cargo.toml`).
`lys-anchor` depends on `lys-core/unstable-anchor` to touch `receipt` at all
(`lys-core/src/lib.rs:38-39`). If `lys` gained `lys anchor …`, the published `lys` binary
would depend on the unstable feature, and **publishing `lys` would freeze
`lys/anchor-receipt/v1` and `lys/verification-bundle/v1` as a side effect** — exactly the
mechanism caught before 0.2.0 (`lys-core/Cargo.toml:41-55`: "Publishing a crate that
exposes a format is one of the two things that FREEZES it"). A separate, unpublished crate
keeps the drafts free.

`crates/lys-anchor` is added to `Cargo.toml:2` `members` and carries `publish = false`
until DP17's sequence completes.

---

## 3. Core types — and what is inside the signed bytes

Signatures below are the intended public surface. **Signed** means covered by an Ed25519
signature a stranger can check; **not signed** means present but authenticated only by
consequence, or absent entirely.

### 3.1 `Signer` — the custody boundary  *(core)*

```rust
pub trait Signer {
    fn public_key(&self) -> [u8; 32];
    fn sign(&self, message: &[u8]) -> AnchorResult<[u8; 64]>;
}
```

`STRAWMAN-SESSION.md:66-73`: the anchor signs through a trait, never through a held
`Ed25519Identity`, so an HSM/KMS backend is a swap and not a rewrite. DP4
(`DECISIONS.md:163`) ratifies software keys in files first with the interface kept narrow.

**The friction is real and must be documented rather than papered over.** `lys-core`
signing entry points take `&Ed25519Identity` concretely — `sign_note(body, name, identity)`
(`checkpoint/note.rs:94`), `sign_receipt(…, anchor_key: &Ed25519Identity)`
(`receipt/sign.rs:121-127`), `build_inclusion_artifact(…, identity, …)`
(`tlog/build.rs:43-49`). A non-file `Signer` cannot be used with them without either
(a) `lys-core` growing signer-generic variants — a semver-bound addition this plan
declines — or (b) `lys-anchor` reimplementing `Sig_structure` assembly, which would be a
second encoder and is precisely what CLAUDE.md's "two copies of a canonical encoder"
warning is about.

**v1: the trait exists and is the type every anchor call site names; its only
implementation wraps `Ed25519Identity`.** The boundary is being reserved, and it is
documented as *not yet* usable with a remote signer, with the sentence naming what would
have to change (signer-generic `lys-core` entry points, behind `unstable-anchor`) written
where the next reader finds it. Claiming a swap is possible when it is not would be worse
than the gap.

### 3.2 `Submission` and `SubmissionOutcome`  *(core)*

```rust
pub struct Submission<'a> {
    /// The statement bytes, verbatim. The anchor does not interpret them.
    pub statement: &'a [u8],
}

pub struct SubmissionOutcome {
    pub leaf_index: u64,
    pub tree_size: u64,
    pub leaf_hash: [u8; 32],
    pub receipt: AnchorReceipt,
}
```

- **Signed by the anchor:** the receipt's RFC 9052 §4.4 `Sig_structure` — protected header
  (`alg -8`, content type, `kid`, `vds 395 = 1`) plus the **detached 32-byte root**
  (`WIRE-DRAFTS.md:79-86`, `:520-540`; `receipt/mod.rs:13-34`).
- **Not signed, authenticated by consequence:** `leaf_index`, `tree_size`, and the
  inclusion path — they sit in the unprotected header and cannot be altered without
  producing a root the anchor never signed (`receipt/mod.rs:36-58`). `leaf_index` is fully
  pinned by the reconstruction and tested (`receipt/sign_tests.rs:182`), which is what DP22
  now leans on. `tree_size` carries a **documented residual malleability**
  (`WIRE-DRAFTS.md:546-588`, `receipt/sign_tests.rs:243`): at some `(index, size)` pairs
  two sizes share a walk. Cross-check it against a checkpoint, which the bundle's rung
  already does (`bundle/verify.rs:~218-224`).
- **Not signed at all:** `leaf_hash` (a convenience — recomputable as
  `SHA-256(0x00 ‖ statement)`), and **the anchor's origin, which appears nowhere in the
  receipt** (`WIRE-DRAFTS.md:778-785`).
- **Not signed by the anchor, and not checked by it:** whatever signature the statement
  itself carries. DP13.

`Submission` deliberately carries no `content_type`, no `producer`, no `kind`. The moment
it does, the anchor knows something about meaning.

### 3.3 `Anchor<S: LeafStore>`  *(core)*

```rust
pub struct Anchor<S: LeafStore, P: AdmissionPolicy> { /* Log<S>, signer, config, policy */ }

impl<S: LeafStore, P: AdmissionPolicy> Anchor<S, P> {
    pub fn create(store: S, genesis: &[u8], signer: …, policy: P, cfg: AnchorConfig) -> AnchorResult<Self>;
    pub fn open(store: S, signer: …, policy: P, cfg: AnchorConfig) -> AnchorResult<Self>;
    pub fn origin(&self) -> &str;
    pub fn tree_size(&self) -> u64;
    pub fn submit(&mut self, submission: Submission<'_>) -> AnchorResult<SubmissionOutcome>;
    pub fn publish_checkpoint(&self) -> AnchorResult<PublishedCheckpoint>;
    pub fn receipt_for(&self, leaf_index: u64) -> AnchorResult<AnchorReceipt>;
    pub fn inclusion_artifact(&self, leaf_index: u64) -> AnchorResult<InclusionProofArtifact>;
    pub fn status(&self) -> AnchorStatus;          // carries WitnessPosture — §2.2
    pub fn recovered_to(&self) -> Option<u64>;
}
```

**Every method above completes with no peer in existence.** That is DP19, expressed as the
absence of any parameter or field that could name one.

Generic over `S: LeafStore` so DP10's file backend is a type parameter and not a commitment
(`lys-log-store/src/store.rs:120`). `recovered_to` forwards `Log::recovered_to`
(`log.rs:154`) and must be surfaced, never dropped — reasoning already written at
`crates/lys/src/commands/log/store.rs:1-14`.

The anchor **does not** re-derive an origin: it reads `Log::origin()` (`log.rs:188`), which
reads `LeafStore::origin()` (`store.rs:126`), fixed immutably at store creation (contract
item 5, `store.rs:116-117`). **This is how DP15 is enforced structurally rather than by
discipline** — there is nowhere in `lys-anchor` for an origin constant to live, because the
origin's only source is a directory somebody created.

### 3.4 `PublishedCheckpoint`  *(core)*

```rust
pub struct PublishedCheckpoint { pub note: String, pub body: CheckpointBody }
```

- **Signed:** the checkpoint body — origin, tree size, base64 root — **including its
  trailing newline** (`WIRE-FORMATS.md:41-49`, `:62`). Produced by
  `CheckpointBody::from_root` (`body.rs:65`) → `encode` (`body.rs:72`) → `sign_note`
  (`note.rs:94`).
- **Not signed:** the signature lines themselves, and any cosignature line another party
  appends later. That is what makes cosignature-by-appending possible at all without
  invalidating the original.

The anchor's checkpoint is signed under **its own origin as the note key name**, which
`verify_checkpoint` then binds (`note.rs:203-218`: `body.origin() != verifier.name()` is a
refusal). That binding is why an anchor cannot have one log's checkpoint accepted for
another.

### 3.5 `Observation` and `OriginState`  *(federation only)*

```rust
pub struct OriginState { pub origin: String, pub tree_size: u64, pub root: [u8; 32] }

pub struct Observation {
    pub recorded: SubmissionOutcome,     // the note, as a leaf, with its receipt
    pub previous: Option<OriginState>,   // what this witness itself last observed
    pub relation: Relation,              // Extends | Identical | Rollback | Conflicting | Unrelated
}
```

- **Signed by the submitter:** the checkpoint body inside the note.
- **Signed by the anchor in response:** the receipt inside `recorded` — over the anchor's
  own root, whose proven leaf is the checkpoint note verbatim. **That is the
  countersignature** (§4.2).
- **Signed by nobody:** `previous` and `relation`. They are the witness's report of what it
  observed, and they are deliberately outside every signature (§4.4).

`OriginState` is deliberately the same shape as `PinnedRoot`
(`lys-log-store/src/store.rs:90-96`).

### 3.6 The delegation entry — `lys/anchor-delegation/v1`  *(core; DRAFT; increment 11)*

DP16 wants two keys and rotation-as-append. One format serves genesis and every later
rotation, because two formats is two things to freeze.

Tagged `COSE_Sign1`, following the shipped pattern exactly (`WIRE-FORMATS.md:133-139`;
`receipt/encoding.rs:1-47`):

- protected: `{1: -8, 3: "application/vnd.lys.anchor-delegation.v1+cbor", 4: <raw 32-byte ROOT key>}`
- payload: **embedded** (unlike a receipt) — deterministic CBOR map
  `{1: origin (tstr), 2: delegated public key (bstr 32), 3: role (uint), 4: not-before unix-ms (int)}`
- signature: Ed25519 over `["Signature1", protected, h'', payload]`

Rationale per choice: byte-0 `0xD2` keeps it disjoint from a signed note (ASCII origin) and
a DER certificate (`0x30`) — the classifiability property at `STRAWMAN.md:16`. The root key
in `kid` is signature-covered, inheriting the v1→v2 attestation fix rather than
rediscovering it (`WIRE-DRAFTS.md:85`). The **origin is in the payload** — runtime
configuration reaching a signed byte, which DP15 permits and requires (a value supplied at
run time, not a committed constant). Embedded payload, not detached, because unlike a
receipt there is no value the verifier independently recomputes.

**This format freezes the moment a real anchor initialises**, because leaf 0 cannot be
inserted afterwards. It must not be built until increment 11, and no instance may be signed
outside tests (DP17, `DECISIONS.md:252-255`).

---

## 4. The witness surface — additive, and it is the submit path

### 4.1 What a witness signs

**A witness signs its own Merkle root, as a detached payload, in a receipt whose proven
leaf is the submitter's checkpoint note verbatim.** Nothing else. That is
`sign_receipt(leaf = checkpoint_note_bytes, …)` (`receipt/sign.rs:121-127`).

### 4.2 Cascading IS witnessing — the unification DP14 asks for, and DP18's structural guard

DP14 (`DECISIONS.md:201-204`) requires: *"An anchor pinning to another countersigns it.
Design so these are the same mechanism rather than two."* The strawman had two edges
(`STRAWMAN.md:20-24`): cosigning via a C2SP note line, and leaf submission yielding a
receipt.

**They are already one mechanism, and the second should not be built.** A countersignature
is a second party's signature over a statement containing your checkpoint. A receipt over
your checkpoint bytes is exactly that: the anchor's Ed25519 signature over a root the
verifier recomputes *from your checkpoint bytes* — alter one byte and the reconstruction,
and therefore the signature check, fails (`receipt/mod.rs:36-48`). The witness cannot have
signed it without possessing it, which is what `STRAWMAN-SESSION.md:141-142` says a witness
is: *"a durable memory, not an auditor."*

Four consequences, and the fourth is what DP18 asks for:

1. **No new wire format.** F5 shows a C2SP cosignature line needs a key type
   `NoteVerifierKey` rejects (`verifier_key.rs:117-119`), a note-line emitter that does not
   exist (`note.rs:94`), and a spec nobody here has read. Each is a freeze risk for zero
   additional v1 property.
2. **The receipt is durable; a cosignature line is not.** A cosignature line lives on the
   child's published checkpoint — lose it and the guarantee evaporates (`STRAWMAN.md:38`,
   option (a)'s stated weakness). A receipt is a portable artifact the child holds.
3. **The bundle already assumes this shape.** `verify_bundle`'s rung requires the next
   link's checkpoint to be *this anchor's own note-signed checkpoint* stating the exact root
   and size the receipt vouched for (`bundle/verify.rs:~215-224`). A cascade of receipts
   drops straight in. A cascade of cosignature lines does not.
4. **⭐ It is the structural guard DP18 prefers over careful wording.** DP18 requires that a
   cosignature never be presentable as an endorsement. Because the witness path produces
   *the same artifact as an ordinary submission, from the same function, with no extra
   input*, **there is no field in which "I also checked this" could be encoded, and no
   artifact a witness can emit that a plain recorder could not.** A reader cannot infer
   endorsement from a receipt, because the receipt does not distinguish the two cases. That
   is a property of the shape, not of the documentation.

**The honest cost:** this gives up *ecosystem* interop with the public C2SP witness
network, which speaks cosignature lines. That network is ~15 devices (`STRAWMAN.md:150`)
and joining it is already a named deferral. The C2SP line, when wanted, is a **rendering**
of the same fact and is additive.

### 4.3 The witness path — record, then check, then report

Per F3's recommendation (**not yet ruled**), and structured so federation never sits in the
core's way:

1. **Record.** The checkpoint note is submitted through the *core* `Anchor::submit`. It is
   appended durably before anything else happens — `Log::append` stores the leaf before
   advancing the pin (`log.rs:168-185`, reasoning at `log.rs:15-25`), and persist-before-
   respond is the atomicity requirement at `STRAWMAN.md:20`. **The record is
   unconditional.** A witness that refuses to record a conflicting checkpoint destroys the
   evidence DP18's detection mechanism runs on (F3).
2. **Check**, in the federation module, against the projection — the `(size, root)` **this
   witness itself previously recorded** for that origin:
   - identical `(size, root)` → `Relation::Identical`. Not an error; a no-op must not be
     one, and that idempotent repeat is precisely the door the check has to stand in
     (`lys-log-store/src/store.rs:176-178`).
   - `new_size < prev_size` → `Relation::Rollback`.
   - `new_size == prev_size && new_root != prev_root` → `Relation::Conflicting`. **This is
     the observation the whole mechanism exists to produce.** It is the registry-level twin
     of `StoreError::PinRootChanged`, for the reason already written there: *"an append-only
     tree has exactly one root per size, so accepting it would record that the log's history
     is two different things"* (`store.rs:171-178`).
   - otherwise `merkle::verify_consistency(prev_root, new_root, proof)`
     (`merkle/proof.rs:251`) → `Extends` or `Unrelated`.
   - no prior record → `previous: None`, and the relation is not computed. **A witness with
     no memory of an origin has nothing to compare and must say so rather than imply
     assent.**
3. **Report.** `Observation` goes to the caller; a non-`Extends` relation is surfaced to the
   operator. **Nothing from step 2 enters any signed artifact** (§4.4).

Note what step 2 is *not*: it never reaches past the witness's own two observations, and
there is no form of the call that could. Signature verification of the note, if it is done
at all, is the **admission policy's** business and is applied uniformly to every
submission — the witness path adds no admission rule of its own, which is what keeps it
additive.

### 4.4 What a witness is explicitly NOT asserting

Write these into `witness/mod.rs`'s module docs, in these words:

- **Not that the submitter's log is honest.** The witness holds one `(size, root)` pair. A
  log that forked before the witness ever saw it presents a perfectly consistent history
  from that point.
- **Not anything about the stretch it never saw.** Between two observations, and before the
  first, a witness knows nothing. Consistency between two roots is a statement about two
  hashes, not about the entries between them. This is DP18's limit and it is the sharpest
  one here.
- **Not that the leaf contents are true.** RFC 9943 registration-policy semantics are out
  (`STRAWMAN-SESSION.md:161-169`): *"the log makes no claim about whether an entry is true,
  only that it is recorded."*
- **Not well-formedness.** DP18, explicitly.
- **Not a timestamp.** No time enters the receipt (DP8, `DECISIONS.md:117-130`, re-put and
  re-accepted at `:241-242`). Time comes from the ordering of the witness's own log and from
  other parties' corroboration.
- **Not that the witness itself is honest.** Already stated for receipts
  (`receipt/mod.rs:60-67`) and bundles (`bundle/mod.rs:29-39`). The anchor inherits it.
- **Not that the check in §4.3 step 2 happened.** The receipt is identical either way, by
  construction (§4.2 point 4). The durable evidence is the recorded note; the check is a
  report.

---

## 5. The library / binary boundary, and whether HTTP is in v1

### 5.1 The rule as this repo applies it

`crates/lys/src/main.rs:1-10`: *"a thin surface over `lys_core`: it parses arguments,
dispatches… All logic lives in the library."* The boundary is drawn with care where it is
genuinely ambiguous: `Log` **returns** the recovery fact and the CLI prints it, because *"a
library that writes to stderr has decided for its caller how a repair gets reported"*
(`crates/lys/src/commands/log/store.rs:9-14`).

### 5.2 What a service layer would and would not contain

Whenever it is built:

**Contains:** byte framing, routing, status-code mapping, rate limiting, connection
lifecycle, and the serialization of `wire/` types.

**Contains no:** signature verification, Merkle work, admission decisions, ordering, receipt
construction, projection state, or error-message composition. Every one of those is a call
into `lys-anchor`.

**And it must not compose error text.** `TrustError`'s verification variants are
deliberately causeless because these types are reachable from network-exposed surfaces
where a distinguishable error is a parsing oracle (`lys-core/src/error.rs:29-35`). A
transport that mapped four internal refusals to four status codes would rebuild the oracle
the library spent effort removing. **Rule: one refusal status for every substantive
refusal.** The witness path is the sole candidate exception, because C2SP's distinguished
codes are part of a contract with a counterparty — adopting them means adopting the oracle
knowingly, a decision for whoever builds transport, recorded here so it is not made by
accident.

### 5.3 The argument that HTTP is not in v1 — subject to F2

`docs/DESIGN.md:101` says *"No network in the core crate… transport lives in `lys-anchor`."*
That cuts against this position and is addressed directly: it settles *where* transport
goes, not *when*.

1. **DP5 already ruled the network surface down** (`DECISIONS.md:165`): serving lookups
   turns decode-success into a parsing oracle. A submission endpoint is a decode-then-decide
   surface with the same property in sharper form.
2. **The read path needs no server.** Static assets (`STRAWMAN.md:127`) — a file server over
   a directory.
3. **DP17's sequence does not require it** (`DECISIONS.md:233-238`): build → run privately
   against a real log → ratify → publish. "Privately against a real log" is a directory and
   a binary.
4. **Dependency mass.** An async runtime plus an HTTP stack is a large surface added to a
   crate whose value is that it can be audited. Pure Rust (CLAUDE.md) is satisfied by
   axum/hyper, so this is a judgement about auditability, not a rule violation — stated as
   such.
5. **Nothing is retrofitted.** With the `wire/` types and the `Anchor` methods designed in
   v1 as *the* contract, a transport layer is a serialization of already-tested functions.

**This is an argument, not a ruling. F2 is open and blocking.** If DP16 meant an HTTP
witness endpoint in v1, increment 12 moves forward and §5.2's oracle discipline becomes
immediately load-bearing.

---

## 6. Build sequence

Five gates green at every increment: `cargo fmt --check`;
`clippy --all-targets --all-features -D warnings`; `clippy --all-targets -D warnings`;
`test --workspace --all-features`; `doc --no-deps --all-features`.

The workspace now has **two** feature-gated shapes to keep honest: `lys-anchor` requires
`lys-core/unstable-anchor`, and `lys-anchor`'s own `federation` is off by default. Verify at
increment 1 that a bare `--all-targets` run does not silently enable `unstable-anchor` for
`lys` — workspace feature unification is exactly where a default-off feature quietly turns
on, and the default run's whole purpose is to prove the shape consumers actually get.

Increments 1–6a and 10–11 are **core**; 7–9 are **federation**; 6b and 12 are **blocked**.

### Increment 1 — `Anchor::create` / `Anchor::open`, genesis as injected bytes  *(core)*

`Anchor<S>` over `Log<S>`; `create` requires `genesis: &[u8]` from the caller and appends it
as leaf 0; `open` refuses a log of extent 0. Config with no `Default`. No signing, no
receipts, no formats.

**Why this is first, and it is not arbitrary:** `sign_receipt` **refuses `tree_size == 1`**
(`receipt/sign.rs:128`), because RFC 9942 types the inclusion path as one-or-more and a
one-leaf tree's path is empty (`WIRE-DRAFTS.md:122-140`) — and the function's own doc names
the remedy: *"seed the anchor's log with a genesis leaf so tree size is never below 2."* So
an anchor whose log lacks a genesis leaf **cannot issue a receipt for its first
submission**, and leaf 0 can never be inserted afterwards, because there is no `insert`, no
`rewrite` and no `fork` in `LeafStore`, deliberately
(`lys-log-store/src/store.rs:3-25`). If genesis is discovered at increment 4, every anchor
initialised before then is unfixable. Nothing else in this plan has that property.

Second reason: it forces DP15 into code on the day the shortcut is cheapest. There is no
origin constant because there is nowhere to put one — origin comes from `LeafStore::origin()`
(`store.rs:126`).

Genesis is *injected bytes* rather than a format, so increment 1 freezes nothing.

**Landable proof:** create → open → tree size 1; a zero-extent log refused; a tampered
leaf 0 → `PinMismatch` from `Log::open` (`log.rs:140-145`); origin round-trips from the
directory.

### Increment 2 — `Signer` trait + file-backed impl  *(core)*

Introduced before a second call site exists, because "give me the private key" leaks into
every layer that touches signing if allowed once (`STRAWMAN-SESSION.md:70-72`). Ships with
§3.1's limitation documented.

### Increment 3 — `publish_checkpoint` and `AnchorStatus`  *(core)*

`CheckpointBody::from_root` (`body.rs:65`) + `sign_note` (`note.rs:94`) over the anchor's own
tree, under its own origin. **The smallest increment that makes DP14's "the anchor is itself
a lys instance" true in code**, and it needs nothing from the receipt layer. `AnchorStatus`
and `WitnessPosture` land here, so the §2.2 disclosure exists from the first moment an
anchor can publish anything.

Proof: the note verifies under `verify_checkpoint` (`note.rs:203`) and, in the Go gate,
under `sumdb/note`; a fresh anchor's status reads `Unwitnessed` and its `Display` carries
the sentence.

### Increment 4 — `submit` → append → receipt  *(core)*

The payoff path. Adds `wire/submission.rs`, `anchor/submit.rs`, `anchor/proof_nodes.rs`.

### Increment 5 — JSON proof artifacts alongside receipts  *(core)*

`build_inclusion_artifact` (`tlog/build.rs:43`) emitted next to every receipt. **Not
optional**: DP2 rules the JSON proof keeps being emitted regardless, because "a receipt that
only specialised tooling can check violates verification must outlive the vendor"
(`DECISIONS.md:54-58`).

### Increment 6a — `AdmissionPolicy` trait + trivial policies  *(core)*

Trait plus `MaxSize` and `AcceptAll`.

### Increment 6b — the cert-gated policy (DP9)  🔴 **BLOCKED ON F1**

Must not start until F1 is ruled on.

### Increment 7 — the witness projection and observation path  *(federation)*

`witness/projection.rs`, `observe.rs`, `report.rs`. The projection is **derived from the
log, rebuildable, and never authoritative** — the same standing the pin has
(`lys-log-store/src/store.rs:79-89`). The module holds no `&mut Anchor` and can refuse
nothing. If it needs splitting, projection-and-state lands before the relation logic.

### Increment 8 — upward pin (the cascade)  *(federation)*

An anchor submits its own published checkpoint to a parent. **Implemented by calling the
same core functions increment 4 exposes** — the DP14 requirement, asserted structurally in
§7.5.

### Increment 9 — bundle production  *(federation)*

Assemble a `VerificationBundle` (`bundle/artifact.rs:47`) from a two-anchor cascade;
`verify_bundle` (`bundle/verify.rs:161`) is the judge. Note `MAX_LINKS = 32`
(`bundle/artifact.rs:37`) and that a populated `counter_anchor` is refused
(`bundle/verify.rs:~177-180`).

### Increment 10 — the CLI binary  *(core, with federation subcommands behind the feature)*

`lys-anchor init | submit | checkpoint | prove | status`, plus `observe | pin` under
`federation`. Parse and format only.

### Increment 11 — `lys/anchor-delegation/v1` and genesis-as-delegation  *(core)*

The first format this crate freezes, deliberately last. `create` changes to build genesis
from the root signer instead of taking injected bytes. **Requires an adversarial review
before landing (CLAUDE.md) and must not be signed outside tests (DP17).**

### Increment 12 — transport  🔴 **BLOCKED ON F2**

Neither started nor assumed absent until F2 is ruled on.

---

## 7. Test strategy — where the second party comes from

The rule this repo enforces: a suite written from the implementation has nothing to
disagree with, so it passes for exactly as long as the behaviour is wrong. **Every item
below names its second party**, because an item that cannot is one that will pass while
wrong.

### 7.1 Things that must NOT be re-tested

The Merkle tree, the receipt encoding, the note format, the bundle rungs. `lys-core`
already has second parties for all of them — `ct-merkle` sweeps, `veraison/go-cose`,
`sumdb/note`, Cloudflare `signed_note` (`WIRE-FORMATS.md:159`). A `lys-anchor` test
asserting them again is one party agreeing with itself through an extra import.

### 7.2 `standalone_is_complete.rs` — the DP19 gate

Create an anchor, submit, publish a checkpoint, issue a receipt, produce a JSON artifact,
read status — **with the `federation` feature off, in a suite that would not compile if any
core path named a federation item.** The second party is the compiler, and the axis is
*reachability*, which is the only axis on which DP19 is a claim at all.

Assert additionally: a fresh anchor's `WitnessPosture` is `Unwitnessed`, and its `Display`
contains the equivocation sentence. A disclosure that can be silently dropped is not a
disclosure.

### 7.3 The stranger gate — the second party for the whole submit path

`tests/stranger_verification.rs`. Build an anchor in a temp directory, submit a leaf,
publish a checkpoint. Then, **with no `lys` code in the loop**:

- Recompute the leaf hash with a shell pipeline: `(printf '\x00'; cat leaf) | shasum -a 256`
  — the reproducibility WIRE-FORMATS states as a contract (`WIRE-FORMATS.md:20`).
- Recompute the root from the JSON artifact with `scripts/verify_inclusion.py` — **committed
  2026-08-08, because the "independent script from Phase 2" this line used to name had never
  been committed at all** (`ROADMAP.md:39` carries the full account). This gate now exists as
  `crates/lys-anchor/tests/stranger_verification.rs`.
- Verify the receipt with the vendored Go `veraison/go-cose`.

**Second party: a different language and a shell.** The axis is *algorithm and toolchain*,
not platform — one machine, one dependency resolution (CLAUDE.md's "say which axis").
**Positive control mandatory**: a shell one-liner needs one, so the shell step must first
produce a known-correct value on a fixture whose hash is written into the test, before it is
trusted on the anchor's output.

### 7.4 Receipts the anchor issues — go-cose parity

Model on `tests/consistency_receipt_conformance.rs:1-46`: lys signs and Go verifies, **and**
Go signs and the artifacts are compared byte-for-byte over a sweep, with the case count
asserted so a zero-iteration loop cannot pass. Byte-identity is available (deterministic
Ed25519, RFC 8949 §4.2 both sides) and is the stronger claim.

The finding that gate produced is the reason to reuse its shape: swapping two proof fields
symmetrically in encoder *and* decoder left all 440 in-crate tests green and failed only the
Go gate (`WIRE-DRAFTS.md:242-251`). **Anything the anchor encodes and decodes with its own
pair of functions is invisible to its own suite.**

### 7.5 The cascade — where the second party is `verify_bundle`

`tests/cascade.rs` (federation): anchor A submits its checkpoint to anchor B; assemble a
bundle; hand it to `verify_bundle` (`bundle/verify.rs:161`) — **code written before
`lys-anchor` existed, by a party that does not know how the cascade was produced.** That is
the strongest available second party for the *relationships*, and it is the point of §4.2's
unification: if cascade and witness were two mechanisms, one of them would have no judge.

The "same mechanism" claim (DP14) is asserted structurally: `upward/pin.rs` calls
`Anchor::submit`, and the test asserts the outcome type is the core's, not a federation
twin — not that two paths produce equivalent results, which is a claim two diverging paths
can satisfy.

### 7.6 Witness observations — the vacuity trap, named

**Every witness negative test opens with a positive control as its first assertion.** Not
caution: a defect this repo has already shipped and caught. Drifting a Go content-type
constant by one character left `go_cose_refuses_what_lys_refuses` **green**, because every
assertion in it said the tool *refused* something, and a verifier that refuses everything
satisfies all of them at once (`WIRE-DRAFTS.md:253-261`).

One rule per case, counts asserted:

| rule | the only case that may fail when it is drifted |
|---|---|
| the record precedes the check | `a_conflicting_checkpoint_is_still_appended` |
| identical resubmission is a no-op | `resubmitting_the_identical_checkpoint_is_Identical` |
| rollback observed | `a_smaller_tree_size_is_Rollback` |
| **equivocation observed** | `the_same_size_with_a_different_root_is_Conflicting` |
| consistency actually checked | `a_forged_consistency_path_is_Unrelated` |
| no prior record implies nothing | `a_first_sighting_reports_previous_None` |
| the check never reaches the artifact | `the_receipt_is_byte_identical_whatever_the_relation` |

Drift each rule and require **exactly one** failure. Count *rules*, not tests: "exactly one
test fails" counts tests while the noun that matters is rules, and a bundled multi-pin case
is already a recorded residual here (`WIRE-DRAFTS.md:339-345`).

The last row is the structural guard of §4.2 point 4, made executable: if a witness receipt
could ever differ from a plain submission's receipt, DP18's no-endorsement property would be
carried by wording rather than by shape.

### 7.7 The delegation format — golden vector, not round trip

A round trip through our own encoder and decoder proves nothing about the wire: any
symmetric change is invisible. Two second parties, both required:

- A **hand-written golden vector** in `WIRE-DRAFTS.md`, and a test that **hardcodes the
  literal rather than importing the constant**, so it cannot move with the code it checks.
  This is the `tests/seal_derivation.rs` pattern, and WIRE-FORMATS records why it exists:
  reversing the two public keys in the seal `info`, and changing the domain tag, each left
  the entire suite green (`WIRE-FORMATS.md:27`).
- **go-cose verification** of the delegation artifact. It is a `COSE_Sign1`, so an
  off-the-shelf library is a real second implementation of the envelope.

Drift injections, one case per rule: wrong content type; root key swapped out of `kid`;
origin omitted from the payload; payload field order permuted symmetrically. Each must fail
exactly one case, and it must be the case built for it.

### 7.8 Origin-is-not-a-constant — a lexical property, checked lexically, with a control

`tests/origin_is_not_a_constant.rs`: walk the committed sources of `lys-anchor` and assert
no configured production origin string appears outside `docs/`.

This is the one place a name-search is the right instrument, because **the property itself
is lexical**: DP15 forbids a committed constant. The repo's caution — that name-search is
structurally blind to the best-built tests (`WIRE-DRAFTS.md:330-336`) — applies to searching
for *behaviour* by name, not to asserting the absence of a string. **The check must carry
its own positive control**: plant the string in a fixture and assert the check fires, in the
same test, first. A guard that only covers the automated path guards the path least likely
to err.

### 7.9 Duplicate submission, and crash recovery

- **Duplicate:** submit identical bytes twice. Assert **two distinct indices**, **two
  receipts**, and that **each verifies independently against its own index** — not merely
  that no error occurred. Count what fired.
- **Recovery:** `Log::open` repairs exactly the one-leaf-ahead state and reports it
  (`log.rs:80-118`, `:122-146`). `lys-anchor`'s job is not to swallow `recovered_to`.
  Fabricate the one-leaf-ahead state on disk, open, assert `Anchor::recovered_to()` is
  `Some`. The second party is the store contract itself — prose written down before this
  crate existed, which is the second party CLAUDE.md's rule prescribes.

### 7.10 The Go harness

`lys-anchor/tests/harness/mod.rs` needs the same environment contract as `lys-core`'s
(`GOFLAGS=-mod=vendor GOPROXY=off GOTOOLCHAIN=local`, a throwaway `GOCACHE`, and
`LYS_REQUIRE_GO` turning a skip into a hard failure — `lys-core/tests/harness/mod.rs:1-17`,
`:55-66`). **That will be a second copy of a contract whose own docs warn that two copies is
two chances to lose a flag** (`harness/mod.rs:8-10`). Either factor it into a shared
dev-dependency crate, or accept the duplication and pin it with a test asserting both
harnesses set the identical environment. Do not leave it unremarked.

---

## 8. What is deliberately NOT in v1

| Not in v1 | Reason |
|---|---|
| **HTTP / any network transport** | §5.3 — DP5's oracle argument, DP17's sequence, nothing retrofitted. **Contingent on F2, which is open and blocking.** |
| **A C2SP cosignature note line** | §4.2 — the receipt already is the countersignature, and its indistinguishability from a plain submission receipt is DP18's structural guard. Needs a key type `NoteVerifierKey` rejects (`verifier_key.rs:117-119`) and an emitter that does not exist (`note.rs:94`). Additive later. |
| **Deduplication of submissions** | Requires a leaf-hash index — a cache whose failure mode is a *false inclusion claim*. Storage growth is what the admission policy is for. **Pre-written rule for whenever it lands: on a cache hit the anchor MUST re-read the leaf via `LeafStore::leaf` (`store.rs:141`) and byte-compare before issuing — the index is a hint, never the authority.** Same discipline as `tlog`'s "redundancy is checked, not trusted" (`tlog/mod.rs:10-14`). |
| **A lookup / query endpoint** | DP5 (`DECISIONS.md:165`), verbatim. |
| **tlog-tiles storage** | DP10 rules files first (`DECISIONS.md:167`). Tiles are a *serving* format and there is no reader here yet. `LeafStore` keeps it a swap. |
| **Consistency receipts (`vdp -2`) issued** | DP2's launch scope (`DECISIONS.md:44-58`). Implemented in `lys-core` (`receipt/consistency.rs:153`) and unused by the anchor. Distinct from the witness path, which *consumes* a consistency proof and issues none. |
| **SCRAPI / RFC 9943 transparent statements** | `STRAWMAN-SESSION.md:161-169`: SCITT answers "what registration policy did this satisfy", and this anchor has no selective policy to describe. |
| **OpenTimestamps counter-anchor** | The bundle slot exists and a populated one is *refused* (`bundle/verify.rs:~177-180`), so nothing is retrofitted. DP11 (`DECISIONS.md:168`). |
| **Cert-gated admission (DP9)** | Trait ships in 6a; the policy is 6b. 🔴 **Blocked on F1.** |
| **Threshold witnesses** | `STRAWMAN-SESSION.md:60-62` — open question; the lean is one witness with format room for more. |
| **Executing a key rotation** | The delegation *format* ships (increment 11). Rotating a live key is an operator ceremony (DP6, `DECISIONS.md:164`) needing a production key, which is Tom's (`DECISIONS.md:252-255`). |
| **A key-history artifact** | **F4** — DP16's verifiability needs one and the bundle has no slot. Its own v1, later. Do not amend `lys/verification-bundle/v1`. |
| **Any log-position claim in an attestation** | DP22. The receipt already binds an entry to its position, and a second statement of one fact is a second thing that can disagree with itself. Note this *removes* a would-be `lys-core` addition (an attestation v3), which is good news for a semver-bound crate. |
| **Publishing anything; any production key; any receipt outside a test** | `DECISIONS.md:246-258`. `publish = false` on the crate is the mechanical guard. |
| **HPKE / `seal` v2** | `STRAWMAN-SESSION.md:178-201` — undecided and unrelated to the anchor. |

---

## 9. The disputes to have before the code exists

Not review comments for afterwards. Each of these is cheaper to lose now than to unpick
later.

1. **F1 — DP9 vs DP13.** 🔴 Open and blocking increment 6b. Not resolved here; a possible
   shape is offered as input to the ruling, not as a position.
2. **F2 — does "the witness API ships in v1" mean an HTTP endpoint?** 🔴 Open and blocking
   increment 12. §5.3 argues one reading; the argument is not a ruling.
3. **F3's recommendation — record, then check, then report.** Not yet ruled. DP18 does not
   decide it either way, and the compatibility argument (checking against what the witness
   itself observed stays inside what a witness can honestly know) is the part most worth
   attacking. If it is rejected, step 2 of §4.3 disappears and nothing else in the plan
   moves — which is itself evidence the boundary is in the right place.
4. **§4.2 — that a receipt *is* a countersignature, so v1 needs no cosignature format.** If
   wrong, the witness surface grows a wire format nobody here has read the spec for, and
   DP18's structural guard reverts to careful wording.
5. **Increment 11's lateness.** Genesis is injected bytes for ten increments and then
   becomes a delegation, changing `Anchor::create`'s signature late. The deferred freeze is
   judged worth the churn. Dispute it now, not after leaf 0 exists somewhere real.
6. **§2.1's `federation` feature being off by default.** It makes the standalone shape the
   one the default gate exercises, which is what DP19 asks for — but every default-off
   feature in this repo has to answer the silent-coverage-loss charge, and the answer here
   is that `--all-features` is already mandatory. If that is judged insufficient, the
   alternative is default-on plus a dedicated `--no-default-features` gate, and the gate set
   grows by one.
