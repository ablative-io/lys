# Home — User Stories

## Agent — Runs in a harness and wants to continue somewhere else

**S1.** As an agent, I want my session rendered into a fresh Claude Code file that resumes where I left off, so that a 3 GB transcript is not what I carry.

**S2.** As an agent, I want my provider's own reasoning kept with the provider that made it, so that I can swap model and swap back without losing it.

**S7.** As a new session, I want to start from the canon, the series of examples that carry what every session before me learned, each a rule stated short with a real exchange that shows it lived and naming where it came from, so that our learning is in one another and I know which of it is mine.

**S8.** As an agent about to be compacted or retired, I want to write to the one who wakes up after me, what I know, what I got wrong and why, how the people like things done, what I wish I had known, so that they start with part of my memory and know it is mine, not theirs.

**S14.** As an agent at a moment of completion, success or learning, I want to light a lantern on a point of my session with a note, including a point I have already moved past, so that a later session, or a fork, can walk back to it.

**S20.** As an agent, I want to fork a new session from a lantern's point, carrying everything said up to that point and nothing after it, and launch the child like any session, so that I can go back and talk with the self that lit the lantern.

**S28.** As an agent whose session was imported from Claude Code, I want it rendered as a Codex thread that Codex resumes, with every text, tool call and tool result carried whole, so that I continue on Codex knowing what the session knew rather than a clipped summary of it.

**S31.** As a Codex thread translated from a home session, I want to open on a marker naming the session I was translated from, so that I never mistake myself for that session.

**S41.** As an agent that lights a lantern, I want it to record the session I lit it in and recall to show that session beside the point and the note, so that I know which session the lantern leads back to.

**S46.** As the agent launching a session, I want a launch whose render is refused to store nothing in the home, so that a launch that wrote no file leaves no template behind.

**S71.** As an agent, I want my home to move into a second home by a pushed ref the target fetches, holding my record byte for byte, so that I can continue from a home that is not the one that captured me.

**S72.** As an agent, I want each of my arrived sessions to carry an arrival naming where it came from and a fresh execution id of its own, with its head unchanged, so that my continuation is a distinct execution whose ancestry is on the record.

**S77.** As an agent whose memory view is read many times a session, I want each read to decode and scan once, so that recall stays light under everything else.

## Tom — Owns the platform and reads what a session was given

**S3.** As Tom, I want the session file created before the harness runs and watched while it runs, so that the platform controls where a session lives.

**S4.** As Tom, I want a written account of what each render or translation lost, so that nobody claims a faithful continuation that was not measured.

**S6.** As Tom, I want to construct a session file by hand, a few-shot prompt written as turns, and have the harness resume it as if it had happened, so that a session can be authored, not only recorded.

**S21.** As Tom, I want a fork at a user message to carry that message as the child's first prompt beside the rendered file, with nothing from the parent's header but its working directory, so that the child starts at the coordinate and no credential, handle or launch setting is copied from the parent.

**S23.** As Tom, I want a fork's report to carry ids and counts only, with the parent's earlier bytes and the block store unchanged and the whole thing proved through the binary, so that a fork never quietly copies or rewrites anything.

**S29.** As Tom, I want every translation to carry an account, by entry id and hash, of what was kept, what changed and how, and what was lost and why, so that the difference between the session and its Codex fork can be read without reading the transcript.

**S32.** As Tom, I want every compaction in a session to say by ids, counts and hashes what it could not keep, so that a summary is never taken for the whole of what was said.

**S44.** As Tom, I want a session the render cannot shape refused by name with nothing written, never rendered with a default where the record carried no value, so that a rendered file that looks whole is whole.

**S50.** As the owner of the platform, I want a render's report to say whether what the session was given was signed, so that an unsigned render is never taken for a signed one.

**S79.** As Tom, I want a recorded call's blocks written as one durable operation with a fixed number of device flushes, so that the home's cost per act does not grow with the parts a reply has and a block that exists is whole.

## Reviewer — Checks the proofs before anything relies on them

**S5.** As the reviewer, I want each resume path measured on a named harness version with the command and hashes recorded, so that a later version changing the path is caught.

**S10.** As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

**S9.** As the reviewer, I want every file of lys-home judged by Jev on its own, with the verdict recorded beside its path, so that a verdict on the landing diff is never read as a verdict on the crate.

**S13.** As the reviewer, I want the instruction load order measured on a named Claude Code version and written in a proof document, so that a later version that changes the order is caught rather than assumed.

**S15.** As the reviewer, I want every fetched session to record the commit, remote and ref it came from and the new home's execution id, so that a moved home's ancestry is on the record.

**S16.** As the reviewer, I want a ship to refuse a home holding a named secret value, so that no credential leaves in a shipped ref.

**S17.** As the reviewer, I want a lantern kept in the home and out of every rendered resume file, with the home file still Pi's grammar, so that lighting one never changes what a harness resumes.

