# Lys-Gate — Checklist

## The gate's test leg

- [ ] **C1** — docs/design/project.json's 'tests' leg command is exactly `cargo test --workspace --all-features --no-fail-fast`.
- [ ] **C2** — The 'tests' leg command of the directory, home, secrets and lys-gate design.json gate arrays is exactly `cargo test --workspace --all-features --no-fail-fast`, and every other leg is byte-identical to before.
- [ ] **C3** — .land/gates.sh's test leg line is exactly `leg cargo test --workspace --all-features --no-fail-fast`.
- [ ] **C4** — CLAUDE.md's 'Gates before any commit' test line is exactly `cargo test --workspace --all-features --no-fail-fast`, the same command as .land/gates.sh's test leg.
- [ ] **C5** — .github/workflows/ci.yml's Test step runs exactly `cargo test --workspace --no-fail-fast`.
- [ ] **C6** — The changes of C1 to C5 are one commit, no other commit on the card's branch changes those files except that the lys-gate design.json's gate array alone is held unchanged outside that commit, and the grep over docs/design/project.json, the directory, home, secrets and lys-gate design.json files and .land/gates.sh, named one by one, for test legs without --no-fail-fast prints 0.

## Gate evidence

- [ ] **C7** — sh scripts/design/gate.sh exits 0 at the card's head, and four green gate rounds at the change, one measured against each of the directory, home, secrets and lys-gate design.json files, each show the tests leg's '$ command' line as `$ cargo test --workspace --all-features --no-fail-fast`.
- [ ] **C8** — One gate round over a local scratch branch with one planted failing test in crates/lys shows exactly one failed test result and as many test results as the lys-gate green round at the same head, and no ref under scratch/ exists on origin after it.
- [ ] **C9** — docs/design/lys-gate/PROOF-LYSGATE-001.md names each of the four green rounds and the scratch round by round id, log path in the gate log store and SHA-256, and the scratch round's counts, holds no line of any of those logs, and lists by file name every other cluster's design.json whose test leg lacks --no-fail-fast as a finding for that cluster's card.
