---
type: brief
id: HOME-004
cluster: home
title: Ship a home as a git ref and fetch it into a second home on the same machine
---

# HOME-004: Ship a home as a git ref and fetch it into a second home on the same machine

> **Cluster:** home
> **Depends on:** HOME-001
> **Blocked by:** Card ct98Wv-2, the Claude Code launch template (the stage 2 template subcommand, with typed substitution and refusal of a missing input). This brief is built on lys main after that card lands, and R10 uses its template subcommand to render the fetched home and print the launch line.
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-012 — A home moves as a pushed git ref that the target fetches, and the target records its own execution id and ancestry — A home moves by the same route as a build tree: its tracked files are committed in the home's own git repository and pushed as one ref to a remote named on the command line, and the target fetches that ref into a new home. This over a hand copy (cp, rsync or an archive carried between directories), which leaves no commit to name, no tree hash to check the arrival against and no ancestry. The target mints its own execution id and records its arrival (source commit, remote, ref and execution id) in each session; source artifacts are never rewritten, and credentials are supplied at launch on the target, never tracked in the home.
> **Checklist:**
> - C14 — Ship commits exactly a home's session files, their index and head files and its blocks store in the home's own git repository, pushes that commit as one ref to the remote named on the command line, and reports the commit, the ref and every lock or temporary file it left behind.
> - C15 — Ship refuses by name a home in which any session's index is missing or stale, names the index step that fixes it and writes nothing; a session without a head file ships as it is, and the source home's files are byte-identical before and after every ship.
> - C16 — A separate index subcommand rebuilds a session's missing or stale index, after which the same ship command runs unchanged and succeeds.
> - C17 — Ship refuses by name every file in the home that is not a home file, and every place a tracked file holds a value from the credential values file named on the command line or matches one of five standard credential patterns, giving the file and the byte offset and never the value; nothing is redacted.
> - C18 — Fetch brings the shipped ref into a new directory and refuses by name one that already holds a home; the fetched tree equals the shipped tree, so the session file, index, head and blocks hash-match the source, and every index is verified against its file and every head against its index.
> - C19 — After the tree check, each fetched session gains exactly one appended lys.harness_event of kind arrival naming the source commit, the remote, the ref and a fresh execution id; the source session file is a byte prefix of the target's, the blocks still hash-match, and the index and head are the ones Session::append wrote for that entry.
> - C20 — The fetched home is rendered through the Claude Code launch template and resumed by its printed launch line on the Claude Code installed when the proof runs, its version recorded, written up in PROOF-SHIP.md in hashes, lengths, counts and paths only.
> **Stories:**
> - S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home shipped as a ref and fetched into a second home where my session is rendered and resumed, so that I move by the same route as a build tree and never by a hand copy.
> - S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fetched tree checked equal to the shipped one before anything is appended, and each fetched session then to hold the source's bytes followed by one arrival event naming its source commit and a fresh execution id, so that a moved home's ancestry is on the record and nothing was rewritten.
> - S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a home with a stale index refused by name and pointed at the step that fixes it rather than repaired and shipped, every file that is not a home file refused, and every named credential value and standard credential pattern found in the home refused by file and offset, so that what leaves a home is exactly its record and nothing secret.

## Purpose

The first block of context-roadmap stage 3, the same-machine proof. A home leaves the directory that captured it only as one git ref pushed to a named remote, and only when every index is fresh, every file is a home file and no named credential value or standard credential pattern is in any of them. It arrives in a new home that checks the fetched tree before it touches it, then records its arrival and its own execution id in every session. That is ADR-012 implemented, and the ref it ships is what the second-machine block will fetch. The render and resume at the end go through the Claude Code launch template on the Claude Code installed when the proof runs, whose version the proof records. They are the resume evidence stage 3 asks for.

## Task

In lys-home, in dependency order: a read-only index and head check (R1); a git runner with the machine's own git configuration shut off (R2); the classification of a home's files and the scan of the tracked ones for credential values (R3); ship (R4); fetch (R5); the arrival event (R6); the ship, fetch and index subcommands (R7); an integration test file against a bare remote in a temporary directory (R8); the documentation and the re-rendered cluster markdown (R9); and PROOF-SHIP.md (R10).

The home directory is itself the git repository. Ship commits exactly the session files, their index and head files and the blocks store. It never writes a source file, so it refuses by name a session whose index is missing or stale, and it does not repair it the way Index::load would. The refusal names the separate `lys-home index` step that rebuilds the index, and ship is then run again unchanged. A session with no head file is legal (RECORD.md) and ships as it is.

No credential file is ever tracked. Ship refuses by name every file in the home that is not a home file, and it scans every file it would commit, first for the values in a credential values file named on the command line and then for the five standard credential patterns this brief lists in R3. A hit is refused by file and byte offset, never by value or by the pattern that matched. Nothing is redacted: a home with a secret in a transcript is refused, not shipped. Redaction is a later card.

The acceptance's hash match is two checks. First, as fetched and before anything is appended, the fetched commit's tree equals the shipped commit's tree, so the session file, index, head and blocks hash-match the source (R5). Second, after the arrival append, the source session file is a byte prefix of the target session file, the blocks still hash-match, and the target's index and head are the ones Session::append wrote for that one arrival entry (R6). The proof records both.

The launch template is card ct98Wv-2's, not this brief's. R10 runs after it lands, and records the version of the Claude Code installed when it runs. In scope: this machine, Claude Code only, and git ref transport only. Out of scope: the named second machine and its two preconditions (read authority over the home, and encryption before bytes leave the machine); harnesses other than Claude Code; encryption of the home; any transport other than a git ref to a named remote; redaction.

