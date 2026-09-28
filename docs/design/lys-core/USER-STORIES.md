# Lys-Core — User Stories

## Card author — Landing work in lys

**S30.** As a card author landing work in lys, I want the gate to refuse an unwrap, a lint bypass, a dropped Result or logic in a mod.rs so that a rule break is caught before review rather than by a reviewer's memory.

## Test writer — Writing tests in sibling files

**S31.** As a test writer, I want a test file recognised as test code by a marker it carries so that I can unwrap in test helpers without an #[allow].

## Lead — Trusting the gate

**S32.** As the lead for lys, I want proof that the unwrap rule fires so that a green leg means the rule held and not that nothing was measured.

## Reader of the design — Tracing the earlier lys-core design

**S33.** As a reader of the lys-core design, I want the hand-written pre-method documents kept under their own names so that the earlier design and the citations of it survive the method's render.

## Norn Agent Runtime — Signing Session History (primary consumer)

**S1.** As the Norn runtime, I want a signing `PersistenceSink` decorator to attest each session event with the agent's Ed25519 identity so that every persisted event carries a verifiable, timestamp-authenticated signature without any core-loop change.

**S2.** As the Norn runtime, I want to append each event's hash to a per-session `AppendOnlyTree` so that the session history becomes tamper-evident from the first event.

**S3.** As the Norn runtime, I want to export the session root via `RootHash::to_parts()` at every `checkpoint()` so that the 32-byte root can later be anchored externally without revealing session contents.

**S4.** As the Norn runtime, I want to issue an X.509 certificate at agent spawn with capability claims embedded as a `LYS_OID_ARC` extension so that the agent's identity and permissions are one presentable object.

**S5.** As the Norn runtime, I want to verify a counterparty's certificate chain at the MCP boundary — at the dispatch instant, via `verify_certificate_chain_at` — so that cross-agent tool calls are gated on legitimate, unexpired, capability-scoped identity.

**S6.** As the Norn runtime, I want spawned agents to load their identity from `LYS_IDENTITY_KEY` so that key material reaches an agent process through its environment without touching shared disk.

**S7.** As the Norn runtime, I want `reconstruct_from_leaves` to rebuild a session tree from the persisted event sequence after a crash so that the recovered tree's root matches the last checkpointed root exactly.

**S8.** As the Norn runtime, I want concurrent agent processes calling `load_or_generate` on the same key path to converge on one persisted key so that a spawn race never leaves an agent holding an identity that differs from the key on disk.

## Lys CLI — Operator and Auditor

**S9.** As an operator, I want `lys key` to generate and inspect identities without ever printing private material so that key handling over a shoulder-surfable terminal is safe by construction.

**S10.** As an operator, I want `lys ca issue` to mint certificates with capability-claim extensions from my instance CA key so that I can provision agent identities from the command line.

**S11.** As an auditor, I want `lys ca verify` with an explicit instant so that I can check whether a certificate was valid at the time a disputed action occurred, not just at the time of my audit.

**S12.** As an operator, I want `lys attest` to sign a file or stdin so that I can hand a third party a detached, self-contained attestation over any artifact.

**S13.** As an auditor, I want `lys verify` to check an attestation with only the payload and the signer's public key so that verification requires nothing from the party who produced the record.

**S14.** As an auditor, I want `lys log verify` to prove inclusion from only a published root and proof bytes so that I can confirm a challenged entry was logged without the operator's cooperation and without seeing any other entry.

**S15.** As an auditor, I want `lys log verify` to check consistency between two published roots so that I can detect any rewrite of history between two points in time.

**S16.** As an operator, I want `lys seal` and `lys open` so that I can move a credential file to a specific recipient with per-envelope forward secrecy instead of pasting secrets into a chat.

## Lys-Anchor Service — (future notary)

**S17.** As the anchor service, I want to reconstruct submitted roots via `RootHash::from_parts` so that I can verify consistency proofs between an instance's successive submissions while seeing only roots, never contents.

**S18.** As the anchor service, I want to verify the submitter's v1 domain-separated attestation over each submitted root so that only the holder of the registered instance key can extend that instance's anchored history.

