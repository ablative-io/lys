# Second backends for `LeafStore`

`lys-log-store` ships one implementation, `FileLeafStore`. The trait exists so
there can be others. This records what a second one has to satisfy, so that
adopting a backend is a check against written criteria rather than a judgement
someone has to remember.

## What a backend must satisfy

The trait's contract is in the `store` module's rustdoc; these are the parts that
are hard to retrofit and therefore worth checking before writing an
implementation, not after.

1. **Durable on return.** `Ok` from `put_leaf` or `pin` means the bytes reached
   stable storage. Not buffered, not queued, not "will be flushed at the next
   commit". A backend that acknowledges writes it has not persisted manufactures
   a hole the log has already counted, arriving from the one direction the log's
   own checks cannot attribute to anybody — they are built to catch a dishonest
   operator, and a lost write is not one.
2. **Write-once, enforced by the backend.** A second write to an occupied index
   must be refused. Not overwritten, and not silently accepted.
3. **Contiguity established at open.** `extent()` promises every index below it
   is present. A backend that computes this lazily, or trusts a counter it stored
   rather than what it holds, has not finished opening.
4. **Read-back of every stored leaf.** The log rebuilds its tree from leaves at
   open. A write-mostly backend that cannot enumerate what it holds cannot host a
   transparency log.
5. **No fork, no merge, no delete, no truncate** — including as capabilities
   reachable *around* the trait. A backend whose native API offers branching is
   not disqualified, but the branching must be unreachable for the log's data,
   because a branch of an append-only log is two histories and that is the exact
   attack the log exists to prevent.

Nothing above mentions keys, signatures, identity, or ordering beyond the index.
That is deliberate: the leaf index *is* the order and the Merkle root is the
authority, so a backend supplying its own ordering, sharding, or identity model
adds nothing the log can use. Key rotation and similar events live *in* the
trail, appended like anything else.

## Standing rule

**Verify adoption criteria at the candidate's source, at a named commit, before
adopting — never on report.** Not distrust: a report is accurate at the moment
it is written and this file will be read later. Record the commit alongside the
finding so the check can be repeated rather than re-litigated.

## haematite

The intended second backend, and currently **blocked**. Verified at
`haematite@cf07bc0`, working tree clean, on `main`:

| # | Blocker | Evidence at `cf07bc0` |
|---|---|---|
| **#11** | **Read side: WAL recovery does not fail closed.** A checksum-mismatched frame is logged at `warn`, sets a flag, breaks the replay loop, and returns `Ok`. The shard boot path never reads that flag, so a corrupt tail opens as a silent prefix and subsequent writes land on top of it. | `wal/recovery.rs:159-165` returns `Ok` after `stopped_at_corruption = true`; the only non-test consumer of `stopped_at_corruption()` is `db/vacuum/shard_scan.rs:86`; `shard/actor.rs:240-275` (`from_recovered_with_clock`) consumes the recovery result without consulting it, and `shard/actor/native/boot.rs:109` is the production path that calls it. |
| **#58** | **Write side: a bare append is not fsynced under the shipped policy.** `FsyncPolicy::CommitOnly` makes `should_sync_after_append()` false, and production constructs exactly that. Wiring `put_leaf` to a plain append would return `Ok` for a leaf not on stable storage — violating criterion 1 through a policy default that neither the trait nor the call site mentions. | `wal/durable.rs:35-42` (the policy enum), `wal/durable.rs:277-283` (`should_sync_after_append` returns false for `CommitOnly`), `shard/actor/native/boot.rs:110` (`DurableWal::new(wal_path, FsyncPolicy::CommitOnly)`). The promise path at `wal/durable.rs:175` always syncs, and `:165` documents that asymmetry — so the codebase already says a plain append is unsynced. |
| **#57** | **No committed `Cargo.lock`.** Gate evidence cannot say which patch versions linked, so "N tests green" cites a dependency resolution nobody recorded — a reproducibility hole in a chain whose product is reproducible verification. | `.gitignore` lists `Cargo.lock`; `git ls-files` does not track it. |

**#11 and #58 are different tasks and neither substitutes for the other:** one is
what recovery does with damage it finds, the other is whether the damage gets a
chance to exist. A fix to either leaves the other's failure fully available.

Two notes that narrow the write-side problem rather than excusing it:

- **The two haematite backends already disagree.** `store/opfs/mod.rs:60` and
  `:110` construct `PerWrite`. So the wasm/OPFS store is per-write durable while
  native production is `CommitOnly`, and nothing states or rules on the split.
  The consequence here is small and useful: requiring `PerWrite` (or routing
  every `put_leaf` through a path that syncs) is **already the convention on one
  of the two backends**, not a concession extracted for lys's benefit.
- **Parent-directory fsync exists somewhere in that codebase**
  (`wal/durable.rs:446`, beside the temp-file seal at `:389`). Whether every
  create and rename reaches it is *not* established — recorded as present, not
  as complete.

`#57` cannot be landed from an agent seat: generating a lockfile means running
cargo on the machine, so it needs an executor and Tom's word.

### On lys's own version of these

Two of the three were true here first, which is why this file states criteria
rather than complaints:

- The file store fsynced **nothing** — not the leaf, not the directory — while
  the trait it was about to satisfy promised durable-on-return. Fixed in the
  commit that created this crate. It had been wrong for as long as the code
  existed and every test passed, because the tests encoded the implementation's
  behaviour and there was no independent statement of intent for them to
  disagree with. **Writing the contract created the second party.**
- `Cargo.lock` was gitignored here too, on the library convention, which is the
  wrong convention for a repository whose gates are its evidence.
