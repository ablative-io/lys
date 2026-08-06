# Anchor design session — strawman

**Status: strawman. Written to be attacked, not adopted.** Every section states a
position, the reasoning, and the question I could not settle alone. Nothing here is
ratified and nothing here is built.

Prepared by Callisto Crisps for the design session Tom required before any anchor is
built. Agenda fixed from the prior round: key lifecycle · signed time · federation ·
receipt contents vs SCITT. Two items from the 2026-08-01 stack-ownership audit are added
because they change wire formats and therefore belong in the same conversation.

---

## 1. Key lifecycle — the largest genuine gap

### 1.1 Rotation: the key history *is* the log history

**Position.** An anchor rotates by **appending its own new public key to its own log,
signed by the outgoing key.** Nothing external is consulted. A verifier walking the log
from the beginning sees every handover in order, and each handover is vouched for by the
key that was already trusted at that point.

The origin string stays constant across rotations — it names the *log*, not the key. Only
the key ID in the checkpoint's signature line changes.

**Why this rather than a key list served over HTTPS:** a key list is a second statement
about the same fact, published somewhere the log's own tamper-evidence does not reach. Two
statements that can disagree is the failure this project exists to prevent. Putting the
rotation *in* the log means a lie about the current key is a lie about log contents, which
is the thing already made expensive.

**Consequence to accept deliberately:** verification of a recent artifact may require
walking back to a rotation. That is a cost paid by verifiers, not by signers. It is the
right way round — the cost lands on the party who wants assurance, not the party asking
to be trusted.

### 1.2 Compromise is *not* rotation, and the log cannot fix it

**Position, stated as a limit rather than a solution.** If a key is stolen rather than
retired, the thief can sign a perfectly valid rotation to a key of their choosing, and the
log will record it as legitimate — because from inside the log it *is* legitimate.

No purely internal mechanism escapes this. Anything that could would amount to the log
adjudicating its own operator, which it cannot do; this is the same shape as the limit
already written into `LeafStore`'s docs (*"an actor able to rewrite both the leaves and the
pin can present a consistent shorter log, and no purely local check can catch that"*).

The escapes are all external, and each costs something:

| Escape | What it buys | What it costs |
|---|---|---|
| Witness co-signature (see §3) | A second party must also have seen the checkpoint, so a forked history needs two compromises | A live second party |
| Origin domain control (DNS/TLS) | Binds the log's name to something an attacker must also seize | Reintroduces the CA world we deliberately left |
| Out-of-band announcement | Humans notice | Not verifiable by machine |

**My recommendation: witness co-signature, and say plainly in the docs that a compromised
anchor key is recoverable only with a witness.** An honest stated limit is worth more than
a mechanism that implies more than it delivers.

**Open question for the session:** is one witness enough, or does the design need a
threshold? A threshold is more robust and much harder to operate; my lean is one witness
for v1 with the format leaving room for more.

### 1.3 Custody: never assume raw key access

**Position.** The anchor signs through a **trait**, not through a held `Ed25519Identity`.
v1's implementation is a 0600 file on disk; an HSM or KMS implementation must be
addable without touching a single call site.

This is cheap to do now and expensive to retrofit, because "give me the private key" leaks
into every layer that touches signing if it is allowed to once. `lys-core` already keeps
seeds in `Zeroizing` and keeps private material out of `Debug`, so the discipline exists —
this extends it to the boundary.

**Open:** does the production key ever exist as raw bytes on a machine, even once, at
generation? If not, generation must happen inside whatever holds it, which constrains the
choice of custody from day one rather than later.

---

## 2. Signed time — the log already keeps better time than the clock

### 2.1 The current position is weak and the audit confirmed it

Every attestation carries `timestamp: i64` taken from the signer's own clock
(`attestation/sign.rs`), authenticated but entirely self-asserted: an offline signer can
name any instant. The 2026-08-01 audit established by word-boundary grep that **nothing
anywhere in the repo binds an attestation to a log position** — the only association is an
unmodelled user convention of appending a signed artifact as a leaf.

### 2.2 Position: log order is the real timestamp

A checkpoint says *at tree size N the root was R*. Any entry at index `i < N` was in the
log before that checkpoint was signed — and because the log is append-only, its position
relative to every other entry is fixed and provable.

That yields something a clock cannot: **verifiable ordering between two events, checkable
by a stranger, with no trust in either signer's clock.** What it does not yield is absolute
time. Absolute time enters only when a checkpoint reaches a party who timestamps its
arrival — which is exactly what a witness is.

**So: order comes from the log, absolute time comes from witnesses, and the wall clock is
advisory.** I would document the `timestamp` field as advisory in so many words.

### 2.3 Recommendation

- **Add an optional signed log-position claim to attestations** — a new `v3` alongside
  `v2`, never a mutation, per the forever rule.
