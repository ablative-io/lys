---
type: brief
id: HOME-028
cluster: home
title: Prove a second put writes nothing by pinned modification times, not by sleeping
---

# HOME-028: Prove a second put writes nothing by pinned modification times, not by sleeping

> **Cluster:** home
> **Design anchor:**
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> **Checklist:**
> - C95 — The block store's gate proves a second put of the same bytes writes nothing without elapsed time: before the second put it pins the shard directory's and the block file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock.
> - C96 — The template store's gate proves a second put of the same template writes nothing without elapsed time: before the second put it pins the template shard directory's and the template file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock.
> **Stories:**
> - S40 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write.

## Purpose

The block store's and the template store's gates tell a second put that wrote nothing from one that rewrote the same bytes by a modification time taken 20 milliseconds apart. That wait is a clock: on a filesystem with coarse timestamps, or on a loaded host where the second put lands inside the same tick, it proves nothing, and the suite pays it on every run. This brief makes both gates prove the write-once property with no elapsed time, by pinning the modification times the second put would change to a fixed instant and asserting they did not move.

## Task

Two tests change and nothing else in crates/lys-home: the_same_bytes_put_twice_occupy_one_block_and_the_second_put_writes_nothing in crates/lys-home/src/record/blocks_tests.rs and a_template_is_kept_once_under_its_hash_and_a_second_put_writes_nothing in crates/lys-home/src/record/templates_tests.rs. In each, the std::thread::sleep of 20 milliseconds before the second put is removed. In its place, before the second put, the test sets the modification time of both the shard directory and the stored file to the fixed instant std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000) with std::fs::File::set_modified, reads both back and asserts each equals that instant; after the second put it asserts both are still exactly that instant and the shard's entry count equals the count taken before the second put. The shard directory is pinned as well as the file because the directory's modification time is what catches a temporary file created and deleted in the shard, which neither the file's modification time nor the entry count shows; the block test's existing check reads that directory, as HOME-001's acceptance measures it. Every assertion each test makes today is kept. The block and template stores (blocks.rs, templates.rs) already write once by hash and do not change. Out of scope: any store's write path, every other test in the workspace including the sleep in the lys-anchor crate's admission certificate tests, and canon.rs's millis_now, which records a timestamp and waits on nothing.

## Requirements

### R1: Pin the block shard and block file modification times in the block store's gate instead of sleeping

WHEN the test the_same_bytes_put_twice_occupy_one_block_and_the_second_put_writes_nothing has put a 1 MiB block once, THE SYSTEM SHALL, before the second put, set the modification time of the shard directory store.root()/<first two hex digits of the hash> and of the block file store.root()/<hh>/<hash> to the instant std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000) with std::fs::File::set_modified, read each back with std::fs::metadata(..).modified() and assert each equals that instant; WHEN the second put of the same bytes returns, THE SYSTEM SHALL assert the shard directory's and the block file's modification times each still equal that instant and the shard's entry count equals the count taken before the second put. THE SYSTEM SHALL NOT sleep, SHALL NOT read the current time, SHALL NOT drop any assertion the test makes today (first.new, !second.new, second.hash == first.hash, the unchanged count, count == 1), SHALL NOT rename, split, ignore or add a timeout to the test, SHALL NOT change any other test in blocks_tests.rs, and SHALL NOT change crates/lys-home/src/record/blocks.rs.

**Acceptance:**
- `grep -n 'sleep' crates/lys-home/src/record/blocks_tests.rs` prints nothing.
- `grep -c 'set_modified' crates/lys-home/src/record/blocks_tests.rs` prints a count of at least 1, and the test pins both the path store.root().join(&first.hash.as_str()[..2]) and the path store.root().join(&first.hash.as_str()[..2]).join(first.hash.as_str()).
- Before the second put the test asserts `std::fs::metadata(p).unwrap().modified().unwrap() == UNIX_EPOCH + Duration::from_secs(1_000_000_000)` for the shard directory and for the block file.
- After the second put the test asserts the same equality for the shard directory and for the block file, asserts the shard's read_dir count equals the count taken before the second put, and asserts that count equals 1.
- The test still asserts `first.new`, `!second.new` and `second.hash == first.hash`.
- `cargo test -p lys-home the_same_bytes_put_twice_occupy_one_block_and_the_second_put_writes_nothing` reports `1 passed` and `0 failed`.
- `git diff --exit-code 7b53625 -- crates/lys-home/src/record/blocks.rs` exits 0.

**Files:**
- modify: crates/lys-home/src/record/blocks_tests.rs