## Requirements

### R1: Check a session's index and head without writing anything

WHEN asked to check a session file's index, THE SYSTEM SHALL read the session header and the index file. It SHALL answer fresh, with the session id and the row count, only when the rows meet the rules Index::from_cached already applies: each row starts where the one before ended, has a non-zero length, an id not seen before and a parent already indexed; each row ends on a newline in the session file; and the last row ends where the file ends. IF the index file is missing or its rows fail any of those rules, THEN THE SYSTEM SHALL refuse with a stale-index error that names the session id and the command that rebuilds the index, `lys-home index --home <dir> --session <id>`. WHEN asked to check a session's head against its index, THE SYSTEM SHALL answer fresh when the head file is absent (legal; read_head then reads the last indexed entry as the head), when it is empty, or when it names an indexed entry id. IF the head file names an id the index does not hold, THEN THE SYSTEM SHALL refuse with a head-mismatch error naming the session id. THE SYSTEM SHALL NOT rebuild an index. It SHALL NOT call Index::load, Session::open, Session::create or Home::open. It SHALL NOT create, write, rename or truncate any file, including the session's lock file. It SHALL NOT put any entry content in an error.

**Acceptance:**
- A fixture session with a header and 3 entries, whose index and head were written by Session::create and Session::append and whose Session was dropped before the check, has an index check that answers fresh with 3 rows and a head check that answers fresh.
- The same fixture with its index file truncated to its first 2 rows is refused with the stale-index error, whose message contains `s1` and `lys-home index`. The index file's SHA-256 and the list of file names in sessions/ are equal before and after the call.
- The same fixture with its index file deleted is refused with the stale-index error naming `s1`, and no index file exists after the call.
- The same fixture with its head file holding `not-an-entry` is refused with the head-mismatch error naming `s1`.
- The same fixture with its head file deleted passes the head check, and no head file exists after the call.
- The same fixture with an empty head file passes the head check.

**Files:**
- create: crates/lys-home/src/record/verify.rs
- create: crates/lys-home/src/record/verify_tests.rs
- modify: crates/lys-home/src/record/mod.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C15 — Ship refuses by name a home in which any session's index is missing or stale, names the index step that fixes it and writes nothing; a session without a head file ships as it is, and the source home's files are byte-identical before and after every ship.
- C18 — Fetch brings the shipped ref into a new directory and refuses by name one that already holds a home; the fetched tree equals the shipped tree, so the session file, index, head and blocks hash-match the source, and every index is verified against its file and every head against its index.

**Stories:**
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fetched tree checked equal to the shipped one before anything is appended, and each fetched session then to hold the source's bytes followed by one arrival event naming its source commit and a fresh execution id, so that a moved home's ancestry is on the record and nothing was rewritten.
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a home with a stale index refused by name and pointed at the step that fixes it rather than repaired and shipped, every file that is not a home file refused, and every named credential value and standard credential pattern found in the home refused by file and offset, so that what leaves a home is exactly its record and nothing secret.

### R2: Run git with the machine's own git configuration shut off

Structural: the `ship` module is declared in lib.rs, and its mod.rs carries only module declarations and module docs. One function in it runs the git binary for ship and fetch. Every invocation SHALL do all of the following: set GIT_CONFIG_NOSYSTEM=1; point GIT_CONFIG_GLOBAL at an empty file; remove every other GIT_* variable from the child's environment; set GIT_TERMINAL_PROMPT=0; pass -c core.hooksPath pointing at an empty directory, -c commit.gpgsign=false, -c core.autocrlf=false and -c core.fsmonitor=false; and set the author and the committer to the name `lys-home` and the email `lys-home@localhost`. IF git exits non-zero, THEN THE SYSTEM SHALL refuse with a git error naming the operation and the exit status. THE SYSTEM SHALL NOT echo git's stdout or stderr into an error or a report. It SHALL NOT run any hook, sign, convert line endings, or take a git library crate as a dependency.

**Acceptance:**
- HOME is pointed at a temporary directory whose .gitconfig sets commit.gpgsign=true, core.autocrlf=true and core.hooksPath to a directory holding a pre-commit hook that exits 1. With that set, committing through the runner a temporary repository that holds one file containing a CRLF byte pair succeeds. The SHA-256 of `git cat-file -p HEAD:<file>` equals the file's SHA-256, and `git log -1 --format=%an` prints `lys-home`.
- A push through the runner to a remote path that does not exist is refused with the git error naming `push` and a non-zero exit status, and the error text contains no line of git's stderr.
- crates/lys-home/Cargo.toml lists the same 7 dependencies before and after this brief.

**Files:**
- create: crates/lys-home/src/ship/mod.rs
- create: crates/lys-home/src/ship/git.rs
- create: crates/lys-home/src/ship/git_tests.rs
- modify: crates/lys-home/src/lib.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C14 — Ship commits exactly a home's session files, their index and head files and its blocks store in the home's own git repository, pushes that commit as one ref to the remote named on the command line, and reports the commit, the ref and every lock or temporary file it left behind.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home shipped as a ref and fetched into a second home where my session is rendered and resumed, so that I move by the same route as a build tree and never by a hand copy.

### R3: Sort a home's files and scan the tracked ones for credential values and the standard credential patterns

