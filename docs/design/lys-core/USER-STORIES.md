# Lys-Core — User Stories

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
