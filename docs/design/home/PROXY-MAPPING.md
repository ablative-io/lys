# How a proxy call record maps onto the harness's session file

Slice 3 of PROXY.md. Written by Waffles from Vesper's line-by-line read of one real run, 3 October
2026; every line below was read off disk by her, nothing is guessed. The run: the slice 1 proof
agent, Claude session `9918e737-4383-4a4b-842a-dc193bf3c264`, three model calls through the proxy on
the installed 9f3fedfb.

## What this document is for

In PROXY.md's words: a call's record is linked to its run by the session id the harness puts in the
request body (`metadata.user_id` for Claude Code), never by guessing from timing; the session files
the harness writes stay where they are and are not rewritten by Lys. This page says how the proxy's
API records map onto those session files, so a reader can move from either side to the other.

## The two sides

**A. The proxy side (Lys).** `~/Library/Application Support/lys/identity/data/proxy/home/sessions/<claude-session-id>.jsonl`,
framed by `<claude-session-id>.index.jsonl` (one line per record: `{id: 32 hex, parent, offset, len,
custom: "lys.call"}`; offset and len slice the record out of the `.jsonl`), with `.head` and `..lock`
beside them. Bodies live under `data/proxy/home/blocks/<first two hex>/<sha256>`, content-addressed.
One record kind: `type: "custom", customType: "lys.call"`.

**B. The harness side (Claude Code).** `~/.claude/projects/<cwd with slashes turned to dashes>/<claude-session-id>.jsonl`,
one JSON object per line, never rewritten by Lys. Record kinds seen in this run's file: `mode`,
`permission-mode`, `atis-latch`, `file-history-snapshot`, `user`, `attachment` (subkinds
`agent_listing_delta`, `credential_org`, `date`, `environment`, `hook_success`, `instructions`,
`mcp_instructions_delta`, `model`, `prompt_snapshot`, `remote_session_change`, `session_context`,
`skill_listing`), `last-prompt`, `ai-title`, `assistant`, `system` (`stop_hook_summary`,
`turn_duration`), `cost-state`.

## The link

1. **Run to run, by file name and id.** The proxy session file name, the harness session file name
   and `sessionId` on every harness record are the same id. The proxy learns it from
   `metadata.user_id` in the request body, present on all three calls and the only key under
   `metadata`. This is the run-level link and it holds today.
2. **Call to assistant record, by message id.** A streamed call's stored `raw_response` (gzip, SSE)
   begins with `message_start`, whose `message.id` equals the harness `assistant.message.id`. Call 3's
   stored response holds the id of the assistant record on disk, so a streamed call pairs to its
   assistant record by id, not by time. Since c627d5cb the `lys.call` record carries it as
   `message_id`, read as `message_start` passes, so the pairing needs no decode; it is there for a
   stream that stopped short too. Not yet: a Messages response that is not an event stream, and the
   two OpenAI grammars.
3. **The harness `requestId` (`req_…`) appears nowhere in the stored bodies.** It comes from a
   response header. Since 5790186b the record carries it as `request_id`, and `head` holds the
   upstream's status and, of each side, every header's name in the order received and the values
   of a named few. Which keep their values is one list in `crates/lys-home/src/proxy/headers.rs`,
   with the credential headers named there as never valued; no header leaves no trace.
4. **Call 1 has no pair by id.** A non-stream request (`model, max_tokens, messages[1], metadata`; no
   system, no tools) answered by a 114-byte response block that is not gzip, zlib, zstd or JSON
   (first bytes `83 38 00 00`). The proxy stored it whole and did not decode it, and the record says
   Unrecorded with both bodies stored and no reason. That is slice 1's finding 1. The record now
   names its reason (`unrecorded_reason`), and for a response that is not JSON as stored the reason
   says what the response's own `content-encoding` named, with the request's `accept-encoding`
   beside it in `head`. The encoding of that first call is therefore read off the next run's
   record, not guessed from four bytes. What is still not done: a response that is not an event
   stream is read as stored and never decoded, so an encoded one stays unrecorded, with its reason.
5. **Model and time agree but are not the link.** `data.model` on the call equals
   `assistant.message.model` and `attachment.identity.modelId`. Timestamps align to the second (call
   3's record time is the assistant record's timestamp; calls 2 and 3 `started_at` are the user
   turn). Useful to a reader, never the join.

## The `lys.call` record, field by field

```
{id, parentId, timestamp, type: "custom", customType: "lys.call",
 data: {api: "anthropic-messages", provider: "anthropic", call_id, model, stream, status,
        started_at, duration_ms,
        message_id, request_id, unrecorded_reason,
        head: {status, request: {names: [<every name>], values: {<name>: [<values>]}},
               response: {names: [<every name>], values: {<name>: [<values>]}}},
        raw_request: <block hash>, raw_response: <block hash>,
        request: [<part block hashes>], response: [<part block hashes>],
        capture: {admission_ns, drain_ns, hash_ns, write_ns, block_syncs, spool_syncs,
                  spool_writes, spool_bytes, pending_bytes, refusals: [],
                  request_placement, response_placement,
                  request_spool_kept, response_spool_kept,
                  durable: Measured | NotPlaced}}}
```

The request block is the exact request body JSON (keys seen: `model`, `max_tokens`, `messages`,
`metadata`, and on the tool calls `system`, `tools`, `stream`, `output_config`, `thinking`,
`context_management`). The response block is the exact upstream bytes as received (gzip when upstream
gzipped).

## What each side lacks

**The proxy record lacks:** the value of every header not on the kept list (its name is there); usage and cost (the harness keeps `message.usage` and
`cost-state`; the proxy has them only inside the raw SSE `message_delta`); cwd, version, gitBranch,
hooks, permission mode; the person's prompt as a turn (only as part of the request body); tool
results as the harness sees them.

**The harness file lacks:** the raw request and response bytes; the system prompt and tool list as
sent; every timing the proxy measures (admission, drain, hash, write, time to durable); placement and
durability; the upstream used; whether the call was lost or unrecorded.

## Lines for the next proxy slice, from this read

Done, each its own commit: `message_id` (c627d5cb), `request_id` and `head` (5790186b), and
`unrecorded_reason` with the response's named encoding.

Next, from what those three leave open:

- Read the first call of a run off its new record: its `head` says what the client offered and what
  the response named, which settles the 114-byte body.
- Decode a response that is not an event stream by its named encoding before reading it as JSON,
  so such a call is complete instead of unrecorded.
- `message_id` for a Messages response that is not an event stream, and for the two OpenAI
  grammars.
