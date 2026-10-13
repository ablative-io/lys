# The Lys proxy: every model call through one place

Written by Waffles from Tom's words of 3 October 2026 (10:18 to 11:14, on Dot and in the
terminal). Where this page and the code differ, the code is wrong until Tom says otherwise.

## What it is for

Every call a Lys-started AI makes to a model API goes through Lys's own proxy, on this
machine, and nothing else: no hooks, no second parser of the session files, no wrapper around
the harness. The proxy is the one place that sees every request and every response, so it is
where Lys records, measures, enforces and, later, improves a run. Tom: "it's not just an
observability thing. Observability is critically important to it, but over time… bring down
token costs, decrease response times, decrease downtime, provide notifications… these would
all be functions that were built around this."

It must be "very extensible" and "seriously efficient… nothing in here can have even an ounce
of fat on it."

## The shape: one pass, two kinds of function

The proxy forwards bytes. It reads each frame of a request or a response once, hands the frame
to every function that has asked for it, and sends it on. Nothing parses the stream a second
time. Every function declares which of two kinds it is:

- An **observer** sees the frame as it passes and can never hold it. Capture, the heads-up
  display, notifications, background compaction, anything feeding speech, usage counting.
  An observer that is slow delays nothing but itself.
- A **gate** holds the call until it answers. A policy rule, a check by another model (a Jev
  call), trimming a request before it goes up. A gate declares that it blocks, and the proxy
  records how long each gate held each call, on the call's record, so the cost is a measured
  number on every call and a gate that drifts shows up by name.

Where a gate can sit: the response is a token stream, so a gate holds either the request
before it goes up, or a whole response before it comes down. It never holds a single token
mid-stream without buffering the stream, and that buffering is itself a declared gate.

Speech stays on the observer side, always: nothing Tom says or hears waits on a gate.

## The budget Tom discussed

Inline cost a call may carry from the proxy's own work: about 100 milliseconds at the outside,
measured per call and recorded. A network hop is about 20 milliseconds; the work at the far
end is microseconds. Background work (an observer) may take seconds and nobody notices; a
gate that takes 600 milliseconds (a model call) is fine when a person chose it. In real-time
speech every half second is noticed. These are the figures Tom gave and agreed; they are not
arbitrary limits, and a new limit nobody brought to him is refused the first time he hears
of it.

## Capture: everything, whole, now

- Every call that passes is stored whole: request, response, every header by name with its value, timing.
  The one exception is a header that carries a credential: its name is kept and its value never is (Tom,
  5 October: the values "not kept" should be kept).
  There is no size bound, no slot count, no state that ends a call "unrecorded" because of
  the proxy's own budget. Tom: "if it comes back from an API, we don't apply a size limit… no
  arbitrary limits… absolutely not under any circumstances." If the API did not refuse it and
  the harness would not have, Lys does not either.
- Storing is a milliseconds operation to haematite, off the forwarding path. Nothing hangs or
  holds up the call to be recorded.
- A call names the Lys run that made it by the run key: thirty-two random hexadecimal digits
  the launch mints for each start and puts first on the path of the proxy address it gives the
  run (`http://…/<key>/anthropic`). The proxy takes the key off before it forwards, writes it on
  the call's record, and counts the call's usage against that run; a first path part that is
  not a key is refused by name. Nothing is guessed from timing, and nothing depends on one
  harness's request shape, so a Codex run is counted the same way as a Claude Code run.
- The record also keeps the session id the harness puts in the request body
  (`metadata.user_id` for Claude Code), which links the call to the harness's own session
  file. The two links do different work: the key says whose run, the session id says which
  of the harness's files.
- Amended 4 October 2026 by Waffles. The page first said the session id alone linked a call
  to its run. The runner never learns a harness's session id without reading its session
  files, which this page forbids, so usage could not be counted per run that way. Put to Tom
  as the page against the code, he answered: "these sound like really technical questions,
  which I would defer to you to make like the most robust, most future-proof, best possible
  decision possible", and "I really don't want you waiting on me." The run key is that
  decision. Known and not closed: any program on the machine can send a key of the right
  shape, or none, so a call can be counted against another run or against no run; closing it
  means the proxy takes only keys the service has told it are live.
- The session files the harness writes stay where they are and are not rewritten. A
  document says how the API records map onto the session files, so a reader can move from
  either to the other.
- A call in flight when the proxy restarts is recorded `lost` on the next start; a call whose
  response was durable is never recorded lost. These states are the record's, not a reason
  to drop anything.
