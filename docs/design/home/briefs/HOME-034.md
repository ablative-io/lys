---
type: brief
id: HOME-034
cluster: home
title: Write the loss account of the Claude Code render down as the code writes it
---

# HOME-034: Write the loss account of the Claude Code render down as the code writes it

> **Cluster:** home
> **Design anchor:**
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-018 — A user-message point is carried as a seed prompt beside the rendered file, never copied into the child — When the point is a user message the cut stops at the assistant message before it and the message is carried, not copied: lys.forked_from records its id with coordinate_carried true and counts, by kind, the parts of it that are not text. The Claude Code render of such a child writes the message's text parts, in order, as a seed prompt beside the rendered file under an in-band marker line naming the parent session, the point and the lantern, and names it in the render report; the template's launch line, printed by render-launch only, passes that file as the resumed session's first prompt. A part that is not text never refuses a fork or a render and never enters the seed. Rejected: copying the user message into the child's chain, refusing a fork for a non-text part, and putting the seed's text in the report or the loss account.
> **Checklist:**
> - C95 — The block store's gate proves a second put of the same bytes writes nothing without elapsed time: before the second put it pins the shard directory's and the block file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock.
> - C96 — The template store's gate proves a second put of the same template writes nothing without elapsed time: before the second put it pins the template shard directory's and the template file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock.
> - C97 — LOSS-ACCOUNT.md quotes every reason string the render passes to the loss constructor, with the line that writes it, and the reason check prints 1 3 3 [] at the landing commit.
> - C98 — LOSS-ACCOUNT.md states that an entry's hash is the SHA-256 of the part as serde_json serialises it and that an entry never carries the part's text, signature or redacted data.
> - C99 — LOSS-ACCOUNT.md lists the four things the render changes without a loss entry, custom entries and labels, compaction, gitBranch and usage, each with the line that does it.
> **Stories:**
> - S40 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write.

## Purpose

HOME-001 R4 names docs/design/home/LOSS-ACCOUNT.md among the files it creates, and the cluster design's structure names the same path, but the document was never written. This brief writes it from the Claude Code render as it stands on main: where the account is written, the shape of the account and of one entry, every reason the render writes quoted with its source line, when an entry is written, that an entry names a part by hash and never carries its content, that every render writes the account, and what the render changes without writing an entry. A reader can then check a render's account against the source without reading the render, and a reason check keyed on the source proves the document and the source agree in both directions.

## Task

Write docs/design/home/LOSS-ACCOUNT.md by hand from render.rs, launch.rs, import.rs, cli.rs, mod.rs and blocks.rs under crates/lys-home/src as they stand on main when the document is written, and name that commit on the document's first line. At the time of this brief main is 7b53625 and every line number below is read there; if the render card that moves the render's refusals and its assistant shaping has landed first, the document is written from main as it then stands, names that commit, and cites the lines that hold the same code there; the reason check reads whichever files call the loss constructor, so it finds the reasons wherever they are written. The document is the one file this brief creates, and all five requirements are sections of it, so R1 to R5 share it as their primary file. The code decides what the document says: the account is named by the rendered file's stem, which is the session uuid only for the default output path and for render-launch; the array named `dropped` also holds signed thinking rendered as text; and the account is written on every render. In scope: the document only. Out of scope: any change to what the render drops or why, any code, the loss account of any harness other than Claude Code, the import-time `lys.loss` entry of the compaction card, pinning the reasons in a test, and changing the silent empty write or plain render's overwrite of an existing account. Transcript content never appears in the document (CN3): it names keys, reasons, line citations and counts.

## Requirements

### R1: Name the commit the document is read from and where the loss account is written

