# lys-log-store — what was asked, what it means, and what was written

## The words, as they were typed

The file store in lys-log-store gives a leaf its final name only after every byte of it is written and flushed to disk. Today a leaf file is created under its final name and then written, so a crash or a failed write leaves a torn leaf that the next open counts and pins. A write that fails to flush also leaves bytes in the page cache that read back as if they were durable. A leaf is written to a hidden temporary file in the leaves directory, flushed, and then linked to its final name by an operation that refuses to replace an existing leaf, and the leaves directory is flushed after that. A second writer that has already taken the index is still refused by name. Opening the store flushes the leaves directory before it counts the leaves, so a leaf that is named at open is durable. A leftover hidden temporary file is never counted as a leaf. Tests show that a write failing before the link leaves no leaf, that a leftover temporary file is ignored at open, and that a second writer on the same index is refused. The identity directory in DIRECTORY-003 waits on this card before it lands, because its uncertain append takes a leaf that reads back as committed.

Rulings of the lead, Archie, given on 27 September 2026 to the run 9aee27d1-ae10-4c12-b0e1-5e6b117439dd in answer to its rounds. That run took every answer and then failed before writing, when the account pool was at its usage limit. They are settled here, they stand over any earlier ruling above that they contradict, and the author reopens none of them.

Leave it where it is. Open never deletes or changes anything, so a read-only open such as Anchor::open_read_only stays read-only. A leftover hidden temporary file is skipped by name and never counted. Clearing leftovers is a further unit not written, named in the brief, for an explicit maintenance act that runs only while holding the write lock. An acceptance line asserts that open with a leftover temp file counts the right leaves and leaves that file byte-identical. Answered by Archie, lead for the identity line.

The link is the commit point. Once the no-replace link succeeds, the leaf is this writer's and the store's extent advances past it whatever happens next, so the store never reports its own leaf as another writer's. If only removing the temp name fails, put_leaf reports Ok: the leaf is whole and named, and the leftover is ignored as ruled above. If flushing the leaves directory fails, put_leaf returns a new named error meaning that the leaf is written but its durability is uncertain, carrying the index. It is never Io as today and never LeafAlreadyWritten. The handle then refuses further appends by name until it is reopened, and reopening flushes the directory and counts the leaf. LeafAlreadyWritten keeps its promise to mean another writer only. DIRECTORY-003's uncertain append treats the new error as uncertain and settles by reading the leaf back. Acceptance lines: a failure injected after the link on temp removal returns Ok with the extent advanced; one on the directory flush returns the named uncertain error, and the next append on that handle is refused by name; and after reopening, the leaf counts and a next append goes to the following index. Answered by Archie.

Yes. LeafDurabilityUncertain carries the index and the std::io::Error from the failed flush as its source, and its message names both, so the cause is never lost. An acceptance line asserts that the error's source is the injected flush error. Answered by Archie, lead for the identity line.

Remove it. A write that fails before the link, and a link refused as LeafAlreadyWritten, each remove this writer's own temporary file before returning, so a failed write leaves nothing behind. The maintenance act is only for leftovers that a crash stranded. Acceptance lines: after a failed temp write, and after a LeafAlreadyWritten refusal, the leaves directory holds no temporary file of this writer's. Answered by Archie.

R1's acceptance builds LeafDurabilityUncertain with index 7340033, which appears nowhere else in the message text, and asserts that to_string() contains `7340033` and contains the source's message. Answered by Archie.

As r1-acceptance-unmeasured: index 7340033, asserted as that substring. Answered by Archie.

The temporary name is made by one pure function in file.rs, leaf_temp_name(pid: u32, index: u64, sequence: u64) -> String, giving `.<pid>-<index padded to LEAF_NAME_WIDTH>-<sequence>.tmp`. write_leaf_temp(leaves_dir, index, write_contents) takes the sequence from a process-wide atomic counter, so it is not deterministic across calls, and opens the file with OpenOptions create_new(true) and never truncate. If create_new meets AlreadyExists, write_leaf_temp takes the next sequence and tries again, so a leftover never blocks the write and is never replaced. R2's first acceptance line: leaf_temp_name gives different names for two sequences with the same pid and index, and for two pids with the same index and sequence. R5's leftover test places its leftover under leaf_temp_name(std::process::id() + 1, 1, 0), which is another writer's crash, and asserts that open leaves it byte-identical and that put_leaf(1) then succeeds. The same-writer case is R2's no-replace test. Answered by Archie.

