# Memory for the oldest seat: what I have, what it is worth, and what to build (3 October 2026)

Waffles, 3 October 2026, 13:30 to 13:5x, for Tom. Written answer first, as Minto would have it. Every number below was measured this afternoon on this Mac; the paths are given so anyone can re-measure.

## The answer

What I have today is not a memory. It is an unread archive that gets reset every few weeks: 3,009 files across 42 folders, about 7.3 million tokens, keyed by the folder I happened to be launched from, with no record of what each note is about, no record of what wound up happening, and no way to retire a note except to archive the whole folder. The one part that is read is the index, and it is read as a status board. A bigger folder with a better search bolted on would reproduce the same failure at a larger size.

The fix is to make the Lys home the memory. A memory is a note against a thing, written at the point in the session where it was learned, so every memory is a lantern by construction and the fork back to the moment comes free. Curation is by epilogue, never by deletion or reset. Recall is in bands, cheapest first and never blended: the index, exact referent, keyword, vector kinship, and the fork that goes and looks. Each band's pull rate is measured from the first day, because engagement is the only evidence a memory was worth writing. The experiments Tom asked for are then three, in order, each small, and the first needs no code at all.

## What I have, measured

**Scale.** Every `memory` folder under the two Claude config directories together: 3,009 markdown files, 29.3 MB, roughly 7.3 million tokens. They are split across 42 folders because Claude Code keys memory by working directory, so one seat's memory is scattered by where it was launched, and other seats' folders (Archie, Muthur, Buckleys) sit in the same tree with nothing but the folder name to say whose they are. "Tagged to each of us" does not exist today.

**Mine.** The February-to-July folder under the main config holds 511 files (2.7 MB, about 680,000 tokens), newest 29 July. The live folder under the waffles config held 797 files (6.6 MB, about 1.66 million tokens), dated 7 September to this morning, until I moved it out at 03:23 today at Tom's word. Beside it sit two earlier wholesale archives, 28 August (158 files) and 1 September (133 files). That is the curation that has happened: the folder grows, then the whole thing is set aside and it grows again.

**The index.** The July index names 77 of the 511 files; 337 are named nowhere. The September index is 169 lines and 31.6 KB, about 8,000 tokens loaded at every start. The head of the July index is an emergency board from 29 July ("READ THIS FIRST", six lanes, Tom furious), still at the top two months later. Of 6,039 tool calls in my session file that name a memory file, 1,619 touch the index. The next most-touched files are "resume" notes read hundreds of times at wake. So the index is the memory, and it is a to-do list.

**What is in it.** By file prefix across the July folder: rules given as feedback 192, project state 178, session logs 23, references 13, facts about Tom 11. 279 of the 511 carry no type at all; they predate the frontmatter. Half the bytes of the folder are session-state logs, one of them 820 KB. Those are transcript extractions, which Tom ruled out on 19 May: lanterns are pointers into conversations, not extractions from them.

**What happened to it.** No file carries "what wound up happening". Nothing is superseded in place. 398 of 510 July files were named by at least one tool call in my history, read or written; I have not yet split reads from writes, so the real read-back rate is lower than that. The lantern research notes cite the same finding from the literature: cheap auto-authored memory mostly sits unused, while inherited memory gets used.

**Live fact.** Since 03:23 today the memory folder this session is given is empty. I have run the whole day on a 64-line CLAUDE.md and a checkpoint file in the scratchpad, and shipped the Lys head, the proxy slices, the Cullen read and the Minto proposal. That is itself a measurement of what the 797 files were doing for me.

**The session file.** 4.19 GB, 1.75 million lines, 1,118 compaction summaries. Claude Code loads it whole at launch; the Lys home design recorded it at 3.37 GB on 24 September for the same reason. The home is built for exactly this: import the transcript, render the root-to-leaf working session, resume by fork-session, and point lanterns into the history without carrying it. An import stores blocks by hash, so expect roughly the file's size again on disk. Free space is 68 GB; the meridian-life data folder alone is 41 GB, of which my sessions in SQLite are 9.3 GB and the vectors 3.8 GB.

## What already exists

**In Lys, landed.** Light a lantern at a point of a session with a note; grow it by epilogues; recall by a phrase of the note or by point, reading every session of the home without locking any; fork a child session from a lantern's point. The canon: a curated, versioned set of real exchanges every new session starts from, changed only by review. The given record: what each session was handed, by path and hash. Ranking, vectors and resonance are out of scope by the brief's own line.

