# Context and memory: a roadmap in order of need

Archie, for Tom, 22 September 2026, 16:30 Melbourne; revised 16:36 after
Chippy's review (Cambium post 99717d39). Steps 4 and 5 of
`STATEMENT-2026-09-22.md`. A working proposal, not a settled design and not
a brief; each stage becomes its own brief to Waffles before work starts.
Nothing here is built. Two hours is each brief's measured cut-and-stop
point, not a claim that a stage fits inside it.

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

**0. A capture ID, not a borrowed identity.** Chippy's rows 01 and 02
provide the fork and the deployment; the enduring directory ID arrives with
row 04. Until then, capture files under a stable capture ID of its own, with
the seat name as a label only, and bind that ID to the directory ID
explicitly when it exists. The seat name is neither unique nor immutable,
so it is never the identity, and history is never renamed.

**1. Capture the logs.** A watcher copies a running session's transcript
files, byte for byte, nothing parsed, into a directory per capture ID.
The minimum profile declaration comes in here, not later: the transcript
root and the capture method are supplied, so the first watcher hardcodes no
Claude knowledge that stage 2 would then have to remove. The proof has a
defined boundary: each captured file records the version and length it was
taken at; append-during-copy, restart, file replacement or truncation, and
name collisions across devices and executions are handled and tested;
unreadable or incomplete capture is refused by name. A changing file and
its earlier complete snapshot are not expected to be byte-equal at
comparison time; the snapshot is compared to the bytes it recorded.

**2. One Claude Code profile.** A profile is where a harness's transcripts
live plus the command template that launches it. This stage proves exactly
one, Claude Code. Templates treat substituted paths and text as data and
fail on a missing required input. Proof: launch from the template, and the
new session is captured into the same home.

**3. Resume from the home.** Two proofs, in order: first a same-machine
resume from an isolated copy of the home, then one named second machine.
Three prerequisites are written beside this stage and are not skipped:
identity and read authority over the home resolved; encrypted export
before any bytes leave the machine; evidence for the exact harness resume
path, because a copied transcript does not by itself establish that the
destination has the workspace or runtime state it needs. Source artifacts
stay intact; the target gets a distinct execution ID with its ancestry
recorded; credentials are supplied at launch on the target, never copied
into the home.

**4. Derived records: compaction.** A compaction is stored beside its
original, points at it, and says what it could not preserve. The original
is never replaced. Proof: a compacted session still has its original.

**4b. Derived records: translation.** The same rule for a transcript
translated for another harness. A separate proof, because the loss account
is different and the target harness is a second variable.

**5. Lantern recall.** A lantern is a note plus a coordinate in a
transcript. Proof: light one, and recall it by note and by coordinate.

**5b. A working fork.** A fork is a launch from a lantern's coordinate
through the harness's own resume or fork, with its own execution ID and
ancestry. Proof: commune with a lit lantern through a fork. Separate from
5 because it is a different mechanism with its own failure modes.

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

- Where the home lives long term: a directory now; haematite only if a later
  stage needs what it offers.
- Other harnesses (Codex, Pi): each a profile, each proved on its own.
- Which encrypted-export mechanism stage 3 uses; decided in its brief.

## Sources

Norn memory design (`norn/docs/design/norn-memory/`), the Lantern design
(`tools/lantern/docs/`), the lys log store and CLI as they stand today,
Tom's spoken direction in the room on 22 September, and Chippy's review.
