# Home — Checklist

## The common record

- [ ] **C1** — The home record is Pi's session tree: a home file parses with Pi's parser unchanged, and lys's harness events and call records are custom entries.
- [ ] **C2** — Content blocks are stored once by SHA-256 and referenced; an identical block put twice occupies one entry.

## Claude Code in and out

- [ ] **C3** — A Claude Code JSONL imports to events: user turns, assistant turns with tool calls, tool results, compaction summaries, and sidechains as child branches; harness bookkeeping records are counted and left in the byte-for-byte original.
- [ ] **C4** — Events render to a Claude Code JSONL under a chosen uuid at the harness path, with a loss account beside it.
- [ ] **C5** — Provider-native opaque blocks are kept whole, keyed by provider, model family and branch, and rendered only to their own provider with the intervening events.

## The two proofs

- [ ] **C10** — A hand-written few-shot session file resumes Claude Code by path from a directory outside the config root; the source is unchanged and the session reports authored.
- [ ] **C6** — One real session imported, rendered and resumed with --fork-session on Claude Code 2.1.281: it continues, no completed tool action repeats, the original's hash is unchanged.
- [ ] **C7** — One seat with a subscription login completes a call through a pass-through proxy; the measurement is written down.

## The two captures

- [ ] **C8** — Claude Code's harness-local records (hook outcomes, permission mode, tool completion) become lys.harness_event entries attached under the message entry they followed.
- [ ] **C9** — A proxy call record (lys.call) names its request and response blocks by hash with provider, api, model and timing, and can be ingested from a captured request and response pair.
- [ ] **C11** — The little proxy passes Messages, Chat Completions and Responses streams through unchanged and appends one lys.call entry per call under the session it links to.

## The canon

- [ ] **C12** — The canon, one curated versioned series of short examples (a rule stated short plus a real exchange that shows it lived), drawn from every agent's sessions and changed only through review, seeds every new session as lys.inherited entries naming each example's source; nothing in it is authored thinking; canon-seeded against plain is measured on a card.

## The handover

- [ ] **C13** — At compaction or retirement the outgoing session's letter to its successor, with its real thinking, becomes the successor's first entry as lys.inherited; it is never authored and replays only to the same provider, api and model; seeded against plain is measured on a card.

## The context record

- [ ] **C23** — The Claude Code version on this Mac, the instruction load order and project slug rule measured from its own behaviour, and one real session render, are written in PROOF-GIVEN.md as paths, counts and hashes only.

## Forks

- [ ] **C22** — Rendering the child prints a `claude --resume` launch line that resumes it, and PROOF-FORK.md records a conversation held with the lantern's self through the fork on a named Claude Code version, as hashes, counts and paths only.
- [ ] **C36** — The two fork custom types, lys.forked_from (parent_session, lantern, point, cut_at, coordinate_carried, carried, seed_left_out) and lys.fork (child), are named beside the other lys custom entries and documented in RECORD.md, and neither adds a field to Pi's header or outside custom.data.
- [ ] **C37** — A fork refuses by name before any file is created: a lantern id no session holds as a lys.lantern entry, an older-record lantern several sessions hold with no session named (lantern_ambiguous, listing them in ascending byte order), a named session that is not the lantern's lit-in session (lantern_not_lit_here), a lantern that sits before any assistant message (nothing_to_fork), and a parent another owner holds; no refusal carries a note, a part's text or any entry's data.
- [ ] **C38** — A fork resolves a lantern to the session it was lit in, read from its data's lit_in or by the older-record rule when the data carries none, and cuts that session's root-to-point chain, read through the index and never by loading the file, at the last assistant message at or before the point, in file order with every side leaf left out and no entry after the cut.
- [ ] **C39** — The child session holds each cut entry as the parent file's own line bytes, under the parent's cwd, with the header's parentSession the parent file's path relative to the home; nothing is re-serialised.
- [ ] **C40** — Ancestry is written on both sides: lys.forked_from is the first entry the fork writes in the child, directly after the copy, and its head; lys.fork is appended at the parent's head naming the child; a user-message point is not copied but carried, its id recorded with coordinate_carried true and its parts that are not text counted by kind.
- [ ] **C41** — The fork report carries the child, the parent, the lantern, the point, the cut entry, the count of entries copied, the distinct block hashes the copied entries name that the store holds and those it does not, whether the coordinate was carried and the carried entry; never a part's text, a note or any entry's data.
- [ ] **C42** — A fork writes no block, rewrites no earlier byte of the parent, and copies nothing from the parent but its cwd: no credential, handle or launch setting enters the child.
- [ ] **C43** — lys-home fork --home --lantern [--session] prints one JSON report on success, exits 1 with the refusal on stderr and nothing on stdout, and takes no point or entry id.
- [ ] **C44** — Rendering a child whose coordinate was carried writes the carried message's text parts as a seed prompt beside the rendered file under an in-band marker line, the render report names it, the template's launch line (printed by render-launch only) passes it as the first prompt and is never run, and PROOF-FORK.md records a fork launched on the installed Claude Code version as hashes, counts, paths, commands, versions and exit codes only.