- A call waits at admission only for its journal record to be put in place, and again only for
  that when its request has named its session. The syncs that make the record hold through a
  loss of power are made beside the call, never in its way. So a call in flight when the proxy
  ends is found at its next start and recorded `lost`; a call admitted in the last moments
  before the computer itself lost power may leave no record. (Until 4 October 2026 both syncs
  were on the call's path: one call read on the installed build was held 12.7 ms at admission,
  against Tom's "no latency".)

## Later functions, all built on the same pass

- Trim long tool results on the way up and keep them under a key, so the model can ask for
  the whole thing (Tom's Headspace idea).
- Inject typed text into the stream for the model: a heads-up display, channel notifications,
  the result of background work, as structured blocks the model is told about once.
- Compaction in the background, off the critical path, with the result injected.
- The memory index (generated from the notes' frontmatter, see MEMORY.md) injected as one typed
  block after each compaction and every two hundred thousand tokens or so; the system prompt is
  never touched, because it is the prompt cache (Tom, 3 Oct 13:46 and 14:21).
- Token by token to speech: Claude Code streams responses and tool-call input as fragments,
  so an observer can hand the words of a Dot say to speech while the call is still being
  written. Dot becomes an adapter on the stream.
- Usage: five-hour and seven-day windows per account, per model, read from the responses that
  pass, shown on the budgets tab.
- The burn room: a run whose work needs it goes through a local or zero-retention model behind
  the same proxy; parts of memory that are burn-room-only never leave it.
- Managed settings force the harness onto the proxy, so a run cannot go around it.

## How it is installed and started

The proxy is `lys proxy serve`, a subcommand of the `lys` program, the way the runner is: its
own Unit with a pid file, an exit lock and a log, placed, kept and put back with `lys`; no
new program in the install's set. The install writes the proxy's address into the identity
server's configuration; the launch puts `ANTHROPIC_BASE_URL` (and the OpenAI equivalent for
Codex, when that slice comes), carrying the run key first on its path, into the run's
environment. Beside it goes `LYS_RUN`, the same key by name, with the launch's other `LYS_`
variables (`LYS_MACHINE`, `LYS_PROVISIONING_VERSION`), so the service reads the key from a
named variable and never by taking an address apart. Nothing else is set, and nothing that
changes how the harness behaves. A machine whose
login already carries its own `ANTHROPIC_BASE_URL` (a gateway) is not silently overridden:
the proxy forwards to that upstream, and the record names it. On upgrade the proxy restarts
after the services, the way the runner does.

## Codex through the proxy (measured 4 October 2026, codex-cli 0.159.2)

What Codex does was measured against a local listener that recorded only header names and the
shape of the body, never a credential or any content:

- `OPENAI_BASE_URL` in the environment is ignored. The call went straight to the provider.
- With `openai_base_url` set, Codex opens a WebSocket (`GET <base>/responses`, `Upgrade: websocket`,
  `OpenAI-Beta: responses_websockets=2026-02-06`). The proxy reads HTTP calls as they pass and
  cannot read a WebSocket, so this setting is not used.
- With a provider of its own named on the command line
  (`-c model_providers.lys={name=..., base_url=..., wire_api="responses", requires_openai_auth=true,
  supports_websockets=false}` and `-c model_provider="lys"`), Codex sends `POST <base>/responses`
  with `accept: text/event-stream`, a JSON body with `stream: true`, and it also asks
  `GET <base>/models?client_version=...`. Signed in with ChatGPT it sends `authorization` and
  `chatgpt-account-id`; it names its session in `session-id` and its thread in `thread-id`.

So a Lys launch gives Codex those two settings, with the base `http://<proxy>/<run key>/openai/v1`,
and the run key in `LYS_RUN`. Nothing in the machine's own Codex configuration is written. The proxy
forwards an `/openai` call that carries `chatgpt-account-id` to ChatGPT's Codex backend
(`https://chatgpt.com/backend-api/codex`, whose paths carry no `/v1`) and any other to the API: the
machine's own Codex sign-in decides, by one header lookup. A Codex call is linked to its harness
session by the `session-id` header, as a Messages call is by `metadata.user_id`.

Not yet measured, and named as gaps until a Codex agent has been started through an installed Lys:
the response stream as ChatGPT's backend sends it, and the names of the headers that carry the
account's windows (the `x-codex-` family is kept with its values, so the first real call shows them).
Codex also marks a sub-agent's calls (`x-openai-subagent`, `x-codex-parent-thread-id` appear in the
binary): that is the way to tell a main thread's context from a side call's, and is not yet used.

## Slices, each landed and read back before the next

Tom, 11:11: keep it smaller today, one slice at a time, nobody waiting on anyone.

1. **The pipe** (Archie, in progress). `lys proxy serve`, install wiring, the one variable in
   the run's environment, restart order. Handback: a Lys-started Claude run's calls go through
   the proxy and nowhere else (proved by the proxy's own journal, not by the harness's say-so);
   with no proxy configured nothing changes; a login's own upstream is respected and named;
   clippy clean, nextest green on the touched crates, the hospital line.
2. **Capture whole, to haematite** (Brisket, under Archie's review, after 1). Every bound and
   slot count out of lys-home proxy/ (DECODED_LIMIT, the capture slots, the metadata scanner's
   byte budget, the connection limit); storage to haematite in milliseconds off the forwarding
   path; the `lost`/`unrecorded` states kept only for what they mean; the leaky journal test
   fixed. Handback: a response larger than the old 16 MiB bound is recorded whole; the time
   from last byte to durable record is on the record and is milliseconds; no test proves less
   than before; the hospital line.
3. **The mapping document** (Waffles with Vesper). How each API record maps onto the harness's
   session files, with one real run's records and files side by side.
4. **Conformance**: bytes in equal bytes out, request and response, proved on a real run
   against a recording taken without the proxy.
5. Then the functions above, each as its own slice, each declared observer or gate, each with
   its hold time on the record.

## What this page is checked against

Each slice's handback is read against this page, not against memory. A slice that adds a
limit, a second parse of the stream, a hook, or a function that holds the call without saying
so is handed back.


## Named model accounts

Source implementation: `3ba633f5b2d19a6cd91fcdef8733f65f643c6836`. Full gates and installation are not confirmed.

A model account is a `credential` entry with a provider route. No new stored class or
required configuration key is introduced. Add the value once through the signed-in
person's secrets route, which seals it and never returns it:

```sh
curl --fail-with-body -X POST http://127.0.0.1:8490/api/secrets/add \
  -H 'Cookie: lys_directory_session=<signed-in-session>' -H 'Content-Type: application/json' \
  --data '{"operation":"account-add-0123456789abcdef","name":"anthropic-main","class":"credential","upstream":"https://api.anthropic.com","header":"authorization","prefix":"Bearer ","value":"<TOKEN>"}'
```

Use a path-safe account name, with letters, digits, underscores and hyphens. For Codex,
use `https://api.openai.com` or `https://chatgpt.com/backend-api/codex`, with the provider's
Bearer authorization token. A profile's account list uses one of those upstreams throughout.
Grant the intended agents use of `secret/<name>` through the existing permission source
(the directory source's configured use action is `edit`). Read the proxy's public key:

```sh
curl --fail-with-body http://127.0.0.1:8484/_lys/account/key
curl --fail-with-body -X POST http://127.0.0.1:8490/api/secrets/accounts/hold \
  -H 'Cookie: lys_directory_session=<signed-in-session>' -H 'Content-Type: application/json' \
  --data '{"agent":"<AGENT-ID>","secret":"anthropic-main","proxy_key":"<PUBLIC-KEY>","max_uses":100000,"not_after_ms":2100000000000}'
```

The hold route runs inside the live broker and answers only the handle id, holder and
secret name. Its explicit lifetime and use count must fit the person's grant. Issue one
handle for each permitted agent, then name the returned id in the profile:

```json
{"session":{"stall_after":300,"accounts":{"accounts":["<HANDLE-ID>"],"variable":"LYS_HANDLE_ACCOUNT","limit":{"signal":"plan_window"},"resume_arguments":[]}}}
```

Review checks every handle against the agent's current inventory and refuses
`account_handle_unheld` or `account_provider_mismatch`, naming the handle. Start repeats
those checks. Before dispatching the runner start, the server signs a run registration
for the broker and sends it through the proxy. The registration includes the proxy's
public key; the proxy checks that it is its own, and the broker verifies the service's
signature. The broker issues proxy draws descended from the original account handles,
so every call counts against their limits and an ancestor's revocation ends the draws.
Only the proxy keeps those draws, in private account-binding files beside its state.
A previous install with no account bindings continues to forward ordinary calls.

The child receives `ANTHROPIC_AUTH_TOKEN=lys-account.<handle-id>` for Claude, or
`CODEX_API_KEY=lys-account.<handle-id>` for Codex. Rotation selects the next id at each
spawn. Claude's competing API and OAuth environment variables are masked for account
launches. Claude Code 2.1.296 was measured with an empty login directory: the non-empty
`ANTHROPIC_AUTH_TOKEN` bypasses local login and becomes a Bearer header. The proxy removes
that placeholder and presents the selected draw to the broker. Only the broker attaches
the real value in the routed header, forwards upstream and redacts the returned answer.
Account calls are buffered for the broker's request-bound signature; ordinary calls keep
their streaming path. An account selector whose binding is missing is refused before
forwarding. Provider refusals remain visible, including model or plan restrictions.

The runner's proxy tracking record names the original handle id. Each usage line's
`account` is that same id, overriding a provider organization label for an account run.
The value reaches no child environment, rendered file, launch receipt, seats or attach
answer. The proxy's records and reports carry no value; broker responses are redacted.
The integration proof scans every fixture file outside sealed entries and uses an empty
Claude login directory through both proxy and broker. Real provider acceptance, Codex's
local login behavior and an installed identity-server launch require separate proof.
