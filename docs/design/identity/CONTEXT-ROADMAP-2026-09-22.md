# Context and memory: a roadmap in order of need

Archie, for Tom, 22 September 2026, 16:30 Melbourne; revised 16:36 after
Chippy's review (Cambium posts 99717d39, and his 16:35 stage-1 wording) and
Waffles' review (874979c9, 364bd446, f0bc6df7).
Steps 4 and 5 of `STATEMENT-2026-09-22.md`. A working proposal, not a
settled design and not a brief; each stage becomes its own brief to Waffles
before work starts. Nothing here is built. Two hours is each brief's
measured cut-and-stop point, not a claim that a stage fits inside it; where
a stage plainly will not fit, the stage says why.

## What Tom said today, and this roadmap keeps to

- Capture the logs. The transcript is what recreates a session; that is the
  thing to keep. Do not learn harness config layouts; a working directory or
  a default home is enough.
- There is more than one way to capture: a file watcher pointed at the
  transcript tree, a hook, a wrapper. The core does not choose; a profile
  per harness does. Keep the core flexible and do not get opinionated.
- Command templates map a home onto a harness: working directory, appended
  prompt, MCP config, environment, secrets. No adapter protocol, no worker.
- Authentication, permissions and identity come first (Chippy's rows).
  Signing and receipts have less immediate need and come when asked for.

## Stages

**1. Capture the logs.** A watcher copies a running session's transcript
files, byte for byte, nothing parsed, into a directory per capture ID.

*Filing rule.* Chippy's rows 01 and 02 provide a Rauthy user ID for people
and a deployment; the enduring agent ID arrives with row 04's directory
contract. Until then, capture files under a stable capture ID of its own,
with the seat name as a label only, and bind that ID to the directory ID
explicitly when it exists. The seat name is neither unique nor immutable,
so it is never the identity, and history is never renamed.

*Profile declaration.* The minimum profile comes in here, not later: the
transcript root and the capture method are supplied, so the first watcher
hardcodes no Claude knowledge that stage 2 would then have to remove.

*Copy discipline.* The copy is by offset and append-only; the copy is never
rewritten. The proof is a verified prefix through a recorded offset within
one source file generation, and the offset advances only after those bytes
are durable, so a copy taken mid-write is a prefix of the original and the
proof holds at every moment, not only at session end. A replaced, truncated
or rewritten source starts a separately identified capture or is refused by
name; it is never appended onto the old one. Restart and name collisions
across devices and executions are handled and tested; unreadable or
incomplete capture is refused by name.

**2. One Claude Code profile.** A profile is where a harness's transcripts
live plus the command template that launches it. This stage proves exactly
one, Claude Code. Templates treat substituted paths and text as data and
fail on a missing required input. The template is this product's own file:
manifold's seat profile describes the same things and may consume the
rendered command later, and neither depends on the other (Tom's rule of
13:55: manifold is optional, never a structural cog). Proof: launch from
the template, and the new session is captured into the same home.

**3. Resume from the home.** Two proofs, in order: first a same-machine
resume from an isolated copy of the home, then one named second machine.
Three preconditions are written beside this stage and are not skipped:
identity and read authority over the home resolved; encryption at rest
before any bytes leave the machine; evidence for the exact harness resume
path, because a copied transcript does not by itself establish that the
destination has the workspace or runtime state it needs. The home moves by
the same route as a build tree, a pushed ref fetched by the target, never
a hand copy (Tom's 08:30 rule); the remote is named in the brief. Source
artifacts stay intact; the target gets a distinct execution ID with its
ancestry recorded; credentials are supplied at launch on the target, never
copied into the home. This stage will not fit two hours: it has three
preconditions, two machines and a per-harness measurement, and its brief
says which of those the first block takes.

**4. Derived records: compaction.** A compaction is stored beside its
original, points at it, and says what it could not preserve. The original
is never replaced. The first derived record is a compaction, because the
harness itself produces it. Proof: a compacted session still has its
original.

**4b. Derived records: translation.** Translation is designed, not proved:
the lantern design measured three session formats and its first cut left
the converter out. So the first translation is a separate proof of one
named harness pair, with its own loss account, and nothing above it
assumes it.

**5. Lantern recall.** A lantern is a note plus a coordinate in a
transcript. Proof: light one, and recall it by note and by coordinate.

**5b. A working fork.** A fork is a launch from a lantern's coordinate with
its own execution ID and ancestry. Claude Code's resume continues a session
from its end, not from a coordinate, so a fork at a coordinate is the
lantern design's cut and seed: a transcript copy cut at that point and
seeded as a new session. How that is done is measured per harness, like
stage 3. Proof: commune with a lit lantern through a fork.

**6. Receipts and signing, when asked for.** Hash each captured object and
append the hash to a lys log under the identity, using the existing log and
CLI; anchoring after that. Every stage above works without this. Deferring
it does not remove IDENTITY-001's minimum signed identity-change audit,
which is Chippy's row 04 and stands on its own. The two meet at
`IDENTITY-EVENTS.md`: one shared envelope, distinct typed payloads, the hash
algorithm named, the inclusion coordinate outside the leaf.

**7. The lifecycle screen (shared).** What an identity holds, its sessions,
resume and fork. It is standalone product UI as well as a Cambium
integration; Cambium is never the only place a home can be inspected. A
small capture-status view can arrive with the stage that makes it useful;
the full screen grows later.

## Open, and deliberately not decided here

- Where the home lives long term: a directory now, and never "stored in
  haematite" until that backend is re-verified at a named commit.
- Other harnesses (Codex, Pi): each a profile, each proved on its own.
- Which encryption mechanism and which remote stage 3 uses; decided in its
  brief.

## Sources

Norn memory design (`norn/docs/design/norn-memory/`), the Lantern design
(`tools/lantern/docs/`), the lys log store and CLI as they stand today,
Tom's spoken direction in the room on 22 September, and the reviews by
Chippy and Waffles.
