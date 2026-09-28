# PROOF-LYSGATE-002

Evidence for LYSGATE-002: the file-length leg, `sh scripts/file-length.sh`, is green on the card's tree and red on a planted file of 501 lines of code.

Both runs were made by hand on the developer's machine in a separate worktree of a local branch, scratch/lysgate-002, cut from the card's base 3f48732. Neither is a ledger round; the card round's own gate record carries the leg's run for this cluster.

## Green

Commit 9d447cc5351cfe28152c1cad5f970a62b01d96a3 on scratch/lysgate-002: 3f48732 plus this round's whole change (the checker, its tests, the leg's script and every declaration), without this file. Its tree is 984d1c3f66d9dff31c11583ae0118049ffe02d2d.

`sh scripts/file-length.sh` ran the checker's 23 tests (OK), then the checker, and exited 0. Its last line:

```
file-length: 498 files measured, 0 over the limit
```

## Red

Commit fd3cfbae5230cc48dea6e0fc6d4d2a645ae2ec78 on scratch/lysgate-002, the green commit plus one file, crates/lys/src/length_probe.rs: 1004 raw lines, of which 501 are `pub const PROBE_n: u32 = n; // trailing comment` code lines, 502 are `//!` or `///` comment lines and one is blank.

`sh scripts/file-length.sh` ran the tests (OK), then the checker, and exited 1. It printed exactly one over-limit line, and its last two lines were:

```
crates/lys/src/length_probe.rs: 501 lines of code (limit 500)
file-length: 499 files measured, 1 over the limit
```

## Scratch branch

scratch/lysgate-002 was never pushed. Its worktree and branch were deleted after the red run, so both commits above are unreachable and exist only as named here.

git ls-remote origin 'refs/heads/scratch/*'
prints nothing, and no ref on origin holds length_probe.rs.