## The chain's judgement of lys-home as found

- [ ] **C21** — The card lands through src_pr and src_land with Jev and the gate; the seven cards are set done only on a green landing with no finding, a card carrying a partly present row stays in review, and one card is filed per finding line.

## The launch template

- [ ] **C14** — A Claude Code launch template schema in docs/design/home names its five slots (transcript, mcp, env, secrets, instructions), and a template with a slot outside them is refused by that slot's name with nothing written.
- [ ] **C15** — The home keeps each template it renders as an object under templates/ named by its SHA-256, written once and never rewritten.
- [ ] **C16** — lys-home render-launch writes the rendered JSONL, its loss account, an MCP configuration file, an environment file and an appended-instructions file into one directory, and a second render of the same template and session writes files with identical SHA-256.
- [ ] **C17** — The launch line in render-launch's report resumes the rendered file by path with --fork-session and the template's flags, and the tool never runs it.
- [ ] **C18** — A use-only secret is written as its handle and never its value, and a template marking a secret readable is refused naming secret_reader_unbuilt and SECRETS-002.
- [ ] **C19** — Every render appends one lys.harness_event of kind template_render beside the context path, naming the template hash, the session head hash and a manifest block of the written paths, and the head does not move.
- [ ] **C20** — PROOF-LAUNCH.md records the launch measured on the installed Claude Code version: template hash, written paths and hashes, launch line, the rendered file unchanged, where the continuation landed; never transcript content.

## The deterministic render

- [ ] **C32** — A fixture session holding a tool record with two results and a tool record with one result beside user text renders twice, to two paths with the same target, into files of equal SHA-256.
- [ ] **C33** — A test pins the fixture's rendered SHA-256, and PROOF-RESUME.md records the same value with the commands that produce it.
- [ ] **C34** — The PROOF-RESUME source renders twice from one import with one recorded command into files of equal SHA-256, and PROOF-RESUME.md records that hash and the command.
- [ ] **C35** — PROOF-RESUME.md names the cause found, the namespace, name form and roles the multi-result case used, and a resume of the rendered file with --fork-session on the installed Claude Code, with the version `claude --version` printed and repeated_tool_use_ids 0.

## Moving a home

