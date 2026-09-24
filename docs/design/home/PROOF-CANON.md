# PROOF-CANON: the canon's mechanics, Pi's reader on the record, and what is still to be measured (HOME-001 R1, R11)

Measured 24 September 2026, 15:32 to 15:34 AEST, with the lys-home binary built
from card/home-001 at 19e3ffe8 and Pi's own reader at `3d5cbe98`.

## Pi's reader on the record (R1 acceptance)

`loadEntriesFromFile`, `parseSessionEntries` and `buildSessionContext` from
`packages/coding-agent/src/core/session-manager.ts` at `3d5cbe98` (a scratch
clone fetched from the checkout at `/Users/tom/Developer/tools/harness/pi`),
run under `bun run <scratch>/pi-parse.ts <file>...`. The script loads each file
with Pi's loader, then builds the context from the leaf named in the file's
`.head` sidecar, exactly as lys-home does.

| file | header | entries (Pi) | kinds | context messages (Pi) | roles | model (Pi) | lys-home's own path |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `home/sessions/real1.jsonl` (the real session of PROOF-RESUME.md) | `real1`, version 2 | 205 | 124 custom, 81 message | 81 from leaf `e28eef93` | 8 user, 49 assistant, 24 toolResult | anthropic / claude-opus-5 | 147 entries on the path, 81 messages: 8 user, 49 assistant, 24 toolResult |
| `<scratch>/proof5/canon/canon.jsonl` (the canon with one real example, below) | `canon`, version 2 | 4 | 1 custom, 3 message | 3 | 1 user, 2 assistant | anthropic / claude-opus-5 | same |
| `canon/canon.jsonl` (the repository's canon, header only) | `canon`, version 2 | 0 | | 0 | | | |

No migration ran (the files are already Pi's version 2), no line was skipped,
and Pi's context from the head equals lys-home's context path message for
message. The 12-entry fixture with a compaction and a moved leaf is exercised
by `record_tests::twelve_entries_one_compaction_and_a_moved_leaf_round_trip_through_pi_grammar`
in the same grammar; it is not run through Pi's reader here because the test
writes it to a temporary directory and removes it.

## One real example added to a scratch copy of the canon (R11 acceptance)

The repository's `canon/canon.jsonl` holds its header only: choosing the
examples every session starts from is a curation, and the first ones are to be
nominated and reviewed like code, not picked by the tool's author to make a
proof pass. The mechanics were proven on a copy.

```
lys-home canon add --canon <scratch>/proof5/canon/canon.jsonl --home <scratch>/proof5/resume/home --from real1 \
  --entries db6f7023-b557-4655-b8e8-42da7b0bedb5 82015a50-7192-4575-a49c-061b7d2ffc73 5e9041ce-… \
  --rule "verify before claiming" --by archie
```

| field | value |
| --- | --- |
| report | `{"examples":1,"inherited_id":"fe0bb6a315c9e756b80531276fba2659","messages":3,"thinking_copied":1}` |
| the file after | 5 lines: the header, one `lys.inherited` entry, three message entries (user, assistant with one signed thinking block, assistant with one tool call) |
| the inherited entry's data | `authored: false`, `from_session: real1`, `from_entries: [the three ids]`, `provider: anthropic`, `api: anthropic-messages`, `model: claude-opus-5`, `curated_at: 2026-09-24T05:32:00.238Z`, `curated_by: archie`, `rule: verify before claiming` |
| copied whole | every message body equals its source entry's body (`==` on the parsed JSON); the thinking block's `thinkingSignature` (932 bytes) equals the source byte for byte |
| parent links | rewritten to chain: inherited → user → assistant → assistant; the ids are the source ids |
| the same example again | refused: `session canon already holds an entry db6f7023-…`, exit 1, nothing written (still 5 lines) |
| Pi's reader | loads it: 4 entries, 3 context messages, model claude-opus-5 (table above) |

`record/canon_tests.rs` covers the same on synthetic entries, plus the authored
path (a turns file with a thinking block is refused by line; without one the
example carries provider, api and model `authored`) and a render with
`--canon` (the canon's messages first, one parentUuid chain through them;
for the same model the inherited thinking renders whole with its signature,
for another model as text with the signature named in the loss account).

## Does 2.1.281 replay a signed thinking block from a resumed file?

Measured as far as the files show (PROOF-RESUME.md): the rendered session
carried 17 assistant records each holding one signed thinking block; the fork
Claude Code wrote copied 64 of the 81 records and none of the 17 thinking
records. So a signed thinking block in a resumed file is not carried into
the continuation. Whether it was sent to the model on the first turn is not
visible in any file; it is measured through R10's proxy, on the first call
that proxy captures for a resumed file. Until then, the canon's thinking
blocks are kept whole for the same model because R4 says so and because
nothing yet shows them to be wasted, and a render for another model already
gives them as text.

## The canon-seeded and plain runs of one card

Not run yet. It needs two things that exist as of this proof but are not
joined: the card chain Waffles is proving on the Cambium defects board (a
card moved to in progress writes its brief, builds, opens the pull request,
lands), and a canon with reviewed examples. The measure will be, for one
named card run twice from the same base: fix rounds, and claims the run made
that its own gate did not verify. Both counts and the card's id go here when
it has run.