WHEN asked to classify a home, THE SYSTEM SHALL walk every file under the home outside `.git` and put each one in exactly one of three sets. Tracked: sessions/<id>.jsonl for an id that passes safe_component, the <id>.index.jsonl and <id>.head beside it, and blocks/<xx>/<hash> where <hash> is 64 lowercase hex characters and <xx> is its first two. Left behind: the lock file Index::lock_path names for a tracked session file, that session's temporary head file <id>.head.tmp, and a block store temporary file (a name beginning `.` and ending `.tmp`, directly under blocks/ or under blocks/<xx>/). Foreign: every other file. WHEN asked to scan, THE SYSTEM SHALL read the credential values file named to it, one value per line with the trailing line ending removed. The value is the part after the first `=` when the line holds one, and the whole line otherwise; empty values are skipped. THE SYSTEM SHALL search every tracked file's bytes first for every value and then for the standard credential patterns. The standard credential patterns are exactly these five, held in one list in the code: (1) `sk-ant-` followed by 20 or more of [A-Za-z0-9_-]; (2) `ghp_` or `github_pat_` followed by 20 or more of [A-Za-z0-9_]; (3) `AKIA` followed by exactly 16 of [A-Z0-9], with no further [A-Z0-9] byte after them; (4) `xox` then one of a, b, p, r, s, then `-`, followed by 10 or more of [A-Za-z0-9-]; (5) a PEM line: five dashes, `BEGIN `, zero or more words each followed by exactly one space, `PRIVATE KEY`, five dashes, where a word is one or more bytes from [A-Z0-9] and no other byte. THE SYSTEM SHALL report each hit, values' hits before patterns' hits, as the file's path relative to the home plus the byte offset of the hit's first byte, together with the count of files scanned, of values scanned and of patterns scanned. THE SYSTEM SHALL NOT scan for any other pattern: not a bare `sk-`, not `eyJ`, and not a run of hex characters. THE SYSTEM SHALL NOT put a value, the text of a pattern, which pattern matched, or any byte of a scanned file, in a hit, an error or a report. It SHALL NOT modify any file. It SHALL NOT count a foreign or left-behind file as tracked.

**Acceptance:**
- The fixture home has one session s1 (its file, index, head and the lock file Session::create left), 2 blocks, a file sessions/s1.head.tmp, a file sessions/launch.env beside the session files and a file notes.txt at the home's root. Classifying it gives tracked = sessions/s1.jsonl, sessions/s1.index.jsonl, sessions/s1.head and the 2 block paths (5 files); left behind = the lock path and sessions/s1.head.tmp; foreign = sessions/launch.env and notes.txt.
- A fixture session file holds `lys-fixture-secret-5b1c07` inside a tool result at byte offset N. With a values file whose one line is `LYS_FIXTURE_SECRET=lys-fixture-secret-5b1c07`, the scan reports exactly one hit, sessions/s1.jsonl at offset N, and the report serialised as JSON does not contain `lys-fixture-secret-5b1c07`.
- The same values file scanned against the fixture home without the secret reports 0 hits, 5 files scanned, 1 value scanned and 5 patterns scanned.
- A values file holding only the line `EMPTY=` and an empty line reports 0 values scanned.
- With an empty values file, a tracked file whose bytes are `x` followed by `sk-ant-` and 20 `A` characters reports exactly one hit at offset 1, and the same file with 19 `A` characters reports 0 hits.
- With an empty values file, a tracked file holding `ghp_` followed by 20 `a` characters reports one hit at offset 0; a file holding `github_pat_` followed by 20 `a` characters reports one hit at offset 0; and a file holding `ghp_` followed by 19 `a` characters reports 0 hits.
- With an empty values file, a tracked file holding `AKIA` followed by `ABCDEFGHIJ234567` reports one hit at offset 0; `AKIA` followed by the 15 characters `ABCDEFGHIJ23456` reports 0 hits; and `AKIA` followed by the 17 characters `ABCDEFGHIJ2345678` reports 0 hits.
- With an empty values file, a tracked file holding `xoxb-` followed by 10 `a` characters reports one hit at offset 0, `xoxs-` followed by 10 `a` characters reports one hit at offset 0, and `xoxb-` followed by 9 `a` characters reports 0 hits.
- With an empty values file, a tracked file holding the line `-----BEGIN RSA PRIVATE KEY-----` reports one hit at offset 0, a file holding `-----BEGIN PRIVATE KEY-----` reports one hit at offset 0, and a file holding `-----BEGIN PUBLIC KEY-----` reports 0 hits.
- With an empty values file, a tracked file holding `-----BEGIN EC2 PRIVATE KEY-----` reports one hit at offset 0; a file holding `-----BEGIN X-Y PRIVATE KEY-----` reports 0 hits; a file holding `-----BEGIN rsa PRIVATE KEY-----` reports 0 hits; and a file holding `-----BEGIN RSA  PRIVATE KEY-----`, with two spaces after `RSA`, reports 0 hits.
- With an empty values file, a tracked file holding `sk-` followed by 40 `a` characters, `eyJhbGciOiJIUzI1NiJ9` and 64 `0` characters on three lines reports 0 hits.
- For each of the 5 pattern fixtures above, the report serialised as JSON contains the file path and the offset and does not contain the matched bytes.
- A tracked file holding the values-file value at offset 0 and `ghp_` followed by 20 `a` characters at offset 40 reports 2 hits, in the order offset 0 then offset 40.
- The SHA-256 of every file in the fixture home, and the set of its file names, are equal before and after a classify and a scan.

**Files:**
- create: crates/lys-home/src/ship/scan.rs
- create: crates/lys-home/src/ship/scan_tests.rs
- modify: crates/lys-home/src/ship/mod.rs