The temporary name is made by one pure function in file.rs, leaf_temp_name(pid: u32, index: u64, sequence: u64) -> String, giving `.<pid>-<index padded to LEAF_NAME_WIDTH>-<sequence>.tmp`. write_leaf_temp(leaves_dir, index, write_contents) takes the sequence from a process-wide atomic counter, so it is not deterministic across calls, and opens the file with OpenOptions create_new(true) and never truncate. If create_new meets AlreadyExists, write_leaf_temp takes the next sequence and tries again, so a leftover never blocks the write and is never replaced. Acceptance: a test pre-creates a file holding known bytes at leaf_temp_name(std::process::id(), index, the counter's next value), then calls put_leaf. put_leaf succeeds, the leaf reads back, and the pre-created file still holds its bytes. Drift injection: replacing create_new(true) with create(true).truncate(true) turns exactly that test red. Two put_leaf calls on two handles over one directory, for indexes 1 and 2, each leave no temporary name behind and never share one. Answered by Archie.

As r2-temporary-uniqueness-unmeasured and r2-acceptance-leaf-path-line-garbled. Answered by Archie.

Take the example as written. Within put_leaf in file.rs, the value returned by leaf_path is passed only to std::fs::hard_link, as the link's destination, and to no OpenOptions::open, File::create, std::fs::write or std::fs::rename call. The check is a read of put_leaf's body, stated as one line. Answered by Archie.

As r2-r5-temporary-name-function-contradicts-uniqueness. R5's leftover belongs to another pid, so it tests that open ignores it and leaves it alone. A leftover under the writer's own next name is R2's test. Answered by Archie.

The seam injects std::io::Error::new(std::io::ErrorKind::TimedOut, "injected leaves directory flush failure") for the leaves/ flush. R4's source test asserts that source's kind is TimedOut and that its to_string() equals that message. Drift injection 10 keeps Other with `flush failed`, so it turns that test red. Answered by Archie.

As r7-spec-untestable: kind TimedOut, message `injected leaves directory flush failure`. Both differ from drift 10's Other and `flush failed`. Answered by Archie.

Yes. design.json's structure sets brief to LYSLOGSTORE-001 for crates/lys-log-store/src/error.rs, store.rs, file.rs and file_tests.rs, and the rendered documents are re-rendered. Answered by Archie.

Accepted. Add a drift injection that gives the final name with an operation that replaces an existing entry (std::fs::rename in place of std::fs::hard_link). Under it exactly one test fails, the_link_alone_refuses_a_leaf_this_store_never_saw, and a_refused_link_leaves_no_temporary_file is rewritten so that it stays green under that injection, because it asserts only the temporary file's removal after a refused link, which it drives through its own refusal. The rewritten absent-check row pairs with this injection. Drift 3 stays as its own row for how the error is reported. Answered by Archie, lead for the identity line.

Accepted. R8 widens to file_tests.rs's module-doc header, the extent row and the comment in extent_check_alone_refuses_a_deleted_leafs_index. Each names the no-replace link, and no assertion changes. Add the acceptance line that `grep -c create_new crates/lys-log-store/src/file_tests.rs` prints 0. The temporary file's own create_new lives in file.rs and is not affected. Answered by Archie.

Take the sequence from a parameter. create_leaf_temp, write_leaf_temp and put_leaf_with take next_sequence: &mut impl FnMut() -> u64. put_leaf passes the process-wide counter, and tests pass their own counter from zero. The test pre-creates leaf_temp_name(pid, index, 0), supplies sequences from zero, and asserts that the write's temporary file was taken at sequence 1, observed by keeping it through the seam, and that the pre-created bytes are unchanged. That line fails whenever the collision did not fire, and it holds under parallel cargo test because no other test shares the counter. Answered by Archie.

State it in the spec. LeafDurabilityUncertain's message tells the caller to reopen the store, and the acceptance keeps the `reopen` substring check. The message is: 'leaf {index} was written, but the leaves directory could not be flushed afterwards, so its durability is uncertain; reopen the store: {source}'. Answered by Archie.

Recording the sequences stands. The test proves the collision by asserting that the sequences its own closure returned were exactly [0, 1], so the write met the pre-created name at 0 and moved to 1. It also asserts that the pre-created file's bytes are unchanged and that the leaf reads back. It does not keep the temporary file through the removal seam. Drift 13 (rename in place of hard_link) then fails exactly one test, as ruled in round 4, and the create(true).truncate(true) drift still fails this test. This supersedes my round-4 wording 'observed by keeping it through the seam'. Answered by Archie, lead for the identity line.

## What the survey found, and its angles

Make FileLeafStore::put_leaf commit a leaf atomically. Each leaf is written to a hidden temporary file in leaves/ and flushed, then given its final name with a no-replace hard link, which is the commit point, and then leaves/ is flushed. A failed write leaves no leaf behind, and a leftover temporary file is never counted and never touched. Open flushes leaves/ before it counts, and when the directory flush after the link fails, put_leaf returns a new named LeafDurabilityUncertain error with the index and the io::Error as its source, then refuses further appends on that handle by name until the store is reopened. The lead's rulings fix the temp-name function, the injected sequence counter, the error message, each drift injection and each acceptance line. DIRECTORY-003's uncertain append waits on this card.

### What the tree holds

- `crates/lys-log-store/src/file.rs` — put_leaf (lines ~226-268) today opens leaf_path(index) with create_new and writes into it under the final name, so a torn write is a counted leaf. The temp-write, hard_link, directory flush, handle refusal, leaf_temp_name, write_leaf_temp/create_leaf_temp/put_leaf_with and the flush seam all land here. contiguous_extent already skips names starting with '.', which covers leftovers. open() needs a leaves/ fsync before contiguous_extent. The module-doc Durability section has to describe the new order.
- `crates/lys-log-store/src/error.rs` — StoreError is #[non_exhaustive] with 13 variants today. LeafDurabilityUncertain { index, #[source] source: io::Error } is added with the message the lead ruled, plus the by-name refusal of later appends. The LeafAlreadyWritten docs have to keep meaning 'another writer' only.
- `crates/lys-log-store/src/store.rs` — The LeafStore contract (durable on return, write-once) and put_leaf's # Errors list only Io, LeafAlreadyWritten and LeafWouldLeaveGap. The new uncertain outcome and the refusal after it have to be written into the contract. The module docs on 'LeafAlreadyWritten is a conflict, not a resume signal' bear on this directly.
- `crates/lys-log-store/src/file_tests.rs` — 22 tests. The module-doc drift table names create_new as the second absent-check, and create_new_alone_refuses_a_leaf_this_store_never_saw becomes the_link_alone_refuses_a_leaf_this_store_never_saw. There are 6 create_new mentions today, and the ruling requires 0. New tests go here: failed-before-link, leftover ignored and byte-identical, refused link leaves no temp, post-link temp-removal failure, dir-flush failure, reopen and next index, and the sequence-collision test.
- `crates/lys-log-store/src/log.rs` — Log::append calls store.put_leaf(index, ..)? before it poisons, so on LeafDurabilityUncertain the store's extent has advanced but the Log's tree has not. The Log relies on the store's by-name refusal to stop a second append, and on reopen, Log::open's one-leaf-ahead repair folds the leaf in.
- `crates/lys-anchor/src/anchor/read_only.rs` — Anchor::open_read_only goes through FileLeafStore::open and Log::open. The ruling that open never deletes or changes a leftover keeps this path read-only as far as leftovers go. The new leaves/ fsync at open runs on this path too.
- `crates/lys-anchor-cli/src/commands/error.rs` — Maps StoreError with a deliberate catch-all (`other => Self::Store(..)`), so the new variant reaches the operator verbatim with no change. crates/lys/src/commands/log/store.rs mirrors it.
- `docs/design/lys-log-store/` — The cluster to continue holds only BACKENDS.md: no design.json, no briefs, no rendered DESIGN.md. The ruling sets design.json's structure brief to LYSLOGSTORE-001 for error.rs, store.rs, file.rs and file_tests.rs, and scripts/design/gate.sh measures a cluster only once it has a design.json.
- `docs/design/directory/briefs/DIRECTORY-003.json` — depends_on is ['DIRECTORY-002'] and it has no mention of LYSLOGSTORE. The words say DIRECTORY-003 waits on this card, and its R2 'reconciling an uncertain write' is the consumer of LeafDurabilityUncertain.
- `docs/design/lys-log-store/BACKENDS.md` — Criterion 1 (durable on return) and criterion 2 (write-once enforced by the backend) are the written criteria this change strengthens for FileLeafStore. It records the old 'fsynced nothing' defect as the precedent.

### What was already decided

- docs/design/lys-log-store/BACKENDS.md — A backend must be durable on return and write-once, with contiguity established at open. This card makes the file store's own leaf commit meet criterion 1 across crashes and failed writes, not only on the success path.
- crates/lys-log-store/src/store.rs (LeafStore contract) — Ok means stable storage, and LeafAlreadyWritten means another writer holds the position, never a resume signal. The ruling keeps that meaning and adds a distinct uncertain outcome.
- CLAUDE.md: A test needs a second party — A drift injection proves something only if exactly one test fails, and it is the test built for that check. The rulings name drift 10 (the Other/'flush failed' source), drift 13 (rename for hard_link → only the_link_alone_refuses_a_leaf_this_store_never_saw) and create(true).truncate(true) → only the collision test.
- CLAUDE.md: Gates before any commit — fmt, both clippy shapes, tests --all-features, and doc in both shapes. file.rs must stay under 500 code lines.
- DIRECTORY-003 R2 / ID001_AUDIT_FAULTS — Every identity change is committed through lys-log-store, and an uncertain write is reconciled before any affected read answers. Uncertain operations resolve once, with no silent loss and no double application.
- ADR-004 — Every project stands alone. The fix lives wholly inside lys-log-store, with no dependency on a Haematite backend (DIRECTORY-003 R2 says the log is lys-log-store's file storage).

### What was measured

- crates/lys-log-store/src/file.rs size: 462 lines, 315 non-comment non-blank lines (limit 500)
- crates/lys-log-store/src/error.rs size: 210 lines, 79 non-comment non-blank lines
- crates/lys-log-store/src/store.rs size: 191 lines
- crates/lys-log-store/src/file_tests.rs: 428 lines, 22 #[test] functions
- crates/lys-log-store/src/log_tests.rs: 13 #[test] functions
- `grep -c create_new crates/lys-log-store/src/file_tests.rs` today: 6 (the ruling requires 0)
- StoreError variants today: 13 (Io, LeafAlreadyWritten, LeafWouldLeaveGap, PinWentBackwards, PinRootChanged, NotInitialized, AlreadyInitialized, Corrupt, PinMismatch, LeafMissingWithinExtent, Poisoned, Trust, Serialize); no LeafDurabilityUncertain
- Files in docs/design/lys-log-store: 1 (BACKENDS.md); no design.json, no briefs directory
- Occurrences of LYSLOGSTORE in docs/design/roadmap.json and DIRECTORY-003.json: 0 and 0
- DIRECTORY-003 depends_on: ['DIRECTORY-002'] only
- Files outside lys-log-store that reference lys_log_store: 10 (lys CLI, lys-anchor, lys-anchor-cli, and their tests)
- Places put_leaf writes under the final name today: 1 (OpenOptions create_new on leaf_path(index), file.rs put_leaf)
- Existing dot-prefix skip in contiguous_extent: present (`if name.starts_with('.') { continue; }`)
- leaves/ fsync in FileLeafStore::open today: absent

### What it means for the other projects

- haematite — None directly. BACKENDS.md stays haematite's adoption criteria, and a future haematite backend would have to give the same guarantee: no torn leaf is ever counted, and there is a distinct uncertain outcome. That may be worth one line in BACKENDS.md.
- aion — The card runs through aion's chain (brief_card → sign-off → card_build_v3 → src_pr → src_land). The cluster needs a design.json and a brief LYSLOGSTORE-001 before the chain can take it, because it has neither today.

### The decisions it stands on

-  (new) — A leaf's no-replace link is the file store's commit point. Before it, a failure leaves nothing. After it, the extent advances, and only the directory flush can make the outcome uncertain, which is reported as LeafDurabilityUncertain, never as Io or LeafAlreadyWritten.
-  (new) — LeafAlreadyWritten means another writer only. The store never reports its own leaf as another writer's, which keeps store.rs's 'a conflict, not a resume signal' rule true under failure.
- ADR-004 (honour) — The fix is self-contained in lys-log-store's file backend, and nothing depends on another project.

### What it requires

- put_leaf writes bytes only to a temporary file named by leaf_temp_name(pid, index, sequence) in leaves/, opened with create_new(true) and never truncate, and sync_all's it before linking
- Within put_leaf, leaf_path's value is passed only to std::fs::hard_link as the destination, and to no OpenOptions::open, File::create, std::fs::write or std::fs::rename call
- hard_link failing with AlreadyExists returns LeafAlreadyWritten { index } and removes this writer's temporary file
- leaves/ is fsynced after a successful link, and a failure there returns LeafDurabilityUncertain { index, source } with the extent advanced
- After LeafDurabilityUncertain, the next put_leaf on that handle is refused by name. After a reopen the leaf counts, and the next append goes to index+1
- Removal of the temporary name failing after the link returns Ok with the extent advanced
- FileLeafStore::open fsyncs leaves/ before contiguous_extent counts
- A leftover under leaf_temp_name(std::process::id() + 1, 1, 0) is ignored at open, left byte-identical, and put_leaf(1) then succeeds
- A failed temp write leaves no temporary file of this writer's in leaves/
- LeafDurabilityUncertain's to_string() is 'leaf {index} was written, but the leaves directory could not be flushed afterwards, so its durability is uncertain; reopen the store: {source}'. A test with index 7340033 asserts it contains `7340033`, the source's message and `reopen`
- The error's source is the injected io::Error with kind TimedOut and message 'injected leaves directory flush failure'
- leaf_temp_name gives different names for two sequences with the same pid and index, and for two pids with the same index and sequence
- The collision test's closure returns exactly [0, 1], the pre-created file keeps its bytes, and the leaf reads back
- Two put_leaf calls on two handles over one directory, for indexes 1 and 2, leave no temporary name behind and never share one
- create_new_alone_refuses_a_leaf_this_store_never_saw is renamed the_link_alone_refuses_a_leaf_this_store_never_saw. Under the rename-for-hard_link drift, exactly that test fails
- Under create(true).truncate(true) in place of create_new(true), exactly the collision test fails. Under drift 10 (Other, 'flush failed'), the source test fails
- `grep -c create_new crates/lys-log-store/src/file_tests.rs` prints 0. The module-doc header, the extent row and the comment in extent_check_alone_refuses_a_deleted_leafs_index name the no-replace link
- docs/design/lys-log-store has a design.json whose structure sets brief LYSLOGSTORE-001 for error.rs, store.rs, file.rs and file_tests.rs, the rendered documents are re-rendered, and scripts/design/gate.sh passes
- DIRECTORY-003 records that it waits on LYSLOGSTORE-001
- All gate legs pass: fmt, clippy in both shapes, tests --all-features, doc in both shapes, design

### What must not change

- The leaf file stays the raw RFC 6962 preimage, with a 20-digit name and no framing
- The LeafStore trait gains no fork, merge, delete, truncate or rewrite
- Open never deletes, renames or rewrites anything in leaves/, including leftovers
- LeafAlreadyWritten keeps meaning another writer only
- The existing error variants and their messages are unchanged, and the change is additive on a #[non_exhaustive] enum published in lys-log-store 0.2.0
- The existing isolation of the extent check (extent_check_alone_refuses_a_deleted_leafs_index) keeps failing only under its own drift
- state.json and log.json write paths are not changed by this card
- No unwrap, expect or panic in library code, and no #[allow] or #[ignore] to pass a gate

### What we must put in place first

- Create the lys-log-store cluster's design.json (and brief LYSLOGSTORE-001) so the design gate measures it and the aion chain can take the card
- Give the card a roadmap row, since none carries LYSLOGSTORE today

### The risks

- The leaves/ fsync at open may fail on read-only mounts or on filesystems that refuse a directory F_FULLFSYNC, turning a working read-only open into an error
- Log::append returns before poisoning when put_leaf fails, so after LeafDurabilityUncertain the Log's in-memory tree lags the store by one. Correctness then rests on the store's by-name refusal, and on Log::open's one-leaf-ahead repair after reopen
- hard_link is unsupported on some filesystems (some FUSE, SMB, FAT). The store would then refuse every write with Io, where today it works
- fsync_dir is a no-op off unix, so on those targets the uncertain outcome can never be reported, and the durability claim stays scoped to unix
- Leftover temporary files accumulate until the maintenance act exists
- file.rs may cross the 500-code-line limit once the temp helpers, the seam and the retry loop are added
- A shared process-wide counter could make tests order-dependent if any test uses it rather than its own next_sequence closure
- Drift-injection claims (exactly one failing test) may break if a new test bundles two rules

### Still open

- If flushing leaves/ fails at open (for example on a read-only mount, where Anchor::open_read_only is used), does open fail with Io, or open without the flush? The sentence of the words it stands on: "Opening the store flushes the leaves directory before it counts the leaves, so a leaf that is named at open is durable.". Why only the lead can settle it: This decides whether a reader of a log on read-only media (a `status` through Anchor::open_read_only, crates/lys-anchor/src/anchor/read_only.rs) gets an error where today it opens. The words require the flush, and no ruling says what a failed flush at open does.

### The units beyond the first

- Clear stranded leaf temporary files under the write lock — The lead ruled it a separate, explicit maintenance act that runs only while holding the write lock. It is named in the brief and not written by this card.
- DIRECTORY-003 settles an uncertain append by reading the leaf back — The consumer of LeafDurabilityUncertain lives in the identity directory's row, not in lys-log-store.

### The smallest complete shape

One card, LYSLOGSTORE-001. It covers:
- **Code:** file.rs's temp-write, no-replace link and leaves/ flush with the open-time flush and the by-name refusal after an uncertain write; error.rs's LeafDurabilityUncertain; store.rs's contract and # Errors text.
- **Tests:** file_tests.rs's renamed and added tests and drift table, meeting every ruled acceptance line.
- **Documents:** the lys-log-store cluster's design.json and rendered brief, and DIRECTORY-003 recording that it waits on this card.
- **Gates:** it lands whole through the full gate set.

## The roadmap row

- **RM-038** — Give a file-store leaf its final name only after it is whole and flushed (fix, idea)
- Summary: lys-log-store's FileLeafStore writes each leaf in a hidden temporary file, flushes it, links it to its final name with an operation that refuses to replace, and flushes the leaves directory; the link is the commit point. A writable open flushes the leaves directory before counting and fails with Io when it cannot; a read-only open counts without flushing and its handle refuses put_leaf and pin as ReadOnly, so Anchor::open_read_only never repairs a store one leaf ahead of its pin and reports the pending repair instead; neither counts or touches a leftover temporary file. A post-link directory-flush failure is a new named error and the handle refuses further appends until reopened. DIRECTORY-003 records that it lands after this.
- Asked by: tom on 2026-09-27T16:42:00+10:00
- Context: The lys-log-store card's words, surveyed and briefed by the brief method as LYSLOGSTORE-001; the lead for the identity line answered four open questions: leftover temporary files are left in place and open never deletes anything; the no-replace link is the commit point, with a named uncertain error for a failed directory flush after it; that error carries the flush's io error as its source; and a write that fails before a successful link removes its own temporary file; and, in a second round, the temporary name is `.<pid>-<padded index>-<sequence>.tmp` from a process-wide counter, created with create_new and never truncated, and the injected leaves/ flush error is of kind TimedOut; and, in a third round, the error's message tells the caller to reopen the store, the temporary file's sequence is a parameter that put_leaf fills from the process-wide counter and tests fill from zero, a drift injection replaces the link with a rename, and file_tests.rs names the no-replace link wherever it named create_new; and, in the rounds after, the collision test proves the collision by the sequences its own closure returned, exactly [0, 1]; and the flush at open is the writer's act: a read-only open never flushes and counts the named leaves, while a writable open whose flush fails returns Io and writes nothing; and, in the last round, put_leaf on a handle from FileLeafStore::open_read_only returns ReadOnly naming the directory and the act and writes nothing, and Anchor::open_read_only's doc example and lys-anchor's read-only tests open their store with FileLeafStore::open_read_only; and, in the round after, pin through a handle from FileLeafStore::open_read_only refuses as ReadOnly because a pin is a write, so Anchor::open_read_only never repairs a store found one leaf ahead: it opens at the pinned head and says in words that a writable open repairs it, and only Log::open through a writable handle repairs.
- Quote: The file store in lys-log-store gives a leaf its final name only after every byte of it is written and flushed to disk. Today a leaf file is created under its final name and then written, so a crash or a failed write leaves a torn leaf that the next open counts and pins. A write that fails to flush also leaves bytes in the page cache that read back as if they were durable. A leaf is written to a hidden temporary file in the leaves directory, flushed, and then linked to its final name by an operation that refuses to replace an existing leaf, and the leaves directory is flushed after that. A second writer that has already taken the index is still refused by name. Opening the store flushes the leaves directory before it counts the leaves, so a leaf that is named at open is durable. A leftover hidden temporary file is never counted as a leaf. Tests show that a write failing before the link leaves no leaf, that a leftover temporary file is ignored at open, and that a second writer on the same index is refused. The identity directory in DIRECTORY-003 waits on this card before it lands, because its uncertain append takes a leaf that reads back as committed.
- Cluster: lys-log-store; briefs: LYSLOGSTORE-001
- Notes: A new row: none of RM-001 to RM-016 on main names lys-log-store, and RM-017 to RM-037 are taken on open brief branches, so this row is RM-038 and its decisions ADR-059 and ADR-060, the next free after the highest on main and on every open brief branch at 7b53625. RM-001 (the identity directory) is left unchanged; LYSLOGSTORE-001 R9 writes the wait into DIRECTORY-003's depends_on. Units beyond the first, by title, not written: Clear stranded leaf temporary files under the write lock; DIRECTORY-003 settles an uncertain append by reading the leaf back.

## The design

---
type: design
cluster: lys-log-store
title: lys-log-store: a named leaf is a whole, flushed leaf
---

# lys-log-store: a named leaf is a whole, flushed leaf

> **Cluster:** lys-log-store

## Intention

A leaf that has a name in the file store is a leaf the store committed: every byte written, flushed to disk, and linked into place by an operation that could not have replaced anyone else's leaf. A reader, a reopening log, or a consumer settling an uncertain append can read a named leaf back and trust that it is whole, without asking how the write that made it ended.

Partial state still exists, because crashes still happen, but it lives only under hidden temporary names that are never counted as leaves and never touched by an open. A read-only open stays read-only: it counts the named leaves without flushing, so a log on read-only media opens as it did, and a handle it returns refuses to write a leaf or a pin by name. A reader of a log a crash left one leaf ahead of its pin opens at the pinned head and is told that a writable open repairs it; only a writable open repairs.

## Problem

FileLeafStore::put_leaf creates the leaf file under its final 20-digit name and then writes and flushes it. A crash or a failed write in between leaves a torn or unflushed leaf under a final name. The in-memory extent does not count it, but the next open does, and Log::open's one-leaf-ahead repair pins it into the tree. A write whose flush fails also leaves bytes in the page cache that read back as if they were durable. A failure after the leaf is named but before put_leaf returns leaves the extent behind the disk, so the next append at that index is refused as LeafAlreadyWritten: the store reports its own leaf as another writer's. The identity directory (DIRECTORY-003) commits every identity change through this store and settles an uncertain append by reading the leaf back, so it cannot land while a named leaf can be torn.

## Solution

put_leaf keeps its existing index checks, then writes the leaf's bytes to a hidden (dot-prefixed) temporary file in leaves/, flushes it, and links it to the leaf's final name with an operation that refuses to replace an existing entry. The temporary name is `.<pid>-<index padded to 20 digits>-<sequence>.tmp`; the sequence is supplied by the caller, and put_leaf supplies it from a process-wide counter while tests supply their own from zero. The file is created with create_new and never truncated; a name already taken is skipped for the next sequence, so a leftover never blocks a write and is never replaced. An existing final name is still LeafAlreadyWritten, raised by the link instead of by create_new. A write that fails before a successful link, including a link refused as LeafAlreadyWritten, removes this writer's own temporary file before returning, so a failed write leaves nothing behind. The successful link is the commit point (ADR-059), and LeafAlreadyWritten keeps meaning another writer only (ADR-060): the extent advances past the leaf whatever happens next. put_leaf then removes the temporary name and flushes leaves/. A failure to remove the temporary name still returns Ok, because the leaf is whole and named and the leftover is ignored. A failure to flush leaves/ returns a new named error, StoreError::LeafDurabilityUncertain carrying the index and the failed flush's std::io::Error as its source, its message naming both, and the store handle refuses every further put_leaf with that error until it is reopened.

The flush at open is the writer's act. FileLeafStore::open flushes leaves/ before it counts, so a leaf that is named at a writable open is durable, and a reopen after an uncertain flush counts the leaf; Log::open's existing one-leaf-ahead repair then pins it. When that flush fails, open fails with Io naming the directory and the flush, because a writer that cannot make a named leaf durable does not go on to write beside it. A reader opens with FileLeafStore::open_read_only, which performs every check open performs except the flush and counts the named leaves, so a store handed to Anchor::open_read_only on read-only media opens as it does today. put_leaf on a handle from open_read_only returns a new StoreError::ReadOnly naming the store's directory and the act, and writes nothing; only a handle from FileLeafStore::open writes a leaf. pin on such a handle refuses as ReadOnly too, because a pin is a write, so the one-leaf-ahead repair is not a read-only act. Log gains open_at_pin, which checks a store exactly as Log::open does but, finding it one leaf ahead of the pin, opens at the pinned head without pinning, reports the tree size a writable open repairs to through pending_repair, and while that repair is pending refuses every append with a new StoreError::RepairPending, whose message says the log has a pending repair that a writable open must make before any append and names the pin's tree size and the leaf count it saw, so the store never reports its own leaf as another writer's and Poisoned keeps its one meaning, an append that failed after storing its leaf on this handle. Anchor::open_read_only opens its log with Log::open_at_pin, whichever store it is handed, so it never repairs; AnchorStatus carries the pending repair, and its notice says in words that one leaf stands ahead of the pin and that a writable open repairs it. Only Log::open through a writable handle performs the repair. Anchor::open_read_only's doc example and lys-anchor's read-only tests open their store with FileLeafStore::open_read_only, so the one read-only path is the one they exercise. contiguous_extent keeps skipping every dot-prefixed name, so a leftover temporary file is never counted, and open never deletes or changes it. The invariant is written into file.rs's module docs as the second party the tests answer to; the trait contract in store.rs names the new error on put_leaf.

A write failure between creating the temporary file and linking it is exercised through a test-only fault seam inside FileLeafStore, compiled only under cfg(test), which fails a chosen step of put_leaf or of open, a test-only constructor arming the open-time step before the leaves are counted: the temporary file's write, the temporary file's flush, the temporary name's removal, the leaves/ flush after the link, or the leaves/ flush at open, the last two with an injected error of kind TimedOut and message `injected leaves directory flush failure`. No caller outside the crate, and no build other than the crate's own tests, can reach it.

The on-disk layout, the LeafStore trait's methods, Log's append order and Log::open's repair are unchanged (CN1 to CN3). Log::append returns put_leaf's error before it poisons, so after LeafDurabilityUncertain its tree lags the store by one leaf; the store's by-name refusal stops a second append on that handle, and Log::open's repair folds the leaf in after a reopen. DIRECTORY-003 records in its depends_on that it waits on this cluster's first brief.

## Principles

- **P1** — A name is a commitment: a file under a 20-digit leaf name is always a whole, flushed leaf, and only hidden temporary files may be partial.
- **P2** — The no-replace link is the commit point: after it succeeds the leaf is this writer's, and the store never reports its own leaf as another writer's.
- **P3** — Open observes; it never deletes or changes a file in the store's directory. Only a writable open flushes leaves/, and a read-only open never flushes.
- **P4** — A failure is reported under the name of what actually happened: another writer is LeafAlreadyWritten, an unflushed directory after a successful link is LeafDurabilityUncertain, a write on a read-only handle is ReadOnly, and none of them is Io.

## Decisions

- ADR-059 — A named leaf in the file store is a whole, flushed leaf, and the no-replace link is its commit point — A file under a 20-digit leaf name is always a whole, flushed leaf; only dot-prefixed temporary files in leaves/ may be partial. A leaf is written and flushed in a hidden temporary file, then linked to its final name by an operation that refuses to replace, then the temporary name is removed and leaves/ is flushed. The successful link is the commit point: the extent advances past the leaf whatever happens next. A failed temporary-name removal still returns Ok; a failed leaves/ flush returns StoreError::LeafDurabilityUncertain carrying the index, and the handle refuses further appends with it until reopened. A writable open flushes leaves/ before counting and fails with Io when that flush fails; a read-only open (FileLeafStore::open_read_only) counts the named leaves without flushing; neither counts, deletes or changes a leftover temporary file. Rejected: naming the leaf first and writing into it (today's torn-leaf shape); deleting leftover temporary files at open, which would make a read-only open mutate the store; flushing at a read-only open, which would turn a status on read-only media into an error; and reporting a post-link failure as Io, which left the extent behind and made the next append report the store's own leaf as LeafAlreadyWritten.
- ADR-060 — LeafAlreadyWritten means another writer holds the index, and the store never reports its own leaf as another writer's — LeafAlreadyWritten is returned only when the index is behind the extent or the no-replace link finds the final name taken by an entry this call did not link. Once this call's link succeeds the extent advances past the leaf whatever happens next, so no later step of the same call and no later call on the same handle reports that leaf as LeafAlreadyWritten; a post-link failure that leaves durability in doubt is LeafDurabilityUncertain (ADR-059). Rejected: reporting a post-link failure as Io and leaving the extent behind, which made the next append report the store's own leaf as LeafAlreadyWritten; and treating LeafAlreadyWritten as a resume signal a caller may retry through.

## Goals

- After a put_leaf that fails before its link, no file exists under that leaf's 20-digit name, no temporary file of this writer's remains in leaves/, and a reopen reports the same extent as before the write.
- A reopen with a leftover hidden temporary file in leaves/ counts exactly the named leaves and leaves the temporary file byte-identical.
- A second writer that put_leafs an index another writer has already linked receives LeafAlreadyWritten for that index, the first writer's bytes are unchanged, and no temporary file of the second writer's remains.
- A leaves/ flush failure after the link returns LeafDurabilityUncertain with the index and the flush error as its source, the next put_leaf on that handle is refused with it, and after a reopen the leaf counts and the next put_leaf goes to the following index.
- A read-only open of a directory whose leaves/ flush fails returns the store with the named leaves counted, and a writable open of it returns Io and leaves every file byte-identical.
- put_leaf on a handle from FileLeafStore::open_read_only returns ReadOnly naming the store's directory, and the directory holds no new file.
- Every Anchor::open_read_only call in lys-anchor's doc examples and read-only tests opens its store with FileLeafStore::open_read_only.
- pin on a handle from FileLeafStore::open_read_only returns ReadOnly and leaves state.json byte-identical.
- Anchor::open_read_only over a store one leaf ahead of its pin opens at the pinned head, leaves state.json byte-identical, and its status says that a writable open repairs it; a writable open over the same store repairs and pins.
- Every leg of the gate array passes.

## Non-Goals

- Clearing leftover hidden temporary files that a crash stranded — A further unit: an explicit maintenance act that runs only while holding the write lock. Open never deletes anything; put_leaf removes only its own temporary file after its own failed write.
- Changing DIRECTORY-003 or its uncertain-append reconciliation — DIRECTORY-003 treats LeafDurabilityUncertain as uncertain and settles by reading the leaf back in its own card; it lands after this one.
- Changing the on-disk layout or the LeafStore trait's methods — The layout is kept byte-identical so earlier stores open, and the trait's absent operations are the crate's reason to exist.
- Directory flushing on non-unix targets — fsync_dir stays a no-op there, as file.rs already says; nothing in this work changes that scope.
- Repairing a store found one leaf ahead of its pin through a read-only open — A pin is a write: pin through a handle from FileLeafStore::open_read_only refuses as ReadOnly, and Anchor::open_read_only opens at the pinned head and reports the pending repair instead. Only Log::open through a writable handle performs the one-leaf-ahead repair.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `crates/lys-log-store/src/error.rs` | StoreError; gains LeafDurabilityUncertain { index, source }, ReadOnly { path, operation } and RepairPending { pinned_tree_size, leaves } | LYSLOGSTORE-001 |
| `crates/lys-log-store/src/store.rs` | the LeafStore contract; put_leaf's errors name LeafDurabilityUncertain | LYSLOGSTORE-001 |
| `crates/lys-log-store/src/file.rs` | FileLeafStore: leaf_temp_name, create_leaf_temp, write_leaf_temp and put_leaf_with taking a caller-supplied sequence, temporary file created with create_new, flush, no-replace link, own-temporary cleanup on failure, commit point, open-time flush at a writable open, open_read_only and its handle's ReadOnly refusal of put_leaf and pin, module-doc invariant, cfg(test) fault seam | LYSLOGSTORE-001 |
| `crates/lys-log-store/src/file_tests.rs` | the file store's gates; gains the eighteen new tests of LYSLOGSTORE-001 R8, the drift-injection table, and module docs that name the no-replace link instead of create_new | LYSLOGSTORE-001 |
| `crates/lys-log-store/src/log.rs` | Log; gains open_at_pin and pending_repair, and Log::append refuses with RepairPending while a repair is pending; Log::open and its repair unchanged | LYSLOGSTORE-001 |
| `crates/lys-log-store/src/log_tests.rs` | Log's gates; gains the open_at_pin test | LYSLOGSTORE-001 |
| `crates/lys-anchor/src/anchor/read_only.rs` | Anchor::open_read_only; opens its log with Log::open_at_pin, its docs say it never repairs, and its two doc examples open the store with FileLeafStore::open_read_only | LYSLOGSTORE-001 |
| `crates/lys-anchor/src/anchor/read_only_tests.rs` | Anchor::open_read_only's gates; their store is opened with FileLeafStore::open_read_only, and they gain the one-leaf-ahead reader and writable-repair tests | LYSLOGSTORE-001 |
| `crates/lys-anchor/src/anchor/status.rs` | AnchorStatus; gains pending_repair and pending_repair_notice | LYSLOGSTORE-001 |
| `crates/lys-anchor/tests/standalone_is_complete.rs` | its read-only open of the anchor takes a store from FileLeafStore::open_read_only | LYSLOGSTORE-001 |
| `docs/design/lys-log-store/BACKENDS.md` | criteria a second LeafStore backend must satisfy; unchanged |  |
| `docs/design/lys-log-store/design.json` | this design | LYSLOGSTORE-001 |
| `docs/design/lys-log-store/DESIGN.md` | rendered from design.json by render-cluster.py | LYSLOGSTORE-001 |
| `docs/design/lys-log-store/checklist.json` | the checklist | LYSLOGSTORE-001 |
| `docs/design/lys-log-store/CHECKLIST.md` | rendered checklist | LYSLOGSTORE-001 |
| `docs/design/lys-log-store/stories.json` | the user stories | LYSLOGSTORE-001 |
| `docs/design/lys-log-store/USER-STORIES.md` | rendered stories | LYSLOGSTORE-001 |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-001.json` | the brief: a named leaf is whole and flushed | LYSLOGSTORE-001 |
| `docs/design/lys-log-store/briefs/LYSLOGSTORE-001.md` | its rendered markdown | LYSLOGSTORE-001 |
| `docs/design/directory/briefs/DIRECTORY-003.json` | the identity directory's contract brief; its depends_on gains LYSLOGSTORE-001 |  |
| `docs/design/directory/briefs/DIRECTORY-003.md` | DIRECTORY-003 rendered from its JSON |  |

## Inventory

- `crates/lys-log-store/src/file.rs` — 462 lines, 315 code lines. put_leaf opens the final leaf path with create_new, then write_all, sync_all and fsync_dir; a failure after create_new leaves a named leaf the next open counts. contiguous_extent already skips names starting with '.'. write_state uses a fixed, non-hidden state.json.tmp and rename. fsync_dir is a no-op on non-unix. No fault-injection seam.
- `crates/lys-log-store/src/file_tests.rs` — 22 tests. create_new_alone_refuses_a_leaf_this_store_never_saw isolates the filesystem absent-check; an_unexpected_leaves_entry_is_detected_but_dotfiles_are_ignored covers .DS_Store only. The module docs carry the drift-injection tables.
- `crates/lys-log-store/src/error.rs` — StoreError, #[non_exhaustive], 13 variants; LeafAlreadyWritten documented as two writers claiming one position; Poisoned belongs to Log's pin-failure case. No read-only variant.
- `crates/lys-log-store/src/store.rs` — The LeafStore contract: durable on return, write-once, contiguous at open; LeafAlreadyWritten is a conflict meaning another writer, not a resume signal.
- `crates/lys-log-store/src/log.rs` — Log::open rebuilds from leaves and repairs exactly one leaf ahead of the pin through the store's pin; Log::append poisons only after put_leaf returns Ok. No open that leaves a store one leaf ahead as it found it.
- `docs/design/lys-log-store/BACKENDS.md` — The cluster's only document before this design: the criteria a backend satisfies.
- `docs/design/directory/briefs/DIRECTORY-003.json` — The dependent card: R2 commits every identity change through lys-log-store's file storage and reconciles an uncertain write (ID001_AUDIT_FAULTS). depends_on is DIRECTORY-002 only; it lands after LYSLOGSTORE-001.
- `crates/lys/src/commands/log/store.rs` — A writer's caller of FileLeafStore::open; picks up the open-time flush and its Io on failure, unchanged in code.
- `crates/lys-anchor-cli/src/commands/anchor/open.rs` — A writer's caller of FileLeafStore::open; picks up the open-time flush and its Io on failure, unchanged in code.
- `crates/lys-anchor/src/anchor/read_only.rs` — Anchor::open_read_only takes a store its caller opened and opens its log with Log::open, so it repairs a store one leaf ahead through the store's pin; it has no production caller. Its two doc examples (one of them compile_fail) open the store with FileLeafStore::open.
- `crates/lys-anchor/src/anchor/read_only_tests.rs` — Its read_only helper and a_reader_refuses_a_log_with_no_genesis_leaf_exactly_as_open_does pass FileLeafStore::open(dir) to Anchor::open_read_only.
- `crates/lys-anchor/tests/standalone_is_complete.rs` — One Anchor::open_read_only call, taking FileLeafStore::open(dir).
- `crates/lys-anchor/src/anchor/status.rs` — AnchorStatus, #[non_exhaustive]: origin, root, posture and recovered_to; nothing reports a repair that is pending rather than done.

## Constraints

- **CN1** — The on-disk layout does not change: log.json, state.json, leaves/<20-digit index> holding the raw RFC 6962 preimage, and the lys/log-dir/v1 marker. Stores written by earlier versions still open.
- **CN2** — The LeafStore trait's methods do not change; no fork, merge, delete, truncate or rewrite is added.
- **CN3** — Log's append order (leaf durable before the pin moves) and Log::open's one-leaf-ahead repair do not change; Log::open through a writable handle is the only open that repairs.
- **CN4** — StoreError::LeafAlreadyWritten means another writer holds the index, and only that.
- **CN5** — No unsafe code, no new dependency, and no unwrap, expect, panic, todo, unimplemented or unreachable in library code.
- **CN6** — crates/lys-log-store/src/file.rs stays under 500 lines of code, excluding tests, comments and blank lines.
- **CN7** — The fault seam, and the constructor that opens with a fault armed, compile only under cfg(test); no public item and no non-test build can reach them.


---
type: brief
id: LYSLOGSTORE-001
cluster: lys-log-store
title: Give a leaf its final name only after it is whole and flushed
---

# LYSLOGSTORE-001: Give a leaf its final name only after it is whole and flushed

> **Cluster:** lys-log-store
> **Design anchor:**
> - ADR-059 — A named leaf in the file store is a whole, flushed leaf, and the no-replace link is its commit point — A file under a 20-digit leaf name is always a whole, flushed leaf; only dot-prefixed temporary files in leaves/ may be partial. A leaf is written and flushed in a hidden temporary file, then linked to its final name by an operation that refuses to replace, then the temporary name is removed and leaves/ is flushed. The successful link is the commit point: the extent advances past the leaf whatever happens next. A failed temporary-name removal still returns Ok; a failed leaves/ flush returns StoreError::LeafDurabilityUncertain carrying the index, and the handle refuses further appends with it until reopened. A writable open flushes leaves/ before counting and fails with Io when that flush fails; a read-only open (FileLeafStore::open_read_only) counts the named leaves without flushing; neither counts, deletes or changes a leftover temporary file. Rejected: naming the leaf first and writing into it (today's torn-leaf shape); deleting leftover temporary files at open, which would make a read-only open mutate the store; flushing at a read-only open, which would turn a status on read-only media into an error; and reporting a post-link failure as Io, which left the extent behind and made the next append report the store's own leaf as LeafAlreadyWritten.
> - ADR-060 — LeafAlreadyWritten means another writer holds the index, and the store never reports its own leaf as another writer's — LeafAlreadyWritten is returned only when the index is behind the extent or the no-replace link finds the final name taken by an entry this call did not link. Once this call's link succeeds the extent advances past the leaf whatever happens next, so no later step of the same call and no later call on the same handle reports that leaf as LeafAlreadyWritten; a post-link failure that leaves durability in doubt is LeafDurabilityUncertain (ADR-059). Rejected: reporting a post-link failure as Io and leaving the extent behind, which made the next append report the store's own leaf as LeafAlreadyWritten; and treating LeafAlreadyWritten as a resume signal a caller may retry through.
> **Checklist:**
> - C1 — StoreError has a variant LeafDurabilityUncertain carrying the leaf index and the failed flush's std::io::Error as its source, its message naming both, distinct from Io and from LeafAlreadyWritten, and LeafStore::put_leaf's documented errors name it.
> - C2 — FileLeafStore::put_leaf writes and flushes a leaf's bytes in a dot-prefixed temporary file in leaves/, named by leaf_temp_name from the process id, the index and a sequence the caller supplies (a process-wide counter for put_leaf) and created with create_new so it never replaces an existing file, before any file exists under the leaf's 20-digit name, and removes that temporary file when writing or flushing it fails.
> - C3 — put_leaf gives the leaf its final name with a link that refuses to replace an existing entry, and an existing final name returns LeafAlreadyWritten for that index with the existing bytes unchanged and this writer's temporary file removed.
> - C4 — A successful link advances the extent; put_leaf then removes the temporary name and flushes leaves/, and a failure to remove the temporary name still returns Ok.
> - C5 — A failure to flush leaves/ after the link returns LeafDurabilityUncertain for that index, and the handle refuses every further put_leaf with that error until the store is reopened.
> - C6 — FileLeafStore::open flushes leaves/ before it counts the leaves, and when that flush fails it returns Io naming the leaves directory and the flush, and writes nothing.
> - C7 — FileLeafStore::open never counts, deletes or changes a leftover dot-prefixed temporary file in leaves/.
> - C8 — file.rs's module docs state that a named leaf is whole and flushed, that only hidden temporary files may be partial, and that the link is the commit point.
> - C9 — A fault seam that fails a chosen step of put_leaf or the leaves/ flush at open exists in FileLeafStore and compiles only under cfg(test).
> - C10 — file_tests.rs holds the eighteen new tests of LYSLOGSTORE-001 R8, names the no-replace link wherever it named create_new, and each drift injection in its module-doc tables fails exactly its own test.
> - C11 — Every leg of the design's gate array passes and file.rs stays under 500 lines of code.
> - C12 — FileLeafStore::open_read_only performs FileLeafStore::open's checks without flushing leaves/, counts the named leaves, and does not fail because a flush of leaves/ would fail.
> - C13 — DIRECTORY-003's depends_on names LYSLOGSTORE-001 after DIRECTORY-002, and no other field of DIRECTORY-003 changes.
> - C14 — put_leaf on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly naming the store's directory and the act, and writes nothing.
> - C15 — Anchor::open_read_only's doc examples and lys-anchor's read-only tests open their store with FileLeafStore::open_read_only.
> - C16 — pin on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly naming the store's directory and the act, and leaves state.json and the pinned root unchanged.
> - C17 — Log::open_at_pin opens a store found one leaf ahead of its pin at the pinned head without pinning, reports the pending repair, and refuses every append by name while it is pending, never as Poisoned; Log::open alone repairs.
> - C18 — Anchor::open_read_only never repairs a store found one leaf ahead of its pin: it opens at the pinned head, and its status says in words that one leaf stands ahead of the pin and that a writable open repairs it.
> - C19 — The lys-log-store cluster's DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSLOGSTORE-001.md are what render-cluster.py renders from its JSON documents, and the design gate passes with them present.
> **Stories:**
> - S1 (Consumer library, Settling an uncertain append by reading the leaf back) — As the identity directory committing through the file store, I want a leaf that reads back under its final name to be whole and flushed so that settling an uncertain append by reading the leaf back gives the committed answer.
> - S2 (Operator, Reopening a log after a crash or a failed write) — As an operator reopening a log after a crash, I want only whole, flushed leaves to be counted so that a torn leaf is never pinned into the tree.
> - S3 (Writer, Racing another writer for the same index) — As a writer whose index another writer has already taken, I want my write refused by name so that neither writer's leaf is replaced.
> - S4 (Writer, Seeing a failure after its leaf was named) — As a writer whose append failed after its leaf was named, I want an error that says the leaf is written but its durability is uncertain so that I never mistake my own leaf for another writer's.
> - S5 (Read-only caller, Opening a store without writing to it) — As a read-only caller opening a store, I want open to leave every file in the directory as it found it so that a read-only open stays read-only.
> - S6 (Read-only caller, Reading a log on read-only media) — As a reader of a log on media that refuses a directory flush, I want a read-only open to count the named leaves without flushing so that a status opens as it did before this change.
> - S7 (Read-only caller, Reading a log that a crash left one leaf ahead of its pin) — As a reader of a log one leaf ahead of its pin, I want a read-only open to leave the store as it found it and tell me that a writable open repairs it so that reading never writes and the pending repair is never silent.

## Purpose

FileLeafStore today names a leaf before writing it, so a crash or a failed write leaves a torn leaf that the next open counts and Log::open pins, and a failure after the leaf is named makes the store report its own leaf as another writer's. This brief makes a named leaf mean a whole, flushed leaf (ADR-059): each leaf is written and flushed in a hidden temporary file, linked into place without replacing anything, and the leaves directory is flushed; a writable open flushes leaves/ before counting, a read-only open counts without flushing and returns a handle that refuses to write a leaf or a pin by name, and neither counts or touches a leftover. Because a pin is a write, a read-only open never repairs a store found one leaf ahead of its pin: Anchor::open_read_only opens at the pinned head and says in words that one leaf stands ahead of the pin and that a writable open repairs it, and only Log::open through a writable handle performs that repair. LeafAlreadyWritten keeps meaning another writer only (ADR-060). DIRECTORY-003 lands after this brief, because its uncertain append settles by reading back a leaf it must be able to trust as committed, and this brief records that wait in DIRECTORY-003's depends_on.

## Task

Change crates/lys-log-store/src/file.rs so put_leaf writes through a hidden temporary file named by leaf_temp_name and created by create_leaf_temp without replacing anything, from a sequence supplied to put_leaf_with, flushes it, gives it its final name with std::fs::hard_link, advances the extent at that link, removes the temporary name and flushes leaves/, and removes its own temporary file whenever it fails before a successful link (R2 to R4). Add StoreError::LeafDurabilityUncertain, carrying the index and the failed flush's std::io::Error as its source, in error.rs and name it in LeafStore::put_leaf's docs (R1). Make FileLeafStore::open flush leaves/ before counting and fail with Io when that flush fails, and add FileLeafStore::open_read_only, which performs open's checks without the flush and returns a handle whose put_leaf and pin are refused as the new StoreError::ReadOnly (R5). Write the invariant into file.rs's module docs (R6). Add a cfg(test) fault seam (R7) and the tests with their drift table (R8). Add LYSLOGSTORE-001 to DIRECTORY-003's depends_on and re-render its markdown (R9). Make Anchor::open_read_only's doc examples and lys-anchor's read-only tests open their store with FileLeafStore::open_read_only (R10). Add Log::open_at_pin, which opens a store found one leaf ahead of its pin at the pinned head without pinning, reports the pending repair and, while it is pending, refuses every append by name with the new StoreError::RepairPending, never Poisoned (R11). Make Anchor::open_read_only open its log with Log::open_at_pin, and give AnchorStatus the pending repair and a notice that says it in words (R12). Render the cluster's DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSLOGSTORE-001.md from their JSON (R13). Order: R1 first, since R4 returns its variant; R7 after the steps it fails exist; R8 after R1 to R7; R10 after R5, which adds the constructor it calls; R11 after R5, whose pin refusal it must not reach; R12 after R10 and R11; R13 last, since it renders the documents as they stand when the rest is done.

In: error.rs's three new variants, store.rs's put_leaf doc line, file.rs, file_tests.rs, log.rs's Log::open_at_pin and Log::pending_repair with their test in log_tests.rs, the three lys-anchor files that open a store for Anchor::open_read_only (crates/lys-anchor/src/anchor/read_only.rs, crates/lys-anchor/src/anchor/read_only_tests.rs and crates/lys-anchor/tests/standalone_is_complete.rs), Anchor::open_read_only's log opening and docs in read_only.rs, AnchorStatus's pending repair in crates/lys-anchor/src/anchor/status.rs, DIRECTORY-003's depends_on with its rendered markdown, and the lys-log-store cluster's rendered DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSLOGSTORE-001.md.

Out: clearing leftover hidden temporary files that a crash stranded, which is a further unit not written here: an explicit maintenance act that runs only while holding the write lock. Open never deletes anything. Also out: any change to Log::open, Log::append or Log::open's one-leaf-ahead repair, to the LeafStore trait's methods, to the on-disk layout, to write_state's state.json.tmp, to any crate other than lys-log-store and lys-anchor (the lys CLI and lys-anchor-cli keep calling FileLeafStore::open and render nothing new), to any lys-anchor file other than the four named above, and to any field of DIRECTORY-003 other than depends_on (DIRECTORY-003's uncertain append treats LeafDurabilityUncertain as uncertain and settles by reading the leaf back, in its own card).

The refusal of a second writer is unchanged in meaning: it is still StoreError::LeafAlreadyWritten, now raised when the no-replace link finds the final name taken rather than by create_new.

## Requirements

### R1: Name the uncertain-durability error

Add to StoreError a variant LeafDurabilityUncertain with two fields: index: u64, and source: std::io::Error marked as the error's source, holding the error from the failed flush of the leaves directory. It is documented as: the leaf at index is written under its final name, but flushing the leaves directory afterwards failed, so its durability is uncertain; reopen the store, which flushes the directory and counts the leaf. Its message SHALL be `leaf {index} was written, but the leaves directory could not be flushed afterwards, so its durability is uncertain; reopen the store: {source}`, naming both the index and the source error so the cause of the failed flush is never lost, and telling the caller to reopen the store. It is a new variant alongside the others: Io and LeafAlreadyWritten keep their existing fields, messages and documentation, and LeafAlreadyWritten continues to mean only that another writer holds the index. LeafStore::put_leaf's `# Errors` section in store.rs gains one line naming the new variant as the answer when a leaf was stored but its durability could not be confirmed. The LeafStore trait's methods and signatures SHALL NOT change. No existing StoreError variant SHALL be renamed, removed, or changed in its fields, message or documentation, and the only variants this brief adds are the three its requirements name: LeafDurabilityUncertain (R1), ReadOnly (R5) and RepairPending (R11).

**Acceptance:**
- Test `the_uncertain_error_names_its_index_its_cause_and_the_reopen` in crates/lys-log-store/src/file_tests.rs: `StoreError::LeafDurabilityUncertain { index: 7340033, source: std::io::Error::other("flush failed") }` constructs, and matching it against `StoreError::Io { .. }` and `StoreError::LeafAlreadyWritten { .. }` fails for both.
- In the same test, `to_string()` of that value equals `leaf 7340033 was written, but the leaves directory could not be flushed afterwards, so its durability is uncertain; reopen the store: flush failed`, so it contains the substring `7340033`, the substring `flush failed` and the substring `reopen`; and `std::error::Error::source` on it returns an error whose `to_string()` is `flush failed`.
- The `# Errors` section of `LeafStore::put_leaf` in crates/lys-log-store/src/store.rs contains an intra-doc link to `StoreError::LeafDurabilityUncertain`, and `cargo doc --no-deps` and `cargo doc --no-deps --all-features` both report zero warnings.
- `git diff` of crates/lys-log-store/src/store.rs touches no line outside doc comments.
- `git diff` of crates/lys-log-store/src/error.rs removes no line, and the StoreError variants it adds are exactly three: `LeafDurabilityUncertain`, `ReadOnly` and `RepairPending`.

**Files:**
- modify: crates/lys-log-store/src/error.rs
- modify: crates/lys-log-store/src/store.rs

**Checklist:**
- C1 — StoreError has a variant LeafDurabilityUncertain carrying the leaf index and the failed flush's std::io::Error as its source, its message naming both, distinct from Io and from LeafAlreadyWritten, and LeafStore::put_leaf's documented errors name it.

**Stories:**
- S4 (Writer, Seeing a failure after its leaf was named) — As a writer whose append failed after its leaf was named, I want an error that says the leaf is written but its durability is uncertain so that I never mistake my own leaf for another writer's.

### R2: Write and flush each leaf in a hidden, uniquely named temporary file before it has a name

WHEN put_leaf is called with the next free index, THE SYSTEM SHALL first apply its existing index checks unchanged, then write every byte of the leaf into a new temporary file in leaves/ and flush it with sync_all, before any file exists under the leaf's 20-digit name. The temporary file's name SHALL be made by one pure function in file.rs, leaf_temp_name(pid: u32, index: u64, sequence: u64) -> String, returning `.` followed by pid, `-`, index zero-padded to LEAF_NAME_WIDTH digits, `-`, sequence, and `.tmp`. The sequence SHALL be supplied by the caller: create_leaf_temp(leaves_dir, index, next_sequence), write_leaf_temp(leaves_dir, index, write_contents, next_sequence) and put_leaf_with(index, bytes, next_sequence) in file.rs each take next_sequence: &mut impl FnMut() -> u64, and LeafStore::put_leaf calls put_leaf_with with a closure over one process-wide atomic counter, so the name put_leaf uses is not deterministic across calls and two writers never share one temporary file. create_leaf_temp SHALL name the file with the current process id, the index and the sequence next_sequence returns, and SHALL open it with OpenOptions create_new(true) and SHALL NOT open it with truncate; WHEN create_new fails with AlreadyExists, THE SYSTEM SHALL call next_sequence again and try the name it gives, so a leftover temporary file never blocks the write and is never replaced. write_leaf_temp SHALL create the file through create_leaf_temp, write the leaf's bytes into it and flush it. IF writing or flushing the temporary file fails, THEN THE SYSTEM SHALL remove this writer's own temporary file and return StoreError::Io with context naming the temporary file, SHALL NOT create any file under the leaf's 20-digit name, and SHALL NOT advance the extent. THE SYSTEM SHALL NOT write any leaf byte through a file opened under the leaf's final name, and SHALL NOT remove, truncate or write any temporary file other than the one this call created.

**Acceptance:**
- Test `leaf_temp_names_differ_by_sequence_and_by_pid`: `leaf_temp_name(42, 1, 7)` equals `.42-00000000000000000001-7.tmp`; `leaf_temp_name(42, 1, 7)` differs from `leaf_temp_name(42, 1, 8)`; and `leaf_temp_name(42, 1, 7)` differs from `leaf_temp_name(43, 1, 7)`.
- Test `a_temporary_file_never_replaces_an_existing_file`: a store holding leaf 0 = b"leaf-0" has a file holding b"pre-created" written into leaves/ under `leaf_temp_name(std::process::id(), 1, 0)`; the test calls `put_leaf_with(1, b"leaf-1", next)` where `next` is the test's own closure that returns 0, 1, 2 and so on from zero and records every value it returns; afterwards the recorded values are exactly `[0, 1]`, so the write met the pre-created file at sequence 0 and took its temporary file at sequence 1, `leaf(1)` is `Some(b"leaf-1".to_vec())`, and the pre-created file still exists and reads b"pre-created".
- Test `two_handles_on_one_directory_leave_no_temporary_name`: on a store directory holding leaf 0, one handle's `put_leaf(1, b"leaf-1")` returns `Ok(())` and leaves/ then holds no entry whose name begins with `.`; a second handle opened on the same directory afterwards returns `Ok(())` from `put_leaf(2, b"leaf-2")` and leaves/ again holds no entry whose name begins with `.`; `leaves/00000000000000000001` reads b"leaf-1", `leaves/00000000000000000002` reads b"leaf-2", and the counter's value after both calls is at least 2 greater than its value before the first.
- Test `a_write_failing_before_the_link_leaves_no_leaf`: a store holding leaf 0 = b"leaf-0", with the temporary file's flush failed by the fault seam, returns `StoreError::Io` from `put_leaf(1, b"leaf-1")`; afterwards `leaves/00000000000000000001` does not exist, `extent()` is 1, and `FileLeafStore::open` on the same directory returns a store whose `extent()` is 1.
- Test `a_failed_temporary_write_leaves_no_temporary_file`: a store holding leaf 0 = b"leaf-0", with the temporary file's write failed by the fault seam, returns an `Err` from `put_leaf(1, b"leaf-1")`, and afterwards leaves/ holds no entry whose name begins with `.`.
- Within `put_leaf` in crates/lys-log-store/src/file.rs, the value returned by `leaf_path` is passed only to `std::fs::hard_link`, as the link's destination, and to no `OpenOptions::open`, `File::create`, `std::fs::write` or `std::fs::rename` call.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C2 — FileLeafStore::put_leaf writes and flushes a leaf's bytes in a dot-prefixed temporary file in leaves/, named by leaf_temp_name from the process id, the index and a sequence the caller supplies (a process-wide counter for put_leaf) and created with create_new so it never replaces an existing file, before any file exists under the leaf's 20-digit name, and removes that temporary file when writing or flushing it fails.

**Stories:**
- S1 (Consumer library, Settling an uncertain append by reading the leaf back) — As the identity directory committing through the file store, I want a leaf that reads back under its final name to be whole and flushed so that settling an uncertain append by reading the leaf back gives the committed answer.
- S2 (Operator, Reopening a log after a crash or a failed write) — As an operator reopening a log after a crash, I want only whole, flushed leaves to be counted so that a torn leaf is never pinned into the tree.

### R3: Give the leaf its final name with a link that refuses to replace

WHEN the temporary file is flushed, THE SYSTEM SHALL give the leaf its 20-digit name with std::fs::hard_link from the temporary name, an operation that fails rather than replace an existing entry. IF the link fails because the final name already exists, THEN THE SYSTEM SHALL remove this writer's own temporary file and return StoreError::LeafAlreadyWritten { index }, and SHALL NOT change, replace or remove the existing file under that name. IF the link fails for any other reason, THEN THE SYSTEM SHALL remove this writer's own temporary file and return StoreError::Io with context naming both paths. THE SYSTEM SHALL NOT use std::fs::rename, or any operation that can replace an existing entry, to create a leaf's final name.

**Acceptance:**
- Test `the_link_alone_refuses_a_leaf_this_store_never_saw` (the existing test `create_new_alone_refuses_a_leaf_this_store_never_saw`, renamed, its assertions unchanged): with leaf 0 stored and `leaves/00000000000000000001` written behind the store's back with b"another writers leaf", `put_leaf(1, b"ours")` returns `StoreError::LeafAlreadyWritten { index: 1 }` and `leaves/00000000000000000001` still reads b"another writers leaf".
- Test `a_refused_link_leaves_no_temporary_file`: with leaf 0 stored and `leaves/00000000000000000001` written behind the store's back with b"another writers leaf", the test calls `put_leaf(1, b"ours")` and ignores its result, and afterwards leaves/ holds no entry whose name begins with `.`; the test asserts nothing about the returned value or the bytes under the final name.
- The existing test `a_reused_index_is_refused_by_the_ordinary_route_too` passes unchanged.
- `grep -n 'fs::rename' crates/lys-log-store/src/file.rs` prints exactly one line, inside `write_state`.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C3 — put_leaf gives the leaf its final name with a link that refuses to replace an existing entry, and an existing final name returns LeafAlreadyWritten for that index with the existing bytes unchanged and this writer's temporary file removed.

**Stories:**
- S3 (Writer, Racing another writer for the same index) — As a writer whose index another writer has already taken, I want my write refused by name so that neither writer's leaf is replaced.

### R4: Make the link the commit point for the steps after it

WHEN the link succeeds, THE SYSTEM SHALL advance the extent past the leaf before any later step, then remove the temporary name, then flush leaves/. IF removing the temporary name fails, THEN THE SYSTEM SHALL continue to the flush and SHALL return Ok when the flush succeeds; the leftover is ignored at open (R5). IF flushing leaves/ fails, THEN THE SYSTEM SHALL return StoreError::LeafDurabilityUncertain for that leaf's index with the flush's std::io::Error as its source, and SHALL NOT return StoreError::Io or StoreError::LeafAlreadyWritten for it. WHILE a handle holds a leaf whose durability is uncertain, WHEN put_leaf is called with any index, THE SYSTEM SHALL return StoreError::LeafDurabilityUncertain carrying the uncertain leaf's index and a source of the same kind and message as the original flush error, before any other check, and SHALL NOT write any file; the refusal ends only when the store is reopened. THE SYSTEM SHALL NOT refuse leaf, extent, origin, pinned or pin on that handle, and SHALL NOT report a leaf this handle linked as LeafAlreadyWritten.

**Acceptance:**
- Test `a_failed_temporary_removal_after_the_link_still_returns_ok`: a store holding leaf 0, with the temporary name's removal failed by the fault seam, returns `Ok(())` from `put_leaf(1, b"leaf-1")`; then `extent()` is 2, `leaf(1)` is `Some(b"leaf-1".to_vec())`, and `put_leaf(2, b"leaf-2")` returns `Ok(())`.
- Test `a_failed_directory_flush_after_the_link_returns_the_uncertain_error`: a store holding leaf 0, with the leaves/ flush failed by the fault seam, returns an error matching `StoreError::LeafDurabilityUncertain { index: 1, .. }` from `put_leaf(1, b"leaf-1")`.
- Test `the_uncertain_error_carries_the_flush_error_as_its_source`: with the leaves/ flush failed by the fault seam, the error returned by `put_leaf(1, b"leaf-1")` has a `std::error::Error::source` whose `to_string()` equals `injected leaves directory flush failure` and whose downcast `std::io::Error` has `kind()` equal to `std::io::ErrorKind::TimedOut`.
- Test `an_uncertain_handle_refuses_further_appends_by_name`: after that failed flush, on the same handle, `put_leaf(2, b"leaf-2")` and `put_leaf(1, b"leaf-1")` each return an error matching `StoreError::LeafDurabilityUncertain { index: 1, .. }`, and `leaves/00000000000000000002` does not exist.
- Test `the_extent_advances_past_a_linked_leaf_whose_flush_failed`: after that failed flush, `extent()` on the same handle is 2.
- Test `a_reopen_after_an_uncertain_flush_counts_the_leaf_and_appends_at_the_next_index`: after that failed flush, `FileLeafStore::open` on the same directory returns a store whose `extent()` is 2 and whose `leaf(1)` is `Some(b"leaf-1".to_vec())`, and on it `put_leaf(2, b"leaf-2")` returns `Ok(())` and `extent()` is then 3.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C4 — A successful link advances the extent; put_leaf then removes the temporary name and flushes leaves/, and a failure to remove the temporary name still returns Ok.
- C5 — A failure to flush leaves/ after the link returns LeafDurabilityUncertain for that index, and the handle refuses every further put_leaf with that error until the store is reopened.

**Stories:**
- S1 (Consumer library, Settling an uncertain append by reading the leaf back) — As the identity directory committing through the file store, I want a leaf that reads back under its final name to be whole and flushed so that settling an uncertain append by reading the leaf back gives the committed answer.
- S4 (Writer, Seeing a failure after its leaf was named) — As a writer whose append failed after its leaf was named, I want an error that says the leaf is written but its durability is uncertain so that I never mistake my own leaf for another writer's.

### R5: Flush leaves/ at a writable open before counting, count without flushing at a read-only open, refuse a leaf write and a pin on a read-only handle, and leave leftovers untouched

WHEN FileLeafStore::open reaches the leaf count, THE SYSTEM SHALL flush leaves/ with the existing fsync_dir helper before contiguous_extent enumerates it. IF that flush fails, THEN THE SYSTEM SHALL return StoreError::Io whose context names the leaves directory and the flush, and SHALL NOT count the leaves, return a store, or write any file. FileLeafStore gains a public constructor open_read_only(dir: &Path) -> StoreResult<Self>, documented as the open for a reader, such as the store handed to Anchor::open_read_only: WHEN it is called, THE SYSTEM SHALL perform every check FileLeafStore::open performs, in the same order, except the flush of leaves/, and SHALL count the leaves that are named; it SHALL NOT flush leaves/ and SHALL NOT fail because a flush of leaves/ would fail. StoreError gains a variant ReadOnly with two fields, path: PathBuf, the store's directory, and operation: &'static str, the act refused, and the message `refusing to {operation} in the log store at {path}: it was opened read-only`; no existing variant's fields, message or documentation change. WHEN put_leaf is called on a handle returned by FileLeafStore::open_read_only, THE SYSTEM SHALL return StoreError::ReadOnly with path equal to the store's directory and operation `write a leaf`, before any other check, and SHALL NOT create, write, link, remove or flush any file; only a handle returned by FileLeafStore::open writes a leaf. WHEN pin is called on a handle returned by FileLeafStore::open_read_only, THE SYSTEM SHALL return StoreError::ReadOnly with path equal to the store's directory and operation `pin a root`, before any other check, and SHALL NOT write, rename or remove state.json or any other file, and SHALL NOT change the handle's pinned root; a pin is a write, and only a handle returned by FileLeafStore::open pins. contiguous_extent SHALL keep skipping every entry whose name begins with '.', so a leftover temporary file is never counted as a leaf. Neither FileLeafStore::open nor FileLeafStore::open_read_only SHALL delete, rename, truncate, write or otherwise change any file in the store's directory, and neither SHALL refuse to open because a leftover temporary file is present.

**Acceptance:**
- In crates/lys-log-store/src/file.rs, on the path `FileLeafStore::open` takes, the call to `fsync_dir` on the `leaves` directory comes before the call to `contiguous_extent`.
- Test `a_leftover_temporary_file_is_ignored_at_open`: a store holding leaf 0 = b"leaf-0", with a file holding b"torn" written into leaves/ under `leaf_temp_name(std::process::id() + 1, 1, 0)`, another writer's crash, opens with `extent()` 1; afterwards that file still exists and reads b"torn"; on the opened store `put_leaf(1, b"leaf-1")` returns `Ok(())`, `leaves/00000000000000000001` reads b"leaf-1", and the leftover still reads b"torn".
- Test `a_read_only_open_never_flushes_the_leaves_directory`: a store holding leaf 0 = b"leaf-0" and leaf 1 = b"leaf-1", opened read-only through the fault seam with the leaves/ flush at open armed to fail, returns `Ok` with `extent()` 2 and `leaf(1)` equal to `Some(b"leaf-1".to_vec())`; and `FileLeafStore::open_read_only` on the same directory, without the seam, returns a store whose `extent()` is 2.
- Test `a_writable_open_whose_leaves_flush_fails_is_refused_and_writes_nothing`: a store holding leaf 0 = b"leaf-0" and leaf 1 = b"leaf-1" has every file under its directory recorded as (path relative to the directory, bytes); opened writable through the fault seam with the leaves/ flush at open armed to fail, it returns `StoreError::Io` whose `context` contains `dir.join("leaves").display().to_string()` and the substring `flush`, and whose source has `kind()` equal to `std::io::ErrorKind::TimedOut`; the set of (path, bytes) recorded afterwards equals the set recorded before.
- Test `a_read_only_handle_refuses_put_leaf_by_name`: a store holding leaf 0 = b"leaf-0" has every file under its directory recorded as (path relative to the directory, bytes); on the handle `FileLeafStore::open_read_only` returns for it, `put_leaf(1, b"leaf-1")` returns `StoreError::ReadOnly { path, operation }` with `path` equal to the directory and `operation` equal to `write a leaf`, whose `to_string()` contains `dir.display().to_string()` and the substring `read-only`; `extent()` is then 1, and the set of (path, bytes) recorded afterwards equals the set recorded before.
- Test `a_read_only_handle_refuses_pin_by_name`: a store created in a temporary directory, with `put_leaf(0, b"leaf-0")`, `pin(PinnedRoot { tree_size: 1, root: [1; 32] })` and `put_leaf(1, b"leaf-1")` applied through `FileLeafStore::create`'s handle, so it stands one leaf ahead of its pin, is opened with `FileLeafStore::open_read_only`; `lys_core::merkle::raw_leaf_hash` of the bytes of `state.json` is recorded; `pin(PinnedRoot { tree_size: 2, root: [2; 32] })` on that handle returns `StoreError::ReadOnly { path, operation }` with `path` equal to the directory and `operation` equal to `pin a root`; afterwards `pinned().tree_size` on the handle is 1, and `raw_leaf_hash` of the bytes of `state.json` equals the recorded value.
- The existing test `an_unexpected_leaves_entry_is_detected_but_dotfiles_are_ignored` passes unchanged.

**Files:**
- modify: crates/lys-log-store/src/error.rs
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C6 — FileLeafStore::open flushes leaves/ before it counts the leaves, and when that flush fails it returns Io naming the leaves directory and the flush, and writes nothing.
- C7 — FileLeafStore::open never counts, deletes or changes a leftover dot-prefixed temporary file in leaves/.
- C12 — FileLeafStore::open_read_only performs FileLeafStore::open's checks without flushing leaves/, counts the named leaves, and does not fail because a flush of leaves/ would fail.
- C14 — put_leaf on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly naming the store's directory and the act, and writes nothing.
- C16 — pin on a handle from FileLeafStore::open_read_only returns StoreError::ReadOnly naming the store's directory and the act, and leaves state.json and the pinned root unchanged.

**Stories:**
- S2 (Operator, Reopening a log after a crash or a failed write) — As an operator reopening a log after a crash, I want only whole, flushed leaves to be counted so that a torn leaf is never pinned into the tree.
- S5 (Read-only caller, Opening a store without writing to it) — As a read-only caller opening a store, I want open to leave every file in the directory as it found it so that a read-only open stays read-only.
- S6 (Read-only caller, Reading a log on read-only media) — As a reader of a log on media that refuses a directory flush, I want a read-only open to count the named leaves without flushing so that a status opens as it did before this change.
- S7 (Read-only caller, Reading a log that a crash left one leaf ahead of its pin) — As a reader of a log one leaf ahead of its pin, I want a read-only open to leave the store as it found it and tell me that a writable open repairs it so that reading never writes and the pending repair is never silent.

### R6: State the invariant in file.rs's module docs

file.rs's module docs gain a section stating: a file under a 20-digit leaf name is always a whole, flushed leaf; only files whose names begin with '.' may be partial, and they are never counted and never changed by open; a write that fails before a successful link removes its own temporary file; the no-replace link is the commit point, after which the leaf is this writer's and the extent advances; a failure to flush leaves/ after the link is LeafDurabilityUncertain and the handle refuses further appends until reopened; a writable open flushes leaves/ before counting, so a leaf named at a writable open is durable, and fails with Io when that flush fails; open_read_only counts the named leaves without flushing; clearing leftover temporary files that a crash stranded is not done here. The existing Layout and Durability sections SHALL NOT be weakened, and the section SHALL NOT claim that the directory flush holds on non-unix targets, where fsync_dir is a no-op.

**Acceptance:**
- The module docs of crates/lys-log-store/src/file.rs contain a heading line `//! # A named leaf is whole`.
- That section names `LeafDurabilityUncertain`, `open_read_only` and the words `commit point`, and states that open never deletes a leftover temporary file.
- The module-doc lines of file.rs present before this change are all still present, byte-identical, in `git diff`.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C8 — file.rs's module docs state that a named leaf is whole and flushed, that only hidden temporary files may be partial, and that the link is the commit point.

### R7: Add a test-only fault seam to FileLeafStore

FileLeafStore gains a private field, compiled only under cfg(test), holding at most one armed fault, and a private enum naming the five steps a fault can fail: the temporary file's write, the temporary file's flush, the temporary name's removal after the link, the leaves/ flush after the link, and the leaves/ flush at open. A private constructor, compiled only under cfg(test), opens a store writable or read-only with one fault already armed, so that the flush at open can be failed before the leaves are counted. WHEN put_leaf or open reaches an armed step, THE SYSTEM SHALL disarm the fault and treat that step as having failed with an injected std::io::Error, taking exactly the path a real failure of that step takes. The error injected for the leaves/ flush after the link, and for the leaves/ flush at open, SHALL be std::io::Error::new(std::io::ErrorKind::TimedOut, "injected leaves directory flush failure"), whose kind is not Other and whose message is not `flush failed`. The seam SHALL NOT be public, SHALL NOT exist in any build other than the crate's own tests, and SHALL NOT change put_leaf's, open's or open_read_only's behaviour when no fault is armed.

**Acceptance:**
- Every line in crates/lys-log-store/src/file.rs that declares or reads the fault field, the fault enum or the constructor that opens with a fault armed is under a `#[cfg(test)]` attribute or inside an item that is.
- `cargo clippy --all-targets -- -D warnings` and `cargo clippy --all-targets --all-features -- -D warnings` pass with no `#[allow]` added to file.rs.
- The fault field, the fault enum and the constructor that opens with a fault armed in crates/lys-log-store/src/file.rs are declared with no visibility modifier: neither `pub` nor `pub(crate)` nor `pub(super)` appears on their declaration lines.
- In crates/lys-log-store/src/file.rs, the seam's error for the leaves/ flush after the link and for the leaves/ flush at open is built as `std::io::Error::new(std::io::ErrorKind::TimedOut, "injected leaves directory flush failure")`.

**Files:**
- modify: crates/lys-log-store/src/file.rs

**Checklist:**
- C9 — A fault seam that fails a chosen step of put_leaf or the leaves/ flush at open exists in FileLeafStore and compiles only under cfg(test).

### R8: Add the tests and prove each against its own drift

file_tests.rs gains the eighteen new tests named in R1, R2, R3, R4 and R5's acceptance lines and renames create_new_alone_refuses_a_leaf_this_store_never_saw to the_link_alone_refuses_a_leaf_this_store_never_saw with its assertions unchanged. Its text stops stating what this brief makes false, in four places, each naming the no-replace link instead of create_new with no assertion changed: the module-doc header sentence that says the index is refused once via create_new at the filesystem; the extent row of the absent-check table, which says create_new succeeds; the comment in extent_check_alone_refuses_a_deleted_leafs_index, which says create_new would succeed; and the comment in the renamed test, which says it isolates the create_new check. The absent-check table's second row is rewritten to name the no-replace link and drift injection 13 of this brief's verification, which replaces the link with an operation that replaces an existing entry. A new drift-injection table holds one row for each of drift injections 1 to 12 and 14 to 18, each naming the one test it fails. Each injection SHALL be applied and reverted before the change lands. No test SHALL be #[ignore]d, and no existing test other than the renamed one SHALL change beyond the comment named above.

**Acceptance:**
- `cargo test -p lys-log-store --all-features` runs 40 tests in `file::tests` and all 40 pass.
- The module docs of crates/lys-log-store/src/file_tests.rs contain a new table with exactly seventeen injection rows, for drift injections 1 to 12 and 14 to 18 of this brief's verification, each naming exactly one test.
- The absent-check table in those module docs has a row naming the no-replace link, drift injection 13 and the test `the_link_alone_refuses_a_leaf_this_store_never_saw`.
- `grep -c create_new crates/lys-log-store/src/file_tests.rs` prints 0.
- `git diff` of crates/lys-log-store/src/file_tests.rs removes no line containing `assert` from any test that exists before this change.
- crates/lys-log-store/src/file.rs has fewer than 500 lines that are not blank, not comments and not under `#[cfg(test)]`.

**Files:**
- modify: crates/lys-log-store/src/file_tests.rs

**Checklist:**
- C10 — file_tests.rs holds the eighteen new tests of LYSLOGSTORE-001 R8, names the no-replace link wherever it named create_new, and each drift injection in its module-doc tables fails exactly its own test.
- C11 — Every leg of the design's gate array passes and file.rs stays under 500 lines of code.

### R9: Record in DIRECTORY-003 that it waits on this brief

DIRECTORY-003's depends_on gains LYSLOGSTORE-001 after DIRECTORY-002, so the dispatcher does not start DIRECTORY-003 before this brief lands, and DIRECTORY-003.md is re-rendered from the JSON by the method's renderer. No other field of DIRECTORY-003.json SHALL change, no other document under docs/design/directory SHALL change, and DIRECTORY-002 SHALL NOT be removed from depends_on.

**Acceptance:**
- `python3 -c "import json; print(json.load(open('docs/design/directory/briefs/DIRECTORY-003.json'))['depends_on'])"` prints `['DIRECTORY-002', 'LYSLOGSTORE-001']`.
- Loading docs/design/directory/briefs/DIRECTORY-003.json before and after the change and deleting the `depends_on` key from both gives equal objects.
- `git diff --name-only` lists no file under docs/design/directory other than briefs/DIRECTORY-003.json and briefs/DIRECTORY-003.md.
- `sh scripts/design/gate.sh` exits 0, so docs/design/directory/briefs/DIRECTORY-003.md is what DIRECTORY-003.json renders to.

**Files:**
- modify: docs/design/directory/briefs/DIRECTORY-003.json
- modify: docs/design/directory/briefs/DIRECTORY-003.md

**Checklist:**
- C13 — DIRECTORY-003's depends_on names LYSLOGSTORE-001 after DIRECTORY-002, and no other field of DIRECTORY-003 changes.

**Stories:**
- S1 (Consumer library, Settling an uncertain append by reading the leaf back) — As the identity directory committing through the file store, I want a leaf that reads back under its final name to be whole and flushed so that settling an uncertain append by reading the leaf back gives the committed answer.

### R10: Open the store for Anchor::open_read_only with FileLeafStore::open_read_only

In lys-anchor, every call to Anchor::open_read_only in Anchor::open_read_only's two doc examples in crates/lys-anchor/src/anchor/read_only.rs, in crates/lys-anchor/src/anchor/read_only_tests.rs and in crates/lys-anchor/tests/standalone_is_complete.rs SHALL take as its store the value of FileLeafStore::open_read_only on the directory it opened with FileLeafStore::open before, so the one read-only path is the one they exercise. No other line of crates/lys-anchor/tests/standalone_is_complete.rs SHALL change; the other lines of read_only.rs and read_only_tests.rs change only as R12 states, Anchor::open_read_only's signature SHALL NOT change, and no assertion in the tests that exist before this change SHALL change.

**Acceptance:**
- `grep -c 'FileLeafStore::open_read_only' crates/lys-anchor/src/anchor/read_only.rs` prints 2, `grep -c 'FileLeafStore::open_read_only' crates/lys-anchor/src/anchor/read_only_tests.rs` prints 2, and `grep -c 'FileLeafStore::open_read_only' crates/lys-anchor/tests/standalone_is_complete.rs` prints 1.
- `grep -rn -A1 'Anchor::open_read_only(' crates/lys-anchor/src/anchor/read_only.rs crates/lys-anchor/src/anchor/read_only_tests.rs crates/lys-anchor/tests/standalone_is_complete.rs` prints no line containing `FileLeafStore::open(`.
- `git diff --name-only -- crates/lys-anchor` lists exactly crates/lys-anchor/src/anchor/read_only.rs, crates/lys-anchor/src/anchor/read_only_tests.rs, crates/lys-anchor/src/anchor/status.rs and crates/lys-anchor/tests/standalone_is_complete.rs, and every line `git diff -- crates/lys-anchor/tests/standalone_is_complete.rs` adds contains `FileLeafStore::open_read_only`.
- `cargo test -p lys-anchor --all-features` passes, including the doc tests of crates/lys-anchor/src/anchor/read_only.rs, the passing example and the compile_fail example each as before.

**Files:**
- modify: crates/lys-anchor/src/anchor/read_only.rs
- modify: crates/lys-anchor/src/anchor/read_only_tests.rs
- modify: crates/lys-anchor/tests/standalone_is_complete.rs

**Checklist:**
- C15 — Anchor::open_read_only's doc examples and lys-anchor's read-only tests open their store with FileLeafStore::open_read_only.

**Stories:**
- S6 (Read-only caller, Reading a log on read-only media) — As a reader of a log on media that refuses a directory flush, I want a read-only open to count the named leaves without flushing so that a status opens as it did before this change.

### R11: Open a log at its pin without repairing it

Log gains a public constructor open_at_pin(store: S) -> StoreResult<Self> and a public accessor pending_repair(&self) -> Option<u64>. WHEN open_at_pin is called, THE SYSTEM SHALL read every leaf in the store's extent, rebuild the tree and compare it with the pin exactly as Log::open does. WHEN the rebuilt tree equals the pin, THE SYSTEM SHALL return the log as Log::open would, with pending_repair() None. WHEN the store holds exactly one leaf more than the pinned tree size and the pinned-size prefix rebuilds to the pinned root, THE SYSTEM SHALL NOT call the store's pin, SHALL hold the tree and the leaves of the pinned prefix only, so tree().len() equals the pinned tree size, SHALL report pending_repair() as Some(the store's extent), the tree size a writable open repairs to, and recovered_to() as None, and SHALL return StoreError::RepairPending from every append on that log without calling put_leaf, because the store already holds a leaf at the next index and the store never reports its own leaf as another writer's; it SHALL NOT return StoreError::Poisoned, which keeps its one meaning, an append that failed after storing its leaf on this handle. StoreError gains a variant RepairPending with two fields, pinned_tree_size: u64, the pin's tree size the log was opened at, and leaves: u64, the count of leaves the store held, and the message `refusing to append: the log has a pending repair, its store holds {leaves} leaves but its pin is at tree size {pinned_tree_size}; a writable open must repair it before any append`; it is documented as: the log was opened at its pin with one leaf standing ahead of the pin, and a writable open (Log::open over a handle from FileLeafStore::open) must repair it before any append. Log::append's `# Errors` section gains one line naming StoreError::RepairPending, and Log::append gains only the check that returns it, placed before its existing checks. Every other divergence SHALL return StoreError::PinMismatch as Log::open does. Log::open SHALL NOT change: it alone performs the one-leaf-ahead repair, through the store's pin, and its pending_repair() is always None. THE SYSTEM SHALL NOT write, rename or delete any file during open_at_pin.

**Acceptance:**
- Test `a_log_opened_at_its_pin_leaves_the_pin_and_refuses_to_append_while_a_repair_is_pending` in crates/lys-log-store/src/log_tests.rs: a directory initialised with `FileLeafStore::create(dir, ORIGIN)`, opened with `Log::open(FileLeafStore::open(dir))` and given one `append(b"leaf-0")`, and that log dropped, then `leaves/00000000000000000001` written behind it with b"an append interrupted before its pin", is opened with `Log::open_at_pin(FileLeafStore::open(dir))`; it returns `Ok` with `tree().len()` 1, `pending_repair()` `Some(2)`, `recovered_to()` `None` and `store().pinned().tree_size` 1; `append(b"leaf-2")` on it returns `Err(err)` where `err` matches `StoreError::RepairPending { pinned_tree_size: 1, leaves: 2 }`, `matches!(err, StoreError::Poisoned)` is false, and `err.to_string()` equals `refusing to append: the log has a pending repair, its store holds 2 leaves but its pin is at tree size 1; a writable open must repair it before any append`; afterwards leaves/ holds exactly the two names `00000000000000000000` and `00000000000000000001`, and the bytes of `state.json` equal the bytes recorded before the open.
- In the same test, after the refused append and with the `open_at_pin` log dropped, `Log::open(FileLeafStore::open(dir))` returns `Ok` with `recovered_to()` `Some(2)`, `pending_repair()` `None` and `store().pinned().tree_size` 2; `append(b"leaf-2")` on that log then returns `Ok` with index 2, and afterwards `tree().len()` is 3 and `store().pinned().tree_size` is 3.
- `git diff` of crates/lys-log-store/src/log.rs removes no line from the body of `Log::open`, `reconcile_with_pin` or `Log::append`, and adds to the body of `Log::append` only the check that returns `StoreError::RepairPending`.

**Files:**
- modify: crates/lys-log-store/src/error.rs
- modify: crates/lys-log-store/src/log.rs
- modify: crates/lys-log-store/src/log_tests.rs

**Checklist:**
- C17 — Log::open_at_pin opens a store found one leaf ahead of its pin at the pinned head without pinning, reports the pending repair, and refuses every append by name while it is pending, never as Poisoned; Log::open alone repairs.

**Stories:**
- S7 (Read-only caller, Reading a log that a crash left one leaf ahead of its pin) — As a reader of a log one leaf ahead of its pin, I want a read-only open to leave the store as it found it and tell me that a writable open repairs it so that reading never writes and the pending repair is never silent.

### R12: Make Anchor::open_read_only open at the pinned head and say that a repair is pending

Anchor::open_read_only SHALL open its log with Log::open_at_pin in place of Log::open, and SHALL keep its refusal of a log with no leaves unchanged. WHEN the store stands exactly one leaf ahead of its pin, THE SYSTEM SHALL open at the pinned head, SHALL NOT repair the store or pin through it, whichever store it is handed, and SHALL report the pending repair. AnchorStatus gains a field pending_repair: Option<u64>, read from the log on every call to Anchor::status: the tree size a writable open repairs the log to, or None when no repair is pending; and a method pending_repair_notice(&self) -> Option<String> which, when pending_repair is Some(n), returns `one leaf stands ahead of the pin at tree size {tree_size}; a writable open repairs it to tree size {n}`, where tree_size is the status's tree size, and returns None otherwise. The module docs of read_only.rs and Anchor::open_read_only's docs SHALL state that a read-only open never repairs a store found one leaf ahead of its pin, that it opens at the pinned head and reports the pending repair through AnchorStatus, and that only a writable open repairs it; they SHALL NOT say that open_read_only repairs an interrupted append. Anchor::open and its repair through Log::open SHALL NOT change, and no existing field of AnchorStatus SHALL change.

**Acceptance:**
- Test `a_reader_opens_a_log_one_leaf_ahead_at_its_pin_and_reports_the_pending_repair` in crates/lys-anchor/src/anchor/read_only_tests.rs: an anchor created with its genesis leaf, then `leaves/00000000000000000001` written behind it with b"an append interrupted before its pin", and the bytes of `state.json` recorded; the helper `read_only(dir)`, which unwraps `Anchor::open_read_only(FileLeafStore::open_read_only(dir).unwrap(), AnchorConfig::unconfigured())`, returns without panicking, and the `ReadOnlyAnchor` it returns has `tree_size()` 1 and `recovered_to()` `None`; its `status().pending_repair` is `Some(2)` and its `status().pending_repair_notice()` is `Some("one leaf stands ahead of the pin at tree size 1; a writable open repairs it to tree size 2".to_string())`; afterwards the bytes of `state.json` equal the recorded bytes.
- Test `a_writable_open_repairs_the_log_a_reader_found_one_leaf_ahead`: over the same fixture, `Log::open(FileLeafStore::open(dir).unwrap())` returns `Ok` with `recovered_to()` `Some(2)` and `store().pinned().tree_size` 2; after that log is dropped, the helper `read_only(dir)` returns without panicking, and the `ReadOnlyAnchor` it returns has `tree_size()` 2, `status().pending_repair` `None` and `status().pending_repair_notice()` `None`.
- Test `a_reader_over_a_clean_log_reports_no_pending_repair` in crates/lys-anchor/src/anchor/read_only_tests.rs: an anchor created with its genesis leaf and two statements appended through it, then dropped; the `ReadOnlyAnchor` the helper `read_only(dir)` returns has `status().pending_repair` `None`, `status().pending_repair_notice()` `None` and `recovered_to()` `None`.
- `grep -c 'Log::open_at_pin' crates/lys-anchor/src/anchor/read_only.rs` prints at least 1, and `grep -n 'Log::open(' crates/lys-anchor/src/anchor/read_only.rs` prints nothing.
- The module docs of crates/lys-anchor/src/anchor/read_only.rs contain the words `never repairs` and `pending_repair`, and contain no sentence stating that open_read_only repairs an interrupted append.

**Files:**
- modify: crates/lys-anchor/src/anchor/read_only.rs
- modify: crates/lys-anchor/src/anchor/read_only_tests.rs
- modify: crates/lys-anchor/src/anchor/status.rs

**Checklist:**
- C18 — Anchor::open_read_only never repairs a store found one leaf ahead of its pin: it opens at the pinned head, and its status says in words that one leaf stands ahead of the pin and that a writable open repairs it.

**Stories:**
- S7 (Read-only caller, Reading a log that a crash left one leaf ahead of its pin) — As a reader of a log one leaf ahead of its pin, I want a read-only open to leave the store as it found it and tell me that a writable open repairs it so that reading never writes and the pending repair is never silent.

### R13: Render the lys-log-store cluster's documents from their JSON

docs/design/lys-log-store/DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSLOGSTORE-001.md SHALL be what render-cluster.py renders from design.json, checklist.json, stories.json and briefs/LYSLOGSTORE-001.json, and WHEN any of those JSON documents changes, THE SYSTEM SHALL have its markdown re-rendered in the same change. The rendered files SHALL NOT be edited by hand, and BACKENDS.md SHALL NOT change.

**Acceptance:**
- `ls docs/design/lys-log-store/DESIGN.md docs/design/lys-log-store/CHECKLIST.md docs/design/lys-log-store/USER-STORIES.md docs/design/lys-log-store/briefs/LYSLOGSTORE-001.md` lists all four files and exits 0.
- With those four files present, `sh scripts/design/gate.sh` exits 0 and prints no line containing `rendered markdown differs`.
- `git diff --name-only -- docs/design/lys-log-store/BACKENDS.md` prints nothing.

**Files:**
- modify: docs/design/lys-log-store/DESIGN.md
- modify: docs/design/lys-log-store/CHECKLIST.md
- modify: docs/design/lys-log-store/USER-STORIES.md
- modify: docs/design/lys-log-store/briefs/LYSLOGSTORE-001.md

**Checklist:**
- C19 — The lys-log-store cluster's DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/LYSLOGSTORE-001.md are what render-cluster.py renders from its JSON documents, and the design gate passes with them present.

## Boundaries

- SHALL NOT delete, rename, truncate or write any file during FileLeafStore::open or FileLeafStore::open_read_only.
- SHALL NOT add a method to the LeafStore trait or change any existing method's signature.
- SHALL NOT change the on-disk layout: log.json, state.json, leaves/<20-digit index> holding the raw leaf bytes, and the lys/log-dir/v1 marker.
- SHALL NOT change Log::open, Log's append order or Log::open's one-leaf-ahead repair, and SHALL NOT change Log::append other than by R11's refusal with StoreError::RepairPending and its line in the `# Errors` section; crates/lys-log-store/src/log.rs changes only by R11's additions.
- SHALL NOT change any file outside crates/lys-log-store other than DIRECTORY-003.json, DIRECTORY-003.md, R10's three lys-anchor files and R12's crates/lys-anchor/src/anchor/status.rs and the lys-log-store cluster's own design documents under docs/design/lys-log-store other than BACKENDS.md; in crates/lys-anchor/tests/standalone_is_complete.rs only the line that opens a store for Anchor::open_read_only changes.
- SHALL NOT pin, write, rename or delete any file in the store's directory while Anchor::open_read_only or Log::open_at_pin opens a store found one leaf ahead of its pin.
- SHALL NOT use StoreError::LeafAlreadyWritten for any failure other than another writer holding the index.
- SHALL NOT implement clearing of leftover temporary files that a crash stranded; that is a further unit. Removing this writer's own temporary file after its own failed write is not that unit.
- SHALL NOT add a dependency, unsafe code, or unwrap, expect, panic, todo, unimplemented or unreachable in library code.
- SHALL NOT silence a lint with #[allow], #[ignore] a test, or use #[cfg(any())].
- SHALL NOT change any document under docs/design/directory other than DIRECTORY-003's depends_on and its re-rendered DIRECTORY-003.md.
- SHALL NOT return StoreError::Poisoned for any failure other than an append that failed after storing its leaf on the same Log handle.

## Verification

- Run every leg of the design's gate array from the repository root: cargo fmt --all then git diff --exit-code, cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features, cargo doc --no-deps, sh scripts/design/gate.sh; each exits 0.
- Run every drift injection below with `cargo test -p lys-log-store --all-features`; every test that uses a sequence supplies its own, so the result does not depend on the order in which tests run.
- Drift injection 1: flush the temporary file after the link instead of before it; exactly one test fails, a_write_failing_before_the_link_leaves_no_leaf. Revert.
- Drift injection 2: make open remove every dot-prefixed entry in leaves/ before counting; exactly one test fails, a_leftover_temporary_file_is_ignored_at_open. Revert.
- Drift injection 3: return StoreError::Io instead of StoreError::LeafAlreadyWritten when the link finds the final name taken; exactly one test fails, the_link_alone_refuses_a_leaf_this_store_never_saw. Revert.
- Drift injection 4: return StoreError::Io when removing the temporary name fails; exactly one test fails, a_failed_temporary_removal_after_the_link_still_returns_ok. Revert.
- Drift injection 5: return StoreError::Io, with the same source, instead of LeafDurabilityUncertain when the leaves/ flush after the link fails, leaving the refusal and the extent as they are; exactly one test fails, a_failed_directory_flush_after_the_link_returns_the_uncertain_error. Revert.
- Drift injection 6: skip the handle's refusal after an uncertain flush; exactly one test fails, an_uncertain_handle_refuses_further_appends_by_name. Revert.
- Drift injection 7: advance the extent only after the leaves/ flush succeeds; exactly one test fails, the_extent_advances_past_a_linked_leaf_whose_flush_failed. Revert.
- Drift injection 8: skip removing the temporary file when writing or flushing it fails; exactly one test fails, a_failed_temporary_write_leaves_no_temporary_file. Revert.
- Drift injection 9: skip removing the temporary file when the link fails; exactly one test fails, a_refused_link_leaves_no_temporary_file. Revert.
- Drift injection 10: build LeafDurabilityUncertain's source as a new std::io::Error of kind Other with the message `flush failed` instead of the flush's own error; exactly one test fails, the_uncertain_error_carries_the_flush_error_as_its_source. Revert.
- Drift injection 11: open the temporary file in write_leaf_temp with create(true).truncate(true) instead of create_new(true); exactly one test fails, a_temporary_file_never_replaces_an_existing_file. Revert.
- Drift injection 12: build the temporary name in leaf_temp_name without the pid; exactly one test fails, leaf_temp_names_differ_by_sequence_and_by_pid. Revert.
- Drift injection 13: give the leaf its final name with std::fs::rename in place of std::fs::hard_link; exactly one test fails, the_link_alone_refuses_a_leaf_this_store_never_saw. Revert.
- Drift injection 14: make FileLeafStore::open_read_only flush leaves/ before counting, as FileLeafStore::open does; exactly one test fails, a_read_only_open_never_flushes_the_leaves_directory. Revert.
- Drift injection 15: make FileLeafStore::open ignore a failed leaves/ flush and go on to count; exactly one test fails, a_writable_open_whose_leaves_flush_fails_is_refused_and_writes_nothing. Revert.
- Drift injection 16: remove `; reopen the store` from LeafDurabilityUncertain's message; exactly one test fails, the_uncertain_error_names_its_index_its_cause_and_the_reopen. Revert.
- Drift injection 17: skip the ReadOnly refusal in put_leaf on a handle from FileLeafStore::open_read_only; exactly one test fails, a_read_only_handle_refuses_put_leaf_by_name. Revert.
- Drift injection 18: skip the ReadOnly refusal in pin on a handle from FileLeafStore::open_read_only; exactly one test fails, a_read_only_handle_refuses_pin_by_name. Revert.
- Drift injection 19, run with `cargo test -p lys-log-store --all-features`: remove the check in Log::append that returns StoreError::RepairPending while a repair is pending; exactly one test fails, a_log_opened_at_its_pin_leaves_the_pin_and_refuses_to_append_while_a_repair_is_pending. Revert.
- Drift injection 20, run with `cargo test -p lys-anchor --all-features`: make Anchor::open_read_only open its log with Log::open instead of Log::open_at_pin; exactly one test fails, a_reader_opens_a_log_one_leaf_ahead_at_its_pin_and_reports_the_pending_repair. Revert.
- Confirm that a_reopen_after_an_uncertain_flush_counts_the_leaf_and_appends_at_the_next_index, two_handles_on_one_directory_leave_no_temporary_name and a_writable_open_repairs_the_log_a_reader_found_one_leaf_ahead have no injection row: the first guards the reopen path, and the open-time flush it rides on is verified by reading the line order in FileLeafStore::open, since no test here observes a power loss; the second guards the absence of leftovers across handles, whose removal steps drift injections 8 and 9 already isolate; the third guards Log::open's existing one-leaf-ahead repair, which this brief does not change and which lys-anchor's an_interrupted_append_is_repaired_and_reported_rather_than_swallowed already guards.
- grep -rn 'unwrap()\|expect(\|panic!' crates/lys-log-store/src/file.rs crates/lys-log-store/src/error.rs finds nothing outside the tests module.

