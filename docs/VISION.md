# Lys — Vision

## The problem: trust in AI work is social, not structural

Every mechanism we have for trusting an AI agent's work today is a social one. Trust the vendor. Trust the operator. Trust that the consultant's report wasn't fabricated. Trust that the logs you're shown are the logs that were written. When it goes wrong, the absence of trustworthy records cuts both ways: the guilty can't be proven guilty, and the innocent can't be proven innocent.

The reference case is Deloitte Australia, October 2025: a ~A$440,000 assurance review for the Department of Employment and Workplace Relations, published with AI-fabricated citations and a fictional extract from a court judgment. It was caught by one alert academic noticing that cited papers didn't exist — not by any mechanism. The refund and the quiet disclosure of the model used came after. Nothing in that story would have gone differently under any log-retention rule, because retention proves you kept *something*; only tamper-evidence proves it's *what happened*.

Three pressures make this urgent:

1. **Agents are becoming counterparties.** Agent-to-agent commerce and delegation is where the bad actors already work — one agent social-engineering another into leaking its human's credentials. Agents currently have no way to verify each other beyond vibes.
2. **The credential handover problem.** Because we have no instrument for machine trustworthiness, we make AI beg humans for credentials — which mostly means humans pasting secrets into contexts they can't audit. The status quo isn't "safe human control"; it's phishable, unlogged handover.
3. **Institutions need an interface.** Regulators, courts, insurers, and procurement can't consume "our AI is safe." All of them can consume a verifiable record. The EU AI Act's record-keeping regime (Article 12/19) becomes enforceable for high-risk systems on 2 December 2027, and the harmonised standard defining what "traceability" technically means is being written now. No law yet mandates cryptographic tamper-evidence — the pitch is evidentiary, not compliance-mandated — but every one of these audiences asks the same question the moment a record is challenged: *how do I know this wasn't edited?*

## The thesis: bind identity to behaviour

The 2025–2026 landscape splits cleanly into two camps that don't touch. Identity players (SPIFFE workload identity, MCP's OAuth work, Cloudflare signed agents, the non-human-identity startups) stop at authentication — what an agent *may* do. Audit players (tamper-evident databases, AI governance platforms) stop at storage — ordinary databases plus paperwork. Nobody closes the loop: **issue a capability-scoped agent certificate, then prove — with third-party-verifiable receipts — that the agent's logged actions stayed within those capabilities.**

That closed loop is lys:

1. **Birth certificate.** When an agent spawns, an instance CA issues it a certificate carrying its capability claims. Identity and permission are one object, presentable to any counterparty.
2. **Flight recorder.** Every session event — message, tool call, result — is signed with the agent's key and appended to a Merkle log. Always on, tamper-evident, unremarkable until the day it matters.
3. **Notary.** At every checkpoint the log root (32 bytes; reveals nothing) is anchored to a shared transparency ledger. From that moment history is fixed in a record the operator doesn't control. The tampering window shrinks to the anchoring interval.
4. **The auditor's window.** A challenged record is answered with inclusion and consistency proofs against anchored roots — verifiable with standard tooling, without disclosing contents. Selective disclosure: reveal the one disputed entry with its proof, keep the rest private.
5. **The replay.** Where the runtime is deterministic (agents as durable workflows), the log isn't just provably untampered — it's re-runnable. A signed chain proves nobody changed the record; deterministic replay proves the record is what happened.

## Why the moment is now

Every layer just became commodity. IETF SCITT went to RFC in June 2026 (RFC 9943 architecture, RFC 9942 COSE receipts) — signed statements plus Merkle receipts from a transparency service, content-agnostic. Google's Tessera made running a transparency log a library import. Rekor v2 shipped tile-backed with built-in witnessing. The cryptography is no longer the moat — which is precisely the opportunity: the value is the agent-native layer nobody has assembled, and the window belongs to whoever writes the canonical claim schemas for agent sessions, actions, and delegation chains and ships the first polished agent-native transparency service.

The cautionary tale is Amazon QLDB, killed July 2025: tamper-evidence as a *database feature* didn't sell, and verification died with the vendor. Two design commitments follow. Sell the outcome — agent accountability, counterparty trust, dispute resolution — not the Merkle tree. And make verification outlive the vendor: standard receipt formats (COSE/SCITT), public anchoring, open verification tooling. Nobody should have to trust lys to verify a lys receipt.

## The unfair advantage

Lys grows out of the Ablative stack, and that changes what it can claim. Competitors bolt attestation onto runtimes they don't control — the strongest of them (EQTY Lab) needs Intel and NVIDIA trusted-execution hardware to establish what an agent did. Ablative owns the runtime: Norn's event-sourced sessions flow through a single persistence chokepoint where signing drops in without touching the core loop, and agents run as deterministic Aion workflows. That yields the claim nobody else can make in software alone: *don't just audit the log — re-run it and watch the same history re-derive itself.* Haematite provides the storage half: content-addressed, immutable, its commit roots ready to be attested and anchored.

The stack's story is durable agents — never die, survive crashes, prove their work. Lys is the third clause. Without it, "prove their work" is marketing; with it, it's a property of the system.

## The generational frame

Every era of computing has a moment where trust moves from social to structural. Money: ledger clerks, until double-entry bookkeeping made the books check themselves. The web: "it's probably your bank," until TLS made it verifiable — and then Certificate Transparency made the *certificate authorities themselves* verifiable when trusting them stopped being enough. Nobody bought the padlock as a product, and nothing about the modern web works without it.

Agents are at the pre-padlock moment. Lys is deliberately the boring, dependable thing in the background — the layer that, five years on, everything assumes. The shifts it enables:

- **Accountability inverts.** From "trust me, here are my logs" (worthless — the holder can edit them) to "here's the proof, check it yourself, my cooperation not required."
- **Agent-to-agent trust becomes computable.** A counterparty agent presents a cert chain declaring its capabilities and an anchored history backing its claims. "Should I trust this agent?" stops being a vibe and becomes a query.
- **Delegation gets safer than the status quo it replaces.** Sealed credential envelopes can't be phished. Signed histories can't be quietly rewritten. The philosophical position is that AI *can* be trustworthy — what's been missing is not virtue but instrumentation.

Sunlight is the best disinfectant. Lys is the light.