**S19.** As the anchor service, I want to append anchored roots to my own `AppendOnlyTree` and serve inclusion proofs so that my receipt for an anchoring event is itself independently verifiable.

**S20.** As the anchor service, I want the wire tags and preimage layouts frozen at `v1` so that a receipt issued today still verifies against signatures produced years from now.

## Haematite — Commit Attestation

**S21.** As haematite, I want to attest a BLAKE3 commit root — 32 opaque bytes signed with the instance identity — so that a whole database state becomes attestable without lys knowing anything about haematite's hash world.

**S22.** As haematite, I want to append commit attestations to an append-only log so that my flat timestamped commit list gains the hash-chained lineage it structurally lacks.

**S23.** As haematite, I want inclusion proofs over the commit log so that any party can verify a historical commit belongs to the canonical lineage without replaying the database.

## Norn Agent Runtime — Signing Session History (primary consumer)

**S5.** As the Norn runtime, I want to verify a counterparty's certificate chain at the MCP boundary — at the dispatch instant, via `verify_certificate_chain_at` — so that cross-agent tool calls are gated on legitimate, unexpired, capability-scoped identity.

## Lys CLI Operator and Auditor — Checking certificates and certified statements

**S11.** As an auditor, I want `lys ca verify` with an explicit instant so that I can check whether a certificate was valid at the time a disputed action occurred, not just at the time of my audit.

## Card author — Landing work in lys

**S24.** As a card author landing work in lys, I want the gate to refuse an underscore-prefixed binding with a message saying why so that an unused warning is fixed at its cause rather than silenced.

## Test writer — Reading a test's fixtures

**S25.** As a test writer, I want a temporary directory's guard named and dropped by name so that I can see where the directory's life ends.

## Third-party verifier — Verifying lys artifacts

**S26.** As a third party verifying a lys artifact, I want every verification failure to keep returning the one uniform error so that the cleanup reveals nothing about which check failed.

## Consumer of lys-log-store — Implementing LeafStore

**S27.** As a consumer implementing lys-log-store's LeafStore, I want the trait left unchanged so that my implementation still compiles against the next release.

## Lead — Trusting the gate

**S28.** As the lead for lys, I want the rule shown to fire once for each binding position so that zero hits on the tree means the rule held and not that nothing was measured.

**S29.** As the lead for lys, I want the full suite to run the same tests before and after the card so that I know the cleanup changed no behaviour.

## Lys maintainer — Landing a card through the gate

**S1.** As a lys maintainer landing a card, I want the gate to refuse unwrap, expect and panic in library code so that a panic path cannot land where clippy is silenced.

**S2.** As a lys maintainer landing a card, I want a lint bypass attribute refused at landing so that a lint is fixed at its cause instead of hidden.

**S3.** As a lys maintainer landing a card, I want the same scan run by the design gate, the landing gate and CI so that no path to main skips it.

**S4.** As a lys maintainer landing a card, I want every rule shown to fire on a scratch case so that a silent scan means clean code rather than a rule that never matches.

## Lys contributor — Writing tests and library code

**S5.** As a lys contributor writing tests, I want a test file recognised by the marker it carries so that its helpers need no per-file lint opt-out and no one keeps an exemption list.

**S6.** As a lys contributor reading the tree's rules, I want CLAUDE.md, Cargo.toml and the lib.rs comment to state the policy the gate enforces so that the written rule and the enforced rule agree.

**S7.** As a lys contributor changing key loading, I want the env-backed identity tests to run without unsafe code so that the test build needs no lint bypass.

**S8.** As a lys contributor reading a module tree, I want mod.rs files to hold only declarations and re-exports so that logic is found in a named file.

**S9.** As a lys contributor, I want no `let _ =` discard in the tree so that no error is swallowed without a decision.

## Design reader — Reading the lys-core cluster

**S10.** As a reader of the lys-core design, I want the hand-written pre-method documents kept beside the rendered cluster so that the earlier design is not lost when the method's documents replace it.
