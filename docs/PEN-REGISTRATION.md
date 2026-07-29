# IANA Private Enterprise Number — the lys OID arc

**Status: ASSIGNED — PEN 66364. The lys arc is `1.3.6.1.4.1.66364`, permanently.**

## What this is

A Private Enterprise Number (PEN) is a permanent, public, free identifier IANA
assigns to an organisation, rooted at `1.3.6.1.4.1.<PEN>` in the global OID
tree. It is the namespace under which lys defines its X.509 certificate
extensions — today the capability-claims extension, later the identity/issuer
extensions the agent-identity work will add.

IANA assigned us **66364**, so `LYS_OID_ARC = 1.3.6.1.4.1.66364`
(`crates/lys-core/src/ca/extensions.rs`). One registration covers the
organisation forever: sub-arcs beneath it are ours to allocate and never
involve IANA again. There was exactly **one** registration to make, ever, and
this was it — submitted through the IANA PEN application form
(https://pen.iana.org/pen/PenApplication.page, free, no account needed), with
the registrant of record listed in the public registry at
https://www.iana.org/assignments/enterprise-numbers/.

Precedent: Sigstore's entire certificate-extension story hangs off their
registered PEN 57264 (`1.3.6.1.4.1.57264.1.*`). One number, managed sub-arcs,
permanent.

## History: the 58888 placeholder

Before the assignment landed the arc ended in `58888` — a number that was
never ours, carried deliberately as a development placeholder. It is recorded
here so the provenance of the change stays legible, and nowhere else.

**Nothing durable was ever signed under it.** No release published, no
long-lived certificate issued; the only certificates minted under the
placeholder were test fixtures that re-mint on every run. Registering before
0.1.0 was the whole point — an arc is a wire contract, and the placeholder
could have collided with somebody else's registered space.

## What the adoption touched

Not the "single constant" this document once predicted. The dotted-decimal
form is spelled out in prose, in `--help` text, and in hardcoded test OIDs, so
the flip touched ten files across both crates and the design docs:

- `LYS_OID_ARC` and its doc comment in `crates/lys-core/src/ca/extensions.rs`
- the hardcoded test OIDs in `crates/lys-core/src/ca/extensions_tests.rs`,
  `crates/lys/src/commands/ca.rs`, and `crates/lys/tests/cli_tests.rs`
- the `lys ca issue` help text in `crates/lys/src/cli.rs`, and the module and
  item docs in `crates/lys/src/commands/ca.rs`
- this document, `docs/DESIGN.md`, `docs/RELEASE-CHECKLIST.md`,
  `docs/design/lys-core/DESIGN.md`, and `docs/design/lys-core/CHECKLIST.md`

The help-text copy is the one a future arc change could silently strand, so it
is pinned: a test renders the `ca issue` help and asserts it names the OID that
`capability_claims_oid()` actually builds.

## Sub-arc allocation under `1.3.6.1.4.1.66364`

Managed by us, documented here as the single source of truth:

| Arc | Purpose | Status |
|---|---|---|
| `.1` | Capability claims (certificate extension carrying operator-reviewed claim bytes; today's `lys ca issue --claims`) | In use |
| `.2` | Reserved: identity/issuer extensions for agent certificates (Sigstore-`.1.8`-style — which issuer vouched, runtime identity, session binding) — allocated when the agent-identity design lands | Reserved |
| `.3+` | Unallocated | — |

Never reuse or renumber an arc once anything has been signed under it — the
same wire-formats-are-forever rule that governs everything else.
