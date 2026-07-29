# lys-anchor — design decisions

**Status: settled by derivation, 2026-07-29.** Decided by Callisto Crisps on Tom's
instruction to draw the answers from the requirements he had already stated rather than
ratify them one by one. Every ruling below shows its derivation, so any of them can be
overturned by disputing the requirement it came from rather than by re-litigating taste.

Supersedes the leans in [STRAWMAN.md](STRAWMAN.md). The byte-exact contracts these rulings
govern are in [WIRE-DRAFTS.md](WIRE-DRAFTS.md), which stays marked DRAFT until the first
one is emitted.

---

## The requirement everything derives from

The stated purpose is not "sign things." It is that a maintainer receiving a contribution
should be able to tell, without having to think about it, whether the contributor did the
work — gates run, tests passed, engaged with review — rather than generating something
plausible and submitting it. The diagnosis was explicit: the AI is not the problem, the
person who takes no responsibility is.

That yields one principle, and it settles every permanent decision below the same way:

> **A claim must not be settable by the party being judged.**

A signature only ever proves that someone holding a key said something. It cannot make the
statement true. So the design question is never "is this signed" but "who signed it, and
could the subject have authored it themselves." Anything a contributor can assert about
their own diligence is worth nothing; the load is carried by claims made by parties they do
not control — the runner that executed the gates, the log that ordered the events, the
witness that countersigned the history.

The second-order requirement follows from "it could be relied upon": verification must not
depend on infrastructure being reachable at check time. A check that fails when a server is
down either blocks everyone or fails open, and both are worse than not having it.

---

## The five permanent decisions

These freeze bytes. Changing one later invalidates artifacts already issued, so they are
decided once.

### DP2 — What the anchor emits on day one

**Ruling: COSE receipts alongside the existing JSON proofs. Not the full standard API, not
JSON alone.**

Derivation. The consumer is a forge deciding whether to trust a contribution, inside CI, in
whatever language the forge is written in. That requires a format stock libraries already
verify — which COSE is, and which a bespoke JSON shape is not. It does *not* require the
surrounding standard API surface: that world is essentially one service in preview, so
front-loading it buys an interface with nothing to test against.

The JSON proof keeps being emitted regardless. A receipt that only specialised tooling can
check violates "verification must outlive the vendor," and the existing fifteen-line
independent verification script is the strongest evidence this project has that its claims
are real. Losing it to gain a nicer encoding would be a regression dressed as progress.

### DP3 — What goes into the certificate log

**Ruling: the whole certificate. A hashed-record alternative exists for genuinely sensitive
claims, its use is itself visible in the log, and the anchor's own CA never uses it.**

Derivation. "Categorised reputation" is only sound if reputation is *derived* from the log
rather than asserted alongside it — otherwise it is a score you must trust, which is the
thing being escaped. Deriving it means asking questions of the log: what else has this key
been certified for, who vouched for this contributor, what has this CA issued. A log of
hashes answers none of those. It can confirm that *something* was issued if you already
have the thing, which is useless for discovering what you were not shown.

This also closes the gap recorded in `ca::request`: a certificate does not record whether
it was issued over a key its holder proved they control, so that property rests on issuer
policy. Logging whole certificates is what makes issuer policy auditable by strangers
instead of promised by the issuer about itself.

Selective disclosure is a real need, hence the hatch — but as the exception, visible when
taken. An anchor that exempted itself from the standard it publishes would be asserting its
own trustworthiness, which the governing principle forbids.

### The verification bundle

**Ruling: one self-contained file carrying every artifact verbatim. Never signed. Chain-link
verification is mandatory, not advisory.**

Derivation. The bundle is what travels with a contribution, so it must be checkable by
someone with the file and nothing else — no network, no lookup, no live service. Artifacts
verbatim, because a bundle that reformats what it carries becomes a thing to trust rather
than a container.

*Never signed* follows directly from the principle. A signature over the container invites a
verifier to check the wrapper and skip the contents, and in a forge that failure has a
specific shape: a green check that verified nothing. Signing packaging offers no property
the enclosed artifacts do not already carry, and buys a way to be wrong.

*Chain links mandatory* is the bundle's one genuinely dangerous failure mode. Each receipt
must prove the previous link's checkpoint; a pile of individually-valid receipts proves
nothing about their relationship. An implementation that verified each receipt and skipped
the joins would pass every receipt and still accept a fabricated history — the same class
of defect as verifying a certificate and an attestation without checking they concern one
identity, which is the bug this project just spent a day closing on the identity side.

