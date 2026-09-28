# LOSS-ACCOUNT: what the Claude Code render keeps, changes and cannot carry (HOME-001 R4)

The render is `crates/lys-home/src/harness/claude_code/render.rs`. It walks the context path, with the canon placed first when one is given, and writes one Claude Code record for each message entry and one `summary` record for each compaction. Beside the rendered file it writes a loss account. This page says what the render does with each thing a home holds, as the code stands.

## The account file

It is written beside the rendered file as `<uuid>.loss.json`:

```json
{"session_id": "<target uuid>", "model": "<target model>", "authored": false,
 "dropped": [{"hash": "<sha256>", "reason": "<why>"}]}
```

- `hash` is the SHA-256 of the part's JSON exactly as the entry holds it. A reader can find the part in the home by that hash, and the account never holds the part itself.
- `reason` is one of the three fixed sentences under "What is lost" below. It names no content.
- `authored` is `true` when the context path holds a `lys.authored` entry. The render report says the same.
- The account holds hashes and reasons only, never a byte of transcript.

## What is kept

| home | rendered as |
| --- | --- |
| a user message's parts | a `user` record, parts as they are |
| a `toolResult` message | a `user` record holding one `tool_result` part, whose `tool_use_id` is the `toolCallId`, so the pair with its `tool_use` stays keyed by id |
| an assistant `text` part | a `text` part |
| an assistant `toolCall` part | a `tool_use` part: `id`, `name`, and `input` from `arguments` |
| a thinking part whose message's provider, api and model equal the target's | `thinking` with its `signature`, byte for byte |
| a redacted thinking part with the same provider, api and model | `redacted_thinking` with its data |
| the parent chain | `parentUuid` is the previous rendered record's `uuid`, and the first record's is null |
| an entry id that is already a uuid (an imported record) | the same uuid |

## What is changed

| home | rendered as | why |
| --- | --- | --- |
| a signed thinking part for a different provider, api or model | a `text` part holding the readable thinking, and one account row: `signed thinking rendered as text: different provider, api or model` | a signature is valid only for the model that made it (Pi's rule, `transform-messages.ts:95-109`) |
| an entry id that is not uuid-shaped (a split tool result's `<uuid>-r<i>`, a hand-authored id, a canon id) | a UUIDv5 under the session's namespace over `<entry id>#record` (ADR-016) | Claude Code requires a uuid, and the same head must render to the same bytes (CN9) |
| a compaction | `{"type": "summary", "summary": ..., "leafUuid": <previous record>}`, followed by the kept entries | Claude Code's own summary record |
| an assistant `stopReason` | `stop_reason`: `toolUse` becomes `tool_use`, `length` becomes `max_tokens`, anything else `end_turn` | Claude Code's vocabulary |
| the assistant message id | `msg_` followed by the first 8 characters of the record's uuid | derived, never drawn fresh |

## What is lost

Each part below is dropped and named in the account by hash:

- a redacted thinking part rendered for a different provider, api or model: `redacted thinking dropped: different provider, api or model`;
- a thinking part with no readable text, when it is not kept whole: `empty thinking dropped`;
- a signed thinking part turned into text (listed above under "changed"): the signature is lost and the row names it.

The entries below are not rendered at all. They are not message parts, so they have no account row, and the home still holds each of them:

- custom entries (every `lys.*` entry: calls, harness events, the authored marker, inherited, given, lanterns, forks), labels, `model_change`, `branch_summary` and `session_info`;
- everything off the context path: side leaves such as `permission_mode` and `tool_completed` events and render events, sidechains, and the entries a compaction left behind;
- a message of any role other than `user`, `assistant` and `toolResult`.

## What is written that the record did not carry

- These fields are the same on every record the render writes: `isSidechain: false`, `userType: "external"`, `gitBranch: ""`, and the target's `cwd`, `sessionId` and `version`.
- Every assistant record gets `stop_sequence: null` and `usage: {input_tokens: 0, output_tokens: 0}`.
- When a field is missing from an entry, the render currently puts in a default value: an empty string for a tool call's `id` or `name`, an empty object for its `input`, an empty `tool_use_id`, an empty `content`, `is_error: false` and an empty `model`. CN13 (ADR-031) says a missing field must refuse the render instead, by session, entry id, field and expected type. That constraint belongs to a later card and is **not yet met** here. Until it lands, a rendered file can carry a value the record never held.