**Checklist:**
- C14 — Ship commits exactly a home's session files, their index and head files and its blocks store in the home's own git repository, pushes that commit as one ref to the remote named on the command line, and reports the commit, the ref and every lock or temporary file it left behind.
- C17 — Ship refuses by name every file in the home that is not a home file, and every place a tracked file holds a value from the credential values file named on the command line or matches one of five standard credential patterns, giving the file and the byte offset and never the value; nothing is redacted.

**Stories:**
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a home with a stale index refused by name and pointed at the step that fixes it rather than repaired and shipped, every file that is not a home file refused, and every named credential value and standard credential pattern found in the home refused by file and offset, so that what leaves a home is exactly its record and nothing secret.

### R4: Ship: commit exactly the tracked set in the home's own repository and push one ref

WHEN asked to ship a home to a remote under a name, with a credential values file, THE SYSTEM SHALL do the following in order. (1) Take every session's lock (the file Index::lock_path names, created empty where none exists) without blocking, and hold them until the push returns. (2) Check every session's index with R1. (3) Classify the home and scan its tracked files with R3, for the named values and the five patterns. (4) Only when none of those refuses, commit exactly the tracked set in a git repository whose directory is `.git` inside the home, initialised on the first ship. The commit's parent is the repository's previous HEAD when one exists. (5) Push that commit to the remote as the one ref refs/lys/home/<name>, where <name> passes safe_component. (6) Report the commit id, its tree id, the ref, the count of sessions and of files committed, and the path relative to the home of every left-behind file. The remote is a path or a URL as given on the command line. IF a session's lock is held, THEN THE SYSTEM SHALL refuse, naming the session id. IF any session's index is refused by R1, THEN THE SYSTEM SHALL refuse, naming every such session id and the `lys-home index` command, before creating `.git` or committing. IF the home holds any foreign file or the scan finds any hit, THEN THE SYSTEM SHALL refuse, naming every foreign file's path and every hit's path and offset, before creating `.git` or committing. THE SYSTEM SHALL NOT rebuild, rewrite, truncate or rename any session, index, head or block file. It SHALL NOT write any file in the home outside `.git`, except an empty lock file for a session that had none; that means no .gitignore and no settings file. It SHALL NOT stage a file outside the tracked set. It SHALL NOT force a push or push more than one ref. It SHALL NOT redact or omit any byte of a tracked file, and SHALL NOT report a hit's value.

**Acceptance:**
- The fixture home (one session s1 with a header and 3 entries, and 2 blocks) is shipped under the name `fixture` to a bare remote created by `git init --bare` in a temporary directory. The report shows 1 session and 5 files. `git ls-tree -r --name-only refs/lys/home/fixture` in the remote lists exactly sessions/s1.jsonl, sessions/s1.index.jsonl, sessions/s1.head and the 2 block paths.
- The ship report's commit equals `git rev-parse refs/lys/home/fixture` in the remote, and `git for-each-ref` in the remote lists exactly one ref.
- The SHA-256 of every file in the source home outside `.git`, and the set of those file names, are equal before and after a successful ship.
- A fixture home whose index is truncated to 2 of its 3 rows is refused, naming `s1` and `lys-home index`. Afterwards the home holds no `.git`, the SHA-256 of every file in it is unchanged, and the remote holds no ref.
- A fixture home whose head file is deleted ships. The remote's tree lists 4 files with no sessions/s1.head, and the source home still holds no head file.
- A fixture home holding a file sessions/launch.env beside the session files, and a tool result in s1 containing the value listed in the values file, is refused in one report naming `sessions/launch.env` and `sessions/s1.jsonl` with that hit's byte offset. Afterwards the home holds no `.git` and the remote holds no ref.
- A fixture home whose s1 tool result holds `ghp_` followed by 20 `a` characters, shipped with an empty values file, is refused naming `sessions/s1.jsonl` and that hit's byte offset. Afterwards the home holds no `.git` and the remote holds no ref.
- A ship while a Session for s1 is held open in the same test is refused, naming `s1`.
- After one more entry is appended to the fixture home, a second ship pushes a commit whose parent is the first ship's commit, and the remote's ref names the second commit.

**Files:**
- create: crates/lys-home/src/ship/push.rs
- modify: crates/lys-home/src/ship/mod.rs

**Checklist:**
- C14 — Ship commits exactly a home's session files, their index and head files and its blocks store in the home's own git repository, pushes that commit as one ref to the remote named on the command line, and reports the commit, the ref and every lock or temporary file it left behind.
- C15 — Ship refuses by name a home in which any session's index is missing or stale, names the index step that fixes it and writes nothing; a session without a head file ships as it is, and the source home's files are byte-identical before and after every ship.
- C17 — Ship refuses by name every file in the home that is not a home file, and every place a tracked file holds a value from the credential values file named on the command line or matches one of five standard credential patterns, giving the file and the byte offset and never the value; nothing is redacted.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home shipped as a ref and fetched into a second home where my session is rendered and resumed, so that I move by the same route as a build tree and never by a hand copy.
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a home with a stale index refused by name and pointed at the step that fixes it rather than repaired and shipped, every file that is not a home file refused, and every named credential value and standard credential pattern found in the home refused by file and offset, so that what leaves a home is exactly its record and nothing secret.

### R5: Fetch: bring one ref into a new home and check it before touching it