**Checklist:**
- C95 — The block store's gate proves a second put of the same bytes writes nothing without elapsed time: before the second put it pins the shard directory's and the block file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock.

**Stories:**
- S40 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write.

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each acceptance row, with its evidence (not yet run):
(1) No sleep: the std::thread::sleep line that was at blocks_tests.rs:22 is removed, and a grep for sleep in the file prints nothing.
(2) set_modified count at least 1: blocks_tests.rs:18-20 defines shard = store.root().join(&first.hash.as_str()[..2]), file = shard.join(first.hash.as_str()) and the pinned instant. Lines 21-24 loop over [&shard, &file] and call std::fs::File::open(p).unwrap().set_modified(pinned).
(3) Read-back before the second put: inside that loop, the test asserts std::fs::metadata(p).unwrap().modified().unwrap() == pinned for both paths.
(4) After the second put: the test asserts files() == count, then the shard's mtime == pinned, then the file's mtime == pinned, then count == 1.
(5) first.new, !second.new and second.hash == first.hash are all still asserted.
(6) The workflow runs the named cargo test.
(7) blocks.rs is unchanged.
- Deviation: (none)
- Files changed:
  - modified: `crates/lys-home/src/record/blocks_tests.rs` — the_same_bytes_put_twice_occupy_one_block_and_the_second_put_writes_nothing no longer sleeps. It pins the shard directory's and the block file's modification times to one fixed instant and reads them back. After the second put it asserts both times are unchanged and the shard's entry count is the same.
- Checklist delivery:
  - [x] C95 — The block store's gate proves a second put of the same bytes writes nothing without elapsed time: before the second put it pins the shard directory's and the block file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock. — The test pins both paths, reads them back, asserts they are unchanged after the second put and checks the entry count, with no clock.
- Story delivery:
  - [x] S40 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write. — The proof no longer depends on how coarse the filesystem's timestamps are or how busy the host is.

### R2: Pin the template shard and template file modification times in the template store's gate instead of sleeping

WHEN the test a_template_is_kept_once_under_its_hash_and_a_second_put_writes_nothing has put the launch template fixture once, THE SYSTEM SHALL, before the second put, take the entry count of the template shard directory <home>/templates/<first two hex digits of the hash>, set the modification time of that shard directory and of the template file <home>/templates/<hh>/<hash> to the instant std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000) with std::fs::File::set_modified, read each back with std::fs::metadata(..).modified() and assert each equals that instant; WHEN the second put of the same bytes returns, THE SYSTEM SHALL assert the shard directory's and the template file's modification times each still equal that instant and the shard's entry count equals the count taken before the second put. THE SYSTEM SHALL NOT sleep, SHALL NOT read the current time, SHALL NOT drop any assertion the test makes today (the root absent before the first put, first.new, first.hash == Hash::of(bytes), the stored bytes, path_of, !second.new, second.hash == first.hash, the shard holding exactly 1 entry, contains, get, and the missing-hash error naming the hash), SHALL NOT rename, split, ignore or add a timeout to the test, SHALL NOT change any other test in templates_tests.rs, and SHALL NOT change crates/lys-home/src/record/templates.rs.

**Acceptance:**
- `grep -n 'sleep' crates/lys-home/src/record/templates_tests.rs` prints nothing.
- `grep -c 'set_modified' crates/lys-home/src/record/templates_tests.rs` prints a count of at least 1, and the test pins both path.parent() and path, where path is home.root().join("templates").join(&first.hash.as_str()[..2]).join(first.hash.as_str()).
- Before the second put the test asserts `std::fs::metadata(p).unwrap().modified().unwrap() == UNIX_EPOCH + Duration::from_secs(1_000_000_000)` for the shard directory and for the template file, and takes the shard's read_dir count.
- After the second put the test asserts the same equality for the shard directory and for the template file, asserts the shard's read_dir count equals the count taken before the second put, and still asserts that count equals 1.
- The test still asserts the store root is absent before the first put, `first.new`, `first.hash == Hash::of(&bytes)`, the file's bytes equal the fixture, `store.path_of(&first.hash) == path`, `!second.new`, `second.hash == first.hash`, `store.contains(&first.hash)`, `store.get(&first.hash) == bytes`, and that the error for Hash::of(b"never stored") contains that hash.
- `cargo test -p lys-home a_template_is_kept_once_under_its_hash_and_a_second_put_writes_nothing` reports `1 passed` and `0 failed`.
- `git diff --exit-code 7b53625 -- crates/lys-home/src/record/templates.rs` exits 0.

