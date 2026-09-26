# PROOF-LANTERN: a home file holding lanterns and epilogues, read by Pi's parser (HOME-004 R7)

Measured 26 September 2026 on this Mac, with the lys-home binary built from
card/home-004-lanterns at 8ccc79c (R6 landed in the tree, R7 under test), bun
1.4.0, and the Pi checkout at `3d5cbe98` under `$PI` (a clone of
github.com/earendil-works/pi; `git cat-file -t 3d5cbe98` answers `commit`
and HEAD is that commit).

## The session

The R4 fixture shape, written by the crate itself and never by hand: five
message entries, two lanterns at the second, two epilogues on the first
lantern. Built in a scratch home (`<scratch>/home`, not committed) with:

```
lys-home import --home <scratch>/home --claude-code <scratch>/transcript.jsonl --session fixture-lantern
lys-home lantern light --home <scratch>/home --session fixture-lantern --point <e2> --note "The Fold Held Under Replay" --by fixture-lighter
lys-home lantern light --home <scratch>/home --session fixture-lantern --point <e2> --note "second look" --by fixture-lighter
lys-home lantern epilogue --home <scratch>/home --session fixture-lantern --lantern <L1> --words "and the replay fold rings true" --by fixture-annotator
lys-home lantern epilogue --home <scratch>/home --session fixture-lantern --lantern <L1> --words "a later word: cobalt" --by fixture-annotator
```

`transcript.jsonl` is five plain `user` records in Claude Code's shape,
each of one short fixture line, so the import writes five message entries and
no event. `<e2>` is the second record's uuid; `<L1>` is the `id` the first
light printed. The session file after the four appends is 10 lines (the
header and 9 entries), SHA-256
`317a4fb555a6101f3c6e401054c19a3cd950e59d869da0c89e2694755f7d9bd6`.

The epilogue reports printed, verbatim (ids, names and times; never the
words):

```
{"added_at":"2026-09-26T04:12:04.439Z","added_by":"fixture-annotator","id":"9f14755b5975b7d8c684f3dcb7eb252a","lantern":"2f4603d275343c40d582582d4adee324","ordinal":1,"session":"fixture-lantern"}
{"added_at":"2026-09-26T04:12:04.473Z","added_by":"fixture-annotator","id":"5a72a03516f209a7516cb7a1f68a140d","lantern":"2f4603d275343c40d582582d4adee324","ordinal":2,"session":"fixture-lantern"}
```

## Pi's parser on it

`parseSessionEntries` is imported from the checkout's own source and given
the file's text; the script prints counts only, never an entry's content.

`<scratch>/pi-parse-lantern.ts`:

```ts
// Parse session files with Pi's own reader at 3d5cbe98 and print counts only: never an entry's content.
import { parseSessionEntries } from "/Users/tom/Developer/tools/harness/pi/packages/coding-agent/src/core/session-manager.ts";
import { readFileSync } from "fs";
for (const file of process.argv.slice(2)) {
  const entries = parseSessionEntries(readFileSync(file, "utf8"));
  const header = entries[0] as any;
  const rest = entries.slice(1) as any[];
  const custom = rest.filter((e) => e.type === "custom");
  const withKeys = rest.filter((e) => "id" in e && "parentId" in e && "timestamp" in e);
  console.log(JSON.stringify({
    header_type: header?.type, header_id: header?.id, header_version: header?.version,
    entries: rest.length, entries_with_id_parent_timestamp: withKeys.length,
    message: rest.filter((e) => e.type === "message").length,
    custom: custom.length,
    lantern: custom.filter((e) => e.customType === "lys.lantern").length,
    lantern_epilogue: custom.filter((e) => e.customType === "lys.lantern_epilogue").length,
  }));
}
```

The run and its output:

```
$ bun run <scratch>/pi-parse-lantern.ts <scratch>/home/sessions/fixture-lantern.jsonl
{"header_type":"session","header_id":"fixture-lantern","header_version":2,"entries":9,"entries_with_id_parent_timestamp":9,"message":5,"custom":4,"lantern":2,"lantern_epilogue":2}
```

The header and 9 entries, every entry carrying `id`, `parentId` and
`timestamp`, of which exactly 2 have `customType` `lys.lantern` and
exactly 2 `lys.lantern_epilogue`, as R7's third acceptance line states
(CN4).

## The render (R7's first two lines)

`crates/lys-home/tests/lantern_home.rs` builds the same shape in a fresh
home, renders it for Claude Code beside a render of a copy taken before the
first light, and finds 0 lines containing `lys.lantern`, the note or an
epilogue's word, and the same line count (5) in both.