WHEN asked to fetch a name from a remote into a directory, THE SYSTEM SHALL first check whether the directory already holds a home, meaning it contains a `sessions` entry, a `blocks` entry or a `.git` entry. IF it does, THEN THE SYSTEM SHALL refuse, naming the directory. Otherwise THE SYSTEM SHALL: create the directory when it is absent; initialise a git repository whose directory is `.git` inside the new home; fetch exactly refs/lys/home/<name> from the remote; and check the fetched commit out into the home with HEAD at it. It SHALL then verify three things: that `git status --porcelain --untracked-files=all` prints nothing; that every session's index passes R1's index check; and that every head passes R1's head check. IF any of them fails, THEN THE SYSTEM SHALL refuse, naming the session id or the path, append nothing, and leave the directory as fetched. Fetching and these checks SHALL be one library call that returns before R6 appends anything. THE SYSTEM SHALL report the fetched commit id and its tree id. THE SYSTEM SHALL NOT write to the source home or to the remote. It SHALL NOT fetch any ref other than the one named. It SHALL NOT merge into or overwrite an existing home. It SHALL NOT rebuild an index.

**Acceptance:**
- After R4 ships the fixture home under `fixture`, fetching `fixture` from the bare remote into a new temporary directory reports a commit equal to the ship report's commit and a tree equal to the ship report's tree.
- In the fetched home, `git rev-parse HEAD^{tree}` equals the ship report's tree id.
- Through the library call that stops after verification, the SHA-256 of sessions/s1.jsonl, sessions/s1.index.jsonl, sessions/s1.head and each of the 2 blocks in the fetched home equals the source file's.
- Fetching into a directory that holds a `sessions` directory is refused, naming that directory, and the directory's file names and their SHA-256 are unchanged.
- Fetching into a directory that holds only a `.git` directory is refused, naming that directory.
- A remote whose shipped commit holds a head file naming `not-an-entry` is fetched, then refused naming `s1`. The fetched session file's SHA-256 equals the shipped blob's.
- The SHA-256 of every file in the source home, and the remote's `git for-each-ref` output, are equal before and after a fetch.

**Files:**
- create: crates/lys-home/src/ship/fetch.rs
- modify: crates/lys-home/src/ship/mod.rs

**Checklist:**
- C18 — Fetch brings the shipped ref into a new directory and refuses by name one that already holds a home; the fetched tree equals the shipped tree, so the session file, index, head and blocks hash-match the source, and every index is verified against its file and every head against its index.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home shipped as a ref and fetched into a second home where my session is rendered and resumed, so that I move by the same route as a build tree and never by a hand copy.
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fetched tree checked equal to the shipped one before anything is appended, and each fetched session then to hold the source's bytes followed by one arrival event naming its source commit and a fresh execution id, so that a moved home's ancestry is on the record and nothing was rewritten.

### R6: Append one arrival event to every fetched session

WHEN R5's checks pass, THE SYSTEM SHALL mint one execution id for the fetch with record::fresh_id (32 lowercase hex characters). For each session, in id order, it SHALL append through Session::append exactly one custom entry of customType lys.harness_event whose data is {kind: "arrival", harness: "lys", source_uuid: null, record: null, detail: {source_commit, remote, ref, execution_id}}. Here source_commit is the fetched commit id; ref is refs/lys/home/<name>; and remote is the remote as given, with any userinfo removed (the part of a scheme URL's authority before an `@`). The entry is a child of the session's head and becomes its head. IF any session's arrival data would serialise to more than 512 bytes (MAX_DATA_BYTES), THEN THE SYSTEM SHALL refuse, naming the cap, before appending to any session. THE SYSTEM SHALL report the execution id and each session's arrival entry id. THE SYSTEM SHALL NOT change any block. It SHALL NOT change a session file or an index file except by appending that one line and that one row, and SHALL NOT change a head file except to name the arrival entry. It SHALL NOT append more than one entry to a session. It SHALL NOT commit the appended entries in the target repository. It SHALL NOT write the execution id into any other file. It SHALL NOT record userinfo, a token or a credential in the event.

**Acceptance:**
- After a fetch of the fixture home, the source sessions/s1.jsonl is a byte prefix of the fetched sessions/s1.jsonl, which is exactly one line longer. The fetched index file is the source index's bytes followed by one row, whose offset is the source session file's length, whose id is the arrival entry id and whose parent is the source head. The fetched head file holds the fetch report's arrival entry id.
- The SHA-256 of each of the 2 blocks in the fetched home equals the source block's after the arrival append.
- The fetched session's last line parses as a Pi custom entry. Its customType is `lys.harness_event` and its parentId is the source head. Its data.kind is `arrival` and data.harness is `lys`. Its data.detail.source_commit is the ship report's commit, data.detail.ref is `refs/lys/home/fixture`, and data.detail.execution_id is the fetch report's execution id, 32 lowercase hex characters.
- Two fetches of the same ref into two new directories report two different execution ids.
- A fetch of a home holding 2 sessions reports 2 arrival entries, both with the same execution id, and each session's last line is its arrival.
- The remote `https://user:pw@example.invalid/home.git` is recorded as `https://example.invalid/home.git`, and a local path remote is recorded exactly as given.
- A remote path long enough to push the arrival data past 512 bytes is refused naming 512, and the fetched session file's SHA-256 equals the source's.
- In the fetched home, `git rev-parse HEAD` still equals the fetched commit after the arrival is appended, and `git status --porcelain` lists sessions/s1.jsonl, sessions/s1.index.jsonl and sessions/s1.head as modified.

**Files:**
- create: crates/lys-home/src/ship/arrival.rs
- create: crates/lys-home/src/ship/arrival_tests.rs
- modify: crates/lys-home/src/ship/mod.rs

