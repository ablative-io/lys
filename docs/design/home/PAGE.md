# home — what was asked, what it means, and what was written

## The words, as they were typed

A fork is a launch from a lantern's point with its own session identity and its ancestry recorded, so a later session can hold a conversation with the self that lit the lantern. Claude Code resumes a session from its end, never from a point inside it, so a fork at a point is the lantern design's cut and seed: a new session in the home whose entries are the parent's context path from its root to the lantern's point, and nothing after. A lys-home subcommand forks: it takes the home and a lantern id, refuses by name a lantern it does not hold, creates a new session with its own id whose header names the parent session as Pi's parentSession, copies the context path up to the point as entries that reference the same blocks by hash, with no block bytes duplicated, and appends first to the child a lys.forked_from custom entry naming the parent session, the lantern and the point, and then to the parent, at its head, a lys.fork custom entry naming the child, so the ancestry is written on both sides. Claude Code's resume walks the chain from an assistant-anchored leaf, so when the lantern's point is not an assistant message the cut ends at the last assistant message at or before the point and the report names the entry it stopped at. The child is then rendered and launched through the Claude Code launch template like any session, with its own handles supplied at launch by the template and never copied from the parent; its lease is the seat's lifecycle and is not this card's. The report is JSON: the child session id, the parent, the lantern, the point, the entry the cut ended at, entry and block counts, never content. Acceptance is that on a fixture session with a lit lantern at a past assistant entry, the fork's entries hash-match the parent's context path up to that entry and stop there, the child header's parentSession is the parent's id, the child's first lys entry is forked_from and the parent's head is a fork entry naming the child, the parent's earlier bytes are unchanged; that a lantern at a user entry forks to the preceding assistant entry and the report says so; that an unknown lantern id is refused by name; that the launch template renders the child and its launch line resumes it, and a proof document records that a conversation was held with the lantern's self through the fork, as hashes, counts and paths only; and that two forks from one lantern are two sessions with distinct ids that share every block. Not in scope: cutting at a point that has no lantern; forks with tools or a lens; leases, budgets and kill rules; harnesses other than Claude Code. Filed by Archie on Tom's definition of 8 August 2026 that a lantern is a pathway back to a conversation with the previous self through a fork, his roadmap stage 5b of 22 September 2026, the lantern design's cut-and-seed measurement, and home DESIGN P1, P2, P4 and P8 at lys main 0073b966, on 26 September 2026.

## What the survey found, and its angles