- [ ] **C24** — lys-home ship commits the home's tracked set (session files, their index and head files, blocks and templates) and pushes it as one ref, refs/lys-home/<commit>, to the remote named on the command line, and its report names the commit and the ref.
- [ ] **C25** — Ship refuses by name a home whose index is stale or whose head file is missing, exits 1, and writes nothing to the home or the remote; no index is rebuilt and no head persisted.
- [ ] **C26** — Ship refuses by name any remote that is not a path on this machine or a file:/// URL, naming its scheme and the encryption-at-rest precondition, and writes nothing.
- [ ] **C27** — No lock, temporary, environment or render file is in a shipped tree, and ship refuses by file and offset a tracked file holding any value of the --secret-values file or matching one of the five standard patterns, naming the pattern and printing no value or matched byte.
- [ ] **C28** — lys-home fetch fetches the named ref into a new directory, refuses by name a directory that already holds a home (a sessions, blocks or templates directory, a .git entry, or an execution-id file) and accepts one holding only other entries, and checks every index against its file and every head against its index without rebuilding either, leaving nothing behind on a refusal.
- [ ] **C29** — Fetch appends to each session, beside the head, one lys.harness_event of kind arrival naming the source commit, the remote without userinfo, the ref and the target home's execution id, within the 512-byte cap.
- [ ] **C30** — On the fixture home, every tracked file at the fetched commit equals its source byte for byte, the source session file and index are byte prefixes of the target's after the arrival, the source home's files are unchanged by ship and fetch, and a search of every object in the remote finds no fixture secret value while a planted one is found.
- [ ] **C31** — PROOF-MOVE.md records the fetched home rendered by render-launch and resumed by the printed launch line on Claude Code 2.1.283, as hashes, counts and paths only.
- [ ] **C169** — A home's tracked set is enumerated exactly, never by glob: for each session the home lists, `sessions/<id>.jsonl`, `sessions/<id>.index.jsonl` and `sessions/<id>.head`; each `blocks/<hh>/<hash>` and each `templates/<hh>/<hash>` whose name is 64 lowercase hex digits under a directory named by its first two; never a lock file, a temporary, a file at the home root or anything under `.git`.
- [ ] **C170** — A home is verified strictly, never rebuilt: a missing index, an index that is not its file's, a missing head and a head naming an id its index does not hold are each named per session, and every block and template whose bytes do not hash to its name is named by that name; nothing is written.
- [ ] **C171** — ship and fetch run the git binary with a pinned environment (no system or global configuration, no hooks, no filters, no line-ending conversion, a fixed identity, the file protocol only), and take a remote only when it is a path on this machine: any ssh remote, https, git or file URL, or hosted service is refused as remote_not_local, with words saying that shipping off this machine waits for stage 3's encryption from the secrets step, and nothing is written.
- [ ] **C172** — An arrival is a seventh lys.harness_event kind, `arrival`, whose detail carries the source commit, the remote, the ref and a fresh execution id, under the 512-byte data cap; the six existing kinds do not change.
- [ ] **C173** — lys-home ship initialises the home as a git repository on first use, commits exactly the tracked set, pushes it without force as the one ref `refs/lys/home` to a remote that is a path on this machine, creates a bare repository at a remote path that does not exist, and reports the commit, the ref, the remote and whether it initialised and created; a home unchanged since its last ship reports that commit as unchanged, makes no new commit and pushes it again; the source home's tracked files are byte-identical before and after.
- [ ] **C174** — ship refuses by name, pushing nothing: a home with no session (empty_home), a remote path that is neither absent nor a bare repository (remote_not_bare, naming the path and what it found), a session whose lock a live owner holds (session_held, with the act 'ship after that seat stops'), a session with no index file (index_missing) or no head file (head_missing), each naming the session, the missing file and the lys-home command that writes it, a stale index (stale_index, in a JSON report naming each session and its reason), a home repository tracking files outside the tracked set (foreign_tracked), and a remote ref the new commit does not descend from (ref_diverged, naming the ref and both commits, the remote's ref unchanged).
- [ ] **C175** — lys-home fetch refuses a remote that is not a path on this machine (remote_not_local), a target that already holds a home (target_holds_home) and a non-empty target (target_not_empty), each before writing anything, fetches the ref into the target, and the fetched commit's tree hash-matches the source for every session file, index, head, block and template.
- [ ] **C176** — After fetch each target session's last line is one arrival event hung beside the head, naming the source commit, the remote and ref and a fresh execution id of that session's own; fetch commits those lines as one commit whose only parent is the fetched commit, whose message names the source commit, the remote and the ref, leaving `git status` in the target clean, and a ship from the target carries the arrival; the target's head file is byte-identical to the source's, and every template a template_render event on the target names is held under the target's templates/.
- [ ] **C177** — When verification fails, fetch prints a JSON report naming each stale index by session and each bad block and template by hash, never by content, records no arrival, and removes exactly the paths it created, deepest first: a target it created is gone, and an empty target that existed before is left existing and empty. When an arrival's append or its commit fails, fetch refuses as arrival_failed and removes exactly the paths it created in the same way.
- [ ] **C178** — A byte search of every object the shipped ref reaches finds the fixture secret value 0 times, with a control that finds it once where it was planted, and a render of the target records the same session_head as a render of the source.
- [ ] **C179** — RECORD.md documents the arrival kind and its detail, and that the home directory is a git repository and exactly what it tracks; the crate README documents ship and fetch with their arguments and refusals, and states that no credential lys-home holds is ever written into the home and why ship stays local.
- [ ] **C180** — A proof document measures a fixture home shipped, fetched, rendered through the launch template and resumed by the printed launch line on Claude Code 2.1.283, as hashes, counts and paths only.

## Translation to Codex

- [ ] **C58** — lys-home translate-codex takes --home, --session, --out, --codex-version and --zone, writes one rollout under <out>/sessions/YYYY/MM/DD/ named as Codex names its own and one loss account beside it, and prints one JSON report of the two paths and the entry, block, kept, changed and lost counts, never content.
- [ ] **C59** — An unmeasured Codex version, an unnamed time zone, an unknown time zone, an existing rollout or account path and an entry stamp that is not RFC 3339 are each refused by name before anything is written, with messages saying render for 0.156.0 or card a measurement of the new version, set TZ to an IANA name, choose another --out and re-import the source file; the shared session-exists refusal of the other commands is unchanged.
- [ ] **C60** — Every text part, tool call and tool result on the context path is carried whole as a Codex message, function_call or function_call_output item, never as a clipped note; a tool result longer than 4,000 characters is carried byte for byte.
- [ ] **C61** — Readable thinking is carried as output_text and counted changed; redacted and empty thinking are dropped and listed lost by hash with a reason; no signature, redacted data or reasoning item enters the rollout.
- [ ] **C62** — The loss account lists kept, changed and lost rows by entry id and the hash of each part as the home entry holds it, in walk order; a changed row names its before and after kinds and every field not carried or reshaped (a key beyond those its item carries, a one-item text array written as a string, an assistant turn's model, usage and stop reason), while a tool_use's caller field, dropped at import, is recorded nowhere yet; a lost row names its reason; every entry of the root-to-head path that a compaction leaves off the context path, and every entry descending from one, is a lost row naming that compaction; a message of any other role, a toolCall with no id or name and a toolResult with no toolCallId are lost rows and never given an empty default; every other entry off the path that descends from the path and is not carried is a lost row whatever its type, the harness events under a sidechain, the entries of a sidechain with no agent label, and every other label, compaction, branch summary and model change off the path among them, while a sidechain's agent label is carried in its marker line and counted changed; the account carries no content.
- [ ] **C63** — The rollout's session_meta holds only id, session_id, timestamp, cwd and cli_version, the thread id is record_uuid of the session head, and the first item is a developer message holding the in-band marker that names the source session id, the source head hash and the Codex version, declares the thread a fork and not that session, and carries no template hash.
- [ ] **C64** — Each translation appends one lys.translation custom entry beside the context path naming the thread, the head, the head hash, the rollout's relative path and the rollout's and account's SHA-256; the head and the session's earlier bytes do not move, and the Claude Code render of the session is unchanged by SHA-256.
- [ ] **C65** — The same session head translated into two fresh --out directories gives rollouts of equal SHA-256, and the second account differs from the first only by the one lost row naming the first translation's side leaf.
- [ ] **C66** — A Codex 0.156.0 rollout recorded in a scratch Codex home outside the repository in at most two attempts of one fixed shell command that reads nothing of the machine, after exactly one request that checks the proof account there can make requests and ends when codex exec exits, is committed as a fixture beside the Claude Code file that mirrors it, neither holding a machine path, an email or a token, and that file imported and translated gives response items equal to the recording's apart from ids and timestamps (id, call_id, turn_id and create_time), every Codex command there run with CODEX_HOME set to the scratch home.
- [ ] **C67** — PROOF-TRANSLATE.md records, with the seat that ran it and the date, Codex 0.156.0 resuming the translated thread in a scratch Codex home outside the repository and answering from its content, and resuming a translated thread that holds an image, with the proof account named by its role, the start checks, the attempts, each retry in a fresh scratch home, the paths, the rollout's hash before and after the resume, the side leaf's rollout hash, counts and exit statuses, and no transcript text, email or credential.
- [ ] **C68** — The home design admits exactly one translated pair, Claude Code to Codex, as roadmap stage 4b, and the cluster's rendered markdown is what its JSON renders to.
- [ ] **C69** — A sidechain is carried as marked text under the entry it hangs from, opening with a line naming its entry id and its agent id, and counted changed, with the agent label that names it counted changed, before label and after marker line, and a part marked text cannot hold, a base64 image included, listed lost with its reason; a branch summary or custom message on the context path is carried as marked developer text under its own marker and counted changed; lanterns, harness events and every other lys entry, render records included, are listed lost by entry id; a child forked at a user message carries that message's text after the walked history as the thread's next user prompt, counted changed with its how, and each of its parts that is not text is listed lost.
- [ ] **C70** — A base64 image part, in a user message or inside a tool result, is carried as Codex's input_image item with no detail key and counted changed, and Codex 0.156.0 is measured resuming a thread that holds one, with any detail value it needs taken from a rollout 0.156.0 wrote itself and named; an image of any other source is listed lost with its source type and part index and is never fetched.

## The lit-in session

- [ ] **C100** — LanternData carries an optional lit_in, the session that held the point when the lantern was lit, keeping a present null and a present non-string value distinct from an absent key, and still refuses an unknown key.
- [ ] **C101** — The light act writes lit_in as the id of the session at whose head it appends the lantern, and its report carries lit_in beside id, session, point and lit_at.
- [ ] **C102** — Recall rows carry lit_in beside point, note and lit_by, the lighting session for a lantern the light act lit and null for a lantern whose data has no lit_in, and a lantern whose lit_in is present and not a session id is never listed as a row but skipped by note and refused by point as lit_in_not_a_session with the fork's reason text.
- [ ] **C103** — The fork reads lit_in through a typed view that does not require lit_at or note and never from a raw JSON key: a lantern with no lit_in resolves by its holders, and a present lit_in that is null, not a string, not a safe session name, or names no session of the home refuses lit_in_not_a_session naming which.
- [ ] **C104** — The fork's fixtures light L2 with the light act and write by hand only the lanterns the light act cannot produce (O2, M2, N3, the copy N2 and N1 to N4), each named in its test as standing for such a record.
- [ ] **C105** — RECORD.md and the lys-home README document lit_in, and PROOF-FORK.md keeps its measured older-record sentence with a note that the light act now records lit_in.

## Compactions

- [ ] **C71** — A compact_boundary record followed by its isCompactSummary user record imports as a lys.harness_event for the boundary and one Pi compaction entry under the summary record's uuid, the child of the boundary's entry, whose firstKeptEntryId is the earliest in file order of compactMetadata.preservedMessages.uuids when that list is non-empty, otherwise the first record whose parent is the summary record, and the empty string when there is none, so its context path is the compaction alone and the pair renders back, and whose tokensBefore is compactMetadata.preTokens; the summary record is not a message entry.
- [ ] **C72** — A legacy summary record imports as a Pi compaction entry whose firstKeptEntryId is the entry its leafUuid names, never the compaction's own id.
- [ ] **C73** — A compaction whose first kept entry is not on record is refused with an error naming that uuid.
- [ ] **C74** — Directly after each compaction entry the importer appends one lys.loss custom entry whose parent is the compaction and whose data holds only ids, counts, byte counts and SHA-256 hashes, with the keys RECORD.md sets down, first_kept among them, null when nothing is kept.
- [ ] **C75** — A lys.loss entry's span is the root-to-first-kept path from the root or the previous compaction to the entry before the first kept one; its three SHA-256s are over those entries' source lines in file order, and side-leaf and sidechain entries hanging from the span are counted by number only.
- [ ] **C76** — A compact_boundary whose logicalParentUuid names no record in the file imports, and its lys.loss entry names that uuid as unresolved.
- [ ] **C77** — Two imports of the compaction fixture into two homes give lys.loss lines equal byte for byte once id, parentId, timestamp and data.compaction_id are masked.
- [ ] **C78** — The context path of an imported compacted session is the compaction, then the kept entries preserved uuids included, then what follows; no span entry is on it and the summary text is on it once.
- [ ] **C79** — lys-home compactions prints one JSON report of a session's compactions, each with its loss entry, the span's ids, counts and hashes and a check that every span entry is readable by id and every named block is held, and exits 0 only when nothing is missing and 1 when anything is, naming the first missing item; a compaction with no loss entry or pointing at itself is reported unaccounted by entry id.
- [ ] **C80** — The Claude Code render writes a compaction as a compact_boundary record followed by an isCompactSummary user record, then the kept entries, with no legacy summary line and no lys.loss line, and the summary text once; the boundary record's uuid is derived under render-uuid/v2, alongside v1, whose non-boundary uuids equal v1's, and a render with no compaction stays v1, byte for byte, with the version it used named in its report.
- [ ] **C81** — PROOF-COMPACTION.md records one real compact_boundary session imported read-only with its listing as counts and hashes and its source SHA-256 equal before and after, and a rendered compacted fixture resumed on Claude Code 2.1.283 answering from the summary, by hashes only.
- [ ] **C82** — A compact_boundary with no isCompactSummary record under it imports as a lys.harness_event only, with no compaction and no lys.loss entry, and lys-home compactions reports it by entry id as a boundary without a summary.

## The signed entry log

- [ ] **C55** — On a home that logs its entries and whose agent's key the identity crate does not hold, a lys.call, lys.harness_event or lys.open append is refused before its line is written, naming the home and the entry kind and the act that answers it, with nothing written.
- [ ] **C56** — lys-home open opens a sealed envelope with open_and_verify, writes the plaintext to --out, created new with mode 0600 on Unix, and appends one lys.open entry (the envelope's SHA-256, the sender's public key and the --out path) to the named session with its leaf; an existing --out is refused by name, and a failed open gives lys open's one failure message and writes no plaintext, entry or leaf.
- [ ] **C57** — A fork on a logging home gives each lys.call, lys.harness_event and lys.open line it copies its own leaf under the child session id, tagged as a copy and naming the parent session id, the parent entry's position and the parent leaf's index, or the checkpoint leaf's index when the parent entry predates the checkpoint, and is never refused for such a line; the child whose copies name parent leaves verifies without the parent's files, and verify refuses a copy whose parent leaf is absent or differs, and a copy naming the checkpoint whose parent line is absent or differs.

## Render refusals

- [ ] **C106** — HomeError names the render's refusals: a field refusal carrying the session, the entry id, the field, the expected type and whether the field was missing, whose Display for a missing provider or api names the act that answers it, a record whose assistant messages carry provider and api; a stopReason refusal carrying the session, the entry id and the value; and a serialisation refusal carrying the session, what would not serialise (`record`, `loss account` or `dropped part`) and the entry id when there is one; none carries a transcript value.
- [ ] **C107** — Every value the Claude Code render copies from a message entry is read through a checked reader that refuses a missing field and a field of another type than the target takes, and no reader substitutes a default; the one field read from its absence is `redacted`, absent meaning not redacted.
- [ ] **C108** — A message entry with no role, or with a role that is not a string, refuses the render by entry id and the field `role`; a role that is a string the render does not know is still skipped and the render succeeds.
- [ ] **C109** — A toolResult with no toolCallId, content or isError, a user message with no content or a single text part with no text, an assistant message with no content array, model, provider, api or stopReason, a text part with no text, a toolCall part with no id, name or arguments, a thinking part with no thinking text, a redacted thinking part with no thinkingSignature, and any of these present with another type than the target takes (a `redacted` or a thinkingSignature among them) each refuse the render by entry id and field, and nothing is written.
- [ ] **C110** — stopReason is mapped by one explicit arm each for toolUse, length, stop, error and aborted, and a wildcard arm: toolUse to tool_use, length to max_tokens and stop to end_turn; the error and aborted arms refuse by entry id and the value; the wildcard arm refuses by entry id and the value and maps to no Claude Code value.
- [ ] **C111** — Every serialisation the render needs, the record lines, the hashes of dropped parts and the loss account, is decided before any directory or file is created; a loss account whose serialiser fails refuses the render and no rendered file, loss account or seed exists afterwards.
- [ ] **C112** — Each refusal case has its own fixture asserting the entry id and the field it names; one test renders a thinking part with no `redacted` field as not redacted, and one renders past an unknown role.
- [ ] **C113** — The render's module doc names gitBranch "", usage {input_tokens 0, output_tokens 0} and stop_sequence null as format constants and absence as the only default of `redacted`; the bytes rendered from a session that carries every field and whose assistant stopReasons are toolUse, length or stop do not change; error, aborted or an unmapped stopReason refuses where it rendered as end_turn, and the pinned render hashes, whose fixtures carry only toolUse, length or stop, are unedited.
- [ ] **C114** — render-launch renders before it stores the template, so a refused launch leaves the home's templates directory with the same entries and hashes as before the call.
- [ ] **C115** — The cluster's rendered markdown, briefs/HOME-015.md among it, is what its JSON renders to, and scripts/design/gate.sh exits 0.

## The chain's judgement of HOME-001

- [ ] **C83** — docs/design/home/PROOF-CHAIN.md names commit 0073b9660f00ecd3ca6f13ffd3a9e59aabd8e6ff and toolchain 1.97.1, maps every HOME-001 row to its files present or absent at that commit, names R7, R10 and R12 unbuilt, and maps the seven step 5 cards to their rows.
- [ ] **C84** — The proof holds one Jev line per file of crates/lys-home at 0073b966, 25 in all, each with model, run id and verdict.
- [ ] **C85** — The proof records the seven commands .land/gates.sh runs at 0073b966 with each exit status and outcome, and the ast-grep leg as not measured naming ngzIkkpd.
- [ ] **C86** — Every finding is its own proof line marked still true at a named main head or answered by a named commit, and only still-true findings in crates/lys-home or docs/design/home are listed for the step 5 board.
- [ ] **C87** — The proof records the missing LOSS-ACCOUNT.md, record/mod.rs's functions and claude_code/mod.rs's consts and fn as findings with their citations and counts.
- [ ] **C88** — The proof gives each of the seven step 5 cards the verdict done or in review by row, and never done to 0BgjD59r or qY9mBz-y.
- [ ] **C89** — The landing changes nothing under crates/lys-home and passes src_pr, src_land and sh scripts/design/gate.sh.
- [ ] **C90** — After the green landing the step 5 board matches the proof: done cards point at the landing, in-review cards at their finding cards.

## The loss account document

- [ ] **C97** — LOSS-ACCOUNT.md quotes every reason string the render passes to the loss constructor, with the line that writes it, and the reason check prints 1 3 3 [] at the landing commit.
- [ ] **C98** — LOSS-ACCOUNT.md states that an entry's hash is the SHA-256 of the part as serde_json serialises it and that an entry never carries the part's text, signature or redacted data.
- [ ] **C99** — LOSS-ACCOUNT.md lists the four things the render changes without a loss entry, custom entries and labels, compaction, gitBranch and usage, each with the line that does it.

## Clock-free write-once gates

- [ ] **C95** — The block store's gate proves a second put of the same bytes writes nothing without elapsed time: before the second put it pins the shard directory's and the block file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock.
- [ ] **C96** — The template store's gate proves a second put of the same template writes nothing without elapsed time: before the second put it pins the template shard directory's and the template file's modification time to one fixed past instant and reads both back, and after it asserts both are still exactly that instant and the shard's entry count is unchanged; the test sleeps on no clock.

## The given statement

- [ ] **C118** — render-launch with no key records the lys.given entry with the same data as before, writes no statement, and its report's signing is unsigned.
- [ ] **C119** — render-launch reports given_sha256, the SHA-256 of the RFC 8785 bytes of the lys.given entry's data, and the template_render event does not carry it.
- [ ] **C120** — render-launch with a key signs those bytes as a lys/attestation/v2 statement kept as a block named by a lys.given_statement entry under the lys.given entry and as given-statement.cose and given-data.json under --out, and its report's signing is signed.
- [ ] **C121** — lys verify --attestation accepts the given statement with given-data.json, and refuses a copy with one byte altered with its one message and exit status 1, under a stated command that names the refused file.
- [ ] **C122** — A stated command compares the verified statement's signer public key with the test key's and exits 0, and exits 1 for a statement signed by a second test key.
- [ ] **C123** — A byte search of the fixture's statement, payload file, statement block and statement entry finds no fixture secret value and no fixture transcript text.
- [ ] **C124** — The statement's signed payload hash equals the report's given_sha256 and the SHA-256 recomputed from the lys.given entry in the session file for the same render.

## Repeated work removed

- [ ] **C45** — A Session asked by find_call for a call id builds, once per open, a map from call id to the first lys.call entry holding it in file order, keeps it current on every append and rebuilds it after a reconcile; once it is built, an ingest through ingest_call, ingest_call_files and ingest_outcome reads no lys.call entry, and a second ingest of a recorded call id records nothing and returns that entry's id.
- [ ] **C46** — A Session reports how many entries it has read from its file and how many syncs its own writes made (its line file, its index, its head and the sessions directory), and the block store reports its own syncs beside them, as counts a test reads.
- [ ] **C47** — The import command builds a new session under `<id>.jsonl.importing` with no per-entry sync and publishes it with one sync each of the line file, the index and the head and a sessions-directory sync before and after the rename to `<id>.jsonl`, five syncs whatever the record count; a crash before the rename leaves no `<id>.jsonl`, and the next import or open of that id removes what was left.
- [ ] **C48** — RECORD.md states the staged import's durability rule beside the per-append rule, as ADR-108 records it.
- [ ] **C49** — resume_check counts each transcript's tool_use ids in one pass and reports the same values as before.
- [ ] **C50** — The canon keeps the id set load builds, and adding an example refuses a repeated id by that set, never by walking the loaded entries.
- [ ] **C51** — Opening a session from its cached index checks every row in memory, reads the final byte of at most three rows (the first, the middle and the last by position) through one buffered reader, and returns a read error other than an unexpected end of file as that error, never as a stale index that is rebuilt.
- [ ] **C52** — context_path moves entries out of the path it read, customs reads only the path's entries of the custom type asked for, and the render's assistant arm and the importer's assistant content iterate a content array by reference, with no rendered or recorded byte changed.
- [ ] **C53** — One function, uuid_string, writes every uuid's 8-4-4-4-12 form: the fewshot's ids are 16 random bytes with the version nibble 4 and the variant nibble 8 set, formatted by it, and every render hash pinned in the tree is unchanged.
- [ ] **C54** — The line bytes of the multi_result fixture imported through the import command hash, under SHA-256 after each fresh 32-hex id is replaced by its order of first appearance and the header timestamp by a fixed token, to the value the same test gives at the parent commit.

## Record hot paths do each piece of work once

- [ ] **C181** — Call idempotency is a lookup, not a replay (HOME-035 R1), proved by a counting test that fails at the base.
- [ ] **C182** — Opening a session checks the index without a syscall per row (HOME-035 R2), proved by a counting test that fails at the base.
- [ ] **C183** — Bulk writers append in batches with one fsync per file (HOME-035 R3), proved by a counting test that fails at the base.
- [ ] **C184** — Rendering a launch moves data once and hashes what it writes as it writes (HOME-035 R4), proved by a counting test that fails at the base.
- [ ] **C185** — Call parts are borrowed and each body file read once (HOME-035 R5), proved by a counting test that fails at the base.

## Hot paths do their work once (HOME-036)

- [ ] **C186** — Records decode without a deep copy (HOME-036 R1).
- [ ] **C187** — Recall scans epilogues once for all lanterns (HOME-036 R2).
