# lys

**Cryptographic trust infrastructure for AI agents.** Identity, tamper-evident history, and verifiable provenance — the accountability layer that lets agents prove their work.

*Lys* is Danish and Norwegian for **light**. Transparency infrastructure has always traded in the metaphor — "sunlight is the best disinfectant," Certificate Transparency's reference log is literally named Sunlight. Lys shines light on agent behaviour: what an agent was authorised to do, what it actually did, and proof that nobody — including the operator — rewrote the record afterwards.

## What it is

- **Agent identity** — Ed25519 keypairs and X.509 capability certificates issued by an instance CA. Identity and permission become one cryptographic object.
- **Tamper-evident history** — RFC 6962 append-only Merkle logs over signed session events. Inclusion proofs ("this happened"), consistency proofs ("nothing was rewritten").
- **Signed attestations** — domain-separated, timestamped statements binding an agent's key to its actions and outputs.
- **Sealed credential transport** — X25519 + AES-256-GCM authenticated envelopes so credentials travel to an agent without the infrastructure operator being able to read them.
- **Anchoring** — periodic publication of log roots to a shared transparency ledger, so history is fixed in a record the operator does not control. Verification without disclosure: auditors check 32-byte roots and proofs, never conversation contents.

## What it is not

Lys is not an authorization system, an observability platform, or a database. Your IdP and policy engine say what an agent *may* do; lys proves what it *did*.

## Status

**Design phase.** The core primitives exist today as the `meridian-trust` crate inside the Meridian workspace (Ed25519 identity, CA, Merkle log, attestations, sealed envelopes — ~110 tests), currently undergoing a security-hardening pass before extraction into this repository.

- [docs/VISION.md](docs/VISION.md) — why this exists and what changes when it works
- [docs/DESIGN.md](docs/DESIGN.md) — architecture, primitive decisions, integration map, roadmap