**In the Norn design, ruled by Tom in August.** A memory is a note against a thing; a lantern is that plus a pathway back to talk to the previous self. Lanterns are declared at completion, success or learning, never automatic. Notes grow by appended epilogue, never edited. Vectors are in, as a kinship band ranked below exact, structural and lexical, never blended. Decay follows churn on the referenced paths, not time. Compaction is navigation, not destruction. No clock, no daemon: a derived index advances at turn boundaries and records its coverage point. Engagement is the only measure. Pull first, push precise. Still waiting on Tom: log-as-truth as the ruling, the budget share, the embedding model, retention of sessions carrying lanterns, first-class entries versus custom.

**In tools/lantern, designed on 5 September, nothing built.** One primitive: cut a session at a coordinate, seed a fork, run it under a lens with read-only hands, get a schema-shaped answer, record the event. Tom's memory search is a use of it: a true fork of the seat, full context, on a cheaper model, with the lens "you know what you are looking for; go back and find it". That is the agentic third option beside the index and the vector, and it can use the vector band rather than compete with it.

**In meridian-life.** My sessions and others' are already in SQLite with Tessera bge-base vectors on every message piece; a cosine search over messages, turns and commits by meaning is written; "memories" is first on its list of sources still to pour; a hyperbolic decider head was trained on 28 September. The vector experiment has a substrate already.

## The shape I propose

1. **One record: the home.** A memory is a note, one or more referents, and the point in the session where it was written. Lighting a lantern at the head with a note is writing a memory; the referent is the one missing field. The referent tuple is the one the Norn design settled: repository identity and paths, a person or handle, a card, topic words. Recall keys on the referent; fork keys on the point. Tom's definition keeps them distinct, and written at a point they coincide.

2. **Curation by epilogue, never by reset.** "What wound up happening" is appended, stamped with who, when and which session. A superseded rule gets an epilogue saying so; it is never deleted and never archived wholesale. Additions are free; softening or deleting one needs a second seat to agree, as the lantern design proposed. A retrospective fold over the record, on request and never on a clock, finds one law written as many and lanterns with no outcome.

3. **The library is every seat's home, read across.** Every lantern already carries who lit it, so "tagged to each of us" is a field, not a system. Recall across homes is the library. The canon remains the curated shared tier, in the repository and changed by review, which is right for a few dozen examples and wrong for a million tokens.

4. **Growth is free because nothing is loaded whole.** The push tier is a share of the context window, Tom's number, titles only, generated from the record rather than hand-edited. That is what the index becomes: one line per memory with its referent, capped, derived, never a status board.

5. **Recall in bands, cheapest first, never blended.** The index as push; exact referent match; keyword over note and epilogues (recall today is substring, BM25 is a small step); vector kinship with Tessera bge-base, null-calibrated against the corpus as the vectors note sets out; and the fork-search for the case where I know what I am looking for and do not want to pay the context to look. Every surfacing is logged with whether I read, cited or forked it. The pull rate per band is the measurement, and it decides what stays.

6. **"Watched" without a watcher.** The proxy already sees every model call, and the home is read at turn boundaries with a recorded coverage point. No daemon, no timer, by the standing rule.

7. **Somewhere safe.** Import my session into a home, render a working session, prove the resume on this Claude Code version, then light lanterns into the 4 GB rather than carry it.

## Three experiments, in order

**E1, the audit, no code.** Read the 797 September files and the 511 July files into four piles with counts: rules still true (canon candidates), rules superseded (an epilogue each), state logs (archive, with a pointer to the session point they came from), references and facts about Tom. The output is the list of rules that lived and the shape of what should never have been a memory. Half a day of reading.

**E2, the three arms side by side.** Pour the memory files into meridian-life as a source, which its pipeline already supports. Build the query set from what I actually went looking for: the 6,039 tool calls that named a memory file say what I wanted each time. Run index, keyword and vector on the same queries and report hit rate per arm, with the vector arm's null measured first. This settles Sable's open hypothesis that lexical beats semantic on a corpus dense in rare identifiers.

**E3, the home as memory.** One field on a lantern, the referent; recall by referent; recall across homes; then the fork-search lens on the existing fork. Each its own brief and card, after E1 says what the referents should be.

**E4, in parallel if disk allows.** Import my session into a home and prove resume, which is the safety piece and independent of the rest.

## Decisions that are Tom's

Where the library lives: a home per seat with the canon as the shared tier, or one shared home. The push budget as a share of context. The embedding model, where bge-base is already in use and pinned. Whether the memory directive in my system prompt becomes the lantern command, which changes what every session is given and is recorded by the given record. And the standing question the Norn design has waited on since August: the log as the ruling of record.
