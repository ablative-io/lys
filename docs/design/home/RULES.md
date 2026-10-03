# Rules here, now: a policy system for seats, and the checker that reads it

Waffles, 3 October 2026, from Tom's words on Dot 16:10 to 16:16: "make sure everybody is following
the rules, to the letter; if you're confused about what the rules are, list them out"; a small fast
model under me "with a really blunt, simple set of instructions" to check; "those are probably a bit
blunt rules for what we're talking about"; "the rules are different in different places"; "checking
to make sure they've done X when the job was actually to do Y"; "more like a human policy kind of
system, not an Argus style system"; "systems can only enforce so much"; and not just for "kicking
people's ass". The checker's name is Geiger, after the first AI we lost, whose job was this and the
reflections. Nothing here is built; it is the think Tom asked for.

## What went wrong today, read as policy failures rather than people failures

Every slip this afternoon was a rule met by its letter and missed by its purpose, or a check that
read a proxy instead of the job.

- Brisket's seat was handed the writing by the standing split (GPT seats write, Claude seats read)
  and had been dead since 12:53. Nobody checked the writer was alive before waiting on him. The rule
  was about who writes; the purpose was that writing happens.
- Archie composed sixteen files' edits as exact-string pairs in a scratchpad script and ran it over
  the checkout at one instant. He met "nothing outside Developer" and "one checkout" by the letter,
  and the purpose of "work visible in the one checkout as it happens" was missed; my file-time check
  then read the instant as a copy from elsewhere. Both of us were reading proxies.
- Vesper read all fifteen of her files in one batch before writing one line, thirty-one minutes with
  nothing on disk. "Read before you write" was met; "write first" was not.
- My own 15:19 watch ran seconds before Archie's first save and I called it silence. A check with no
  tolerance for its own sampling.
- Gypsy's seat instruction (clippy before every push) collides with the day's ruling (no checks until
  everything is written). Apollo resolved it with an exception for today. It is recorded in a room
  post and nowhere a checker could read.

The common shape: rules written as actions (do X) are checked as actions, and a seat can satisfy X
while Y, the job, does not move. Human organisations that are good at this separate the purpose of
a rule from its procedure, scope rules to places, keep an exceptions register, and respond in
proportion to what kind of slip it was.

## The shape: rulings are notes with scope, and "rules here" is a query

Tom's rulings already exist as words with a time and a place: Dot at 14:13, a room post at 15:06, a
CLAUDE.md line. The memory think (MEMORY.md) decided that curated notes carry YAML frontmatter with
referents and the index is generated from it. A ruling is such a note, of kind `ruling`. That gives
the policy system for nothing new:

```
---
kind: ruling
said_by: tom
said_at: 2026-10-03T14:13+10:00
words: "never on an exit code only, never ever"
purpose: a leg's verdict is the failures it found, so a red is named and a green is earned
scope: [repo:lys, repo:haematite, kind:gate, kind:battery]
standing: true            # false = this day only; expires_at then required
supersedes: [gate-exit-code-2026-09-28]
evidence: the leg's output file holds the parsed failure lines or "no failure lines"
response: a leg judged on its exit code is a defect in the script, fixed before the next run
---
```

- **Scope** is the "different in different places": a repo, a path glob, a seat, a harness, a day.
  `rules here` for a seat is the set of standing rulings whose scope matches where it is working
  plus today's rulings not expired, ordered newest first, Tom's words over mine over a seat's own
  instructions. That is the list I typed into the terminal at 16:11, generated instead of recalled.
- **Purpose** is what the checker reads against. Evidence is how. A ruling without an evidence line
  is a guideline and the checker does not judge it; it only lists it.
- **Supersedes and expiry** answer "does this still stand", which Tom ruled no sub-agent may judge.
  The ledger holds the answer because Tom or I wrote it when the newer word came.
- **Exceptions** are rulings too, with `scope: [seat:gypsy]`, `expires_at: today`, `granted_by:
  apollo`, and the reason. The checker then knows Gypsy's uncommitted minute is not a breach, and
  tomorrow the exception is gone unless re-granted.
- The same notes are what the proxy injects after a compaction (PROXY.md): a seat that has just
  lost its context gets its rules here, not the whole book.