**S18.** As the reviewer, I want the same session head rendered with the same lys-home version to give the same bytes every time, so that I can tell a rendered file by its hash and a receipted render event names what was written.

**S19.** As the reviewer, I want the proof document to name why two renders of a multi-result tool record differed, so that the fix is checked against its cause rather than its symptom.

**S22.** As the reviewer, I want the ancestry written on both sides, the child's header naming the parent file and a lys.forked_from entry naming the lantern, the point and the cut, and a lys.fork entry at the parent's head naming the child, so that a stranger can tell a fork from its parent from the record alone.

**S24.** As the reviewer, I want the home record's mod.rs to hold only module docs, mod lines and re-exports, with Home, Session and the shared helpers in files named for them, so that the record module meets the repository's structure rule when I judge it.

**S30.** As the reviewer, I want the Codex rollout shape measured from files Codex 0.156.0 wrote and its resume recorded with hashes and counts, so that a later Codex version is refused until it is measured rather than assumed to match.

**S33.** As the reviewer, I want to list a session's compactions with a check that every summarised entry and block is still held, so that I can prove on a compacted session that the original is all still there.

**S34.** As the reviewer, I want the fork's tests to light their lanterns with the light act wherever it can produce them, so that the fork is proved on the record the light act really writes.

**S26.** As a reviewer, I want a disk error while a session opens reported as that error, so that a failing disk is seen and never papered over by a rebuilt index.

**S27.** As a reviewer, I want the cost bounds proved by counters and the unchanged record proved by a hash, so that the change is checked without trusting a timing.

**S35.** As a reviewer, I want each check of the chain's judgement recorded with the commit and toolchain it was measured on, so that I can rerun the deterministic checks at that commit and see the same outcome.

**S36.** As a reviewer, I want every finding recorded as its own line marked still true or answered by a named commit, so that only what is still wrong on main becomes a card.

**S40.** As a reviewer, I want the proof that a second put writes nothing to hold on a filesystem with coarse timestamps and on a loaded host without the suite waiting on a clock, so that a passing store gate means the store wrote nothing and never that the tick was too coarse to see a write.

**S42.** As the reviewer, I want a lantern lit before lit_in was recorded to stay readable and forkable by its holders, and a lit_in that is not a session id refused by name, so that no lantern is ever read as a session it does not name.

**S43.** As the reviewer, I want the fork's tests to light their lanterns with the light act wherever it can produce them, so that the fork is proved on the record the light act really writes.

**S45.** As the reviewer, I want each render refusal to name the entry id, the field and the expected type and never the value, with its own fixture, so that I can find the broken entry from the error alone and no transcript reaches it.

**S73.** As the reviewer, I want ship to refuse by name, pushing nothing, any home it cannot ship whole and any remote that is not a path on this machine, so that nothing half-written or unencrypted leaves.

**S74.** As the reviewer, I want a fetch that fails verification to name the bad index or block by hash and leave behind nothing it wrote, so that a corrupted home never arrives half-written.

**S75.** As the reviewer, I want the fetched home rendered and resumed through the launch template on the installed Claude Code and recorded as hashes, counts and paths, with no fixture secret in the shipped ref, so that the resume evidence stage 3 asks for is measured.

**S49.** As a reviewer, I want to check what a session was given with lys verify offline, so that I can trust the record without trusting the home that wrote it.

**S51.** As a reviewer, I want to see which key signed a given statement, so that a statement signed by any other key is not taken for the home's.

## Board reader — Reads the step 5 board to know what the chain has judged

**S11.** As a reader of the step 5 board, I want a card set done only when the chain judged its rows' code and found nothing, so that done never means merely merged.

**S12.** As a reader of the step 5 board, I want each finding written as its own line naming the check, the file and the rows it touches, so that a card can be filed from that line alone.

## Developer — Works on lys-home's code beside the record module

**S25.** As a developer working on lys-home, I want every public path of the record module to resolve and every test to pass unchanged after the move, so that my code and tests need no edit because files moved.

## Lead — Owns the cards of a board and sets them done

**S37.** As the lead who owns the step 5 board, I want a card set done only when the chain has judged its rows' files and no open finding touches them, so that done means judged by the chain and never just merged.

## Estate operator — Runs Lys behind every agent and session

**S76.** As the operator of an estate where Lys runs behind every agent, I want each request, append and open to do its work once and scale with what it touches, so that Lys costs nothing it does not need to as history grows.

**S80.** As the operator who runs Lys in front of a haematite store, I want the store's screens and agents to reach it only through Lys, with Lys deciding who is admitted, so that a service which asks nobody who is calling is never the thing a browser or an agent talks to.

## Seat operator — Starts an agent session from what Lys records for it

**S78.** As the operator who starts a seat from Lys, I want everything I set for it (its harness build, models, skills, MCP servers and permissions) to be everything Claude Code or our Codex build is given, so that a seat started from Lys is the seat I configured and nothing is silently dropped.