### DP12 — How anchors are named

**Ruling: a URL under a domain the operator controls — `anchor.lys.dev/prod-01`. Child
anchors sit under their own domains. Each forge instance is its own origin.**

Derivation. Origins land inside signed checkpoint bytes, so the convention is permanent.
Origins were made mandatory in the first place because collisions are a security defect
rather than an inconvenience — two logs answering to one name is indistinguishable from one
log with two histories, which is precisely the attack the anchor exists to prevent.

Domain-scoping delegates uniqueness to a system that already solves it globally. It also
means a self-hosted forge gets its own origin and anchors upward, rather than needing
permission to exist — which is the federation property, not an accident of naming.

### DP8 — Whether a timestamp rides inside the receipt

**Ruling: no. Time comes from signing over checkpoint bytes; witness cosignatures supply
corroborating time. Public timestamp authorities are never load-bearing.**

Derivation. The property actually wanted is "this record was not manufactured after the
fact" — and that comes from the log's ordering plus independent countersignature, not from a
number inside a receipt. A self-asserted timestamp is exactly a claim settable by the party
being judged, so under the governing principle it must not be the thing anyone relies on.
Including it invites reliance on it.

If a genuine RFC 3161 timestamp is ever required, we run our own, and only when something
concrete demands it. Making an external authority load-bearing would reintroduce the
reachability dependency ruled out above.

---

## G2 — how much a verifier must fetch

**Ruling: Option B — a shallow, self-contained chain. Delegation is carried as signed claims
rather than certificate depth, and a verifier fetches exactly one long-lived root key.**

Derivation, and this one is forced rather than preferred. Option C — deep X.509 path
building with a trust store and CRL/OCSP — makes a contribution's trustworthiness depend on
revocation infrastructure being reachable when the check runs. In a forge that means either
blocking contributions when a network path is down, or failing open and calling it a pass.
The second is the silent-success failure this project refuses; the first makes the check the
least reliable part of the pipeline.

Option A (everything inline, no fetch) fails differently: with no fetched anchor there is
nothing a verifier trusts independently of the artifact, so the bundle becomes
self-referential.

One fetched long-lived root is the smallest thing that still gives a verifier an independent
anchor, and it can be pinned in a config file, baked into a CI image, or committed to the
repository being protected.

---

## The seven operational decisions

These change nothing already issued, so they are chosen for present convenience and expected
to be revised. Recorded so the choices are visible, not because they are settled.

| # | Decision | Choice |
|---|---|---|
| DP4 | Key custody | Software keys in files at first, with the interface kept narrow enough that an HSM or KMS backend is a swap and not a rewrite. `Ed25519Identity` already reaches rcgen through a remote-signer adapter, so the shape exists. |
| DP6 | Ceremony weight | Light. A documented, scripted, reproducible generation with recorded public keys. Heavy ceremony before anyone depends on the anchor is theatre. |
| DP5 | Lookup endpoint | Not on day one. Serving lookups over a network turns decode-success into a parsing oracle, and nothing needs it yet. |
| DP9 | Write-path gate | Certificate-gated, now that G1 makes a certificate mean something. Deliberately *after* the receipt and bundle work: the gate is an access-control decision, and per `ca::request` it must not be built on a request's signature. |
| DP10 | Storage | Files, same layout the existing log store uses. Object storage when scale demands it; the log's backing is already deliberately abstracted. |
| DP11 | Deferrals | Witness cosigning, federation upward, and the reputation derivation all wait until a single anchor issues verifiable receipts. Each is additive. |
| — | Language/crate | A new `lys-anchor` crate in this workspace, depending on published `lys-core`. |

---

## What remains Tom's

**Whether to stand an anchor up, and under what domain.** Building the crate freezes
nothing — a format is frozen by publishing a crate or signing a durable artifact under a
tag, not by writing code that could. So implementation can proceed against these rulings
and stay reversible. What is not reversible, and so is not inferred here:

- publishing `lys-anchor` to crates.io;
- generating a production anchor key;
- emitting any receipt outside a test, under `lys/anchor-receipt/v1` or any other tag;
- choosing the domain an origin is minted under.

Until those, `WIRE-DRAFTS.md` stays DRAFT and every one of the five rulings above is still
cheap to overturn.