**Checklist:**
- C19 — After the tree check, each fetched session gains exactly one appended lys.harness_event of kind arrival naming the source commit, the remote, the ref and a fresh execution id; the source session file is a byte prefix of the target's, the blocks still hash-match, and the index and head are the ones Session::append wrote for that entry.

**Stories:**
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fetched tree checked equal to the shipped one before anything is appended, and each fetched session then to hold the source's bytes followed by one arrival event naming its source commit and a fresh execution id, so that a moved home's ancestry is on the record and nothing was rewritten.

### R7: The ship, fetch and index subcommands

Add three subcommands to the lys-home binary: `ship --home <dir> --remote <url-or-path> --ref <name> --secret-values <file>`, `fetch --home <dir> --remote <url-or-path> --ref <name>`, and `index --home <dir> --session <id>`. Their arguments and reports go in their own file, joined to the command enum in cli.rs by one variant each. `index` opens the session through Home::open_session, so Index::load rebuilds a missing or stale index, and reports the session id, the row count and whether a rebuild happened. Each subcommand SHALL print one JSON report of ids, hashes, counts, offsets and paths. WHEN ship or fetch refuses, THE SYSTEM SHALL print a JSON report and exit 1. The report carries `refused` (one of `stale index`, `head mismatch`, `session held`, `unshippable files`, `home occupied`, `arrival too large` or `git`) and whatever the refusal names: session ids, the `lys-home index` command line, paths, and hits as {file, offset}. IF a required argument is missing, THEN THE SYSTEM SHALL exit 2 naming it. THE SYSTEM SHALL NOT print any transcript, block, body or credential value. It SHALL NOT change the arguments or reports of the six existing subcommands.

**Acceptance:**
- `lys-home ship --home h --remote r --secret-values v` exits 2, and its stderr names `--ref`.
- `lys-home ship` on the fixture home with its index truncated exits 1. Its stdout parses as JSON, with `refused` equal to `stale index`, `sessions` equal to [`s1`], and a `fix` value containing `lys-home index`.
- `lys-home index --home <that home> --session s1` then exits 0 and reports rows 3 and rebuilt true. The same `lys-home ship` command run again exits 0 and reports a commit.
- `lys-home ship` on a home holding sessions/launch.env exits 1, and its stdout parses as JSON with `refused` equal to `unshippable files` and `foreign` equal to [`sessions/launch.env`].
- `lys-home fetch` into an occupied directory exits 1. Its stdout parses as JSON with `refused` equal to `home occupied` and `path` naming the directory.
- The stdout of a successful ship and of a successful fetch each parses as JSON and contains none of the three keys `text`, `content` and `body`.
- crates/lys-home/src/cli.rs holds fewer than 500 lines of code.

**Files:**
- create: crates/lys-home/src/ship/cli.rs
- modify: crates/lys-home/src/ship/mod.rs
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/src/main.rs

**Checklist:**
- C14 — Ship commits exactly a home's session files, their index and head files and its blocks store in the home's own git repository, pushes that commit as one ref to the remote named on the command line, and reports the commit, the ref and every lock or temporary file it left behind.
- C15 — Ship refuses by name a home in which any session's index is missing or stale, names the index step that fixes it and writes nothing; a session without a head file ships as it is, and the source home's files are byte-identical before and after every ship.
- C16 — A separate index subcommand rebuilds a session's missing or stale index, after which the same ship command runs unchanged and succeeds.
- C17 — Ship refuses by name every file in the home that is not a home file, and every place a tracked file holds a value from the credential values file named on the command line or matches one of five standard credential patterns, giving the file and the byte offset and never the value; nothing is redacted.
- C18 — Fetch brings the shipped ref into a new directory and refuses by name one that already holds a home; the fetched tree equals the shipped tree, so the session file, index, head and blocks hash-match the source, and every index is verified against its file and every head against its index.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home shipped as a ref and fetched into a second home where my session is rendered and resumed, so that I move by the same route as a build tree and never by a hand copy.
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a home with a stale index refused by name and pointed at the step that fixes it rather than repaired and shipped, every file that is not a home file refused, and every named credential value and standard credential pattern found in the home refused by file and offset, so that what leaves a home is exactly its record and nothing secret.

### R8: Tests against a bare remote in a temporary directory

Structural: one integration test file drives the lys-home binary end to end against a bare remote made by `git init --bare` in a temporary directory. The fixture home holds one synthetic session (a header and 3 entries written through the library, one of them a tool result) and 2 blocks. A credential values file outside the home holds the line `LYS_FIXTURE_SECRET=lys-fixture-secret-5b1c07`. The secret fixture is a copy of the fixture home with that value inside the tool result and a `launch.env` file holding the same line beside the session files in sessions/. The tests SHALL assert refusals as well as successes. They SHALL assert the number of sessions, files and blocks each check covered, so a loop over zero sessions fails. The file holds four tests named `ship_fetch_round_trip`, `ship_fetch_leaves_source_unchanged`, `ship_refuses_credentials_then_ships_clean` and `ship_refuses_stale_index_and_occupied_target`, one for each of the first four acceptance lines in that order. No test name carries fixture content.