**Files:**
- modify: crates/lys-home/src/record/templates_tests.rs

**Checklist:**
- C96 — The template store's gate proves a second put of the same template writes nothing without elapsed time: before the second put it pins the template shard directory's and the template file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock.

**Stories:**
- S40 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write.

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each acceptance row, with its evidence (not yet run):
(1) No sleep: the sleep that was at templates_tests.rs:31 is removed.
(2) Pinning: templates_tests.rs:30 sets shard = path.parent().unwrap() and line 31 takes count = read_dir(shard).count(). Lines 33-36 loop over [shard, path.as_path()] and call File::open(p).set_modified(pinned).
(3) Read-back before the second put: inside that loop, the test asserts metadata(p).modified() == pinned for each path.
(4) After the second put: the test asserts the shard's mtime == pinned, the file's mtime == pinned, read_dir(shard).count() == count, and count == 1.
(5) Every earlier assertion is kept: the store root is absent before the first put, first.new, the hash equality, the stored bytes, path_of, !second.new, second.hash == first.hash, contains, get, and the missing-hash error naming the hash.
(6) The workflow runs the named cargo test.
(7) templates.rs is unchanged.
- Deviation: (none)
- Files changed:
  - modified: `crates/lys-home/src/record/templates_tests.rs` — a_template_is_kept_once_under_its_hash_and_a_second_put_writes_nothing no longer sleeps. It takes the shard's entry count, pins the shard directory's and the template file's modification times and reads them back. After the second put it asserts both times and the entry count are unchanged, and that the count is 1.
- Checklist delivery:
  - [x] C96 — The template store's gate proves a second put of the same template writes nothing without elapsed time: before the second put it pins the template shard directory's and the template file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock. — The test pins both paths, reads them back, asserts they are unchanged after the second put and checks the entry count, with no clock.
- Story delivery:
  - [x] S40 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write. — The template store's proof no longer depends on elapsed time.

## Boundaries

- No change to crates/lys-home/src/record/blocks.rs or crates/lys-home/src/record/templates.rs: BlockStore::put, BlockStore::put_file and TemplateStore::put stay byte-for-byte as they are.
- No change to any test other than the two this brief names, in lys-home or in any other crate; the sleep in the lys-anchor crate's admission certificate tests stays.
- No change to crates/lys-home/src/record/canon.rs; millis_now stays.
- No new dependency: std::fs::File::set_modified only.
- No test renamed, split, ignored or given a raised timeout, and no #[allow], #[ignore] or cfg added to silence anything.
- The change is one commit on the card branch.

## Verification

- From the repository root: `grep -rnE 'thread::sleep|tokio::time::sleep' crates/lys-home` prints nothing.
- `git diff --name-only 7b53625 -- crates/` prints exactly crates/lys-home/src/record/blocks_tests.rs and crates/lys-home/src/record/templates_tests.rs.
- `git log --oneline $(git merge-base HEAD origin/main)..HEAD` on the card branch lists exactly one commit; the card branch starts from origin/main, which contains 7b53625.
- File drift check, run and reverted, never committed: in each test, after the read-back assertions on the pinned instant and before the second put, rewrite the stored file with its own bytes — in the block test `std::fs::write(store.root().join(&first.hash.as_str()[..2]).join(first.hash.as_str()), &block).unwrap();`, in the template test `std::fs::write(&path, &bytes).unwrap();`. Each injection makes exactly that test fail, and the failing assertion is the one after the second put on the stored file's modification time; the shard directory's modification time and the entry count assertions still hold, because rewriting an existing file adds and removes no entry.
- Directory drift check, run and reverted, never committed: in each test, after the read-back assertions on the pinned instant and before the second put, create a file named drift in the shard directory and remove it — in the block test `let shard = store.root().join(&first.hash.as_str()[..2]); std::fs::write(shard.join("drift"), b"").unwrap(); std::fs::remove_file(shard.join("drift")).unwrap();`, in the template test the same with `path.parent().unwrap()` as the shard. Each injection makes exactly that test fail, and the failing assertion is the one after the second put on the shard directory's modification time; the stored file's modification time and the entry count assertions still hold, because the file is untouched and the created file is gone before the count is taken.
- cargo fmt --all leaves the tree unchanged.
- cargo clippy --all-targets --all-features -- -D warnings and cargo clippy --all-targets -- -D warnings exit 0.
- cargo test --workspace --all-features exits 0 and lists both named tests as passed.
- cargo doc --no-deps --all-features and cargo doc --no-deps exit 0.
- sh scripts/design/gate.sh exits 0.
