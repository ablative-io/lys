# The loss account of the Claude Code render

Written from commit d41fa4b3be152a47eb380116800dcaf2a67e4d12.

Every line this document cites is read at that commit, written as a
repository-relative path, a colon and a line number or a first-last range. It
records the code as found: it names keys, reasons, line citations and counts,
and carries no text, signature or hash from any session.

## The loss account

**Where it is written.** The account is written beside the rendered file and
named by the rendered file's stem with `.loss.json`, because the render
computes its path with `path.with_extension("loss.json")`
(crates/lys-home/src/harness/claude_code/render.rs:263). The stem is the
session uuid in two cases only:

- the default output path of `lys-home render`, which is `<session id>.jsonl`
  under Claude Code's own directory for the cwd
  (crates/lys-home/src/harness/claude_code/render.rs:85-93);
- `render-launch`, whose targets are `{uuid}.jsonl` and `{uuid}.loss.json`
  under its `--out` directory
  (crates/lys-home/src/harness/claude_code/launch.rs:99-100).

Otherwise the stem is whatever path the caller gave: `lys-home render` takes
an optional `out` argument (crates/lys-home/src/cli.rs:131-133), so
`lys-home render --out other.jsonl` writes the account as `other.loss.json`.

Plain render writes the account with `std::fs::write`, which replaces a file
already at that path (crates/lys-home/src/harness/claude_code/render.rs:265-269).
`render-launch` refuses an existing loss path by name, with
`LaunchTargetExists`, before it writes any file
(crates/lys-home/src/harness/claude_code/launch.rs:127-133), and hashes the
account as the second file of its render manifest, after the rendered file
(crates/lys-home/src/harness/claude_code/launch.rs:155-165).

**Its shape.** The account is one JSON object with exactly four keys
(crates/lys-home/src/harness/claude_code/render.rs:264):

- `session_id`: the session id the rendered file carries;
- `model`: the model the file is rendered for;
- `authored`: true when an entry on the rendered path is the `lys.authored`
  custom entry (crates/lys-home/src/harness/claude_code/render.rs:128);
- `dropped`: an array of entries.

One entry is the `Loss` struct's two string fields, `hash` and `reason`
(crates/lys-home/src/harness/claude_code/render.rs:48-55).

**Every render writes it.** The account is written with no condition on what
was lost (crates/lys-home/src/harness/claude_code/render.rs:263-269), so a
render that drops nothing still writes an account, whose array reads
`dropped: []`. Two measured runs show it:
docs/design/home/PROOF-RESUME.md:38 and docs/design/home/PROOF-LAUNCH.md:28.

As found at the named commit, and not as intended behaviour, the account is
serialised with `serde_json::to_vec_pretty(&account).unwrap_or_default()`
(crates/lys-home/src/harness/claude_code/render.rs:267): a serialisation
failure there writes an empty file and returns no error. This is recorded as
the code stands, not as a guarantee.

**The reasons.** Every reason string the source passes to the loss
constructor, copied character for character, one per line, with the line that
writes it:

- reason: "signed thinking rendered as text: different provider, api or model" (crates/lys-home/src/harness/claude_code/render.rs:198)
- reason: "redacted thinking dropped: different provider, api or model" (crates/lys-home/src/harness/claude_code/render.rs:201)
- reason: "empty thinking dropped" (crates/lys-home/src/harness/claude_code/render.rs:201)

**When an entry is written.** Only the thinking branch writes an entry
(crates/lys-home/src/harness/claude_code/render.rs:184-202). An assistant
message is the same as the target when its provider, api and model all equal
the target's (crates/lys-home/src/harness/claude_code/render.rs:170-174): the
provider `PROVIDER` and the api `API`
(crates/lys-home/src/harness/claude_code/names.rs:7 and
crates/lys-home/src/harness/claude_code/names.rs:9, re-exported by
crates/lys-home/src/harness/claude_code/mod.rs:50), and the model the render
is for. A thinking part is then decided in this order:

1. When the message is the same as the target and the part is redacted, or
   carries a signature, it is kept whole and no entry is written.
2. Otherwise, when its text is not empty after trimming whitespace and it is
   not redacted, it is rendered as a text part. An entry with the signed
   thinking reason is written only when the part carries a signature
   (crates/lys-home/src/harness/claude_code/render.rs:197); a thinking part
   with no signature rendered as text writes no entry, for the same model as
   for another.
3. Otherwise it is dropped, with an entry carrying the redacted reason when it
   is redacted and the empty reason when it is not. So an empty thinking part
   with no signature is dropped even for the same model, and a signed empty
   thinking part for another model carries the empty reason.

The array named `dropped` therefore holds entries for signed thinking rendered
as text as well as for dropped parts: not every entry names a dropped part.
The report's `dropped` count is the number of entries in that array
(crates/lys-home/src/harness/claude_code/render.rs:279), while
`thinking_as_text` counts every thinking part rendered as text, signed or not
(crates/lys-home/src/harness/claude_code/render.rs:196). No part other than
thinking writes an entry: text, tool calls and every other part type are
rendered without one (crates/lys-home/src/harness/claude_code/render.rs:183
and crates/lys-home/src/harness/claude_code/render.rs:204-210).

**An entry names its part by hash.** An entry's `hash` is the SHA-256 of the
part as serialised by `serde_json::to_vec` from the assistant message, written
as 64 lowercase hexadecimal characters
(crates/lys-home/src/harness/claude_code/render.rs:286-292 and
crates/lys-home/src/record/blocks.rs:19-20). Object keys are serialised in
sorted order, because serde_json's `preserve_order` feature is not enabled in
the workspace. For an imported part this equals the hash the importer stored
the part's block under, because the importer serialises a part the same way
before it stores it
(crates/lys-home/src/harness/claude_code/import/content.rs:43-47). A part that
came from a canon or a hand-authored session may name no block in the store,
so an entry's hash is not a promise that a block exists.

An entry carries only the hash and the reason, never the part's thinking text,
signature or redacted data. As found at the named commit, the part's
serialisation uses `unwrap_or_default()`
(crates/lys-home/src/harness/claude_code/render.rs:287), so a serialisation
failure there hashes zero bytes.

## What the render changes without a loss entry

Each of these is recorded as found at the named commit, with the line that
does it:

- Custom entries, the lys ones included, and labels are not rendered
  (crates/lys-home/src/harness/claude_code/render.rs:9-10 and
  crates/lys-home/src/harness/claude_code/render.rs:237).
- A compaction becomes a `summary` record carrying the compaction's summary
  and the previous record's uuid as `leafUuid`
  (crates/lys-home/src/harness/claude_code/render.rs:234-236).
- Every record's `gitBranch` is written as the empty string
  (crates/lys-home/src/harness/claude_code/render.rs:148).
- Every assistant record's `usage` is written as zero input and zero output
  tokens (crates/lys-home/src/harness/claude_code/render.rs:226).

None of these writes a loss entry. Whether any of them should is a question
for its own card, which this document does not answer.