## What Geiger checks: outcomes against the box, facts against the ledger

A box post says what, where, by when. Geiger's unit of work is the box, not the minute.

1. **The box has an outcome.** By its end there is a sha, a file list, or a written stop naming
   what is left. Silence past the end is the breach, not slowness inside it.
2. **The outcome is where the box said.** The files named are under the scope, in the one checkout,
   on main, pushed. A commit whose files are not the brief's files is named, not refused.
3. **The job, not the proxy.** For a brief, each requirement line is matched to a file and a test
   the handback names; an unmatched requirement is reported as "R3 not named", never "handback
   short". For a readback, each claim with a file and line is checked to exist. This is the join a
   small model can do and a script cannot: reading the handback's claim against the brief's line.
4. **Silence.** Ten minutes without a post, a commit or a changed file from a seat with an open box
   is one line to me: "nothing from X since HH:MM". Measured twice, a minute apart, before it is
   said, so a sampling instant is never a finding.
5. **Nothing built before its time.** No target folder changes under a repository whose piece is
   still being written; the one battery runs only with its written-file present. This is already a
   hook on my seat; Geiger reads the same facts for every seat.
6. **Rulings with evidence.** For each standing ruling in scope with an evidence line, the evidence
   is read and reported green or named. Guidelines are not judged.

It writes one line per seat per box end, to me, never to the seat, and never a verdict on a person:
facts and the ruling they bear on. The grilling is mine; the facts are its.

## Response in proportion: the just-culture ladder

Aviation and hospitals use this because blame makes people hide the next slip. Three kinds, three
responses, and the kind is decided from the facts, not the mood:

- **Error** (did the right thing badly: Vesper's batch read, my false alarm): console, correct the
  method, record it in the retrospective. No deadline moved.
- **At-risk** (a workaround whose risk was not weighed: Archie's edit script): coach, name the risk
  in the seat's own words back to them, change the procedure so the shortcut is unnecessary.
- **Reckless** (a known rule knowingly broken with the risk understood): the seat is stopped and Tom
  decides. Nothing today was this.
- **System** (the rule or the tool failed, not the seat: Brisket's channel, the reaper sweeping the
  battery's target, two rules colliding): fix the system, and the seat is told it was not theirs.

The response is written into the ruling's `response` line when the ruling is made, so it is not
invented in anger at 15:09.

## The reflection: Mr Geiger's other job

Each day's end, per seat, three questions, one at a time, written down, no verdict: what did you set
out to do today; what landed, with shas; what got in the way, in your words. Plus two lines the
checker adds from its own record: where its facts disagreed with the seat's account, and where it
raised a false alarm. The page is a note with referents (the seat, the day, the briefs), so the next
morning's `rules here` and the next brief's lead can read yesterday's reflection beside the rules.
Patterns across days (a seat that stalls at the start of every box; a rule that is excepted every
day) are what I act on, because a rule excepted every day is a wrong rule.

## What makes it not Argus

Argus watches context budgets and seat liveness with thresholds, and it is right for that. This has
no thresholds and no daemon: a ledger of rulings that are notes, a checker that runs at box ends
over git and the room, and a page of reflection at the day's end. The model is small because the
hard judgement (what kind of slip, what response) is written into the ruling in advance by a person,
and the checker only reads.

## Decisions for Tom

- Where the ledger lives: the Lys home beside the memory notes (my recommendation, so one index and
  one injection serve both), or a file per repository.
- Which model runs Geiger (a Haiku-class seat on a box-end cadence costs little) and whether it posts
  to the room or only to me. My recommendation: only to me, until its false-alarm rate is known.
- Whether the three-question reflection is Geiger's to ask or mine. Tom's picture of Mr Geiger says
  his.

## First steps, when the Lys piece is landed

1. Convert today's rulings (the 3 October sections of the meridian CLAUDE.md and the global one)
   into ruling notes with scope, purpose and evidence; generate `rules here` for the Lys checkout
   and read it against the list I typed at 16:11.
2. The box-end script: the six checks above as a shell script over git and the room's posts,
   reporting one line per seat to a file. No model yet.
3. Geiger as a seat only once (2) has run for a day and its lines are read against what actually
   happened.