- **No RFC 3161 timestamp authority by default.** A TSA is a third party, and the standing
  ruling is *hostable, never third-party*. Leave it possible for a consumer who needs it,
  and do not build the anchor around one.

**Open:** does the log-position claim go in the attestation (signer asserts where it
landed) or only in the receipt (anchor asserts it)? The receipt is the honest place — the
signer cannot know its own index before submitting. That may mean the answer is *no
attestation change at all*, and the audit finding is satisfied entirely by receipts. I lean
that way and want it argued.

---

## 3. Federation — a witness need not understand the log

### 3.1 Position: federation is just leaves

An anchor is a lys instance. A parent anchor witnesses a child by **appending the child's
signed checkpoints as its own leaves.** That is the whole mechanism. Recursion is free
because nothing about the parent is special.

### 3.2 The parent must NOT verify the child

**Position, and this is the part I most want attacked.** A parent stores what it is handed
and does not check the child's internal consistency.

Reasoning: verification would require the parent to hold the child's entire tree. That
does not scale, and it re-centralises — the parent becomes an authority over the child,
which is the architecture we are trying not to rebuild.

And it is unnecessary. What defeats equivocation is not that the witness *understood* the
checkpoint but that the witness *possesses* it. If a log shows two different roots at one
size, both signed, both witnessed, the contradiction is provable by anyone holding the two
witnessed records — the witness never needed to detect it. **A witness is a durable memory,
not an auditor.**

This mirrors the ruling already made for the local pin: the pin's job is not cryptographic
assurance, it is detection of a change; real tamper-evidence lives in artifacts already in
someone else's hands.

**Open:** should the parent refuse a checkpoint whose *signature* is invalid? That check is
cheap and stateless, unlike consistency. My lean is yes — refuse garbage, verify nothing
about history.

---

## 4. Receipt contents, and whether SCITT applies

**Position: stay minimal. A receipt proves inclusion or consistency and says nothing else.**

Specifically, do **not** add an issuer name, a policy identifier, or a submission
timestamp. The anchor is identified by its key in the signature-covered `kid`, and a second
name for the same thing is a second thing that can disagree with itself. This repeats the
reasoning already applied when the origin was deliberately kept out of the receipt.

**On SCITT (RFC 9943):** it answers a question we do not yet have — *what registration
policy did this statement satisfy?* That matters when a log admits entries selectively. Our
anchor does not: it appends what it is given, and the log makes no claim about whether an
entry is *true*, only that it is *recorded*.

**Recommendation:** stay with RFC 9942 `vdp` receipts. Revisit only when a named consumer
needs SCITT interop — and if that day comes, adopt the transparent-statement envelope
rather than reinventing near it.

---

## 5. Two format decisions carried in from the audit

Both are Tom's to ratify; both change wire behaviour on shipped formats.

### 5.1 `seal` vs RFC 9180 HPKE

`seal` is a bespoke X25519 + HKDF-SHA256 + AES-256-GCM construction described in its own
docs as the NaCl `crypto_box_seal` pattern. HPKE standardises exactly this, with a
registered ciphersuite matching our primitives and independent implementations in several
languages. `hpke` appears nowhere in this repo, and **no document records a decision to
reject it** — so the one non-standard construction in the crate currently reads as an
oversight rather than a choice.

It is also **the only frozen wire contract in `lys-core` with no second implementation of
any kind** — every other artifact is cross-checked against Go, and `seal` is not, because
no other implementation of it exists or ever will.

**Recommendation: migrate to HPKE as `lys/sealed-envelope/v2`, keeping v1 verifiable
forever.** Against the governing rule — *strangers must be able to verify it* — a format
only we can implement is the sharpest inconsistency in the codebase.

**Second, independent of that decision:** `SealedEnvelope` is the only serde-deriving type
in `lys-core` without `deny_unknown_fields`, while `lys/sealed-envelope/v1` is equally
frozen. And there is an unproven hypothesis on record that the KDF is
symmetric-change-invisible — no golden vector pins the construction. **That drift injection
should run before any migration decision**, because it determines whether v1 was ever
pinned at all.

### 5.2 Attestation log-position claim

Covered in §2.3. Likely resolved by receipts rather than by a new attestation version — but
it needs deciding rather than defaulting.

---

## What I am asking for from the session

1. **Rule on §1.2**: witness co-signature as the compromise story, and the limit stated
   plainly in the docs.
2. **Rule on §3.2**: witness stores, does not verify. This is the load-bearing
   architectural claim and I want it attacked before it is built on.
3. **Decide §2.3**: does the position claim live in the receipt only?
4. **Decide §5.1**: HPKE migration, yes or no. If no, the reason gets written down where
   the next reader will find it.
5. **Ratify or send back** the two `vdp` wire drafts.

Origin domain and production key are downstream of all of the above and should be settled
after, not during — they are the point at which formats freeze for real.
