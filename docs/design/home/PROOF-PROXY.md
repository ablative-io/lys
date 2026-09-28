# PROOF-PROXY: a subscription login through a pass-through proxy (HOME-001 R7, R10)

**Status: not yet run. No outcome of the subscription proof is claimed here, neither `completed` nor `failed`.** This page says what is built and how it is tested. It also records the one attempt made from the seat that built it, which was stopped by that seat's own login before any call was made, and it sets out the run that is still owed, with the fields it must fill.

## What is built

- `crates/lys-home/examples/passthrough.rs`, the R7 pass-through. It forwards every request to the base URL given on its command line, and returns the response unchanged, through the one transport `lys-proxy` uses (`lys_home::proxy::forward::{Upstream, pass_through, serve}`). It writes one line per call on stderr, `{"method", "path", "status", "duration_ms"}`, when the response head arrives. The path is written without its query. It writes no header value and no body byte.
- `lys-proxy` (`crates/lys-home/src/bin/lys-proxy.rs`, `crates/lys-home/src/proxy/`), the R10 proxy. It forwards by path prefix (`/anthropic`, `/openai`) and records one `lys.call` per model call.
- The transport. It is HTTP/1 over rustls (ring, Mozilla roots), or plain HTTP to a loopback base, and it sets Hyper's `retry_canceled_requests(false)`. It passes every header as it came except `host`, which it sets to the upstream's authority because the provider serves its own name. It reads one response header, `content-type`, and reads it only to know whether the response is an event stream.

## What is tested without a provider

Each of these runs against a loopback fake, with no clock in any test. They are in `crates/lys-home/src/proxy/forward_tests.rs`, `link_tests.rs` and `journal_tests.rs`.

| row | test |
| --- | --- |
| R7: GET / answered 418 with the upstream's 3 chunks, in order | `the_pass_through_answers_with_the_upstream_status_and_its_three_chunks_in_order`. It runs the example's own code path (`pass_through` under `serve`), not a copy of it. The fake sends the next chunk only after the client holds the one before. |
| R10: 200 SSE events, the first delivered before the last is sent | `a_stream_of_200_events_reaches_the_client_before_its_last_event_is_sent` |
| R10: one keyed call is one `lys.call` whose request blocks are the request's parts | `one_keyed_call_is_one_lys_call_whose_request_blocks_are_the_request_parts` |
| R10: a call without a key lands under `unlinked` and the report names it | `a_call_without_a_key_lands_under_unlinked_and_the_report_says_so` |
| R10: an upstream disconnect produces exactly one upstream request | `an_upstream_that_drops_mid_request_sees_one_request_and_the_call_is_partial` |
| R10: a client that closes mid-stream is `cancelled`, with no response parts | `a_client_that_closes_mid_stream_is_recorded_cancelled_with_no_response_parts` |
| R10: an SSE stream that ends early is `partial` | `an_upstream_stream_that_ends_before_message_stop_is_partial` |
| R10: a capture directory made read-only is `unrecorded`, and the client gets the whole response | `a_read_only_capture_directory_is_unrecorded_and_the_client_gets_it_all` |
| R10: an open call is recorded `lost` by the next start, once | `a_call_open_when_the_proxy_stops_is_recorded_lost_by_the_next_start`, `a_restart_records_each_open_call_lost_once_with_what_was_spooled`, `a_call_whose_outcome_was_durable_is_not_recorded_lost`. The kill is simulated: a second start runs over the same state while the first call is still open, and no process is killed. |
| R10: two unkeyed calls never land under an earlier keyed session | `two_unkeyed_calls_after_a_keyed_one_never_land_under_its_session` |
| R10: a journal that cannot be written refuses the call, with zero upstream requests | `a_journal_that_cannot_be_written_refuses_the_call_before_it_is_sent` |
| R10: a journal lost after admission still forwards, then records `unrecorded` once it can be written | `a_journal_lost_after_admission_forwards_and_records_unrecorded_once_writable` |
| R10: with a capture bound of 0, a keyed call is `unrecorded` under its own session | `with_no_capture_slot_a_keyed_call_is_unrecorded_under_its_own_session` |

## The attempt on 28 September 2026

The seat that built this card tried to measure Claude Code's session key first, as R10 requires before linking is relied on. It pointed `claude -p` at a local listener that kept only header names and the shape of `metadata`, never a value.

| field | value |
| --- | --- |
| Claude Code version | `2.1.283 (Claude Code)` (`claude --version`). This is not 2.1.281, the version the brief names. |
| command | `ANTHROPIC_BASE_URL=http://127.0.0.1:18765 claude -p --strict-mcp-config --mcp-config '{"mcpServers":{}}' --max-turns 1 "Say ok"`, run from a scratch directory |
| outcome | Claude Code exited 1 before sending anything: its OAuth session had expired and could not be refreshed. The listener saw **zero requests**. |
| status codes seen | none |
| what was stored | nothing. The listener and its output were removed, and no credential was read or copied. |

This attempt measured the seat, not the proxy. It is not the R7 proof.

## The run that is owed

The run needs a seat whose Claude Code subscription login is live. Use the proof account, not a live seat's session, and copy no credential anywhere.

1. `cargo run -p lys-home --example passthrough -- https://api.anthropic.com 127.0.0.1:8485`
2. From a scratch directory, run `ANTHROPIC_BASE_URL=http://127.0.0.1:8485 claude -p --strict-mcp-config --mcp-config '{"mcpServers":{}}' --max-turns 1 "Say ok"`.
3. Record: `claude --version`; `completed` or `failed`; every status code the stderr lines show; which headers had to pass (the answer expected is all of them unchanged except `host`, and the run should confirm or refute it); and what failed, if anything did.
4. Measure the session key, by field name and spelling only. Record which member of the request body names the Claude Code session in the version measured. `lys-proxy` reads `metadata.user_id` in two spellings, `user_<hex>_account_<uuid>_session_<uuid>` and a JSON object string with `session_id` (`crates/lys-home/src/proxy/link.rs`). If the measured field is neither, `link.rs` changes before `lys-proxy` is relied on.
5. Run `lys-proxy --home <scratch home> --state <scratch state>` with `ANTHROPIC_BASE_URL=http://127.0.0.1:8484/anthropic`. Record the report line it prints for the call and the `lys.call` entry's status, by id and hash only.

Until steps 1 to 4 are recorded here, R7's proof row and R10's "measured first" condition are open.
