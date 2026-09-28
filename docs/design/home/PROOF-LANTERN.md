# PROOF-LANTERN: a home file holding lanterns and epilogues, read by Pi's parser (HOME-004 R7)

Measured 26 September 2026 on this Mac, with the lys-home binary built from
card/home-004-lanterns at 8ccc79c (R6 landed in the tree, R7 under test),
node v26.4.0, and the Pi checkout at `3d5cbe98` under `$PI`: a scratch
clone of github.com/earendil-works/pi (`git clone --shared` of the local
mirror, `git checkout 3d5cbe98`; `git rev-parse HEAD` answers
`3d5cbe98c3bc67ef8433bdeee45fbe5f0d8a24db` and `git status --short` is
empty), with its locked dependencies installed and its packages built by
its own tools, unchanged.

## The session

The R4 fixture shape, written by the crate itself and never by hand: five
message entries, two lanterns at the second, two epilogues on the first
lantern. Built in a scratch home (`<scratch>/home`, not committed) with:

```
lys-home import --home <scratch>/home --claude-code <scratch>/transcript.jsonl --session fixture-lantern
lys-home lantern light --home <scratch>/home --session fixture-lantern --point <e2> --note <note> --by fixture-lighter
lys-home lantern light --home <scratch>/home --session fixture-lantern --point <e2> --note <second note> --by fixture-lighter
lys-home lantern epilogue --home <scratch>/home --session fixture-lantern --lantern <L1> --words <words one> --by fixture-annotator
lys-home lantern epilogue --home <scratch>/home --session fixture-lantern --lantern <L1> --words <words two> --by fixture-annotator
```

`transcript.jsonl` is five plain `user` records in Claude Code's shape,
each of one short fixture line, so the import writes five message entries and
no event. `<e2>` is the second record's uuid; `<L1>` is the `id` the first
light printed; `<note>`, `<second note>`, `<words one>` and `<words two>`
stand for the fixture's note and epilogue words, which the proof does not
carry (P7). The session file after the four appends is 10 lines (the
header and 9 entries), SHA-256
`317a4fb555a6101f3c6e401054c19a3cd950e59d869da0c89e2694755f7d9bd6`.

The epilogue reports printed, verbatim (ids, names and times; never the
words):

```
{"added_at":"2026-09-26T04:12:04.439Z","added_by":"fixture-annotator","id":"9f14755b5975b7d8c684f3dcb7eb252a","lantern":"2f4603d275343c40d582582d4adee324","ordinal":1,"session":"fixture-lantern"}
{"added_at":"2026-09-26T04:12:04.473Z","added_by":"fixture-annotator","id":"5a72a03516f209a7516cb7a1f68a140d","lantern":"2f4603d275343c40d582582d4adee324","ordinal":2,"session":"fixture-lantern"}
```

## Pi's parser on it

Pi's source imports `uuid` and names its own modules as `.js`, so under node
it runs as Pi ships it: dependencies installed from the checkout's lockfile
and the packages compiled by the checkout's own TypeScript build, in the
order the root build script names, nothing edited.

```
$ cd $PI && npm ci --ignore-scripts --no-audit --no-fund --cache <scratch>/npm-cache
$ for pkg in tui ai agent coding-agent; do (cd $PI/packages/$pkg && npx tsgo -p tsconfig.build.json); done
```

`parseSessionEntries` is then imported from the built checkout through
`$PI` and given the file's text; the script prints counts only, never an
entry's content.

`<scratch>/pi-parse-lantern.mjs`:

```js
// Parse session files with Pi's own reader at 3d5cbe98 (the checkout under $PI, built with its own tsgo), printing counts only: never an entry's content.
import { readFileSync } from "node:fs";
const pi = process.env.PI;
if (!pi) { console.error("PI is not set: the Pi checkout at 3d5cbe98"); process.exit(2); }
const { parseSessionEntries } = await import(`${pi}/packages/coding-agent/dist/core/session-manager.js`);
for (const file of process.argv.slice(2)) {
  const entries = parseSessionEntries(readFileSync(file, "utf8"));
  const header = entries[0];
  const rest = entries.slice(1);
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
$ PI=<scratch>/pi node <scratch>/pi-parse-lantern.mjs <scratch>/home/sessions/fixture-lantern.jsonl
{"header_type":"session","header_id":"fixture-lantern","header_version":2,"entries":9,"entries_with_id_parent_timestamp":9,"message":5,"custom":4,"lantern":2,"lantern_epilogue":2}
```

The header and 9 entries, every entry carrying `id`, `parentId` and
`timestamp`, of which exactly 2 have `customType` `lys.lantern` and
exactly 2 `lys.lantern_epilogue`, as R7's third acceptance line states
(CN4). The session file is the one the section above hashes
(`317a4fb5…d9bd6`, unchanged). An earlier run of the same script's
predecessor under bun 1.4.0, importing the checkout's `.ts` source
directly, answered the same counts.

## The render (R7's first two lines)

`crates/lys-home/tests/lantern_home.rs` builds the same shape in a fresh
home, renders it for Claude Code beside a render of a copy taken before the
first light, and finds 0 lines containing `lys.lantern`, the note or an
epilogue's word, and the same line count (5) in both.

## Lines of code per file (the brief's verification)

A line of code is a non-blank line that is not a comment, counted per file
with `grep -vcE '^\s*$|^\s*//' <file>` over every `.rs` file under
`crates/lys-home/src`, tests included. The eight largest, measured on this
tree after the rebase onto main `dfcca65` and the review changes. Every non-test file is at most
500; the one file over it, `src/record/call_tests.rs` at 519, is a test
file the repository's rule excludes, held by main before this card and not
changed by it:

```
$ cd crates/lys-home && for f in $(find src -name '*.rs' | sort); do printf "%s %s\n" "$(grep -vcE '^\s*$|^\s*//' $f)" "$f"; done | sort -rn | head -8
519 src/record/call_tests.rs
473 src/harness/claude_code/import.rs
460 src/record/call.rs
457 src/record/mod.rs
430 src/cli.rs
407 src/harness/claude_code/given_tests.rs
396 src/record/index.rs
393 src/record/record_tests.rs
```