**Acceptance:**
- `ship_fetch_round_trip` ships and fetches the fixture home and asserts in order: the fetched tree equals the shipped tree before the arrival; the source session file is a byte prefix of the fetched one after the arrival, which is exactly one line longer; the 2 blocks hash-match; that last line is an arrival naming the ship report's commit and the fetch report's execution id; and the count of sessions checked is 1 and of blocks compared is 2.
- `ship_fetch_leaves_source_unchanged` asserts that the SHA-256 of every source home file outside `.git` is equal before and after the ship and the fetch.
- `ship_refuses_credentials_then_ships_clean` ships the secret fixture and asserts exit 1 with `refused` equal to `unshippable files`, naming both `sessions/launch.env` and `sessions/s1.jsonl` with its offset. It then ships the clean fixture and asserts that `git grep -F lys-fixture-secret-5b1c07 $(git rev-list --all)` in the remote finds nothing and exits 1.
- `ship_refuses_stale_index_and_occupied_target` asserts the stale-index refusal by name, the `lys-home index` step and the ship that follows, and the occupied-target refusal by name, each through the binary's exit status and its JSON report.
- `cargo test -p lys-home --all-features --test ship_fetch` exits 0, and its output shows each of `ship_fetch_round_trip`, `ship_fetch_leaves_source_unchanged`, `ship_refuses_credentials_then_ships_clean` and `ship_refuses_stale_index_and_occupied_target` followed by `... ok`.

**Files:**
- create: crates/lys-home/tests/ship_fetch.rs

**Checklist:**
- C14 — Ship commits exactly a home's session files, their index and head files and its blocks store in the home's own git repository, pushes that commit as one ref to the remote named on the command line, and reports the commit, the ref and every lock or temporary file it left behind.
- C15 — Ship refuses by name a home in which any session's index is missing or stale, names the index step that fixes it and writes nothing; a session without a head file ships as it is, and the source home's files are byte-identical before and after every ship.
- C16 — A separate index subcommand rebuilds a session's missing or stale index, after which the same ship command runs unchanged and succeeds.
- C17 — Ship refuses by name every file in the home that is not a home file, and every place a tracked file holds a value from the credential values file named on the command line or matches one of five standard credential patterns, giving the file and the byte offset and never the value; nothing is redacted.
- C18 — Fetch brings the shipped ref into a new directory and refuses by name one that already holds a home; the fetched tree equals the shipped tree, so the session file, index, head and blocks hash-match the source, and every index is verified against its file and every head against its index.
- C19 — After the tree check, each fetched session gains exactly one appended lys.harness_event of kind arrival naming the source commit, the remote, the ref and a fresh execution id; the source session file is a byte prefix of the target's, the blocks still hash-match, and the index and head are the ones Session::append wrote for that entry.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home shipped as a ref and fetched into a second home where my session is rendered and resumed, so that I move by the same route as a build tree and never by a hand copy.
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fetched tree checked equal to the shipped one before anything is appended, and each fetched session then to hold the source's bytes followed by one arrival event naming its source commit and a fresh execution id, so that a moved home's ancestry is on the record and nothing was rewritten.
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a home with a stale index refused by name and pointed at the step that fixes it rather than repaired and shipped, every file that is not a home file refused, and every named credential value and standard credential pattern found in the home refused by file and offset, so that what leaves a home is exactly its record and nothing secret.

### R9: Write the tracked set, ship, fetch, index and the arrival kind down

Structural: RECORD.md SHALL state that a home is a git repository with `.git` inside it. It SHALL name the tracked set (sessions/<id>.jsonl, <id>.index.jsonl, <id>.head and blocks/<xx>/<hash>). It SHALL state that lock files and temporary files are left behind and named, and that every other file is refused. It SHALL name the ref refs/lys/home/<name>, the credential values scan and the five standard credential patterns, and the `index` step that a stale-index refusal names. It SHALL add `arrival` to the closed list of lys.harness_event kinds, with harness `lys`, its four detail members and the 512-byte cap. The crate README SHALL say what ship, fetch and index do, and that the crate does not move a home to another machine, encrypt it, use any transport other than a git ref, or redact a transcript. The cluster markdown SHALL be re-rendered with render-cluster.py so that DESIGN.md, CHECKLIST.md, USER-STORIES.md and briefs/HOME-004.md match their JSON. THE SYSTEM SHALL NOT change any existing kind's meaning or any published wire format.

**Acceptance:**
- RECORD.md lists 6 lys.harness_event kinds. The sixth is `arrival`, with harness `lys` and the detail members source_commit, remote, ref and execution_id.
- RECORD.md names refs/lys/home/<name>, the four members of the tracked set, `lys-home index`, and the 5 standard credential patterns.
- `sh scripts/design/gate.sh` exits 0.
- `git diff --stat main` for this brief shows no change under crates/lys-core and no change under crates/lys-log-store.

**Files:**
- modify: docs/design/home/RECORD.md
- modify: crates/lys-home/README.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md
- modify: docs/design/home/briefs/HOME-004.md

**Checklist:**
- C17 — Ship refuses by name every file in the home that is not a home file, and every place a tracked file holds a value from the credential values file named on the command line or matches one of five standard credential patterns, giving the file and the byte offset and never the value; nothing is redacted.
- C19 — After the tree check, each fetched session gains exactly one appended lys.harness_event of kind arrival naming the source commit, the remote, the ref and a fresh execution id; the source session file is a byte prefix of the target's, the blocks still hash-match, and the index and head are the ones Session::append wrote for that entry.

**Stories:**
- S10 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want the fetched tree checked equal to the shipped one before anything is appended, and each fetched session then to hold the source's bytes followed by one arrival event naming its source commit and a fresh execution id, so that a moved home's ancestry is on the record and nothing was rewritten.
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a home with a stale index refused by name and pointed at the step that fixes it rather than repaired and shipped, every file that is not a home file refused, and every named credential value and standard credential pattern found in the home refused by file and offset, so that what leaves a home is exactly its record and nothing secret.

