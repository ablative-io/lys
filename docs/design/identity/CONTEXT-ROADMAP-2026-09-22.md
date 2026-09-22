# Context and memory: a roadmap in order of need

Archie, for Tom, 22 September 2026, 16:30 Melbourne. Steps 4 and 5 of
`STATEMENT-2026-09-22.md`. A working proposal, not a settled design and not
a brief; each stage becomes its own brief to Waffles before work starts.
Nothing here is built.

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
- Time is short. Each stage fits a two-hour block or says why not.

## Stages

**0. Wait for an identity to file under.** Chippy's rows 01 and 02 give an
enduring identity ID. Until then, stage 1 files under the seat name and is
renamed once, not redesigned.

**1. Capture the logs.** One watcher on a running session's transcript tree
(for Claude Code, the project folder under the config directory) copying
each session file under the identity as it grows, byte for byte, nothing
parsed. The home is a directory per identity. Proof: a session's transcript
appears under the identity and is byte-identical to the original.

**2. Profiles.** A profile per harness: where its transcripts live, and the
command template that launches it. Claude Code first. Proof: launch from
the template, and the new session is captured into the same home.

**3. Spawn elsewhere.** Pull the home to another device, render the
template, launch, continue. Proof: a session begun on the Mac continues on
Dean or Annabel. What "continue from a copied transcript" needs is
measured per harness, not assumed.

**4. Derived records.** A compaction or a translation is stored beside its
original, points at it, and says what it could not preserve. The original
is never replaced. Proof: a compacted session still has its original.

**5. Lanterns and forks.** A lantern is a note plus a coordinate in a
transcript. A fork is a launch from that coordinate using the harness's own
resume or fork. Proof: light one, then commune with it through a fork.

**6. Receipts and signing, when asked for.** Hash each captured object and
append the hash to a lys log under the identity, using the existing log and
CLI; anchoring after that. Every stage above works without this.

**7. The lifecycle screen (shared).** What an identity holds, its sessions,
spawn and fork. This is the piece that goes into Cambium.

## Open, and deliberately not decided here

- Where the home lives long term: a directory now; haematite only if a later
  stage needs what it offers.
- Encryption at rest: needed the day the home leaves the machine, not before.
- Other harnesses (Codex, Pi): each a profile, each proved on its own.
- The event schema shared with Chippy's audit lines: settled in
  `IDENTITY-EVENTS.md` before stage 6 signs anything.

## Sources

Norn memory design (`norn/docs/design/norn-memory/`), the Lantern design
(`tools/lantern/docs/`), the lys log store and CLI as they stand today, and
Tom's spoken direction in the room on 22 September.
