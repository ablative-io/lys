# PROOF-LYSGATE-001

Evidence for LYSGATE-001. The change is commit bc7cecd3c05cfb2c0114a465cc3641b91614b912 (<change>). Logs are named by path and SHA-256 only; no line of any log is copied here.

## R6: the grep over the declared test legs

At <change>:

grep -rn 'cargo test --workspace --all-features' docs/design/project.json docs/design/directory/design.json docs/design/home/design.json docs/design/secrets/design.json docs/design/lys-gate/design.json .land/gates.sh | grep -vc -- --no-fail-fast
prints: 0

At <change>~1 the same grep prints 6.

## Rounds

The rounds were run by hand at the venue that serves heavy builds and full gates, not by the ledger, so they carry no ledger round id and their logs sit in a hand log store on that venue, not in the ledger's gate log store. Each log opens with the tests leg header and its command line, then cargo's output, then the exit line.

Log store: ~/archie-scratch/lysgate/logs on the build venue.

| Round | Head | Measured command | Log | SHA-256 |
|---|---|---|---|---|
| green (tests leg, measured against docs/design/lys-gate/design.json) | bc7cecd | `cargo test --workspace --all-features --no-fail-fast` | logs/green-bc7cecd.log | e54206166b6d764b10bf406ef6d3d747b0e74395f358d836221839817e75f028 |
| scratch (tests leg, measured against docs/design/lys-gate/design.json) | 80ba012, local branch scratch/lysgate-001 on bc7cecd | `cargo test --workspace --all-features --no-fail-fast` | logs/scratch-80ba012.log | 511423c532156cc71a5226caaefb1b9242e5ee63ea4f3ab64b451755dd9d34c5 |

The tests leg command is identical in the directory, home, secrets and lys-gate gates, so the one green run measures the command each of the four declares. Ledger rounds per cluster are not recorded here.

## Counts (R8)

- Green log: 48 `test result: ` lines, 0 `test result: FAILED` lines, exit 0.
- Scratch log: 48 `test result: ` lines, 1 `test result: FAILED` line, the planted `scratch_planted_failure` reported FAILED, exit 101.
- Both logs run 40 test binaries. The planted test sits in crates/lys, the first workspace member, and every binary after it still ran and reported.

## Scratch branch

scratch/lysgate-001 held one commit adding scratch_planted_failure (assert_eq!(1, 2)) to crates/lys/tests/certified_attestation_tests.rs. It was never pushed; its worktree and branch were deleted after the round.

git ls-remote origin 'refs/heads/scratch/*'
prints nothing.

## Findings for other cards

docs/design/roots/design.json