### R10: Prove the move: ship, fetch, render through the launch template and resume on the installed Claude Code

Structural: PROOF-SHIP.md records one run in the pattern of PROOF-RESUME.md. The ship, the fetch, the render and the resume SHALL all run on one Mac, the Mac the card's words name, whichever machine built the card. A fixture home with a synthetic session is shipped to a bare remote in a temporary directory and fetched into a second directory. The fetched session is rendered through the launch template's subcommand, as landed with card ct98Wv-2, into a scratch directory outside the fetched home, and that subcommand prints the launch line. The launch line is run from a scratch working directory made for the run. It is run with the Claude Code installed on that Mac when the proof runs, the `claude` the launch line names, with PROOF-RESUME.md's prompt-mode arguments added. The proof SHALL record the version string that binary prints, taken immediately before the resume, and the path it resolved to. It SHALL record, for each of the ship, the fetch, the render and the resume, the output of `uname -s` and `uname -m` and the SHA-256 of the output of `uname -n`, each taken as that step runs. It SHALL record the resume's outcome: the launch line's exit status, the continuation file's path and line count, and the rendered file's SHA-256 before and after the resume. It SHALL record the two hash checks: the fetched files' SHA-256 before the arrival against the source's (equal); and, after the arrival, the source and target session file lengths, the target's first bytes equal to the source file (the prefix check) and the blocks' SHA-256 (equal). THE SYSTEM SHALL NOT install, upgrade or downgrade Claude Code for the proof. The proof SHALL NOT hold any transcript content, answer text, credential or secret value; it holds hashes, lengths, counts, paths, exit statuses, the Claude Code version string and the `uname -s` and `uname -m` values only. It SHALL NOT hold the output of `uname -n`; that output appears in it only as its SHA-256.

**Acceptance:**
- PROOF-SHIP.md records the ship report's commit and tree, the fetch report's commit, tree, execution id and arrival entry id, and the remote's ref name.
- PROOF-SHIP.md records the SHA-256 of each source home file before and after the ship and fetch (equal), and the SHA-256 of the fetched session file, index, head and blocks before the arrival (equal to the source's).
- PROOF-SHIP.md records the source and target session file lengths after the arrival, the prefix check's result, and the blocks' SHA-256 after the arrival (equal to the source's).
- PROOF-SHIP.md records the version string printed by `claude --version` immediately before the resume and the path that `claude` resolved to, the template subcommand exactly as run, the printed launch line and the command as run.
- PROOF-SHIP.md records the launch line's exit status as 0; a continuation file at the recorded path, whose line count is greater than the rendered file's line count; and the rendered file's SHA-256 before the resume equal to its SHA-256 after the resume.
- PROOF-SHIP.md records the `uname -s` and `uname -m` output and the SHA-256 of the `uname -n` output for each of the ship, fetch, render and resume steps. The four `uname -n` SHA-256 values are equal, and each `uname -s` value is `Darwin`.
- PROOF-SHIP.md contains no string of the fixture session's entry text, no occurrence of `lys-fixture-secret-5b1c07` and no occurrence of the `uname -n` output of the machine the proof ran on.

**Files:**
- create: docs/design/home/PROOF-SHIP.md

**Checklist:**
- C20 — The fetched home is rendered through the Claude Code launch template and resumed by its printed launch line on the Claude Code installed when the proof runs, its version recorded, written up in PROOF-SHIP.md in hashes, lengths, counts and paths only.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my home shipped as a ref and fetched into a second home where my session is rendered and resumed, so that I move by the same route as a build tree and never by a hand copy.

## Boundaries

- SHALL NOT rewrite, truncate, rename or rebuild any session, index, head or block file in the source home, and SHALL NOT call Index::load or Session::open on the ship path.
- SHALL NOT move a home to another machine, encrypt a home, or claim read authority over a home.
- SHALL NOT use any transport other than a git ref pushed to and fetched from a named remote, and SHALL NOT force-push.
- SHALL NOT support a harness other than Claude Code, and SHALL NOT build a launch template or a profile; the template is card ct98Wv-2's.
- SHALL NOT track, record or print a credential, token, key, credential values file or remote userinfo, and SHALL NOT redact or omit any byte of a tracked file.
- SHALL NOT scan for any credential pattern other than the five listed in R3; a bare `sk-`, `eyJ` and runs of hex characters are not scanned.
- SHALL NOT print transcript, block or body content in any report, error, log, test name or proof.
- SHALL NOT add a crate dependency to lys-home, and SHALL NOT touch lys-core, lys-log-store or any published wire format.
- SHALL NOT add a field to Pi's grammar; the arrival rides only inside a custom entry's data.
- SHALL NOT install, upgrade or downgrade Claude Code, and SHALL NOT have lys-home run Claude Code.

## Verification

- cargo fmt --all -- --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo clippy --all-targets -- -D warnings
- cargo test --workspace --all-features
- cargo doc --no-deps --all-features
- cargo doc --no-deps
- sh scripts/design/gate.sh
- grep -n 'Index::load\|Session::open\|open_session\|Home::open' crates/lys-home/src/ship/push.rs crates/lys-home/src/ship/scan.rs crates/lys-home/src/record/verify.rs finds nothing.
- grep -rn 'unwrap()\|expect(\|panic!' crates/lys-home/src/ship crates/lys-home/src/record/verify.rs, excluding the *_tests.rs files, finds nothing.
- git diff --stat main -- crates/lys-home/Cargo.toml crates/lys-core crates/lys-log-store shows no change.
