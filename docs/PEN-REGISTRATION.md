# IANA Private Enterprise Number — what to register and why

**Status: ACTION NEEDED (operator). Nothing blocks this — register now.**

## What this is

A Private Enterprise Number (PEN) is a permanent, public, free identifier IANA
assigns to an organisation, rooted at `1.3.6.1.4.1.<PEN>` in the global OID
tree. It is the namespace under which lys defines its X.509 certificate
extensions — today the capability-claims extension, later the identity/issuer
extensions the agent-identity work will add.

lys currently uses a **placeholder**: `LYS_OID_ARC = 1.3.6.1.4.1.58888`
(`crates/lys-core/src/ca/extensions.rs:29`). 58888 is not ours. Any
certificate minted before the real PEN lands carries an OID arc that could
collide with someone else's registered space — fine for development, not
acceptable in anything durable. **The real PEN must land before 0.1.0
publishes or anything long-lived is signed.**

Precedent: Sigstore's entire certificate-extension story hangs off their
registered PEN 57264 (`1.3.6.1.4.1.57264.1.*`). One number, managed sub-arcs,
permanent.

## Does anything need to be developed first?

**No.** The registration is independent of all code. One PEN covers the
organisation forever — sub-arcs beneath it are ours to allocate and never
involve IANA again. There is exactly **one** registration to make, ever.

Timing note: IANA turnaround is typically days to a few weeks, so submitting
now means the number is in hand well before 0.1.0. Registering early costs
nothing; registering late gates the release.

## What to submit

Form: **https://pen.iana.org/pen/PenApplication.page** (IANA PEN application —
free, no account needed).

| Field | Value to enter |
|---|---|
| Organisation name | The legal entity name — e.g. `Ablative Pty Ltd` (use the exact registered business name; this appears verbatim in the public registry) |
| Organisation address | The business address |
| Contact name | Tom Whiting |
| Contact email | `tom@ablative.com.au` (public in the registry — use a role address like `iana@ablative.com.au` instead if you prefer not to expose a personal one) |
| Contact phone | Business number (registry-public as well) |

That's the whole application — Bob's your uncle. IANA emails the assigned
number; the registration is permanent, free, and appears in the public
registry at https://www.iana.org/assignments/enterprise-numbers/.

## What happens after assignment

One small, mechanical change plus docs — I handle all of it:

1. `LYS_OID_ARC` flips from `58888` to the assigned number (single constant).
2. The OID literals in docs, `--help` text, and tests update to match.
3. Test certificates re-mint automatically (nothing durable was ever signed
   under the placeholder — that is the point of doing this before 0.1.0).

## Planned sub-arc allocation under `1.3.6.1.4.1.<PEN>`

Managed by us, documented here as the single source of truth:

| Arc | Purpose | Status |
|---|---|---|
| `.1` | Capability claims (certificate extension carrying operator-reviewed claim bytes; today's `lys ca issue --claims`) | In use (under placeholder) |
| `.2` | Reserved: identity/issuer extensions for agent certificates (Sigstore-`.1.8`-style — which issuer vouched, runtime identity, session binding) — allocated when the agent-identity design lands | Reserved |
| `.3+` | Unallocated | — |

Never reuse or renumber an arc once anything has been signed under it — the
same wire-formats-are-forever rule that governs everything else.