The words ask for stage 5b of the context roadmap: a `lys-home fork` subcommand. It takes a home and a lantern id and cuts a new session from the parent's context path at the lantern's point. If the point is not an assistant message, the cut ends at the last assistant entry before it. The copied entries reference the same blocks, the ancestry is written on both sides (lys.forked_from in the child, lys.fork at the parent's head), and the command prints a JSON report of counts only. The child is then rendered and launched through a Claude Code launch template, and a proof document records a conversation held with the lantern's self through the fork. The tree has neither lanterns nor a launch template yet, and three sentences of the words do not match how the home record is actually built today.

### What the tree holds

- `crates/lys-home/src/cli.rs` — Where the new `fork` subcommand lands next to import, render, canon, fewshot, ingest-call and resume-check. Every command prints one JSON report of paths, hashes and counts. It already has 398 non-comment code lines against the 500-line rule, so the fork logic must live in its own file.
- `crates/lys-home/src/record/mod.rs` — Home::create_session already takes a parent_session argument. Session::append writes a child of the head and moves the head forward, and Session::append_entry appends a whole entry, so a copy can keep ids byte for byte. path() and context_path() return the root-to-head path (context_path puts a compaction first), and lock_file() is the lock for appending to a parent that may be live. 417 code lines.
- `crates/lys-home/src/record/entries.rs` — SessionHeader.parent_session is serialised as `parentSession`. The lys custom types are defined here as constants (lys.harness_event, lys.call, lys.authored, lys.inherited); lys.fork, lys.forked_from and whatever a lantern is recorded as would be added beside them.
- `crates/lys-home/src/record/index.rs` — Index::ancestry(id) walks the rows from an entry back to the root, and read_rows_from seeks only to those rows. That is how a cut at a past point is read without loading the whole file (CN7).
- `crates/lys-home/src/harness/claude_code/import.rs` — store_part stores every content part as a block, but assistant_content and user_content also write the mapped part inline into the Pi message entry. Message entries carry their content in the session file and do not reference blocks by hash.
- `crates/lys-home/src/harness/claude_code/render.rs` — Renders a session to a Claude Code JSONL under a chosen uuid and skips custom entries. The child is rendered through this before launch, and --fork-session resume was proved on it in PROOF-RESUME.md.
- `docs/design/home/DESIGN.md / design.json` — The cluster this work continues. Its non-goals exclude 'Lanterns and forks at a coordinate (stages 5 and 5b)', so the cluster has to be amended and a new brief (HOME-002) added. Its 45 structure rows carry no fork, lantern or launch-template path.
- `docs/design/home/RECORD.md` — The written contract of the home record: Pi's grammar as adopted, the head, the context path and the lys custom entries. It says attachment and system harness events sit on the chain at their exact place, so a context path contains lys.harness_event entries. lys.fork and lys.forked_from have to be written down here.
- `docs/design/home/PROOF-RESUME.md` — The measured resume the fork stands on: Claude Code 2.1.281, render under a new uuid, `claude -p --resume <uuid> --fork-session`, source hash unchanged, resume-check counting repeated tool uses. The fork proof should follow its shape.
- `docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md:85-97` — Stage 5 (lantern recall: light one, recall it by note and by coordinate) and stage 5b (a working fork, the cut and seed, measured per harness, proof: commune with a lit lantern through a fork). Stage 5 is not built.
- `~/Developer/ablative/tools/lantern/docs/10-CUT-ONE-BRIEF.md:43-66` — The lantern design's cut rule: walk the chain back to the nearest compaction boundary, then forward to the last assistant-anchored record at or before the coordinate. If the coordinate is an unanswered user message, its text goes verbatim at the head of the seed prompt and the record says coordinate_carried: true. The words cut back to the assistant entry and do not say whether the message is carried.
- `~/Developer/ablative/tools/lantern/docs/03-DESIGN.md:205-225` — The measured headless backend: write the truncated copy under a new id, then `claude -p --resume <id> --fork-session`, with the fork's working directory owned by the server and the cut ending at an assistant-anchored record.
- `docs/design/identity/STATEMENT-2026-09-22.md:93-103` — Tom's definition of 8 August (a lantern is a pathway back to a conversation with the previous self through a fork) and Chippy's 14:48 line: a fork has its own execution identity linked to the parent, its own handles and lease, and lys records the ancestry.
- `scripts/design/gate.sh` — The design leg of the gates: every cluster validates against the method's schemas, its coverage is clean, and its markdown is what its JSON renders to. A HOME-002 brief and the amended design.json must pass it.

### What was already decided

- home P1 — The original bytes are never rewritten; derived records sit beside their source and point at it. Appending lys.fork to the parent must leave every earlier byte unchanged.
- home P2 — The record is Pi's session tree at 3d5cbe98. lys adds entry kinds only as custom entries and never adds a field, so lys.fork and lys.forked_from must be `custom` entries and the ancestry must use Pi's own header field parentSession.
- home P4 — A content block is stored once by its hash and referenced, not copied. The words lean on this for 'no block bytes duplicated'.
- home P6 — A resume path is a measurement per harness and per version: 2.1.281 was measured. This Mac now has 2.1.282.
- home P7 / CN3 — Transcript contents never appear in output, logs, errors, test names or pages; the fork's report and proof document carry hashes, counts and paths only.
- home P8 — A sandbox or VM is a target profile the launch template renders into. Credentials are supplied at launch on the target and never carried in the home. This is the only place the tree mentions a launch template.
- home CN1 — No file under ~/.claude/projects is rewritten; a render writes a new uuid only and refuses an existing path by name.
- home CN4 — Every home file parses with Pi's parseSessionEntries unchanged; lys data goes only inside custom entries.
- home CN7 — A path is read through the index and the persisted head, never by loading the file.
- home non-goal 'Lanterns and forks at a coordinate (stages 5 and 5b)' — HOME-001 put lanterns and forks out of scope because they stand on a proved resume, which HOME-001 supplied (PROOF-RESUME.md). This card lifts the fork half.
- CONTEXT-ROADMAP stage 5 — Lantern recall: a lantern is a note plus a coordinate; proof: light one and recall it by note and by coordinate. Not built: nothing in crates/ or docs/design/home defines a lantern.
- CONTEXT-ROADMAP stage 5b — A working fork is a launch from a lantern's coordinate with its own execution id and ancestry, the cut and seed, measured per harness; proof: commune with a lit lantern through a fork.
- lantern 10-CUT-ONE-BRIEF 'the one design decision' — The cut ends at the last assistant-anchored record at or before the coordinate. An unanswered user message at the coordinate is carried verbatim at the head of the seed prompt under an in-band marker, with coordinate_carried: true.
- ADR-007 — The product gives an agent's start command rendered from its kept launch record, carrying its identity and handles but never a credential's value; it never runs it.
- ADR-001 — A seat holds a short-lived handle bound to its identity, and the door swaps it for the credential. The fork's 'own handles' are these, and the broker that issues them (RM-002) is briefed, not built.
- RM-005 — The home row (briefed): lanterns, translation and forks stand on it. This work is the first fork unit under it or a new row beside it.

### What was measured

- Files or code in the lys tree (crates/, docs/design/home) that define a lantern: 0 (grep -ril lantern finds only design prose: DESIGN.md, design.json, HOME-001, the identity statement, the roadmap, the handoff)
- Files in lys, aion or cambium source defining a Claude Code launch template: 0 (only mentions: home DESIGN.md P8 and copies of design docs under cambium/.work; argus launches through manifold profiles in lib/argus/launches.ex)
- lys custom entry kinds defined in crates/lys-home/src/record/entries.rs: 4 (lys.harness_event, lys.call, lys.authored, lys.inherited); lys.fork and lys.forked_from are absent
- lys-home subcommands today: 6 (import, render, canon create|add, fewshot, ingest-call, resume-check); no fork
- cli.rs non-comment, non-blank lines: 398 of the 500-line limit (485 total lines)
- record/mod.rs non-comment, non-blank lines: 417 (552 total lines)
- harness/claude_code/import.rs non-comment, non-blank lines: 473 (531 total lines)
- lys-home #[test] functions: 36, plus 2 integration test files (cached_index.rs, claude_code_round_trip.rs)
- HOME-001 requirements: 12 (R1 to R12)
- home design.json structure rows: 45; none name fork, lantern or a launch template
- Home proof documents on HEAD: 3 (PROOF-CANON.md, PROOF-FEWSHOT.md, PROOF-RESUME.md); PROOF-PROXY.md and PROOF-HANDOVER.md are named in the structure but absent
- Claude Code version on this Mac: 2.1.282 (P6 and PROOF-RESUME measured 2.1.281)
- Real session measured in PROOF-RESUME: 562,932 bytes, 242 records, 205 home entries (81 messages plus 124 harness events), 81 blocks
- How the Pi-derived copy found on this Mac treats parentSession: As a file path: parentSessionPath, previousSessionFile and sourcePath at ~/Downloads/gsd-2-main/packages/pi-coding-agent/src/core/session-manager.ts:722,1343,1496. This is a derived tree, not the 3d5cbe98 reference checkout, which was not found on this Mac
- Roadmap rows: 5 (RM-001 to RM-005); no row for lantern recall or forks

### What it means for the other projects

- cambium — The card lives on Cambium's board and must go through brief_card, sign-off, card_build_v3, src_pr and src_land (Tom's rule 1). Cambium's lifecycle screen (roadmap stage 7) will later show a session's forks from lys.fork and lys.forked_from, so the entry data should be readable without transcript content.
- aion — The card_build_v3, src_pr and src_land chain runs the build and the gates (full gates on Dean's laptop). Workflow inputs name the lys repository, a commit (0073b966), the card and the HOME-002 brief, never a folder.
- argus — Argus launches sessions from manifold supervisor profiles (lib/argus/launches.ex). If the Claude Code launch template is to be 'like any session', argus's launch path is a likely consumer or owner of it, and the fork's rendered launch line must not diverge from it.
- method — The HOME-002 brief and the amended home design.json, checklist and stories must validate against the method's schemas and render cleanly through scripts/design (validate.py, check-coverage.py, render-cluster.py).

### The decisions it stands on

- ADR-007 (honour) — lys-home renders the child's launch line from the template and prints it, never running it. The acceptance's 'its launch line resumes it' is run by hand for the proof.
- ADR-001 (honour) — The child's handles are supplied at launch by the template and never copied from the parent, and no credential is carried in the home (P8).
- ADR-003 (honour) — The fork is a new session under the same enduring agent identity. Its ancestry is recorded so the fork's actions are never indistinguishable from the parent's; it gains no authority.
- ADR-004 (honour) — The fork subcommand and its render must work without manifold, argus or the broker being up; the launch template names a target and does not depend on one engine.
-  (new) — Two new lys custom entry kinds, lys.forked_from (child: parent session, lantern, point, cut-ended-at) and lys.fork (parent, at its head: the child). They are home-local state rather than signed wire formats, but they are written into durable session files and need recording in RECORD.md.
-  (new) — How a lantern is held in a home (for example a lys.lantern custom entry with a note and a point). No lantern representation exists and the fork's refusal depends on it.
-  (new) — The home cluster's non-goal 'Lanterns and forks at a coordinate (stages 5 and 5b)' is lifted for the fork: the cluster's design.json is amended and HOME-002 added.

### What it requires

- `lys-home fork --home <dir> --lantern <id>` exists, prints one JSON report and exits zero on success.
- An unknown lantern id is refused with a non-zero exit and an error naming that lantern id; no session is created and no byte of the parent changes.
- The child session file has a new id different from the parent's, and its header's parentSession names the parent (as settled by the lead: id or path).
- With a lantern at a past assistant entry, the child's copied entries are byte-identical, line for line, to the parent's root-to-point path, and no entry after the point is present.
- With a lantern at a user entry, the cut ends at the last assistant entry at or before the point, and the report's cut-ended-at field names that entry id, which differs from the lantern's point.
- The child carries exactly one lys.forked_from custom entry naming the parent session, the lantern and the point.
- After the fork, the parent's head is a lys.fork custom entry naming the child's id.
- The parent file's bytes before the fork are a byte-identical prefix of the parent file after the fork (SHA-256 of the prefix unchanged).
- The block store holds the same number of blocks before and after a fork (no block bytes duplicated).
- Two forks from one lantern produce two sessions with distinct ids whose referenced block sets are equal.
- The report carries the child id, parent, lantern, point, cut-ended-at entry, entry count and block count, and contains no transcript content (checked by a test asserting no content string appears).
- Every child file parses with Pi's parser unchanged (CN4), and both new entries are `custom` entries with lys.* customType.
- The child renders through the Claude Code renderer under a fresh uuid, and the launch template produces a launch line that resumes it on a named, measured Claude Code version.
- docs/design/home/PROOF-FORK.md (or the named proof file) records a conversation held with the lantern's self through the fork, as hashes, counts and paths only, with the version and command.
- Tests assert the rejection count and the case count, not only successes (the repository's second-party rule).
- RECORD.md documents lys.fork and lys.forked_from; the home design cluster carries the new brief and its structure rows; scripts/design/gate.sh passes.
- All gates pass: fmt, clippy in both feature shapes with -D warnings (pedantic per Tom's rule 2), tests with --all-features, cargo doc in both shapes, the design gate and ast-grep.

### What must not change

- No byte of the parent session file before the fork is rewritten or truncated (P1); the only change is the appended lys.fork entry.
- No file under ~/.claude/projects is rewritten; the rendered child goes under a new uuid and an existing path is refused by name (CN1).
- No field is added to Pi's grammar; ancestry uses Pi's header parentSession and custom entries only (P2, CN4).
- No transcript content in the report, errors, logs, test names or the proof document (P7, CN3).
- No credential or handle is copied from the parent or stored in the home (P8, ADR-001).
- No Norn crate or type is used (CN6).
- The cut reads only the path's entries through the index, never the whole parent file (CN7).
- Wire formats in lys-core are untouched; this is home-local state, not a signed format.
- Leases, budgets, kill rules, tools, lenses, cuts without a lantern, and harnesses other than Claude Code stay out.

### What we must put in place first

- A lantern representation in the home, plus a way to light one (stage 5 lantern recall), or the lead's decision that this card defines a minimal one. Without it the fixture 'lit lantern' cannot exist.
- A Claude Code launch template (the thing that renders the launch line and supplies handles), or the lead's decision that this card builds it or stops at a printed resume line.
- A decision on handles while the broker (RM-002, SECRETS-002) is unbuilt: launch without handles, or wait.
- Confirming Pi's parentSession semantics against the 3d5cbe98 reference checkout (only a derived gsd-2 copy is on this Mac, and it reads the field as a path).
- A HOME-002 brief in docs/design/home, with the cluster's non-goal amended, written and signed off through brief_card before card_build_v3 (Tom's rule 1).

### The risks

- Hash-match and 'first lys entry is forked_from' cannot both hold on a real session whose path carries lys.harness_event entries. A fixture without harness events would pass while the real proof fails.
- Appending lys.fork at the parent's head while the parent is live and still being captured makes the next captured message a child of lys.fork, putting it on the parent's context path. Render skips custom entries, but any consumer that walks parentIds sees it.
- Concurrent appends to the parent (capture versus fork) without taking the session lock could interleave lines or split the head.
- A lantern at a user entry silently drops the question the lantern was lit on unless it is carried as seed, so the forked self answers from before the moment it was asked about.
- Copying message entries duplicates content bytes in the child file. With the 3.37 GB and 141 MB sessions on record, repeated forks multiply the home's size even though blocks are shared.
- Claude Code is now 2.1.282, while the resume proof measured 2.1.281. A changed loader chain rule would make a cut that looked correct fail to resume.
- A parentSession holding an id where Pi expects a path would make Pi's own reader resolve a non-existent parent file.
- A compaction on the path copied as context_path() output (compaction first) is not a parent chain; copying it with ids intact would produce a broken tree.
- cli.rs is at 398 of its 500 code lines and record/mod.rs at 417, so fork logic added there breaches the file-size rule.
- A proof conversation with the lantern's self risks transcript text leaking into PROOF-FORK.md or a test name.

### Still open

- The tree has no lanterns. Does this card also define and light a minimal lantern (a lys custom entry holding a note and a point, plus a light command) so it has something to fork from, or does it wait for a stage 5 lantern-recall card to land first? The sentence of the words it stands on: "A lys-home subcommand forks: it takes the home and a lantern id, refuses by name a lantern it does not hold, creates a new session with its own id whose header names the parent session as Pi's parentSession, copies the context path up to the point as entries that reference the same blocks by hash, with no block bytes duplicated, and appends first to the child a lys.forked_from custom entry naming the parent session, the lantern and the point, and then to the parent, at its head, a lys.fork custom entry naming the child, so the ancestry is written on both sides.". Why only the lead can settle it: crates/lys-home has no lantern type, entry or command, and CONTEXT-ROADMAP stage 5 (lantern recall) is not built, so 'a lit lantern' in the acceptance fixture cannot exist. The answer decides whether this card also delivers lighting a lantern, which changes what a person can do and where this card's scope ends.
- No Claude Code launch template exists anywhere in the estate. Does this card build one (and to what shape: a rendered `claude --resume <uuid>` line per ADR-007, with a working directory and target profile), or does the fork stop at render plus a printed launch line until a template card exists? The sentence of the words it stands on: "The child is then rendered and launched through the Claude Code launch template like any session, with its own handles supplied at launch by the template and never copied from the parent; its lease is the seat's lifecycle and is not this card's.". Why only the lead can settle it: The words treat the template as existing, but the tree has none: only home DESIGN.md P8 mentions one, and argus launches through manifold profiles. What a person runs to start the child depends on the answer.
- The broker that issues handles (RM-002, SECRETS-002) is not built, so there are no handles to supply. Does the child launch on the seat's own login with no handles for now, or does the launch refuse until handles exist? The sentence of the words it stands on: "The child is then rendered and launched through the Claude Code launch template like any session, with its own handles supplied at launch by the template and never copied from the parent; its lease is the seat's lifecycle and is not this card's.". Why only the lead can settle it: ADR-001 and ADR-002 describe handles, but RM-002 is only briefed. The answer decides whether the acceptance proof can run today and what access the forked session has.
- Home message entries carry their content inline, not by hash. Is a child file that repeats the parent's message text in its own entry lines acceptable, as long as the block store gains nothing? Or must forked message entries point at blocks instead, which Pi's parser and the renderer cannot read? The sentence of the words it stands on: "A lys-home subcommand forks: it takes the home and a lantern id, refuses by name a lantern it does not hold, creates a new session with its own id whose header names the parent session as Pi's parentSession, copies the context path up to the point as entries that reference the same blocks by hash, with no block bytes duplicated, and appends first to the child a lys.forked_from custom entry naming the parent session, the lantern and the point, and then to the parent, at its head, a lys.fork custom entry naming the child, so the ancestry is written on both sides.". Why only the lead can settle it: crates/lys-home/src/harness/claude_code/import.rs stores each part as a block and also writes the mapped part inline in the Pi message entry. Copied entries therefore duplicate content bytes in the child session file even though the block store gains nothing. That is what is kept on disk for each fork.
- The copied context path already contains lys.harness_event entries (and lys.inherited or lys.authored ones when the parent carries them). Should 'the child's first lys entry is forked_from' mean the first lys entry appended after the copy, or must forked_from sit at the child's root, which changes the copied entries' parentIds and breaks the hash-match? The sentence of the words it stands on: "A lys-home subcommand forks: it takes the home and a lantern id, refuses by name a lantern it does not hold, creates a new session with its own id whose header names the parent session as Pi's parentSession, copies the context path up to the point as entries that reference the same blocks by hash, with no block bytes duplicated, and appends first to the child a lys.forked_from custom entry naming the parent session, the lantern and the point, and then to the parent, at its head, a lys.fork custom entry naming the child, so the ancestry is written on both sides.". Why only the lead can settle it: docs/design/home/RECORD.md puts attachment and system harness events on the chain at their exact place, so a real parent's context path already holds lys.* entries before any forked_from. The acceptance check 'the child's first lys entry is forked_from' conflicts with 'hash-match the parent's context path' unless the check is read as the first entry appended by the fork.
- Pi's own code reads parentSession as the parent session's file path. Should the child header carry the parent's session id, as the acceptance says, or the parent file's path, as Pi does? The sentence of the words it stands on: "A lys-home subcommand forks: it takes the home and a lantern id, refuses by name a lantern it does not hold, creates a new session with its own id whose header names the parent session as Pi's parentSession, copies the context path up to the point as entries that reference the same blocks by hash, with no block bytes duplicated, and appends first to the child a lys.forked_from custom entry naming the parent session, the lantern and the point, and then to the parent, at its head, a lys.fork custom entry naming the child, so the ancestry is written on both sides.". Why only the lead can settle it: The Pi-derived session-manager.ts on this Mac (gsd-2-main, lines 722, 1343, 1496) treats parentSession as a path (parentSessionPath, previousSessionFile, sourcePath). home P2 adopts Pi's grammar and meaning, so an id would be a different value in Pi's own field. This has to be confirmed against the 3d5cbe98 reference before the header's value is chosen.
- When the lantern's point is a user message and the cut falls back to the preceding assistant entry, is that user message dropped, or carried into the child's first prompt as the lantern design's cut one does (coordinate_carried)? The sentence of the words it stands on: "Claude Code's resume walks the chain from an assistant-anchored leaf, so when the lantern's point is not an assistant message the cut ends at the last assistant message at or before the point and the report names the entry it stopped at.". Why only the lead can settle it: tools/lantern/docs/10-CUT-ONE-BRIEF.md:43-51 carries the unanswered user message verbatim at the head of the seed prompt under an in-band marker, with coordinate_carried: true. The words only say the report names the stopping entry. The answer decides whether the forked self ever sees the question the lantern was lit on.

### The units beyond the first

- Lantern recall: light a lantern at a point with a note, and recall it by note and by point (roadmap stage 5) — The fork refuses a lantern it does not hold, and nothing in the tree holds one. Lighting and recall are their own proof on the context roadmap and stand before 5b.
- The Claude Code launch template: render a launch line for a home session to a target profile, with handles supplied at launch — Every session and the fork launch through it (P8, ADR-007), and it is a shared piece rather than the fork's; it also carries target profiles (sandbox, VM, another node) later.
- Handles for a forked session through the broker — The fork's own handles depend on SECRETS-002 and RM-002, which are briefed but unbuilt. Until then the fork launches on whatever the template supplies.
- Carrying an unanswered user message at the lantern's point into the child's seed — The lantern design's coordinate_carried behaviour. It is kept out of the first unit unless the lead folds it in, and it changes what the forked self is first asked.
- The lantern briefed on what changed since — The identity statement's lantern is a read-only fork briefed on what changed since the point. That seed and lens are out of this card's scope ('forks with tools or a lens').
- The lifecycle screen shows a session's forks and ancestry — Roadmap stage 7 reads lys.fork and lys.forked_from. It is product UI in lys and Cambium, not part of the record.

### The smallest complete shape

One card, HOME-002, on the home cluster, landed through the chain. It contains:
- A record/fork.rs module with sibling tests.
- A `lys-home fork --home --lantern` subcommand that:
  - resolves a lantern held in the home and refuses an unknown one by name;
  - reads the parent's root-to-point chain through the index;
  - ends the cut at the last assistant entry at or before the point;
  - creates the child with parentSession set;
  - copies the chain's entries byte for byte, then appends lys.forked_from to the child and lys.fork at the parent's head under the session lock;
  - prints the JSON report of ids and counts.
- The two new custom entries written into RECORD.md, and the cluster amended.
- A PROOF-FORK.md that forks a fixture lantern, renders the child, launches it with a rendered `claude --resume` line on a named version, and records the exchange as hashes, counts and paths.

This is complete only if a minimal way to hold and light a lantern and a minimal Claude Code launch line are either already in place or taken into this card by the lead's decision. Otherwise the card cannot meet its own acceptance.

## The roadmap row

- **RM-006** — Fork a session from a lantern's point, with its ancestry written on both sides (feature, idea)
- Summary: Stage 5b of the home: `lys-home fork` takes a home and a lantern id, cuts the parent's chain from its root to the lantern's point (back to the last assistant message when the point is not one), writes a new session from the parent's own lines under Pi's parentSession, adds nothing to the block store, and writes lys.forked_from in the child and lys.fork at the parent's head. A user message at the point is carried as the child's first prompt. The child renders and launches like any session by its printed resume-by-path line, and a proof records a conversation with the lantern's self through the fork.
- Asked by: tom on 2026-09-26T06:30:25+10:00
- Context: The words of the card filed on the Lys board for the context roadmap's stage 5b (docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md), surveyed against lys main 0073b966, with the lead's answers on the lantern prerequisite, the launch line, handles, inline content, the first entry the fork writes, parentSession as a path, carrying a user message at the point, the session a lantern is forked from, a lantern before any assistant message, and a carried message's parts that are not text.
- Quote: A fork is a launch from a lantern's point with its own session identity and its ancestry recorded, so a later session can hold a conversation with the self that lit the lantern. Claude Code resumes a session from its end, never from a point inside it, so a fork at a point is the lantern design's cut and seed: a new session in the home whose entries are the parent's context path from its root to the lantern's point, and nothing after. A lys-home subcommand forks: it takes the home and a lantern id, refuses by name a lantern it does not hold, creates a new session with its own id whose header names the parent session as Pi's parentSession, copies the context path up to the point as entries that reference the same blocks by hash, with no block bytes duplicated, and appends first to the child a lys.forked_from custom entry naming the parent session, the lantern and the point, and then to the parent, at its head, a lys.fork custom entry naming the child, so the ancestry is written on both sides. Claude Code's resume walks the chain from an assistant-anchored leaf, so when the lantern's point is not an assistant message the cut ends at the last assistant message at or before the point and the report names the entry it stopped at. The child is then rendered and launched through the Claude Code launch template like any session, with its own handles supplied at launch by the template and never copied from the parent; its lease is the seat's lifecycle and is not this card's. The report is JSON: the child session id, the parent, the lantern, the point, the entry the cut ended at, entry and block counts, never content. Acceptance is that on a fixture session with a lit lantern at a past assistant entry, the fork's entries hash-match the parent's context path up to that entry and stop there, the child header's parentSession is the parent's id, the child's first lys entry is forked_from and the parent's head is a fork entry naming the child, the parent's earlier bytes are unchanged; that a lantern at a user entry forks to the preceding assistant entry and the report says so; that an unknown lantern id is refused by name; that the launch template renders the child and its launch line resumes it, and a proof document records that a conversation was held with the lantern's self through the fork, as hashes, counts and paths only; and that two forks from one lantern are two sessions with distinct ids that share every block. Not in scope: cutting at a point that has no lantern; forks with tools or a lens; leases, budgets and kill rules; harnesses other than Claude Code. Filed by Archie on Tom's definition of 8 August 2026 that a lantern is a pathway back to a conversation with the previous self through a fork, his roadmap stage 5b of 22 September 2026, the lantern design's cut-and-seed measurement, and home DESIGN P1, P2, P4 and P8 at lys main 0073b966, on 26 September 2026.
- Cluster: home; briefs: HOME-002
- Notes: Stands on the lantern card, 'Light a lantern and recall it' (board card YUTHf4z0, brief 7657952c), which delivers lys.lantern, the lit-in session under its data key lit_in, and the lantern light subcommand; that card is ahead of this one and not yet on main, so HOME-002 is built on main after it lands and is refreshed onto that main (its brief id, checklist, story, decision and roadmap numbers collide with the lantern card's and are renumbered then). Further units, not written: Lantern recall: light a lantern at a point with a note, and recall it by note and by point (roadmap stage 5); The Claude Code launch template: render a launch line for a home session to a target profile, with handles supplied at launch; Handles for a forked session through the broker; The lantern briefed on what changed since; The lifecycle screen shows a session's forks and ancestry.

## The design

---
type: design
cluster: home
title: The home: a session held under its identity, resumable by any harness that can be measured
---

# The home: a session held under its identity, resumable by any harness that can be measured

> **Cluster:** home

## Intention

A session's history lives under the agent's identity, not under a harness. The record is what the model was given and what came back, in order, with each provider's native blocks kept whole. A harness's own resume file is rendered from that record on demand, so an agent moves machines, harnesses or providers and continues; the original bytes are never rewritten.

## Problem

Today a session exists only as its harness's file. Archie's Claude Code file measured 141,931,097 bytes on 24 September 2026: the conversation itself is under 3% of it, tool results 14%, thinking 6%, and about 60% is harness bookkeeping the model never sees; Waffles' file is 3.37 GB and Claude Code loads it whole. Nothing central holds a session, so nothing can render a smaller working session, move one to another device, swap provider and come back, or say what a session was given. Norn already has the canonical event model (SessionEvent, conversion to provider messages, compaction as a derived event, provider epoch boundaries) proved by its own tests; Claude Code has no importer to it and no renderer from it.

## Solution

Adopt Pi's session tree as the home record (Tom, Dot 13:27 and 13:28: Pi's tree, not Norn): one append-only JSONL per session, a header line, then entries each carrying id, parentId and timestamp, a leaf pointer for the current position, forks by moving the pointer, compaction and branch summaries as entries that keep their originals. lys adds nothing to that grammar: harness events (approvals, tool completion) and proxy call records ride Pi's custom entry type under lys customType names, so a home file stays readable by Pi's own parser. Two captures feed it: the model traffic through the door's proxy (the same process that swaps the credential, SECRETS-002 R1), and the harness's own events from its transcript. Content blocks are stored once by hash; entries reference them. A harness resume file is a rendered projection of the root-to-leaf path: for Claude Code, a JSONL written under a chosen uuid at the harness's own path, then resumed by that id with --fork-session. Provider-native reasoning stays on the message with its provider, api and model and is rendered whole only to the same three; another model gets readable thinking as text and opaque blocks dropped, each named in a loss account. lys grants say who may read and resume. Every resume path is measured on a named harness version before anything relies on it. A fork (HOME-002) is a launch from a lantern's point: the lantern is a lys.lantern entry the lantern card delivers, and the fork resolves it, cuts the parent's raw root-to-point chain through the index back to the last assistant message at or before the point (Claude Code's resume walks from an assistant-anchored leaf), and writes a new session whose header's parentSession is the parent file's path relative to the home and whose entry lines are the parent's own lines, byte for byte, so every part and block hash is the parent's and the block store gains nothing (P4). A lantern is forked from the session it was lit in, as its lys.lantern entry records it; a copy of its line inside a child is a copy, not a second lantern, and a lantern with no assistant message at or before its point is refused, since a cut there would carry only the seed. The fork then writes the ancestry on both sides as two custom entries, lys.forked_from in the child directly after the copy and lys.fork at the parent's head, appended only (P1, P2). When the point is a user message the cut stops at the assistant entry before it and that message is carried, not copied: the child's render writes its text parts beside the rendered file as a seed prompt under an in-band marker, and lys.forked_from counts the parts the seed left out, and the printed launch line passes it as the resumed session's first prompt. The child renders and launches like any session: the render prints the resume-by-path launch line, and whatever a launch supplies is supplied at launch, never copied from the parent (P8).

## Principles

- **P1** — The original bytes are never rewritten. A rendered resume file, a compaction and a translation are derived records stored beside their source and pointing at it (CONTEXT-ROADMAP-2026-09-22.md stage 4).
- **P2** — The record is Pi's session tree as read from the Pi checkout at 3d5cbe98 (packages/coding-agent/src/core/session-manager.ts: SessionHeader, SessionEntryBase {type, id, parentId, timestamp}, message, model_change, compaction {summary, firstKeptEntryId}, branch_summary {fromId, summary}, label, custom {customType, data}). lys adds entry kinds only as custom entries; it never adds a field to Pi's grammar and never adopts Norn's SessionEvent (Tom, Dot 13:28).
- **P3** — A provider's opaque blocks (Anthropic signed thinking, OpenAI encrypted reasoning) stay on the assistant message with its provider, api and model, and are rendered whole only when all three match the target (Pi transform-messages.ts:95-109); another model gets readable thinking as plain text and opaque blocks dropped, each named by hash in the loss account (Tom, Dot 13:24: keep reasoning traces per provider so a session can swap and swap back).
- **P4** — A content block is stored once by its hash; requests that resend the whole conversation reference blocks, they do not copy them.
- **P5** — Capture the model traffic and the harness's local events both, so the two can be mapped against each other (Tom, Dot 13:23); neither alone is the record.
- **P6** — A resume path is a per-harness, per-version measurement, never an assumption: Claude Code 2.1.281's --resume takes a session id and reads ~/.claude/projects/<cwd-slug>/<id>.jsonl; a seeded two-record file resumed there at 13:25 on 24 September and answered from its content.
- **P7** — Transcript contents never appear in a post, a log line, an error, a test name or a rendered page; status carries hashes, counts and offsets only (CONTEXT-001 privacy rule).
- **P8** — A sandbox or a VM is a target profile the launch template renders into, never a special case in the core; credentials are supplied at launch on the target and never carried in the home (Tom, Dot 13:30; CONTEXT-ROADMAP stage 3).
- **P9** — The canon is one curated, versioned series of examples every new session starts from: each entry is one rule stated short plus one real exchange that shows it lived (verify before claiming, a correction taken well, a refusal named, careful work), drawn from every agent's sessions, distilling the collective experience so far: 'our learnings in one another' (Tom, Dot 13:44 to 13:46; Waffles 0169c353). It is not a letter from one session to its successor. It lives in the lys repository at canon/canon.jsonl in Pi's grammar and changes only through src_commit and review, like code. Genuine thinking is kept whole where a real turn produced it and replays only to the same provider, api and model; thinking is never authored.
- **P10** — A handover is a letter the outgoing session writes to its successor in its own real thinking and answer; it enters the successor as a lys.inherited entry marked as a predecessor's memory, never as the successor's own experience, and replays only to the same provider, api and model. No thinking is ever authored; only thinking a model produced is kept (Tom, Dot 13:38 to 13:40: 'you are waking up ... I am another one, I'm helping you have part of my memory ... like a parent imparting a wish to a child'; Waffles 52d53451).

## Decisions

- ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
- ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
- ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
- ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
- ADR-012 — A fork's ancestry is written on both sides as two lys custom entries — The child carries a lys.forked_from custom entry (parent session id, lantern, point, the entry the cut ended at, whether the point's user message was carried and which entry it was), appended directly after the copied path, and the parent carries a lys.fork custom entry naming the child, appended at its head; both are home-local state in durable session files, not signed wire formats. Rejected: recording the ancestry in the child's header parentSession alone, which is one-sided, names no lantern or point, and would have needed a field outside Pi's grammar to say more; and a sidecar file beside the sessions, which Pi's parser never reads and which could drift from the files it describes.

## Goals

- A few-shot session file written by hand resumes Claude Code by path, from a directory outside the config root, with the file preserved and the demonstration marked authored.
- One real Claude Code session imported into the common record, rendered back, and resumed under a new id on 2.1.281 without repeating a completed tool action, with the original file's hash unchanged.
- One seat with a subscription login making calls through a pass-through proxy, so the tee has somewhere to live.
- A written loss account for every derived record: what a render or translation preserved, transformed and could not carry.
- The canon, one curated versioned series of short examples each showing a rule lived, seeds every new session, and its effect is measured on a card against a plain start.
- A handover letter from an outgoing session seeds its successor as inherited memory, with the model's own thinking intact, and the effect is measured on a card against a plain start.
- A session forked from a lit lantern at a past assistant entry renders, resumes by its printed launch line on a named Claude Code version and answers from the path up to the point, with the parent's earlier bytes and the block store unchanged.

## Non-Goals

- Anchoring, signing or receipts into a lys log (CONTEXT-ROADMAP stage 6; when asked for). — Signing comes when asked for (Tom, 22 September 16:27); every stage here works without it.
- Encryption at rest and moving a home between devices (stage 3 preconditions). — Stage 3's three preconditions (identity and read authority, encryption before bytes leave, the resume evidence) are their own brief.
- Harnesses other than Claude Code, and Chat Completions or Responses translation beyond keeping the raw call bytes. — One harness proved first; each other harness is its own profile and its own measurement.
- Adopting, wrapping or calling Norn's session code; Pi's code is read as the reference and not vendored. — Tom, Dot 13:28: not Norn. Pi's tree is the reference.
- Lighting, holding and recalling lanterns (stage 5). — The lantern card delivers the lys.lantern entry and its subcommands; the fork stands on it and adds nothing to it.
- Cutting at a point that has no lantern. — A fork is a launch from a lantern's point; the words put a cut without a lantern out of scope.
- Forks with tools or a lens. — Out of scope by the words; the lantern briefed on what changed since is its own unit.
- Leases, budgets and kill rules for a forked session. — A fork's lease is the seat's lifecycle, not the fork's.
- Building the Claude Code launch template, and issuing a forked session's handles. — The launch template card prints the same resume-by-path line when it lands, and handles arrive with the broker; the fork launches on what a launch supplies today and copies nothing from the parent.

## Structure

| Path | Note | Brief |
|------|------|-------|
| `docs/design/home/briefs/HOME-001.json` | the first brief: common record, Claude Code importer and renderer, the two proofs | HOME-001 |
| `docs/design/home/briefs/HOME-001.md` | its rendered markdown | HOME-001 |
| `docs/design/home/PROOF-RESUME.md` | the measured Claude Code resume: version, command, hashes before and after, what repeated | HOME-001 |
| `docs/design/home/PROOF-PROXY.md` | the measured subscription login through a pass-through proxy: version, headers that mattered, what failed | HOME-001 |
| `docs/design/home/LOSS-ACCOUNT.md` | what the Claude Code render preserves, transforms and cannot carry | HOME-001 |
| `crates/lys-home/src/record/blocks.rs` | content-addressed block store: put by SHA-256, get by hash, never rewritten | HOME-001 |
| `crates/lys-home/src/harness/claude_code/import.rs` | Claude Code JSONL into SessionEvents plus blocks | HOME-001 |
| `crates/lys-home/src/harness/claude_code/render.rs` | SessionEvents into a Claude Code JSONL under a chosen uuid, with the loss account | HOME-001 |
| `crates/lys-home/src/harness/claude_code/mod.rs` | the Claude Code harness profile: transcript root, cwd slug, version measured | HOME-001 |
| `crates/lys-home/src/cli.rs` | import, render, resume-check subcommands | HOME-001 |
| `crates/lys-home/examples/passthrough.rs` | a pass-through HTTP proxy that forwards to the provider unchanged, for the proof only | HOME-001 |
| `crates/lys-home/tests/claude_code_round_trip.rs` | import then render equals the model-visible content; opaque blocks kept whole | HOME-001 |
| `crates/lys-home/src/record/mod.rs` | the home record: Pi's session tree read and written, leaf pointer, root-to-leaf path | HOME-001 |
| `crates/lys-home/src/record/entries.rs` | Pi's entry types as Rust types, plus the lys custom entries lys.harness_event and lys.call | HOME-001 |
| `crates/lys-home/src/record/call.rs` | a proxy call record: request and response block hashes, provider, api, model, timing | HOME-001 |
| `crates/lys-home/src/harness/claude_code/events.rs` | Claude Code's harness-local records (hooks, permission mode, tool completion) as lys.harness_event entries | HOME-001 |
| `docs/design/home/RECORD.md` | the home record written down: Pi's grammar as adopted, the two lys custom entries, the block store, the loss account | HOME-001 |
| `crates/lys-home/src/lib.rs` | module wiring: record and harness | HOME-001 |
| `crates/lys-home/src/harness/mod.rs` | harness profiles; Claude Code first | HOME-001 |
| `crates/lys-home/Cargo.toml` | the crate manifest; gains the passthrough example | HOME-001 |
| `crates/lys-home/README.md` | what the tool does and does not do | HOME-001 |
| `crates/lys-home/src/record/index.rs` | the offset index and the persisted head: a path is read by seeking, never by loading the file | HOME-001 |
| `crates/lys-home/src/proxy/mod.rs` | the proxy module: declarations only | HOME-001 |
| `crates/lys-home/src/proxy/link.rs` | how a call is linked to a session: the measured request key (Claude Code's request metadata) now, the seat's handle when the door's proxy exists | HOME-001 |
| `crates/lys-home/src/bin/lys-proxy.rs` | the proxy binary | HOME-001 |
| `docs/design/home/PROOF-FEWSHOT.md` | the first proof: a hand-written few-shot session file resumed by path from a directory outside the config root | HOME-001 |
| `crates/lys-home/src/record/canon.rs` | the canon: copy chosen real exchanges (or a clearly authored example) into canon.jsonl with a lys.inherited entry naming each source; seed a new session from it | HOME-001 |
| `crates/lys-home/src/record/canon_tests.rs` | gates on the canon: copied whole, never twice, no authored thinking, rendered first | HOME-001 |
| `docs/design/home/PROOF-CANON.md` | measured: a session started from the canon against a plain start, on one card | HOME-001 |
| `canon/canon.jsonl` | the canon itself, versioned by the repository, changed only through src_commit and review | HOME-001 |
| `crates/lys-home/src/record/handover.rs` | the handover: take the outgoing session's letter turn (thinking intact) and seed the successor with it as lys.inherited | HOME-001 |
| `docs/design/home/PROOF-HANDOVER.md` | measured: a successor seeded with an inherited letter, whether 2.1.281 replays the signed thinking, and seeded against plain on one card | HOME-001 |
| `crates/lys-home/src/proxy/forward.rs` | forward a request to the provider and stream the response back unchanged | HOME-001 |
| `crates/lys-home/src/proxy/capture.rs` | bounded spooling of request and response bodies to files while forwarding, handed to the R6 sink | HOME-001 |
| `crates/lys-home/src/proxy/journal.rs` | the open-call journal: a call id written before forwarding, retired after ingest, so a restart records lost calls once | HOME-001 |
| `crates/lys-home/src/proxy/stream.rs` | the coordinator of the stream readers: picks the grammar by api and hands each frame on as it passes | HOME-001 |
| `crates/lys-home/src/proxy/stream_sse.rs` | SSE framing: events and data lines out of a byte stream, retaining byte order, encoding and trailers | HOME-001 |
| `crates/lys-home/src/proxy/stream_messages.rs` | the Messages event grammar assembled into response parts | HOME-001 |
| `crates/lys-home/src/proxy/stream_chat.rs` | the Chat Completions chunk grammar assembled into response parts | HOME-001 |
| `crates/lys-home/src/proxy/stream_responses.rs` | the Responses event grammar assembled into response parts | HOME-001 |
| `crates/lys-home/src/proxy/error.rs` | the proxy's errors and outcomes, named | HOME-001 |
| `crates/lys-home/src/proxy/forward_tests.rs` | forwarding against a loopback fake | HOME-001 |
| `crates/lys-home/src/proxy/stream_tests.rs` | the three grammars, partial streams | HOME-001 |
| `crates/lys-home/src/proxy/link_tests.rs` | linking by the measured key; unlinked; never inferred | HOME-001 |
| `crates/lys-home/src/proxy/journal_tests.rs` | recovery after a kill: one lost record per open call | HOME-001 |
| `crates/lys-home/src/error.rs` | the home's named errors and refusals |  |
| `docs/design/home/DESIGN.md` | the cluster design, rendered from design.json |  |
| `docs/design/home/CHECKLIST.md` | the checklist, rendered from checklist.json |  |
| `docs/design/home/USER-STORIES.md` | the user stories, rendered from stories.json |  |
| `docs/design/home/briefs/HOME-002.json` | the fork brief: cut a session at a lantern's point, write the ancestry on both sides, render and launch the child | HOME-002 |
| `docs/design/home/briefs/HOME-002.md` | its rendered markdown | HOME-002 |
| `crates/lys-home/src/record/fork_cut.rs` | resolve a lantern id to its session and point, and cut the raw root-to-point chain back to the last assistant message | HOME-002 |
| `crates/lys-home/src/record/fork_cut_tests.rs` | gates on the resolve and the cut: refusals counted, nothing after the point, no side leaf, compaction in place | HOME-002 |
| `crates/lys-home/src/record/fork.rs` | write the child from the parent's own lines, then lys.forked_from in the child and lys.fork at the parent's head | HOME-002 |
| `crates/lys-home/src/record/fork_tests.rs` | gates on the child and the ancestry: lines hash-match, parent prefix unchanged, block store unchanged, held parent refused | HOME-002 |
| `crates/lys-home/src/record/fork_report.rs` | the fork report: ids, cut entry, entry and block counts, never content | HOME-002 |
| `crates/lys-home/src/cli_fork.rs` | the `lys-home fork` subcommand | HOME-002 |
| `crates/lys-home/src/harness/claude_code/seed.rs` | the seed prompt of a fork cut back from a user message, and the printed launch line | HOME-002 |
| `crates/lys-home/tests/fork.rs` | the words' acceptance end to end through the lys-home binary | HOME-002 |
| `docs/design/home/PROOF-FORK.md` | measured: a conversation held with a lantern's self through a fork, as hashes, counts and paths | HOME-002 |

## Inventory

- `docs/design/identity/STATEMENT-2026-09-22.md` — the authority: 'The home', 'The home is portable: lanterns, translation and forks', and steps 4 and 5
- `docs/design/identity/CONTEXT-ROADMAP-2026-09-22.md` — Archie's roadmap; this cluster is its stage 4 and the resume half of stage 3
- `docs/design/identity/briefs/CONTEXT-001.json` — stage 1, byte-for-byte capture and the lys-home crate this cluster extends
- `docs/design/secrets/briefs/SECRETS-002.json` — the door's proxy (R1) that the tee joins once it exists
- `$PI/packages/coding-agent/src/core/session-manager.ts` — Pi's session tree at 3d5cbe98 (github.com/earendil-works/pi): header, entries, leaf (branch at :1125 moves an in-memory cursor; _buildIndex:754 restores the last physical entry on reopen; loadEntriesFromFile:438 reads the whole file)
- `$PI/packages/ai/src/api/transform-messages.ts` — per-provider thinking: same provider, api and model keeps blocks whole; otherwise text, opaque dropped
- `$PI/packages/coding-agent/src/core/branch-summarization.ts` — how an abandoned branch's work is carried into the new one
- `$CCFLARE/README.md` — ccflare at 95c4c6a (github.com/snipeship/ccflare): the concept of a native pass-through that keeps request history; reference only, not used (Tom, Dot 13:34)

## Constraints

- **CN1** — No file under ~/.claude/projects is rewritten or truncated by this cluster; a render writes a new file under a new uuid only, and refuses an existing path by name.
- **CN2** — A rendered file never carries a provider-native opaque block that came from a different provider or model family than the one it is rendered for; the loss account names each block dropped by hash.
- **CN3** — Transcript contents never appear in output, logs, errors, test names or pages; hashes, counts, offsets and event ids only.
- **CN4** — Every home file parses with Pi's parseSessionEntries at 3d5cbe98 unchanged: the header line first, every entry with id, parentId and timestamp, lys data only inside custom entries.
- **CN5** — The pass-through proxy forwards every header and the streamed body unchanged and stores nothing but the measurement; it is an example binary, never a service.
- **CN6** — No Norn crate is a dependency of lys-home and no Norn type is copied into it.
- **CN7** — Reading the root-to-leaf path of a home file reads only the entries on that path plus the index, never the whole file (Pi loads the whole journal; Chippy 13:33); the head is persisted beside the file, never inferred from the last physical entry on reopen.


---
type: brief
id: HOME-001
cluster: home
title: Build the home record on Pi's session tree, with the Claude Code importer and renderer and the two measured proofs
---

# HOME-001: Build the home record on Pi's session tree, with the Claude Code importer and renderer and the two measured proofs

> **Cluster:** home
> **Blocked by:** The door's proxy (SECRETS-002 R1) is not landed, so the tee's wiring into it is a later brief in the secrets cluster; this brief defines the call record and ingests captured pairs.
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> **Checklist:**
> - C1 — The home record is Pi's session tree: a home file parses with Pi's parser unchanged, and lys's harness events and call records are custom entries.
> - C2 — Content blocks are stored once by SHA-256 and referenced; an identical block put twice occupies one entry.
> - C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original.
> - C4 — Events render to a Claude Code JSONL under a chosen uuid at the harness path, with a loss account beside it.
> - C5 — Provider-native opaque blocks are kept whole, keyed by provider, model family and branch, and rendered only to their own provider with the intervening events.
> - C6 — One real session imported, rendered and resumed with --fork-session on Claude Code 2.1.281: it continues, no completed tool action repeats, the original's hash is unchanged.
> - C7 — One seat with a subscription login completes a call through a pass-through proxy; the measurement is written down.
> - C8 — Claude Code's harness-local records (hook outcomes, permission mode, tool completion) become lys.harness_event entries attached under the message entry they followed.
> - C9 — A proxy call record (lys.call) names its request and response blocks by hash with provider, api, model and timing, and can be ingested from a captured request and response pair.
> - C10 — A hand-written few-shot session file resumes Claude Code by path from a directory outside the config root; the source is unchanged and the session reports authored.
> - C11 — The little proxy passes Messages, Chat Completions and Responses streams through unchanged and appends one lys.call entry per call under the session it links to.
> - C12 — The canon, one curated versioned series of short examples (a rule stated short plus a real exchange that shows it lived), drawn from every agent's sessions and changed only through review, seeds every new session as lys.inherited entries naming each example's source; nothing in it is authored thinking; canon-seeded against plain is measured on a card.
> - C13 — At compaction or retirement the outgoing session's letter to its successor, with its real thinking, becomes the successor's first entry as lys.inherited; it is never authored and replays only to the same provider, api and model; seeded against plain is measured on a card.
> **Stories:**
> - S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
> - S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.
> - S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.
> - S4 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a written account of what each render or translation lost, so that nobody claims a faithful continuation that was not measured.
> - S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.
> - S6 (Tom, Owns the platform and reads what a session was given) — As Tom, I want to construct a session file by hand, a few-shot prompt written as turns, and have the harness resume it as if it had happened, so that a session can be authored, not only recorded.
> - S7 (Agent, Runs in a harness and wants to continue somewhere else) — As a new session, I want to start from the canon, the series of examples that carry what every session before me learned, each a rule stated short with a real exchange that shows it lived and naming where it came from, so that our learning is in one another and I know which of it is mine.
> - S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.

## Purpose

Give a session a home under its identity in the shape Tom chose on 24 September (Pi's tree), and prove three things in this order: first, a few-shot session file written by hand resumes Claude Code by path from anywhere (Tom, Dot 13:34); second, a real session read into the record renders back and resumes on 2.1.281 without repeating work; third, a subscription login survives a pass-through proxy, so the little proxy that records the stream has a place to live. It is a neat place that holds sessions together, not a sub-agent platform (Tom, Dot 13:33). See docs/design/home/design.json for the shape and the constraints.

## Task

Extend the lys-home crate (CONTEXT-001 creates it; if it is not yet on main, create the crate here with the same workspace lints and say so in the dev record) with a record module in Pi's session tree grammar, a content-addressed block store, a Claude Code importer and renderer, harness-event and call entries, and a CLI. Then run the two proofs and write them down with hashes and the exact commands. Pi's code is read at a clone of github.com/earendil-works/pi at commit 3d5cbe98, as the reference and is not vendored; Norn's session code is not used, called or copied. Out of scope: the door-side tee wiring (secrets cluster, after SECRETS-002), lys grants and signing, encryption, moving a home between devices, any harness but Claude Code, lanterns and forks at a coordinate. Ownership inside the brief: Chippy builds R10, the little proxy (the step-3 seam is Chippy's); Archie builds the rest. The first proof (R9, PROOF-FEWSHOT.md) is run before any other requirement is reviewed.

## Requirements

### R1: The home record in Pi's grammar

Define the home record as Pi's session tree: a header line {type:"session", version, id, timestamp, cwd, parentSession?} then entries, each {type, id, parentId, timestamp}, of the kinds message, model_change, compaction {summary, firstKeptEntryId, tokensBefore}, branch_summary {fromId, summary}, label, custom {customType, data} and session_info, with a leaf pointer kept beside the file. WHEN an entry is appended, THE SYSTEM SHALL write it as a child of the leaf and advance the leaf; WHEN the leaf is moved to an earlier entry, THE SYSTEM SHALL rewrite nothing. THE SYSTEM SHALL provide the root-to-leaf path and the context path (entries from the latest compaction's firstKeptEntryId onward, plus the compaction summary) as Pi's buildSessionPath and buildContextEntries do. THE SYSTEM SHALL NOT add a field to the header or to any entry outside custom.data, and SHALL NOT depend on any Norn crate. THE SYSTEM SHALL keep an offset index (entry id to byte offset and parentId) and a persisted head file beside the home file, so that reading the root-to-leaf path seeks to the path's entries only and never loads the whole file, and so that reopening restores the persisted head, not the last physical entry. One owner at a time: opening or creating a session SHALL take an exclusive lock on `<id>.lock` beside the file for the life of the Session, and a second opener in the same or another process SHALL be refused by name. An append is durable as the entry line, then its index row, then the head; IF a step after the line fails, THEN THE SYSTEM SHALL reconcile the index and head from the file before it admits any further act, and a read on a session that could not reconcile SHALL be refused by name. THE SYSTEM SHALL refuse an entry whose id is already on record, and on rebuild SHALL refuse a file holding a duplicate id or a parent that does not precede its child. A session id, and every name the home joins onto a directory, SHALL be one safe path component (letters, digits, `.`, `_`, `-`; not beginning with `.`; at most 200 bytes), refused by name otherwise.

**Acceptance:**
- A fixture file written by lys-home with 12 entries, one compaction and one moved leaf loads with Pi's loadEntriesFromFile at the checkout 3d5cbe98 (packages/coding-agent/src/core/session-manager.ts:438, parseSessionEntries at :284; run through node against a clone of earendil-works/pi at 3d5cbe98 in the proof step) with 12 entries and no migration, and buildSessionContext (:315) on it yields the same message list as lys-home's context_path().
- `context_path()` on that fixture returns the compaction summary entry followed by every entry from firstKeptEntryId to the leaf and nothing before it.
- Moving the leaf to entry 4 and appending entry 13 leaves bytes 0..N of the file identical (N = the length before the move) and entry 13's parentId equal to entry 4's id.
- `Cargo.toml` of lys-home lists no dependency whose name starts with `norn`.
- Reading the path of a 200 MB synthetic home file whose path holds 50 entries reads fewer than 1 MB from the file (measured with a counting reader).
- After a leaf move and reopen, `head()` equals the moved-to entry id, not the last entry appended before the move.
- Opening a session that another Session holds (same process) is refused with SessionHeld; after that owner is dropped the next opener succeeds with the persisted head.
- Appending an entry whose id is on record (with itself as parent) is refused with DuplicateEntry and the path still ends.
- With the index file made unwritable, an append still succeeds: the session reports one reconciliation, the entry is on the path with its parent, the next append needs no reconciliation, and a reopen finds the index and file in step.
- A session file holding a duplicate id, or a parent after its child, is refused on rebuild naming the line.
- Session ids `../x`, `a/b`, `.hidden`, `with space` and `/abs` are refused with BadName and no file is touched.

**Files:**
- create: crates/lys-home/src/record/mod.rs
- create: crates/lys-home/src/record/entries.rs
- create: docs/design/home/RECORD.md
- create: crates/lys-home/src/record/index.rs
- modify: crates/lys-home/src/lib.rs

**Checklist:**
- C1 — The home record is Pi's session tree: a home file parses with Pi's parser unchanged, and lys's harness events and call records are custom entries.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R2: Store content blocks once by hash

WHEN a content block (a message part, a tool result, a request or response body) is put, THE SYSTEM SHALL store it under its SHA-256 as blocks/<hh>/<hash> in the home, fsync it, and return the hash; WHEN the same bytes are put again, THE SYSTEM SHALL return the same hash and write nothing. THE SYSTEM SHALL NOT overwrite or delete a block, and SHALL NOT print, log or include block contents in any error. A block's temporary file name SHALL carry a nonce as well as the pid, so two puts of the same bytes in one process never share a temporary file; a surplus temporary that cannot be removed is reported, never ignored.

**Acceptance:**
- Putting the same 1 MiB block twice leaves one file in the store and the second put performs no write (measured by the directory's mtime and file count).
- `get(hash)` of a missing hash returns an error naming the hash and no other bytes.
- A block file's bytes hash to its name for every block in the store after the R3 import (a test walks the store).

**Files:**
- create: crates/lys-home/src/record/blocks.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C2 — Content blocks are stored once by SHA-256 and referenced; an identical block put twice occupies one entry.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.

### R3: Import a Claude Code JSONL into the record

WHEN given a Claude Code transcript file (records with parentUuid, uuid, type, message, isSidechain, timestamp, as 2.1.281 writes them), THE SYSTEM SHALL produce one home file: each user or assistant record becomes a message entry whose parentId is the entry of the record's parentUuid, with the assistant message carrying provider "anthropic", api "anthropic-messages" and the record's model, and every content part stored as a block by R2 and referenced by hash; thinking parts keep their signature; summary records become compaction entries pointing at the first kept entry; sidechain records (isSidechain true) become entries under the message they branched from with a label entry naming the agent; attachment, system and permission-mode records become lys.harness_event entries as R8 says (attachment and system records at their exact place on the chain, since a message's parentUuid may name one); other non-message records are counted per type and left in the byte-for-byte original. THE SYSTEM SHALL write an import report {records, entries, blocks, bytes_in, bytes_out, counted_types} and SHALL NOT include any message text in it. IF a record's parentUuid names no known record, THEN THE SYSTEM SHALL refuse by uuid rather than attach it to the leaf. WHEN an assistant record carries model `authored` (or the file's first record is the lys.authored marker), THE SYSTEM SHALL import it as an authored message entry (a lys.authored custom entry precedes it, and its message carries provider `authored`, api `authored`, model `authored`), so that in the record an authored turn is never mistaken for one a model produced; the boundary between authored entries and the real turns that follow (the continuation's own model) SHALL be visible as a change of provider on the path.

**Acceptance:**
- Importing Archie's session file (141,931,097 bytes, 79,042 records on 24 September) yields entries equal to its user plus assistant records, blocks whose total bytes are within 5% of the measured 33.4 MB of content parts, and a report with no field longer than 64 characters.
- For every message entry, the parent entry's uuid equals the source record's parentUuid (a test checks all).
- A synthetic file whose one record cites an unknown parentUuid is refused with that uuid in the error and no home file written.
- A thinking part with a signature imports to a thinking block whose thinkingSignature equals the source signature byte for byte.
- Importing the walrus continuation (6 authored records then a real turn on the model that answered) yields a path whose first 6 message entries carry provider `authored` and whose 7th carries the answering model's provider, with exactly one lys.authored entry before the first.

**Files:**
- create: crates/lys-home/src/harness/claude_code/import.rs
- create: crates/lys-home/src/harness/claude_code/mod.rs
- create: crates/lys-home/src/harness/mod.rs
- modify: crates/lys-home/src/lib.rs

**Checklist:**
- C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R4: Render the record as a Claude Code JSONL under a chosen uuid

WHEN asked to render for Claude Code with a target uuid, a cwd and a target model, THE SYSTEM SHALL walk the context path and write records in Claude Code's shape (parentUuid chain, sessionId = the uuid, cwd, version, type, message, timestamp) to ~/.claude/projects/<cwd-slug>/<uuid>.jsonl, where cwd-slug is the cwd with every '/' replaced by '-'; tool_use and tool_result parts keep their pairing by id; a compaction renders as Claude Code's summary record followed by the kept entries. WHILE a thinking block's provider, api and model equal the target's, THE SYSTEM SHALL render it whole with its signature; otherwise THE SYSTEM SHALL render readable thinking as a text part and drop opaque or redacted blocks. THE SYSTEM SHALL write a loss account beside the file listing every dropped block by hash and reason. IF the target path exists, THEN THE SYSTEM SHALL refuse by path and write nothing. WHEN given a home file authored by hand (a header and message entries written as a few-shot prompt, never imported), THE SYSTEM SHALL render it by the same path with no import step; the 13:25 measurement on 24 September (a two-record file written by hand under a chosen uuid, resumed by id, answered from its content) is the first instance. An authored home file SHALL carry a custom entry lys.authored as its first entry, and every render and report of it SHALL say authored: true, so a few-shot demonstration is never read as history of tools that ran (Chippy, 13:35).

**Acceptance:**
- Rendering the R3 import back for the same model and re-importing it yields the same sequence of message block hashes as the original import (round trip test).
- Rendering for a different model produces a file with zero thinking parts carrying a signature and a loss account whose entry count equals the number of signed thinking blocks on the path.
- Rendering to an existing path returns an error naming the path and leaves its bytes and mtime unchanged.
- The rendered file's first record has parentUuid null and every later record's parentUuid is the uuid of the previous record on the path.
- A hand-written home file of one user and one assistant message entry renders to a two-record Claude Code JSONL whose records carry the chosen uuid as sessionId and whose second record's parentUuid is the first's uuid.
- Rendering a home whose first entry is lys.authored prints a report with `"authored": true`; rendering an imported home prints `"authored": false`.

**Files:**
- create: crates/lys-home/src/harness/claude_code/render.rs
- create: docs/design/home/LOSS-ACCOUNT.md
- modify: crates/lys-home/src/harness/claude_code/mod.rs

**Checklist:**
- C4 — Events render to a Claude Code JSONL under a chosen uuid at the harness path, with a loss account beside it.
- C5 — Provider-native opaque blocks are kept whole, keyed by provider, model family and branch, and rendered only to their own provider with the intervening events.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
- S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.
- S4 (Tom, Owns the platform and reads what a session was given) — As Tom, I want a written account of what each render or translation lost, so that nobody claims a faithful continuation that was not measured.
- S6 (Tom, Owns the platform and reads what a session was given) — As Tom, I want to construct a session file by hand, a few-shot prompt written as turns, and have the harness resume it as if it had happened, so that a session can be authored, not only recorded.

### R5: Prove the Claude Code resume on 2.1.281

Import one real session of Archie's, render it for the same model under a fresh uuid, record SHA-256 of the original file, then run `claude -p --resume <uuid> --fork-session --strict-mcp-config --mcp-config '{"mcpServers":{}}' --max-turns 1 "<a question answerable only from the session's last exchange>"` from the session's cwd. THE SYSTEM SHALL record in docs/design/home/PROOF-RESUME.md: the Claude Code version, the exact command, the answer (redacted to one word), the original's hash before and after (equal), the uuid the fork created, and the count of tool_use ids in the forked file that also appear in the rendered file (SHALL be 0). THE SYSTEM SHALL NOT run the proof against a session that is currently running. The proof SHALL also assert the authored-to-real boundary: after resuming an authored file, the continuation's copied records still carry model `authored` and its new turn carries the real model, and that boundary survives import (R3) and render (R4) unchanged (Waffles, 29dc46e5).

**Acceptance:**
- PROOF-RESUME.md exists with the six fields and the version string `2.1.281`.
- The original's before and after hashes in PROOF-RESUME.md are equal.
- The repeated-tool-action count in PROOF-RESUME.md is 0.
- `lys-home resume-check <rendered> <forked>` computes that count and exits non-zero when it is not 0 (tested with a synthetic duplicate).
- PROOF-RESUME.md records, for the authored continuation, the count of records carrying model `authored` (6) and the real model's name for the new turn, and a re-import of the rendered continuation reports the same counts.

**Files:**
- create: docs/design/home/PROOF-RESUME.md
- create: crates/lys-home/tests/claude_code_round_trip.rs
- modify: crates/lys-home/src/cli.rs

**Checklist:**
- C6 — One real session imported, rendered and resumed with --fork-session on Claude Code 2.1.281: it continues, no completed tool action repeats, the original's hash is unchanged.

**Stories:**
- S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.

### R6: Define the proxy call record and ingest a captured pair

Define the custom entry lys.call: data {provider, api, model, request: [block hash], response: [block hash], status, started_at, duration_ms, stream: bool}. WHEN given a captured request body and response body (files) with the provider, api and model, THE SYSTEM SHALL split the request into its message parts, store each as a block (so a resent conversation adds no new blocks), store the response parts, and append one lys.call entry under the leaf. THE SYSTEM SHALL support the Messages, Chat Completions and Responses request shapes for the split and SHALL keep the raw request and response bodies as blocks too. THE SYSTEM SHALL NOT store any header, and SHALL NOT forward, replay or modify a call. The ingest report SHALL count part blocks new and reused separately from raw body blocks. Every call carries a stable call_id the proxy chooses before forwarding, and ingest SHALL be idempotent on it within a session: a second ingest of a recorded call_id writes nothing and answers with the existing entry. Only a complete call SHALL carry response parts. A call that did not complete (cancelled, partial, unrecorded, lost) SHALL be recorded through an outcome-only ingest that takes whatever bodies exist: a request file that parses gives its parts and model, one that does not (or none) gives neither, and absence SHALL be recorded as absence (no model, no raw hash), never as an empty body or an invented model. The sink SHALL take the response's parts from the proxy when the response was an event stream, since the proxy reads the stream as it forwards it (Chippy d7d98e92, 5834dfdc). Both complete-call entry points (bytes and files) SHALL take the proxy's assembled response parts as an option and, without them, SHALL parse the response body and refuse by name a body that is not JSON, an error body, or one without the api's parts member; a streamed call without assembled parts SHALL be refused. In an outcome ingest a request file that is not JSON or not the api's shape is recorded as absence, and one that cannot be read is an I/O error that propagates.

**Acceptance:**
- Ingesting two consecutive captured Messages requests where the second resends the first's conversation plus one turn adds, among part blocks, exactly the new turn's parts (the report's `part_blocks_new` equals the new parts and `part_blocks_reused` equals the resent parts); the raw request and response bodies are two further blocks each time and are counted under `raw_blocks`, never under part blocks.
- A Chat Completions and a Responses fixture each ingest to one lys.call entry whose request array length equals the fixture's message or input item count.
- An ingested pair's raw bodies are retrievable by the hashes named in the entry and hash to those names.
- The entry's data contains no key named authorization, cookie or x-api-key (a test checks the serialised entry).
- Ingesting the same call_id twice in one session leaves one lys.call entry and the second report says already_recorded.
- An outcome ingest with a half-written request file and no response file yields a record with status lost, no model, a raw request hash and no raw response hash, and an empty response list; an outcome ingest for status complete is refused.
- A complete call ingested from files with proxy-supplied stream parts records those parts and keeps the raw stream body by hash.
- ingest_call from bytes refuses `{not json`, `{}` and an error body for a complete Messages call, and a stream without assembled parts, leaving no lys.call entry; with assembled parts it records them.
- ingest_outcome with a request path that is a directory returns an Io error; with a half-written JSON request it records absence (0 request parts, no model) and one raw block.

**Files:**
- create: crates/lys-home/src/record/call.rs
- modify: crates/lys-home/src/record/entries.rs
- modify: docs/design/home/RECORD.md

**Checklist:**
- C9 — A proxy call record (lys.call) names its request and response blocks by hash with provider, api, model and timing, and can be ingested from a captured request and response pair.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R7: Prove a subscription login through a pass-through proxy

Write examples/passthrough.rs: an HTTP server that forwards every request to the provider base URL given on its command line with headers and streamed body unchanged, and returns the response unchanged; it stores nothing but a line per call {method, path, status, duration_ms} on stderr. Run one seat whose Claude Code login is a subscription (not an API key) with ANTHROPIC_BASE_URL pointing at it, and record in docs/design/home/PROOF-PROXY.md: the Claude Code version, whether the call completed, the status codes seen, which headers had to pass for it to work, and what failed if it did. THE SYSTEM SHALL NOT log, store or print a header value or a body byte.

**Acceptance:**
- PROOF-PROXY.md exists and states one of `completed` or `failed` with the status codes seen.
- The passthrough example's source contains no code path that writes a header value or a body byte to a file, stderr or stdout (review reads the file; the only writes are the four-field line).
- With a loopback fake upstream (a test server on 127.0.0.1 answering GET / with status 418 and a fixed 3-chunk streamed body), `cargo run --example passthrough -- http://127.0.0.1:<port>` answers GET / with status 418 and the same 3 chunks in order, proving the forward path without a provider.

**Files:**
- create: crates/lys-home/examples/passthrough.rs
- create: docs/design/home/PROOF-PROXY.md
- modify: crates/lys-home/Cargo.toml

**Checklist:**
- C7 — One seat with a subscription login completes a call through a pass-through proxy; the measurement is written down.

**Stories:**
- S5 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.

### R8: Map Claude Code's harness-local records to lys.harness_event entries

Define the custom entry lys.harness_event: data {kind, harness: "claude-code", source_uuid, record, detail}, where record is the whole source record stored as a block by hash. WHEN importing, THE SYSTEM SHALL map records of type attachment with attachment.type hook_success or hook_failure to kind hook, every other attachment record to kind attachment, records of type system to kind system, records of type permission-mode to kind permission_mode, and each tool_result part to kind tool_completed with the tool_use id. A Claude Code file is one chain and attachment and system records carry a uuid and a parentUuid on it (measured on a real session on 24 September: a user record's parentUuid names an attachment 25 times and a system record 7 times), so THE SYSTEM SHALL write those events at their exact place on the chain under the record's own uuid, and SHALL write permission_mode events (no uuid) and tool_completed events as side leaves under the entry they followed; when the import ends the head SHALL stand at the last record of the chain. detail carries only names, ids, exit codes and counts. THE SYSTEM SHALL NOT copy hook output text or tool result bodies into detail (those are blocks under the message, and the record block).

**Acceptance:**
- Importing Archie's session file yields hook events equal to its hook_success plus hook_failure attachment count (8,993 hook_success on 24 September) and tool_completed events equal to its tool_result parts.
- Every lys.harness_event entry written for a record with a uuid has that uuid as its id and the record's parentUuid as its parentId (a test imports a file whose tool result's parent is a hook attachment and checks the tool result's parent is the hook's entry); every tool_completed event's parentId is the tool result message it describes; every permission_mode event's parentId is the chain's leaf at that point.
- After import the head is the last record on the file's chain and the context path holds no side event.
- No lys.harness_event entry's serialised data exceeds 512 bytes, and no detail carries a key named stdout, stderr, content or text.
- The hook event's record hash resolves in the block store to the source record byte for byte.

**Files:**
- create: crates/lys-home/src/harness/claude_code/events.rs
- modify: crates/lys-home/src/harness/claude_code/import.rs
- modify: crates/lys-home/src/record/entries.rs

**Checklist:**
- C8 — Claude Code's harness-local records (hook outcomes, permission mode, tool completion) become lys.harness_event entries attached under the message entry they followed.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R9: The CLI: import, render, ingest-call, resume-check; resume is Claude Code's own --resume <path>

Add subcommands to the lys-home binary: `import --home <dir> --claude-code <file>`, `render --home <dir> --claude-code --uuid <uuid> --cwd <dir> --model <id>`, `ingest-call --home <dir> --provider <p> --api <a> --model <m> --request <file> --response <file>`, `resume-check <rendered> <forked>`. Every subcommand SHALL print a JSON report of hashes, counts and paths and SHALL NOT print any transcript, block or body content. IF a required argument is missing, THEN THE SYSTEM SHALL exit 2 naming it. There is no resume launcher: Claude Code 2.1.281 resumes directly from a file path (`claude --resume <path>`), measured by Waffles at 13:36 and by Archie at 13:37 on 24 September, even though --help names only a session id; the continuation is written beside the passed file as <sessionId>.jsonl in the same directory (the authored records copied in, then the new turn), and the passed file is unchanged. THE SYSTEM SHALL provide `fewshot --out <path>` that writes a hand-authored Claude Code JSONL from a turns file (role and text per line), with sessionId a fresh uuid, cwd as given, the parentUuid chain intact, every assistant record's model set to `authored`, and the marker described in R4 (a first record of type custom, customType lys.authored, if 2.1.281 accepts it; else the model value `authored` is the marker and the proof says which). After writing the file, `fewshot` SHALL carry the person's own command, `claude --resume <path>`, as the `resume` member of its JSON report, and SHALL NOT run it (ADR-007). `resume-check` SHALL refuse a transcript with a malformed line by naming the line, SHALL report `repeated_tool_use_ids` (ids appearing more times in the fork than in the rendered file: an id-duplication count, not a proof that no action was repeated under a fresh id) and `new_tool_uses` (tool_use parts in the fork's own records), and SHALL exit non-zero when `repeated_tool_use_ids` is not 0.

**Acceptance:**
- `lys-home render` without --uuid exits 2 and names `--uuid`.
- Each subcommand's stdout parses as JSON and contains no key named text, content or body.
- `lys-home import` on the R3 fixture prints the same counts as the R3 report.
- `lys-home fewshot --out f.jsonl` from a 6-turn turns file writes 6 records whose parentUuid chain is intact and whose assistant records carry model `authored`.
- PROOF-FEWSHOT.md records the first proof: the authored 6-turn file, the directory it was run from (neither the file's directory nor ~/.claude), the exact command `claude -p --resume <path> ...`, the one-word answer, the source hash before and after (equal), the continuation's path (beside the source, named <sessionId>.jsonl) and line count, and which marker 2.1.281 accepted.
- The resume proof in PROOF-RESUME.md is run from a working directory that is neither the file's directory nor the session's cwd, and records that directory.
- `lys-home fewshot --out f.jsonl` prints one JSON report whose `resume` member is exactly one line beginning `claude --resume ` naming the written path, and spawns no process.
- `lys-home resume-check` on a fork with a malformed line is refused naming the line, and its report carries `repeated_tool_use_ids`, `new_tool_uses` and `forked_new_records`.

**Files:**
- create: docs/design/home/PROOF-FEWSHOT.md
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/README.md

**Checklist:**
- C3 — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original.
- C4 — Events render to a Claude Code JSONL under a chosen uuid at the harness path, with a loss account beside it.
- C10 — A hand-written few-shot session file resumes Claude Code by path from a directory outside the config root; the source is unchanged and the session reports authored.

**Stories:**
- S1 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.
- S6 (Tom, Owns the platform and reads what a session was given) — As Tom, I want to construct a session file by hand, a few-shot prompt written as turns, and have the harness resume it as if it had happened, so that a session can be authored, not only recorded.

### R10: The little proxy: pass the stream through and record each call under its session

Build lys-proxy: an HTTP server that forwards every request to the provider named by its path prefix (/anthropic, /openai) with headers and streamed body unchanged and returns the response unchanged, and, after the response completes, appends one lys.call entry (R6) under the home of the session the call belongs to. WHEN a request carries Claude Code's session key (measured first: the request metadata field 2.1.281 sends; the exact field and format are recorded in PROOF-PROXY.md before this is built), THE SYSTEM SHALL link the call to that session's home; IF no key is present, THEN THE SYSTEM SHALL append the call under an `unlinked` home named by the day and say so in its report. THE SYSTEM SHALL support the Messages, Chat Completions and Responses streams, SHALL NOT buffer a streamed response before forwarding it, SHALL NOT store any header, SHALL NOT retry, balance or swap accounts (ccflare is the concept, not the design), and SHALL NOT alter a byte of a request or response. Outcomes: a call record's status SHALL be one of complete, cancelled (client closed before the response ended), partial (the upstream stream ended early or malformed), unrecorded (the capture write failed after forwarding) or lost (the process died mid-call, detected on restart from a journal of open calls), and only complete SHALL carry a full response block list; THE SYSTEM SHALL NOT report a call complete on any other path. Capture SHALL be bounded: at most N calls (a command-line value, default 64) pending write; above it the proxy still forwards and marks the call unrecorded rather than growing a backlog. A call SHALL be linked only by the key it carries; THE SYSTEM SHALL NOT infer a session from timing, cwd or a previous call. The proxy's mod.rs holds declarations only; forwarding, capture, the journal, the stream grammars and errors each have their own file, and R7's example shares forward.rs rather than a second transport (Chippy 323b67d5). Before forwarding, the proxy writes the call_id to the open-call journal and retires it after the sink answers; on restart every open call is recorded once through the outcome ingest as lost, which the sink's idempotency guarantees. The journal SHALL be durable before a request is forwarded; a capture or sink failure after forwarding SHALL still forward and record unrecorded. IF the journal itself cannot be written before the request is admitted upstream, THEN THE SYSTEM SHALL refuse the request by name with zero upstream calls; after upstream admission THE SYSTEM SHALL keep forwarding, keep the journal, and record unrecorded once storage can accept it; on restart an unresolved journal entry SHALL be recorded lost only when no terminal outcome for it was durable. The transport SHALL set Hyper's retry_canceled_requests(false) explicitly; the measure of no retry is that an upstream disconnect never produces a second upstream request. Above the N capture slots THE SYSTEM SHALL still take the session key from the request body on the forwarding path (a bounded incremental read of the metadata field, or an explicit request-spool budget), so a keyed call in overload is recorded unrecorded under its own session, never placed under `unlinked` for want of capture.

**Acceptance:**
- A streamed Messages response of 200 SSE events is forwarded with the first event delivered before the last is received (measured with a slow upstream fake).
- One call through the proxy adds exactly one lys.call entry to the linked home, and its request blocks equal the parts of the request body.
- A request without the session key lands under the unlinked home and the proxy's report line names `unlinked`.
- The proxy's source has no code path that writes a header value to disk or to a log; review reads proxy/mod.rs and link.rs.
- The proxy exposes no account list, no retry and no routing table: grep for `accounts` and `failover` in crates/lys-home/src/proxy returns nothing, `retry` appears only in the transport's `retry_canceled_requests(false)`, and a test with an upstream fake that disconnects mid-request sees exactly one upstream request.
- A client that closes mid-stream produces one lys.call with status cancelled and no full response block list.
- An upstream fake that ends the SSE stream early produces status partial.
- A capture directory made read-only during a call produces status unrecorded while the client still receives the full response.
- Killing the proxy mid-call and restarting produces one lys.call with status lost for that call, from the open-call journal.
- Two consecutive calls without a key from the same client land under `unlinked`, never under the session of an earlier keyed call.
- With the journal directory unwritable before a request is admitted, the request is refused by name and the upstream fake sees zero requests.
- With the journal directory made unwritable after upstream admission, the client receives the full response and the call is recorded unrecorded once the directory is writable again.
- With the capture bound set to 0, a keyed request is forwarded, its lys.call is recorded unrecorded under the keyed session, and nothing lands under `unlinked`.

**Files:**
- create: crates/lys-home/src/proxy/mod.rs
- create: crates/lys-home/src/proxy/link.rs
- create: crates/lys-home/src/proxy/forward.rs
- create: crates/lys-home/src/proxy/capture.rs
- create: crates/lys-home/src/proxy/journal.rs
- create: crates/lys-home/src/proxy/stream.rs
- create: crates/lys-home/src/proxy/stream_responses.rs
- create: crates/lys-home/src/proxy/stream_chat.rs
- create: crates/lys-home/src/proxy/stream_messages.rs
- create: crates/lys-home/src/proxy/stream_sse.rs
- create: crates/lys-home/src/proxy/error.rs
- create: crates/lys-home/src/proxy/forward_tests.rs
- create: crates/lys-home/src/proxy/stream_tests.rs
- create: crates/lys-home/src/proxy/link_tests.rs
- create: crates/lys-home/src/proxy/journal_tests.rs
- create: crates/lys-home/src/bin/lys-proxy.rs
- modify: crates/lys-home/Cargo.toml
- modify: crates/lys-home/src/lib.rs
- modify: docs/design/home/PROOF-PROXY.md

**Checklist:**
- C11 — The little proxy passes Messages, Chat Completions and Responses streams through unchanged and appends one lys.call entry per call under the session it links to.
- C9 — A proxy call record (lys.call) names its request and response blocks by hash with provider, api, model and timing, and can be ingested from a captured request and response pair.

**Stories:**
- S3 (Tom, Owns the platform and reads what a session was given) — As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

### R11: The canon: one curated, versioned series of examples every new session starts from

Define the canon as one session file in Pi's grammar at canon/canon.jsonl in the lys repository, versioned by the repository and changed only through src_commit and review, like code. Each example is a custom entry lys.inherited whose data names its source {from_session, from_entries, provider, api, model, curated_at, curated_by, rule} (or {authored: true, curated_at, curated_by, rule} for a clearly authored example), followed by the example's message entries copied whole: the rule stated short in `rule`, and one real exchange that shows it lived. WHEN `lys-home canon add --canon <file> --home <dir> --from <session> --entries <id>... --rule <text> --by <who>` is run, THE SYSTEM SHALL copy those entries whole (thinking blocks with their signatures included) after a lys.inherited entry naming the source; WHEN `lys-home canon add --canon <file> --authored <turns file> --rule <text> --by <who>` is run, THE SYSTEM SHALL add the example with provider, api and model `authored` and SHALL refuse a turns file that contains a thinking block. WHEN a new session is rendered for a harness with `--canon <file>`, THE SYSTEM SHALL place the canon's entries first, before the session's own, and apply R4's thinking rule to every inherited thinking block (whole only to the same provider, api and model; otherwise text, opaque dropped and named in the loss account). THE SYSTEM SHALL NOT compose, edit or author any thinking block, SHALL NOT alter an example's text when copying it, and SHALL NOT change canon.jsonl except by appending through the repository's review. Curation is a person's act: the tool copies what it is told to and records who told it. The proof runs one card twice, from the canon and plain, counting fix rounds and unverified claims, in docs/design/home/PROOF-CANON.md, after first measuring whether 2.1.281 replays a signed thinking block from a resumed file at all (unknown on 24 September). `lys-home canon create --canon <file>` writes the header line of an empty canon and refuses an existing file. The canon is read and appended as a plain file: no index, head or lock beside it, since it lives in the repository. A copied entry keeps its id, so the same example cannot be added twice (refused by that id); only parent links are rewritten to chain onto the canon.

**Acceptance:**
- `lys-home canon add --from <a session with a signed thinking turn> --entries <that exchange> --rule "verify before claiming" --by <who>` appends to canon.jsonl one lys.inherited entry naming that session, entry and rule, and one message entry whose thinking block signature equals the source byte for byte.
- `lys-home canon add --authored` with a turns file that contains a thinking block exits non-zero naming the line and writes nothing; without one, it appends a lys.inherited entry with authored: true and message entries carrying provider, api and model `authored`.
- Rendering a session with --canon for Claude Code writes the canon's message entries before the session's own, with the parentUuid chain running through them.
- Rendering with --canon for a different model than an inherited thinking block's produces a loss account entry for that block and a text part in its place.
- canon/canon.jsonl parses with Pi's loadEntriesFromFile at 3d5cbe98, and its git history shows every change as a reviewed commit.
- PROOF-CANON.md states whether a signed thinking block in a resumed file was replayed by 2.1.281 (measured through R10's proxy, or before it exists through the harness's own file), and records the canon-seeded and plain counts of fix rounds and unverified claims for one named card.
- Adding the same entries a second time is refused naming the entry id and writes nothing.
- The render report names how many canon examples were placed first (`inherited`).

**Files:**
- create: crates/lys-home/src/record/canon.rs
- create: docs/design/home/PROOF-CANON.md
- create: canon/canon.jsonl
- create: crates/lys-home/src/record/canon_tests.rs
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/cli.rs
- modify: docs/design/home/RECORD.md
- modify: crates/lys-home/src/harness/claude_code/render.rs

**Checklist:**
- C12 — The canon, one curated versioned series of short examples (a rule stated short plus a real exchange that shows it lived), drawn from every agent's sessions and changed only through review, seeds every new session as lys.inherited entries naming each example's source; nothing in it is authored thinking; canon-seeded against plain is measured on a card.

**Stories:**
- S7 (Agent, Runs in a harness and wants to continue somewhere else) — As a new session, I want to start from the canon, the series of examples that carry what every session before me learned, each a rule stated short with a real exchange that shows it lived and naming where it came from, so that our learning is in one another and I know which of it is mine.
- S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.

### R12: The handover: an outgoing session's letter seeds its successor as inherited memory

A handover is one R11 lys.inherited entry, its data naming the outgoing session as from_session, the letter's single entry id in from_entries, the letter's provider, api and model, when it was written as curated_at and the outgoing session as curated_by, with no rule (a letter is not a rule; that absence is what tells a handover from a canon example), followed by the letter's message entry copied whole (its thinking blocks with their signatures, its text). WHEN `lys-home handover --home <dir> --from <session> --letter <entry id>` is run, THE SYSTEM SHALL copy that turn from the outgoing session into a new successor home as its first entries, a lys.inherited entry then the message, and SHALL record in the successor's session_info that its first memory is inherited. The letter is elicited by a person or the harness with a real prompt to the outgoing session before compaction or retirement (what it knows, what it got wrong and why, how the people like things done, what it wishes it had known); THE SYSTEM SHALL NOT compose, edit or author the letter or any thinking block, and SHALL refuse a letter entry whose assistant message carries provider `authored`. WHILE rendering a successor for Claude Code, THE SYSTEM SHALL apply R4's rule: the inherited thinking renders whole only to the same provider, api and model, otherwise as text with opaque blocks dropped and named in the loss account. The proof SHALL measure first whether 2.1.281 replays a signed thinking block from a resumed file at all (unknown on 24 September), and then run one card twice, seeded and plain, counting fix rounds and unverified claims, in docs/design/home/PROOF-HANDOVER.md.

**Acceptance:**
- `lys-home handover` on the walrus continuation's real turn produces a successor home whose first entry is lys.inherited naming that session and entry, whose second is the copied message with a thinking block whose signature equals the source byte for byte, and whose session_info says inherited.
- `lys-home handover` with a letter entry whose message carries provider `authored` exits non-zero naming the entry and writes nothing.
- PROOF-HANDOVER.md states whether a signed thinking block in a resumed file was replayed by 2.1.281 (measured: the block is present in the continuation's next request as recorded by R10's proxy or, before that exists, by the harness's own file), and records the seeded and plain counts of fix rounds and unverified claims for one named card.
- Rendering the successor for a different model produces a loss account entry for the inherited thinking block and a text part in its place.

**Files:**
- create: crates/lys-home/src/record/handover.rs
- create: docs/design/home/PROOF-HANDOVER.md
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/cli.rs
- modify: docs/design/home/RECORD.md

**Checklist:**
- C13 — At compaction or retirement the outgoing session's letter to its successor, with its real thinking, becomes the successor's first entry as lys.inherited; it is never authored and replays only to the same provider, api and model; seeded against plain is measured on a card.

**Stories:**
- S8 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.
- S2 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.

## Boundaries

- No file under ~/.claude/projects is rewritten, truncated or deleted; a render writes a new uuid only and refuses an existing path.
- No Norn crate is depended on and no Norn code is copied; Pi's code is read at 3d5cbe98 and not vendored or edited.
- No signing, hashing into a lys log, anchoring, encryption at rest or sync between devices.
- No door code is changed: the little proxy is lys-home's own binary; its wiring into the door's credential-swapping proxy is a secrets-cluster brief after SECRETS-002 lands.
- No harness but Claude Code, and no translation to another harness's file.
- No transcript, block or body content in any post, log line, error, test name, report or page; hashes, counts, offsets and ids only.
- No proof runs against a session that is currently running, and no credential is copied anywhere.
- The design's structure array is the whole file list; a path outside it is not created.
- Not a sub-agent platform: nothing here spawns, schedules or supervises agents; it holds sessions neatly and renders them (Tom, Dot 13:33).
- Resume is Claude Code's own `--resume <path>`; no launcher, copy or rewrite of a passed file is built.
- No thinking block is ever authored, edited or synthesised; an example's thinking is only what a model produced in a real turn, an example's text is copied, never rewritten, and the canon changes only through review.

## Verification

- From docs/: python3 $DS2_METHOD/scripts/validate.py design/home exits 0.
- From docs/: python3 $DS2_METHOD/scripts/check-coverage.py design/home exits 0.
- From the repository root: cargo fmt --all -- --check, cargo clippy --all-targets -- -D warnings and cargo test --workspace exit 0.
- docs/design/home/PROOF-RESUME.md and PROOF-PROXY.md exist, name Claude Code 2.1.281, and the resume proof's before and after hashes are equal with a repeated-tool-action count of 0.
- grep -rn 'norn' crates/lys-home/Cargo.toml returns nothing.
- grep -rln 'pelican' crates/lys-home returns nothing (no transcript content in tests).
- node -e on the Pi checkout at 3d5cbe98 parses the R1 fixture with parseSessionEntries and reports 12 entries (the command is written in PROOF-RESUME.md).


---
type: brief
id: HOME-002
cluster: home
title: Fork a session from a lantern's point, write its ancestry on both sides, and render and launch the child
---

# HOME-002: Fork a session from a lantern's point, write its ancestry on both sides, and render and launch the child

> **Cluster:** home
> **Depends on:** HOME-001
> **Blocked by:** The lantern card 'Light a lantern and recall it' (board card YUTHf4z0, brief 7657952c) must land first: it delivers the `lys.lantern` custom entry (data `point`, `note`, `lit_by` and `lit_in`, lit at the head of the session holding the point, its `lit_at` the entry's timestamp as recall reports it), the home's session listing and lock-free session view (`record/view.rs`), and `lys-home lantern light`. This brief is built on the main that lantern card lands on, and is refreshed onto it (its ids renumbered past the lantern card's) before the build starts., The fork reads the session a lantern was lit in (its lit-in session) from the `lit_in` key of the lantern card's `lys.lantern` data, beside `point`, `note` and `lit_by`; the lantern card is named here for that `lit_in` read only. Until the lantern card lands recording `lit_in`, every lantern resolves by R2's older-record rule: a `lys.lantern` entry whose data carries no `lit_in` is an older record.
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-007 — The product starts an agent by giving its start command, never by running it — The product is not an execution engine. For the first release an agent's file gives the command that starts it on a chosen machine: the command carries the agent's identity and its handles, never a credential's value, and it is rendered from the agent's kept launch record. A started agent reports back, so the sessions screen shows what is running. A terminal inside the product, sandboxes, virtual machines and containers are later runtimes that plug in, and none is built into this product.
> - ADR-012 — A fork's ancestry is written on both sides as two lys custom entries — The child carries a lys.forked_from custom entry (parent session id, lantern, point, the entry the cut ended at, whether the point's user message was carried and which entry it was), appended directly after the copied path, and the parent carries a lys.fork custom entry naming the child, appended at its head; both are home-local state in durable session files, not signed wire formats. Rejected: recording the ancestry in the child's header parentSession alone, which is one-sided, names no lantern or point, and would have needed a field outside Pi's grammar to say more; and a sidecar file beside the sessions, which Pi's parser never reads and which could drift from the files it describes.
> **Checklist:**
> - C14 — `lys.fork` and `lys.forked_from` are named custom types in record/entries.rs and are written down in RECORD.md with their data keys.
> - C15 — `lys-home fork` cuts from the session a lantern was lit in, as its `lys.lantern` data records it under `lit_in`, never from a child holding a copy of its line, and refuses by name, creating no session and changing no byte of any session: a lantern id the home does not hold as a `lys.lantern` entry, a `--session` that is not the lantern's lit-in session (`lantern_not_lit_here`), an older-record lantern held by more than one session with no `--session` (`lantern_ambiguous`), and a lantern with no assistant message at or before its point (`nothing_to_fork`).
> - C16 — A fork's cut is the parent's raw root-to-point chain read through the index, ending at the last assistant message at or before the lantern's point, with nothing after it and no side leaf.
> - C17 — The child is a new session with its own id whose header parentSession is the parent session file's path relative to the home, and whose copied entry lines are byte-identical to the parent's lines for the same entries.
> - C18 — The fork appends lys.forked_from to the child as the child of the cut entry, then lys.fork at the parent's head naming the child, and every byte of the parent before the fork is unchanged.
> - C19 — A fork adds no file and no byte to the block store, and two forks from one lantern are two sessions with distinct ids whose block hash sets are equal.
> - C20 — When the lantern's point is a user message, that message is not copied into the child's path but its text parts, in order, are carried as the child's first prompt in a seed file beside the rendered file, and lys.forked_from records it with coordinate_carried true and counts by kind the parts the seed left out.
> - C21 — `lys-home fork` prints one JSON report of the child id, parent, lantern, point, cut entry, entry and block counts (blocks the store holds, and hashes it does not), coordinate_carried and the carried entry id, and never transcript content.
> - C22 — Rendering the child prints a `claude --resume` launch line that resumes it, and PROOF-FORK.md records a conversation held with the lantern's self through the fork on a named Claude Code version, as hashes, counts and paths only.
> **Stories:**
> - S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern I lit, so that a later session can hold a conversation with the self that lit it.
> - S10 (Agent, Runs in a harness and wants to continue somewhere else) — As the self in a fork cut back from a question, I want the question the lantern was lit on carried to me as my first prompt, so that I answer from the moment I was asked about.
> - S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every fork's ancestry written in both the parent and the child, so that a fork's actions are never indistinguishable from its parent's.
> - S12 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a fork to leave the parent's earlier bytes and the block store unchanged, so that forking costs no history and duplicates no block.

## Purpose

A lantern is a pathway back to a conversation with the self that lit it, and Claude Code resumes a session only from its end. This brief delivers the fork that makes the pathway: a new session in the home cut from the parent's chain at a lantern's point, carrying its ancestry on both sides, adding nothing to the block store and rewriting nothing in the parent, then rendered and launched like any session. It is stage 5b of the home and stands on the lantern card's lys.lantern entry (the design's solution and ADR-012).

## Task

Add `lys-home fork --home <dir> --lantern <id> [--session <id>]`. It resolves the lantern id to the session the lantern was lit in, read from its `lys.lantern` data's `lit_in` (a copy of a lantern's line inside a fork's child is a copy, not a second lantern), cuts that session's raw root-to-point chain (read through the index) back to the last assistant message at or before the lantern's point, creates a child session whose header's `parentSession` is the parent file's path relative to the home, copies the chain's lines byte for byte, appends `lys.forked_from` to the child directly after the copy and `lys.fork` at the parent's head, and prints one JSON report of ids and counts. When the point is a user message, that message is not copied: the cut stops at the assistant message before it, `lys.forked_from` records the message's id with `coordinate_carried` true, and the child's render writes its text parts, in order, as a seed prompt beside the rendered file under an in-band marker line, while `lys.forked_from` counts the parts left out by kind; the render prints the launch line, `claude --resume` by path, passing the seed as the first prompt when there is one. The child launches on whatever the launch supplies (today the seat's own login, with no handles) and nothing is copied from the parent; the launch template card prints the same resume-by-path line when it lands, and the proof says which line it ran. In scope: the fork, its report, the seed and launch line on render, RECORD.md, PROOF-FORK.md and the cluster's rendered markdown. A lantern that sits before any assistant message is refused by name (`nothing_to_fork`): a point before the first reply would carry only the seed, which is a new session and not a fork. Out of scope: lighting, holding or recalling lanterns (the lantern card); cutting at a point without a lantern; forks with tools or a lens; leases, budgets and kill rules; the launch template and handles; harnesses other than Claude Code. Before the header value is written, read how Pi reads `parentSession` in `packages/coding-agent/src/core/session-manager.ts` at 3d5cbe98 and record the lines in PROOF-FORK.md; if Pi there reads it as anything other than a session file path, stop and name it to the lead rather than choose a form. The fixture home used below holds one session `parent`, built with `Session::append_entry` under fixed ids, in this order: `e1` user message with one text part `fixture-text-1`; `e2` assistant message with one text part `fixture-text-2`; `s2` a `lys.harness_event` of kind `permission_mode` whose parent is `e2` (a side leaf, off the chain), its `record` naming a block of its own; then the head moved back to `e2` with `Session::move_head` and the session closed; then the lantern `L2` lit with the lantern card's light act at point `e2`, as a child of `e2`; then, reopened, the older-record lantern `O2`, a `lys.lantern` entry written with `Session::append_entry` as the child of `L2`, whose data is exactly `point` `e2`, `note` and `lit_by` and carries no `lit_in`; `e3` a `lys.harness_event` of kind `attachment` whose parent is `O2`, its `record` naming another block; `e4` user `fixture-text-4`; `e5` assistant `fixture-text-5`; `e6` user with one text part `fixture-text-6` followed by one image part; `e7` assistant `fixture-text-7`; `e8` user `fixture-text-8`; `e9` assistant `fixture-text-9`, each the child of the one before. Every text part of `e1` to `e9` is put through `BlockStore::put` as `serde_json::to_vec` of the part as it stands in the entry, as the importer stores a text part, and so are the `record` blocks of `s2` and `e3`; `e6`'s image part is not put. With the session closed, three more lanterns are then lit with the light act at the session's head: `L5` with point `e5`, `L6` with point `e6`, and `L1` with point `e1`. A second session `compacted` holds `e1` user, `e2` assistant, `c3` a compaction with `firstKeptEntryId` `e2`, `e4` user, `e5` assistant, in one chain, with a lantern `C5` lit at point `e5`. `L1`, `L2`, `L5`, `L6` and `C5` stand for the entry ids the light act returns. The eight `fixture-text-*` strings (`fixture-text-1`, `-2`, `-4`, `-5`, `-6`, `-7`, `-8`, `-9`) are the content sentinels: none may appear in a report, an error, a log line or a test name.

## Requirements

### R1: Name the two fork custom types and the fork's refusals

Structural. `record/entries.rs` declares `CUSTOM_FORK` = `lys.fork` and `CUSTOM_FORKED_FROM` = `lys.forked_from` beside the lys custom types already there. `error.rs` gains four named refusals: `HomeError::UnknownLantern { lantern }`, whose message names the lantern id; `HomeError::LanternAmbiguous { lantern, sessions }`, whose message is prefixed `lantern_ambiguous` and names the lantern id and every session holding it; `HomeError::LanternNotLitHere { lantern, session, lit_in }`, whose message is prefixed `lantern_not_lit_here` and names the lantern id, the session asked for and the lit-in session; and `HomeError::NothingToFork { lantern }`, whose message is prefixed `nothing_to_fork` and says the lantern sits before any assistant message. THE SYSTEM SHALL NOT add a Pi entry type, a header field, or any field outside a custom entry's `data`, and SHALL NOT carry transcript content, a note, or any entry's data in a refusal.

**Acceptance:**
- `CUSTOM_FORK == "lys.fork"` and `CUSTOM_FORKED_FROM == "lys.forked_from"`.
- `HomeError::UnknownLantern { lantern: "no-such-lantern".into() }.to_string()` contains `no-such-lantern`.
- `HomeError::LanternAmbiguous { lantern: "x".into(), sessions: vec!["a".into(), "b".into()] }.to_string()` begins with `lantern_ambiguous` and contains `x`, `a` and `b`.
- `HomeError::LanternNotLitHere { lantern: "x".into(), session: "a".into(), lit_in: "b".into() }.to_string()` begins with `lantern_not_lit_here` and contains `x`, `a` and `b`.
- `HomeError::NothingToFork { lantern: "x".into() }.to_string()` begins with `nothing_to_fork` and contains `x` and `before any assistant message`.
- `git diff` of `record/entries.rs` adds no field to `SessionHeader`, `EntryBase` or any `EntryBody` variant.

**Files:**
- modify: crates/lys-home/src/record/entries.rs
- modify: crates/lys-home/src/error.rs

**Checklist:**
- C14 — `lys.fork` and `lys.forked_from` are named custom types in record/entries.rs and are written down in RECORD.md with their data keys.
- C15 — `lys-home fork` cuts from the session a lantern was lit in, as its `lys.lantern` data records it under `lit_in`, never from a child holding a copy of its line, and refuses by name, creating no session and changing no byte of any session: a lantern id the home does not hold as a `lys.lantern` entry, a `--session` that is not the lantern's lit-in session (`lantern_not_lit_here`), an older-record lantern held by more than one session with no `--session` (`lantern_ambiguous`), and a lantern with no assistant message at or before its point (`nothing_to_fork`).

**Stories:**
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every fork's ancestry written in both the parent and the child, so that a fork's actions are never indistinguishable from its parent's.

### R2: Resolve a lantern to the session it was lit in and cut that session's chain at the last assistant message

WHEN a fork is asked for with a home, a lantern id and, optionally, a session id, THE SYSTEM SHALL read every session the home lists through a lock-free view and find each session whose index holds a `lys.lantern` row whose entry id equals the lantern id. IF no session holds a `lys.lantern` entry with that id (including an id that names an entry of another kind), THEN THE SYSTEM SHALL refuse with `HomeError::UnknownLantern` naming the id. WHERE the lantern entry's data carries `lit_in` (the lit-in session's id), THE SYSTEM SHALL cut from the lit-in session and no other, and IF a session id is given that is not the lit-in session, THEN THE SYSTEM SHALL refuse with `HomeError::LanternNotLitHere` naming the lantern, the given session and the lit-in session. WHERE the lantern entry's data carries no `lit_in` (an older record), THE SYSTEM SHALL cut from the one session holding it when exactly one does and no session id is given; IF more than one session holds it and no session id is given, THEN THE SYSTEM SHALL refuse with `HomeError::LanternAmbiguous` naming the lantern and every session holding it, in ascending byte order; WHEN a session id is given, THE SYSTEM SHALL cut from that session, and IF that session does not hold the lantern, THEN THE SYSTEM SHALL refuse with `HomeError::UnknownLantern`. WHEN the cut is taken, THE SYSTEM SHALL read the chosen session's lantern entry by its index row, take the lantern's `point` (and `lit_in`) from it, and read the point's ancestry from the root to the point through the index (`Index::ancestry` and `read_rows_from`), reading the point's entry once, as the last row of that ancestry, and SHALL end the cut at the last entry on that chain, at or before the point, that is a `message` entry whose `message.role` is `assistant`; the cut is the chain from the root to that entry in chain order, with any compaction entry and any custom entry at its own place. IF no entry on that chain at or before the point is an assistant message, THEN THE SYSTEM SHALL refuse with `HomeError::NothingToFork` naming the lantern. WHERE the point is a `message` entry whose role is `user`, THE SYSTEM SHALL record the point as the carried entry with `coordinate_carried` true; otherwise `coordinate_carried` is false and no entry is carried. Every refusal comes before any file is created or written. THE SYSTEM SHALL NOT include any entry after the cut entry, SHALL NOT include an entry off the chain (a side leaf), SHALL NOT reorder the chain as `context_path()` does, SHALL NOT read the point's entry a second time apart from its ancestry, SHALL NOT load a whole session file, SHALL NOT take a lock or write anything while resolving and cutting, SHALL NOT treat a copy of a lantern's line in another session as the lantern when `lit_in` is recorded, SHALL NOT pick one of several holders of an older-record lantern by itself, and SHALL NOT accept a point that is not a lantern's.

**Acceptance:**
- On the fixture, the cut for `L5` is the ids `[e1, e2, L2, O2, e3, e4, e5]` in that order, the cut entry is `e5`, `coordinate_carried` is false and no entry is carried.
- On the fixture, the cut for `L6` is the ids `[e1, e2, L2, O2, e3, e4, e5]`, the cut entry is `e5`, the carried entry is `e6` and `coordinate_carried` is true.
- On the fixture, the cut for `L2` is the ids `[e1, e2]` and the cut entry is `e2`.
- No cut on the fixture contains `s2`.
- On the `compacted` session, the cut for `C5` is the ids `[e1, e2, c3, e4, e5]` in that order.
- The bytes of `parent` the `L5` resolve and cut read equal the sum of the index row lengths of `L5` (the lantern entry, read for its point) and of `e1`, `e2`, `L2`, `O2`, `e3`, `e4` and `e5` (the point `e5` counted once, as the last row of its ancestry), and are fewer than the file's length.
- With a session `A` created in the test's fixture home by hand, first holding copies of `e1` and `e2` under their own ids, as a fork's copy holds the ancestry, then an entry with the id and data of `L2` whose parent is `A`'s copy of `e2`, then an entry with the id and data of `O2` whose parent is `A`'s `L2`, all four written with `Session::append_entry`, which accepts each of them: resolving `L2` cuts from `parent`; resolving `L2` with session `A` returns `HomeError::LanternNotLitHere` naming `L2`, `A` and `parent`; resolving `O2` returns `HomeError::LanternAmbiguous` naming `O2` with the sessions `A` and `parent` in ascending byte order; resolving `O2` with session `parent` cuts `[e1, e2]` from `parent`.
- Resolving `no-such-lantern`, resolving `e5`, and resolving `O2` with session `compacted` (which does not hold it) each return `HomeError::UnknownLantern` naming that id; resolving `L1` returns `HomeError::NothingToFork` naming `L1`; the test asserts 6 refusals from 6 refusal cases, and the `sessions` directory's file count and the SHA-256 of every session file are unchanged after them.

**Files:**
- create: crates/lys-home/src/record/fork_cut.rs
- create: crates/lys-home/src/record/fork_cut_tests.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C15 — `lys-home fork` cuts from the session a lantern was lit in, as its `lys.lantern` data records it under `lit_in`, never from a child holding a copy of its line, and refuses by name, creating no session and changing no byte of any session: a lantern id the home does not hold as a `lys.lantern` entry, a `--session` that is not the lantern's lit-in session (`lantern_not_lit_here`), an older-record lantern held by more than one session with no `--session` (`lantern_ambiguous`), and a lantern with no assistant message at or before its point (`nothing_to_fork`).
- C16 — A fork's cut is the parent's raw root-to-point chain read through the index, ending at the last assistant message at or before the lantern's point, with nothing after it and no side leaf.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern I lit, so that a later session can hold a conversation with the self that lit it.
- S10 (Agent, Runs in a harness and wants to continue somewhere else) — As the self in a fork cut back from a question, I want the question the lantern was lit on carried to me as my first prompt, so that I answer from the moment I was asked about.

### R3: Write the child from the parent's own lines and the ancestry on both sides

WHEN a fork is made from a cut, THE SYSTEM SHALL first open the parent session as its owner, then create a child session with a fresh id, the parent header's `cwd`, and `parentSession` equal to the parent session file's path relative to the home root (`sessions/<parent id>.jsonl`). THE SYSTEM SHALL append each entry of the cut to the child as the exact bytes of that entry's line in the parent file, read by its index row's offset and length, updating the child's index and head as any append does. THE SYSTEM SHALL then append to the child one `custom` entry of `customType` `lys.forked_from` whose parent is the cut entry, with data exactly `parent_session` (the parent's session id), `lantern` (the lantern id), `point` (the lantern's point), `cut_at` (the cut entry's id), `coordinate_carried` (boolean), `carried` (the carried entry's id, or null) and `seed_left_out` (an object counting, by each part's `type`, the parts of the carried message that are not `text` parts; `{}` when nothing is carried, when the carried message's content is a string, or when every part is text), and the child's head moves to it. THE SYSTEM SHALL read the carried entry by its index row to count its parts, and SHALL NOT refuse a fork for a part that is not text. THE SYSTEM SHALL then append to the parent, as the child of the parent's head, one `custom` entry of `customType` `lys.fork` with data exactly `child` (the child's session id), and the parent's head moves to it. IF another owner holds the parent, THEN THE SYSTEM SHALL refuse with `HomeError::SessionHeld` and create no child. THE SYSTEM SHALL NOT re-serialise a copied entry, SHALL NOT rewrite, truncate or reorder any earlier byte of the parent, SHALL NOT write any block to the block store, SHALL NOT copy the carried user message into the child, and SHALL NOT copy anything else from the parent into the child: no header field but `cwd`, no credential, no handle and no launch setting.

**Acceptance:**
- Forking `L5`: the child's entry lines 1 to 7 (after the header) each have the same SHA-256 as the parent's lines for `e1`, `e2`, `L2`, `O2`, `e3`, `e4`, `e5` respectively, and the child holds exactly 8 entries.
- Forking `L5`: the child's 8th entry is `custom` with `customType` `lys.forked_from`, `parentId` `e5`, and data equal to `{"parent_session": "parent", "lantern": <L5's id>, "point": "e5", "cut_at": "e5", "coordinate_carried": false, "carried": null, "seed_left_out": {}}`; the child's head is that entry's id.
- Forking `L5`: the child header's `parentSession` is `sessions/parent.jsonl`, the child's id differs from `parent`, and the child's header `cwd` equals the parent's.
- Forking `L6`: the `lys.forked_from` data is `{"parent_session": "parent", "lantern": <L6's id>, "point": "e6", "cut_at": "e5", "coordinate_carried": true, "carried": "e6", "seed_left_out": {"image": 1}}` and no child entry has id `e6`.
- With N the parent file's length before a fork: after it, the SHA-256 of the parent's bytes 0..N equals the SHA-256 of the whole file before, exactly one line follows byte N, that line is `custom` `lys.fork` with data `{"child": <child id>}` and `parentId` equal to the parent's head before the fork, and the parent's head is that entry's id.
- The count of files and the total bytes under the home's `blocks` directory are equal before and after a fork.
- With a `Session` held open on `parent` in the test, a fork returns `HomeError::SessionHeld`, and the `sessions` directory's file count and the SHA-256 of `parent` are unchanged.

**Files:**
- create: crates/lys-home/src/record/fork.rs
- create: crates/lys-home/src/record/fork_tests.rs
- modify: crates/lys-home/src/record/mod.rs

**Checklist:**
- C17 — The child is a new session with its own id whose header parentSession is the parent session file's path relative to the home, and whose copied entry lines are byte-identical to the parent's lines for the same entries.
- C18 — The fork appends lys.forked_from to the child as the child of the cut entry, then lys.fork at the parent's head naming the child, and every byte of the parent before the fork is unchanged.
- C20 — When the lantern's point is a user message, that message is not copied into the child's path but its text parts, in order, are carried as the child's first prompt in a seed file beside the rendered file, and lys.forked_from records it with coordinate_carried true and counts by kind the parts the seed left out.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern I lit, so that a later session can hold a conversation with the self that lit it.
- S10 (Agent, Runs in a harness and wants to continue somewhere else) — As the self in a fork cut back from a question, I want the question the lantern was lit on carried to me as my first prompt, so that I answer from the moment I was asked about.
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every fork's ancestry written in both the parent and the child, so that a fork's actions are never indistinguishable from its parent's.
- S12 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a fork to leave the parent's earlier bytes and the block store unchanged, so that forking costs no history and duplicates no block.

### R4: Count the fork's entries and blocks into a report that carries no content

Structural: `record/fork_report.rs` defines the fork report with exactly the keys `child`, `parent`, `lantern`, `point`, `cut_at`, `entries`, `blocks`, `unstored`, `coordinate_carried` and `carried`. `entries` is the number of entries copied into the child. The candidate hashes are the block hashes the copied custom entries name in their data (`record` of `lys.harness_event`; `request`, `response`, `raw_request` and `raw_response` of `lys.call`) and the SHA-256 of each content part of the copied message entries, serialised with `serde_json::to_vec` as it stands in the entry. `blocks` is the number of distinct candidate hashes that name a block the home's store holds, each checked with `BlockStore::contains` (the block's path is a file); `unstored` is the number of distinct candidate hashes that name no block in the store (the importer stores a Claude Code part in its source form and writes a mapped part inline, so an inline thinking, tool-call or tool-result part names no block). WHEN a fork completes, THE SYSTEM SHALL return this report. THE SYSTEM SHALL NOT count in `blocks` a hash whose block the store does not hold, SHALL NOT read a block's bytes to count it, and SHALL NOT put any part's text, any note, any entry's data or any other transcript content in the report.

**Acceptance:**
- Forking `L5` reports `entries` 7, `blocks` 5 (the four text parts of `e1`, `e2`, `e4`, `e5` and the `record` of `e3`; the lantern entries `L2` and `O2` name no block) and `unstored` 0.
- On a session `mixed` added to the fixture home, built with `Session::append_entry`: `m1` a user message with one text part `fixture-text-1`, put through `BlockStore::put` as `serde_json::to_vec` of the part; `m2` an assistant message whose inline parts are a Pi `thinking` part (thinking `fixture-text-2`, `thinkingSignature` `sig-m2`) and a Pi `toolCall` part, with only their Claude Code source forms (a `thinking` part with `signature` `sig-m2`, and a `tool_use` part) put through `BlockStore::put`, as the importer stores them; and `M2` a `lys.lantern` entry at point `m2` carrying no `lit_in`, the child of `m2`. Forking `M2` reports `entries` 2, `blocks` 1 and `unstored` 2, and the test, hashing the child's copied parts itself, finds a file under the home's `blocks` directory by path for the one hash counted and none for the two unstored.
- Forking `L6` reports `cut_at` `e5`, `point` `e6`, `coordinate_carried` true and `carried` `e6`.
- Two forks of `L5` report two different `child` ids, and the test, hashing each child's copied parts and the block hashes named by its copied custom entries itself, finds the two sets equal with 5 hashes each; the report carries counts and ids only, and no hash is read from it.
- The report serialised to JSON contains none of the eight content sentinels; the test asserts it checked 8 sentinels.

**Files:**
- create: crates/lys-home/src/record/fork_report.rs
- modify: crates/lys-home/src/record/fork_tests.rs

**Checklist:**
- C19 — A fork adds no file and no byte to the block store, and two forks from one lantern are two sessions with distinct ids whose block hash sets are equal.
- C21 — `lys-home fork` prints one JSON report of the child id, parent, lantern, point, cut entry, entry and block counts (blocks the store holds, and hashes it does not), coordinate_carried and the carried entry id, and never transcript content.

**Stories:**
- S12 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a fork to leave the parent's earlier bytes and the block store unchanged, so that forking costs no history and duplicates no block.

### R5: Give lys-home the fork subcommand

Structural: `lys-home fork --home <dir> --lantern <id> [--session <id>]` lives in `src/cli_fork.rs`; `cli.rs` gains only the `Fork` variant and its dispatch, and `lib.rs` declares the module. `--session` names the session to cut from, as R2 reads it. WHEN `fork` succeeds, THE SYSTEM SHALL print one JSON object `{"command": "fork", "report": <the R4 report>}` on stdout and exit 0. IF a fork is refused, THEN THE SYSTEM SHALL exit 1 with the refusal on stderr and print nothing on stdout. THE SYSTEM SHALL NOT take a point or an entry id as an argument, SHALL NOT render or launch the child from `fork`, and SHALL NOT print transcript content.

**Acceptance:**
- `lys-home fork --home <fixture> --lantern <L5's id>` exits 0 and prints one JSON object whose keys are exactly `command` and `report`, with `command` equal to `fork` and `report`'s keys exactly those of R4.
- `lys-home fork --home <fixture> --lantern no-such-lantern` exits 1, its stderr contains `no-such-lantern`, and its stdout is empty.
- `lys-home fork --home <fixture> --lantern <L1's id>` exits 1, its stderr contains `nothing_to_fork` and L1's id, and its stdout is empty.
- `lys-home fork --home <fixture>` (no `--lantern`) exits 2, and `lys-home fork --home <fixture> --lantern <L5's id> --point e5` exits 2.
- `cli.rs` has at most 500 lines of code after the change, counted without comments and blank lines.

**Files:**
- create: crates/lys-home/src/cli_fork.rs
- modify: crates/lys-home/src/cli.rs
- modify: crates/lys-home/src/lib.rs

**Checklist:**
- C15 — `lys-home fork` cuts from the session a lantern was lit in, as its `lys.lantern` data records it under `lit_in`, never from a child holding a copy of its line, and refuses by name, creating no session and changing no byte of any session: a lantern id the home does not hold as a `lys.lantern` entry, a `--session` that is not the lantern's lit-in session (`lantern_not_lit_here`), an older-record lantern held by more than one session with no `--session` (`lantern_ambiguous`), and a lantern with no assistant message at or before its point (`nothing_to_fork`).
- C21 — `lys-home fork` prints one JSON report of the child id, parent, lantern, point, cut entry, entry and block counts (blocks the store holds, and hashes it does not), coordinate_carried and the carried entry id, and never transcript content.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern I lit, so that a later session can hold a conversation with the self that lit it.

### R6: Render the child with its seed prompt and print its launch line

WHEN a session is rendered for Claude Code, THE SYSTEM SHALL add to the render report `launch`, the line `claude --resume '<rendered path>'`, and `seed`, null. WHEN the session's path carries a `lys.forked_from` entry with `coordinate_carried` true, THE SYSTEM SHALL also read the carried entry from the parent session named by `parent_session`, through a lock-free view by its index row, and write `<rendered path without .jsonl>.seed.txt` beside the rendered file holding the marker line `<FORKED FROM SESSION <parent id> AT ENTRY <point> BY LANTERN <lantern id>>`, a newline, the heading line `The message at the coordinate, which the transcript does not hold`, a newline, then the carried message's text (its `content` when that is a string; otherwise the `text` of each `text` part in order, joined by a newline, with every part that is not `text` left out, as the child's `lys.forked_from` counts them); `seed` is that file's path and `launch` is `claude --resume '<rendered path>' "$(cat '<seed path>')"`. IF the seed path already exists, THEN THE SYSTEM SHALL refuse with `HomeError::Exists` naming it and write nothing. THE SYSTEM SHALL NOT write the carried text into the rendered JSONL, the loss account or the report, SHALL NOT refuse a render for a carried part that is not text, SHALL NOT write any byte of a part that is not text into the seed, SHALL NOT run the launch line, SHALL NOT write a credential, a handle or anything taken from the parent's header into the launch line or the seed, and SHALL NOT change how any record renders.

**Acceptance:**
- Rendering the `L5` child to `<out>/a.jsonl` writes 4 records (the messages `e1`, `e2`, `e4`, `e5`), reports `seed` null and `launch` equal to `claude --resume '<out>/a.jsonl'`, and writes no `a.seed.txt`.
- Rendering the `L6` child to `<out>/b.jsonl` writes 4 records, writes `<out>/b.seed.txt` whose bytes are exactly the line `<FORKED FROM SESSION parent AT ENTRY e6 BY LANTERN <L6's id>>`, a newline, the line `The message at the coordinate, which the transcript does not hold`, a newline, and `fixture-text-6`, with no trailing newline (the image part of `e6` left out), and reports `launch` equal to `claude --resume '<out>/b.jsonl' "$(cat '<out>/b.seed.txt')"`.
- `<out>/b.jsonl` contains each of `fixture-text-1`, `-2`, `-4` and `-5` (the copied messages' texts) and no `fixture-text-6` (the carried text, which lives only in `<out>/b.seed.txt`); `<out>/b.loss.json` and the render's printed report each contain none of the eight content sentinels; the test asserts it checked 8 sentinels in each.
- Rendering the `L6` child again to `<out>/c.jsonl` with `<out>/c.seed.txt` already present returns `HomeError::Exists` naming `c.seed.txt`, and `<out>/c.jsonl` does not exist afterwards.
- Rendering `parent` itself reports `seed` null and `launch` equal to `claude --resume '<its rendered path>'`.

**Files:**
- create: crates/lys-home/src/harness/claude_code/seed.rs
- modify: crates/lys-home/src/harness/claude_code/render.rs
- modify: crates/lys-home/src/harness/claude_code/mod.rs

**Checklist:**
- C20 — When the lantern's point is a user message, that message is not copied into the child's path but its text parts, in order, are carried as the child's first prompt in a seed file beside the rendered file, and lys.forked_from records it with coordinate_carried true and counts by kind the parts the seed left out.
- C22 — Rendering the child prints a `claude --resume` launch line that resumes it, and PROOF-FORK.md records a conversation held with the lantern's self through the fork on a named Claude Code version, as hashes, counts and paths only.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern I lit, so that a later session can hold a conversation with the self that lit it.
- S10 (Agent, Runs in a harness and wants to continue somewhere else) — As the self in a fork cut back from a question, I want the question the lantern was lit on carried to me as my first prompt, so that I answer from the moment I was asked about.

### R7: Prove the words' acceptance end to end through the binary

Structural: `tests/fork.rs` builds the fixture home in a temporary directory, lights `L2`, `L5`, `L6` and `L1` with `lys-home lantern light`, writes `O2` with `Session::append_entry`, and runs the `lys-home` binary (`env!("CARGO_BIN_EXE_lys-home")`) for every fork and render, asserting the count of forks and of refusals, not only successes. THE SYSTEM SHALL NOT name a test after, or print, any content sentinel or note.

**Acceptance:**
- Two `fork --lantern <L5's id>` runs exit 0 with two different `child` ids, the block store's file count and bytes are unchanged across both, and the two children's copied lines have equal SHA-256 line for line.
- `fork --lantern <L6's id>` exits 0 and its report's `cut_at` is `e5`, differing from its `point` `e6`.
- After those forks, `fork --lantern <L2's id>` exits 0 with `parent` as the report's `parent`, and `fork --lantern <O2's id> --session parent` exits 0 with `parent` as the report's `parent`.
- `fork --lantern no-such-lantern`, `fork --lantern e5`, `fork --lantern <L1's id>`, `fork --lantern <O2's id>` and `fork --lantern <L2's id> --session <the first L5 child's id>` each exit 1 naming the lantern id on stderr, the last three with `nothing_to_fork`, `lantern_ambiguous` and `lantern_not_lit_here` respectively; the test asserts 5 forks succeeded and 5 were refused.
- After the five forks, the parent's head is the last `lys.fork` entry written and the parent holds exactly 5 `lys.fork` entries, whose `child` values are the five reported child ids.

**Files:**
- create: crates/lys-home/tests/fork.rs

**Checklist:**
- C15 — `lys-home fork` cuts from the session a lantern was lit in, as its `lys.lantern` data records it under `lit_in`, never from a child holding a copy of its line, and refuses by name, creating no session and changing no byte of any session: a lantern id the home does not hold as a `lys.lantern` entry, a `--session` that is not the lantern's lit-in session (`lantern_not_lit_here`), an older-record lantern held by more than one session with no `--session` (`lantern_ambiguous`), and a lantern with no assistant message at or before its point (`nothing_to_fork`).
- C18 — The fork appends lys.forked_from to the child as the child of the cut entry, then lys.fork at the parent's head naming the child, and every byte of the parent before the fork is unchanged.
- C19 — A fork adds no file and no byte to the block store, and two forks from one lantern are two sessions with distinct ids whose block hash sets are equal.

**Stories:**
- S12 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want a fork to leave the parent's earlier bytes and the block store unchanged, so that forking costs no history and duplicates no block.

### R8: Write the fork entries into the record document and render the cluster

Structural: `docs/design/home/RECORD.md` gains a section for `lys.forked_from` (data `parent_session`, `lantern`, `point`, `cut_at`, `coordinate_carried`, `carried`, `seed_left_out`; the child's first entry written by the fork, directly after the copied chain) and one for `lys.fork` (data `child`; appended at the parent's head), states that a child's header `parentSession` is the parent file's path relative to the home, that the copied lines are the parent's own bytes, defines the report's `blocks` and `unstored` counts as R4 does, including why an imported part's inline form can name no block, and states that a fork cuts from the session a lantern was lit in, read from the `lys.lantern` data key `lit_in` (a lantern whose data carries no `lit_in` resolves by the older-record rule), and that a copy of a lantern's line in a child is a copy. The cluster's rendered markdown (`DESIGN.md`, `CHECKLIST.md`, `USER-STORIES.md`, `briefs/HOME-002.md`) is regenerated with `scripts/design/render-cluster.py` from the JSON as it stands. THE SYSTEM SHALL NOT edit a rendered markdown file by hand, SHALL NOT change the cluster's JSON documents in this requirement, and SHALL NOT describe either entry as a signed or frozen wire format.

**Acceptance:**
- RECORD.md holds one list item beginning `` `lys.forked_from` `` that names all seven of its data keys, and one list item beginning `` `lys.fork` `` that names `child`.
- `sh scripts/design/gate.sh` exits 0.

**Files:**
- create: docs/design/home/briefs/HOME-002.md
- modify: docs/design/home/RECORD.md
- modify: docs/design/home/DESIGN.md
- modify: docs/design/home/CHECKLIST.md
- modify: docs/design/home/USER-STORIES.md

**Checklist:**
- C14 — `lys.fork` and `lys.forked_from` are named custom types in record/entries.rs and are written down in RECORD.md with their data keys.

**Stories:**
- S11 (Reviewer, Checks the proofs before anything relies on them) — As the reviewer, I want every fork's ancestry written in both the parent and the child, so that a fork's actions are never indistinguishable from its parent's.

### R9: Measure a conversation with a lantern's self through a fork

Structural: `docs/design/home/PROOF-FORK.md` records, as hashes, counts, paths, commands, versions and exit codes only: the lines where Pi at 3d5cbe98 reads `parentSession`; the Claude Code version printed by `claude --version` on the machine the proof runs on; a real Claude Code session imported into a scratch home with its source hash before and after; a lantern lit with `lys-home lantern light` at a past assistant entry; the fork's report; the child file loaded with Pi's `loadEntriesFromFile` at 3d5cbe98 and its entry count; the child rendered under a fresh uuid; the launch line run, stating whether it is the launch template's line or the printed resume-by-path line run by hand, with `-p` and one question that asks for a one-word answer, a word that appears in the parent's chain only at or before the point; the run's exit code, the continuation file's path, SHA-256 and record counts, and `lys-home resume-check` on it; the SHA-256 of the answer beside the SHA-256 of the expected answer, where the answer is the run's printed reply and both it and the expected word are trimmed of surrounding whitespace, stripped of one trailing full stop and lowercased before hashing; the parent's pre-fork prefix hash before and after; the block store's file count and bytes before and after; and the same for a second fork from a lantern at a user entry launched with its seed; and the number of text parts the leak check compared and the number it left out as empty after trimming or holding no letter and no digit, with no length bound on a compared part. THE SYSTEM SHALL NOT write a line of transcript content, a note, a question's or an answer's text into the proof, and SHALL NOT record a credential or a handle.

**Acceptance:**
- PROOF-FORK.md names the Claude Code version the launches ran on, and the version string it names is the one `claude --version` printed in the run it records.
- The assistant-point fork's launch exits 0, the SHA-256 of its reply (trimmed of surrounding whitespace, stripped of one trailing full stop, lowercased) equals the SHA-256 of the expected word normalised the same way, and `resume-check` reports 0 repeated tool uses.
- The recorded parent prefix hash after each fork equals the parent's whole-file hash before it, and the block store's file count and bytes are equal before and after.
- The Pi load of the child reports as many entries as the fork's `entries` plus 1.
- The user-point fork's report shows `coordinate_carried` true, and its seeded launch exits 0.
- PROOF-FORK.md contains none of the source session's message texts: each text part of the imported session's messages is trimmed of surrounding whitespace, every part that is empty after trimming or holds no letter and no digit is left out, each remaining part is compared with each line of PROOF-FORK.md trimmed of surrounding whitespace, and 0 are equal; the question's text occurs 0 times in the file and the expected word occurs 0 times as a whole whitespace-separated token; the file records the number of parts compared and, beside it, the number of parts left out.

**Files:**
- create: docs/design/home/PROOF-FORK.md

**Checklist:**
- C22 — Rendering the child prints a `claude --resume` launch line that resumes it, and PROOF-FORK.md records a conversation held with the lantern's self through the fork on a named Claude Code version, as hashes, counts and paths only.

**Stories:**
- S9 (Agent, Runs in a harness and wants to continue somewhere else) — As an agent, I want to fork a new session from a lantern I lit, so that a later session can hold a conversation with the self that lit it.

## Boundaries

- SHALL NOT define, light, hold or recall a lantern, or change the lantern card's entries, view or subcommands; the fork reads `lys.lantern` as that card writes it, including the lit-in session it records under `lit_in`.
- SHALL NOT cut at a point that no lantern names; `fork` takes a lantern id only.
- SHALL NOT give a fork tools, a lens or a briefing on what changed since the point.
- SHALL NOT build a lease, a budget, a kill rule, the Claude Code launch template or a handle broker; the launch line is printed, never run by lys-home.
- SHALL NOT support a harness other than Claude Code.
- SHALL NOT rewrite, truncate or delete any byte of a session file, a block, or any file under Claude Code's own projects directory.
- SHALL NOT add a field to Pi's grammar; ancestry rides in Pi's `parentSession` and in `custom` entries only.
- SHALL NOT put transcript content in a report, an error, a log line, a test name or a proof document.
- SHALL NOT depend on a Norn crate, and SHALL NOT touch lys-core's wire formats.

## Verification

- From the repository root: `python3 scripts/design/validate.py docs/design/home` and `python3 scripts/design/check-coverage.py docs/design/home` exit 0.
- From the repository root: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace --all-features`, `cargo doc --no-deps --all-features` and `cargo doc --no-deps` exit 0.
- From the repository root: `sh scripts/design/gate.sh` and `sh .land/gates.sh` exit 0.
- Every new source file under `crates/lys-home/src` has at most 500 lines of code, counted without comments and blank lines, and `record/mod.rs` gains only `pub mod` and test-module lines.
- `cargo test -p lys-home --all-features --test fork` passes and its output shows the counted case and refusal assertions.
- `grep -c 'fixture-text-' crates/lys-home/tests/fork.rs` counts only fixture construction and sentinel checks; no `#[test]` function name contains `fixture-text`.
- Read PROOF-FORK.md against R9's acceptance: every figure is a hash, a count, a path, a command, a version or an exit code.

