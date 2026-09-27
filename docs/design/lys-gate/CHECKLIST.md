# Lys-Gate — Checklist

## The gate's test leg

- [ ] **C1** — docs/design/project.json's 'tests' leg command is exactly `cargo test --workspace --all-features --no-fail-fast`.
- [ ] **C2** — The 'tests' leg command of the directory, home, secrets and lys-gate design.json gate arrays is exactly `cargo test --workspace --all-features --no-fail-fast`, and every other leg is byte-identical to before.
- [ ] **C3** — .land/gates.sh's test leg line is exactly `leg cargo test --workspace --all-features --no-fail-fast`.
- [ ] **C4** — CLAUDE.md's 'Gates before any commit' test line is exactly `cargo test --workspace --all-features --no-fail-fast`, the same command as .land/gates.sh's test leg.
- [ ] **C5** — .github/workflows/ci.yml's Test step runs exactly `cargo test --workspace --no-fail-fast`.
- [ ] **C6** — The changes of C1 to C5 are one commit, and the grep over docs/design/project.json, every docs/design/*/design.json and .land/gates.sh for test legs without --no-fail-fast prints 0.

## Gate evidence

- [ ] **C7** — sh scripts/design/gate.sh exits 0 at the card's head, and the green gate round's log shows the tests leg's '$ command' line as `$ cargo test --workspace --all-features --no-fail-fast`.
- [ ] **C8** — One gate round over a local scratch branch with one planted failing test in crates/lys shows exactly one failed test result and as many test results as the green round at the same head, and no ref under scratch/ exists on origin after it.
- [ ] **C9** — docs/design/lys-gate/PROOF-LYSGATE-001.md names the scratch round's log by its path in the gate log store, its SHA-256 and its counts, and holds no line of the log.