Structural: `docs/design/home/LOSS-ACCOUNT.md` is created, a hand-written markdown document titled `# The loss account of the Claude Code render`. Its first line after the title is `Written from commit <id>.`, where <id> is the full 40-character id of main's head at the time the document is written: `7b536253165f920cd8bc5f1d1dfdd987339e3e12` while main stands there, and main's head as it then stands if the render card that moves the render's refusals has landed first. Every source line the document cites is written as a repository-relative path, a colon and a line number or a first-last range (`crates/lys-home/src/harness/claude_code/render.rs:263`, `crates/lys-home/src/harness/claude_code/render.rs:85-93`), read at that commit. The document has the section `## The loss account` first and the section `## What the render changes without a loss entry` second. The loss account section states that the account is written beside the rendered file and named by the rendered file's stem with `.loss.json`, because the render computes its path with `path.with_extension("loss.json")`, citing that line of render.rs; that the stem is the session uuid in two cases, the default output path of `lys-home render` (citing render.rs's `default_path`, lines 85-93 at 7b53625) and `render-launch` (citing launch.rs's `{uuid}.jsonl` and `{uuid}.loss.json` lines, 80-81 at 7b53625); and that `lys-home render --out other.jsonl` writes the account as `other.loss.json` (citing the `out` argument in cli.rs, lines 105-107 at 7b53625). It states that plain render writes the account with `std::fs::write`, which replaces a file already at that path (citing render.rs's lines 265-269 at 7b53625), that `render-launch` refuses an existing loss path by name before it writes any file (citing launch.rs's lines 96-101 at 7b53625), and that `render-launch` hashes the account as the second file of its render manifest (citing launch.rs's lines 123-133 at 7b53625). The document SHALL NOT state that the account is named by the session uuid for every render, SHALL NOT propose a change to the account's path or name, and SHALL NOT cite a line at any commit other than the one it names.

**Acceptance:**
- `git cat-file -e HEAD:docs/design/home/LOSS-ACCOUNT.md` exits 0 on the landing commit.
- The document holds exactly one line matching `^Written from commit [0-9a-f]{40}\.$`, and the id on it equals the output of `git merge-base origin/main HEAD` on the card's branch (`7b536253165f920cd8bc5f1d1dfdd987339e3e12` while main stands at 7b53625).
- `python3 -c "import re;ls=[l for l in open('docs/design/home/LOSS-ACCOUNT.md').read().splitlines() if l.strip()];i=ls.index('# The loss account of the Claude Code render');print(bool(re.fullmatch(r'Written from commit [0-9a-f]{40}\.',ls[i+1])))"` run from the repository root prints `True`: the first non-blank line after `# The loss account of the Claude Code render` is the `Written from commit <id>.` line.
- `python3 -c "import re,subprocess;d=open('docs/design/home/LOSS-ACCOUNT.md').read();c=re.search(r'(?m)^Written from commit ([0-9a-f]{40})\.$',d).group(1);cs=re.findall(r'((?:crates|docs)/[A-Za-z0-9_./-]+\.[a-z]+):([0-9]+)(?:-([0-9]+))?',d);n=lambda p:len(subprocess.run(['git','show',c+':'+p],capture_output=True,text=True,check=True).stdout.splitlines());print(c,sorted({x[0] for x in cs}),[x for x in cs if not 1<=int(x[1])<=int(x[2] or x[1])<=n(x[0])])"` run from the repository root prints the named commit, a sorted list of cited paths that contains each of the three files this requirement cites, `crates/lys-home/src/cli.rs`, `crates/lys-home/src/harness/claude_code/launch.rs` and `crates/lys-home/src/harness/claude_code/render.rs`, and `[]`.
- At the named commit, the render.rs line the location paragraph cites holds `with_extension("loss.json")` (line 263 at 7b53625), and the launch.rs range it cites holds `{uuid}.loss.json` (range 80-81 at 7b53625).
- The document contains the string `--out other.jsonl` and the string `other.loss.json`.
- At the named commit, the launch.rs range the document cites for the refusal holds `LaunchTargetExists` (range 96-101 at 7b53625), and the render.rs range it cites for the plain write holds `std::fs::write(` (range 265-269 at 7b53625).
- `grep -c '^## ' docs/design/home/LOSS-ACCOUNT.md` prints 2, and the two headings are `## The loss account` then `## What the render changes without a loss entry`.

**Files:**
- create: docs/design/home/LOSS-ACCOUNT.md

**Checklist:**
- C95 — The block store's gate proves a second put of the same bytes writes nothing without elapsed time: before the second put it pins the shard directory's and the block file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock.

**Stories:**
- S40 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write.

### R2: State the account's shape and that every render writes it

Structural: the loss account section states that the account is one JSON object with exactly four keys, `session_id` (the session id the rendered file carries), `model` (the model the file is rendered for), `authored` (true when an entry on the rendered path is the `lys.authored` custom entry) and `dropped` (an array of entries), citing the render.rs line that builds the object (264 at 7b53625) and the line that sets `authored` (128 at 7b53625); that one entry is the `Loss` struct's two string fields, `hash` and `reason`, citing render.rs's lines 48-55 at 7b53625; and that the account is written on every render, including a render that drops nothing, whose account carries `dropped: []`, citing the render.rs lines that write it with no condition (263-269 at 7b53625) and the two measured runs that show it, docs/design/home/PROOF-RESUME.md:38 and docs/design/home/PROOF-LAUNCH.md:28. It states, as found at the named commit and not as intended behaviour, that the account is serialised with `unwrap_or_default()`, so a serialisation failure writes an empty file and returns no error, citing that render.rs line (267 at 7b53625). The document SHALL NOT name a key or field the code does not write, SHALL NOT propose a change to the account's keys, fields or bytes, and SHALL NOT present the empty-file behaviour as a guarantee.

**Acceptance:**
- The loss account section contains each of the strings `session_id`, `model`, `authored`, `dropped`, `hash` and `reason` in backticks.
- At the named commit, the render.rs line the document cites for the object holds `"dropped": losses` (line 264 at 7b53625), and the range it cites for the entry holds `pub struct Loss` (range 48-55 at 7b53625).
- The document contains the string `dropped: []` and cites `docs/design/home/PROOF-RESUME.md:38` and `docs/design/home/PROOF-LAUNCH.md:28`, and at 7b53625 each of those lines contains `dropped: []`.
- At the named commit, the render.rs line the document cites for the empty write holds `to_vec_pretty(&account).unwrap_or_default()` (line 267 at 7b53625).

**Files:**
- create: docs/design/home/LOSS-ACCOUNT.md

**Checklist:**
- C96 — The template store's gate proves a second put of the same template writes nothing without elapsed time: before the second put it pins the template shard directory's and the template file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock.

**Stories:**
- S40 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write.

### R3: Quote every reason the render writes, with its line, and state when each is written

WHEN the document states the reasons, THE DOCUMENT SHALL write each reason string the source passes to the loss constructor on its own line of the form `- reason: "<reason>" (<path>:<line>)`, the reason copied character for character, one line per reason, where the source is every file under `crates/lys-home/src` holding a line that calls `loss(&`; at 7b53625 that is render.rs alone, with one reason on line 198 and two on line 201. It SHALL state when an entry is written as the thinking branch decides it at the named commit, citing render.rs's lines 170-174 (the provider, api and model comparison) and 184-202 (the thinking branch) at 7b53625 and mod.rs's `PROVIDER` and `API` lines (52 and 54 at 7b53625): a thinking part is kept whole with no entry when provider, api and model all equal the target's and the part is redacted or carries a signature; otherwise, when its text is not empty after trimming whitespace and it is not redacted, it is rendered as a text part, and an entry with the signed-as-text reason is written only when it carries a signature, so a thinking part with no signature rendered as text writes no entry, for the same model as for another (citing the `if sig.is_some()` line, 197 at 7b53625); otherwise it is dropped with an entry carrying the redacted reason when it is redacted and the empty reason when it is not, so an empty thinking part with no signature is dropped even for the same model, and a signed empty thinking part for another model carries the empty reason. It SHALL state that the array named `dropped` therefore holds entries for signed thinking rendered as text as well as for dropped parts, that the report's `dropped` count is the number of entries in it (citing render.rs line 279 at 7b53625) while `thinking_as_text` counts every thinking part rendered as text (line 196 at 7b53625), and that no part other than thinking writes an entry (citing lines 183 and 204-210 at 7b53625). The document SHALL NOT write on a `- reason:` line any string the source does not pass to the loss constructor, SHALL NOT paraphrase a reason, and SHALL NOT state that every entry names a dropped part.

**Acceptance:**
- `python3 -c "import re,pathlib;fs=[p for p in sorted(pathlib.Path('crates/lys-home/src').rglob('*.rs')) if re.search(r'\bloss\(&',p.read_text())];src={s for p in fs for l in p.read_text().splitlines() if re.search(r'\bloss\(&',l) for s in re.findall(r'\"([^\"]+)\"',l)};doc=re.findall(r'(?m)^- reason: \"([^\"]+)\"',open('docs/design/home/LOSS-ACCOUNT.md').read());print(len(fs),len(src),len(doc),sorted(src^set(doc)))"` run from the repository root at the landing commit prints `1 3 3 []`: one source file found, three reasons in the source, three reason lines in the document, and no reason on one side only.
- At 7b53625 the document's three reason lines are exactly `- reason: "signed thinking rendered as text: different provider, api or model" (crates/lys-home/src/harness/claude_code/render.rs:198)`, `- reason: "redacted thinking dropped: different provider, api or model" (crates/lys-home/src/harness/claude_code/render.rs:201)` and `- reason: "empty thinking dropped" (crates/lys-home/src/harness/claude_code/render.rs:201)`; at a later named commit, the same three strings with the path and line that write each there.
- With the line carrying `"empty thinking dropped"` removed from a working copy of the document, the reason check prints `1 3 2 ['empty thinking dropped']`; with a line `- reason: "invented" (crates/lys-home/src/harness/claude_code/render.rs:1)` added instead, it prints `1 3 4 ['invented']`; the document is restored after each.
- At the named commit, the render.rs range the document cites for the thinking branch holds `Some("thinking") =>` on its first line (range 184-202 at 7b53625), and the line it cites for the signature condition holds `if sig.is_some()` (line 197 at 7b53625).
- At the named commit, the render.rs line the document cites for the report's count holds `dropped: losses.len()` (line 279 at 7b53625).

**Files:**
- create: docs/design/home/LOSS-ACCOUNT.md

**Checklist:**
- C97 — LOSS-ACCOUNT.md quotes every reason string the render passes to the loss constructor, with the line that writes it, and the reason check prints 1 3 3 [] at the landing commit.

**Stories:**
- S40 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write.

### R4: State that an entry names its part by hash and never carries the part's content

Structural: the loss account section states that an entry's `hash` is the SHA-256 of the part as serialised by `serde_json::to_vec` from the assistant message, written as 64 lowercase hexadecimal characters, with object keys in sorted order because serde_json's `preserve_order` feature is not enabled in the workspace, citing render.rs's `loss` function (lines 286-292 at 7b53625) and blocks.rs's `Hash::of` (lines 19-20 at 7b53625); that for an imported part this equals the hash the importer stored the part's block under, because the importer serialises a part the same way before it stores it, citing import.rs's lines 420-424 at 7b53625; and that a part from a canon or a hand-authored session may name no block in the store. It states that an entry carries only the hash and the reason, never the part's thinking text, signature or redacted data, and, as found at the named commit, that the part's serialisation uses `unwrap_or_default()`, so a serialisation failure hashes zero bytes (citing render.rs line 287 at 7b53625). The document SHALL NOT state that every entry's hash names a stored block, SHALL NOT carry any text, signature or hash taken from a real session, and SHALL NOT propose a change to how the hash is computed.

**Acceptance:**
- At the named commit, the render.rs lines the document cites for the hash hold `serde_json::to_vec(part)` and `Hash::of(&bytes)` (lines 287 and 289 at 7b53625), the import.rs range it cites holds `serde_json::to_vec(part)` (range 420-424 at 7b53625), and the blocks.rs range it cites holds `Sha256::digest` (range 19-20 at 7b53625).
- The document contains the string `preserve_order` and the string `64`.
- `grep -cE '[0-9a-f]{64}' docs/design/home/LOSS-ACCOUNT.md` prints 0, and `grep -c 'thinkingSignature": *"' docs/design/home/LOSS-ACCOUNT.md` prints 0.

**Files:**
- create: docs/design/home/LOSS-ACCOUNT.md

**Checklist:**
- C98 — LOSS-ACCOUNT.md states that an entry's hash is the SHA-256 of the part as serde_json serialises it and that an entry never carries the part's text, signature or redacted data.

**Stories:**
- S40 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write.

### R5: Record what the render changes without writing a loss entry

Structural: the section `## What the render changes without a loss entry` states, each with the render.rs line that does it at the named commit: custom entries, the lys ones included, and labels are not rendered (lines 9-10 and 237 at 7b53625); a compaction becomes a `summary` record carrying the compaction's summary and the previous record's uuid as `leafUuid` (lines 234-236 at 7b53625); every record's `gitBranch` is written as the empty string (line 148 at 7b53625); and every assistant record's `usage` is written as zero input and zero output tokens (line 226 at 7b53625). It states that none of these writes a loss entry at the named commit, records them as found, and names whether any of them should write a loss entry as a question for its own card, which this document does not answer. The section SHALL NOT list any other change, SHALL NOT propose a change to the render, and SHALL NOT state that any of these is carried by the loss account.

**Acceptance:**
- At the named commit, the render.rs lines this section cites hold, in turn, `Custom entries` (range 9-10 at 7b53625), `_ => {}` (line 237 at 7b53625), `"type": "summary"` (range 234-236 at 7b53625), `"gitBranch": ""` (line 148 at 7b53625) and `"usage": {"input_tokens": 0, "output_tokens": 0}` (line 226 at 7b53625).
- The section names exactly four items, as four list items: custom entries and labels, compaction, `gitBranch`, `usage`.
- The section contains the sentence `None of these writes a loss entry.`

**Files:**
- create: docs/design/home/LOSS-ACCOUNT.md

**Checklist:**
- C99 — LOSS-ACCOUNT.md lists the four things the render changes without a loss entry, custom entries and labels, compaction, gitBranch and usage, each with the line that does it.

**Stories:**
- S40 (Reviewer, Checks the proofs before anything relies on them) — As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write.

## Boundaries

- No file under crates/ changes: not render.rs, launch.rs, import.rs, cli.rs, their tests or the crate README.
- No reason string, no field of the Loss struct, no key of the account, no byte of the account and no part of its path is changed or proposed changed.
- No change to what the render drops or why, and no loss account of any harness other than Claude Code, including the import-time lys.loss entry.
- The build writes docs/design/home/LOSS-ACCOUNT.md and no other file: no existing brief, no row of HOME-001, no JSON document of the cluster, no rendered markdown, RECORD.md and none of the scripts under scripts/design is edited.
- No transcript text, signature or hash taken from a real session appears in the document.
- The document cites no line at a commit other than the one it names.

## Verification

- `git cat-file -e HEAD:docs/design/home/LOSS-ACCOUNT.md` exits 0 on the landing commit.
- `git diff --name-only $(git merge-base origin/main HEAD) HEAD -- crates` prints nothing.
- `python3 -c "import re,pathlib;fs=[p for p in sorted(pathlib.Path('crates/lys-home/src').rglob('*.rs')) if re.search(r'\bloss\(&',p.read_text())];src={s for p in fs for l in p.read_text().splitlines() if re.search(r'\bloss\(&',l) for s in re.findall(r'\"([^\"]+)\"',l)};doc=re.findall(r'(?m)^- reason: \"([^\"]+)\"',open('docs/design/home/LOSS-ACCOUNT.md').read());print(len(fs),len(src),len(doc),sorted(src^set(doc)))"` prints `1 3 3 []`.
- `python3 -c "import re,subprocess;d=open('docs/design/home/LOSS-ACCOUNT.md').read();c=re.search(r'(?m)^Written from commit ([0-9a-f]{40})\.$',d).group(1);cs=re.findall(r'((?:crates|docs)/[A-Za-z0-9_./-]+\.[a-z]+):([0-9]+)(?:-([0-9]+))?',d);n=lambda p:len(subprocess.run(['git','show',c+':'+p],capture_output=True,text=True,check=True).stdout.splitlines());print(c,sorted({x[0] for x in cs}),[x for x in cs if not 1<=int(x[1])<=int(x[2] or x[1])<=n(x[0])])"` prints the commit on the document's `Written from commit` line, a list of cited paths containing `crates/lys-home/src/cli.rs`, `crates/lys-home/src/harness/claude_code/launch.rs` and `crates/lys-home/src/harness/claude_code/render.rs`, and `[]`.
- `sh scripts/design/gate.sh` exits 0.
- cargo fmt --all leaves the tree unchanged; cargo clippy --all-targets --all-features -- -D warnings, cargo clippy --all-targets -- -D warnings, cargo test --workspace --all-features, cargo doc --no-deps --all-features and cargo doc --no-deps each exit 0.
