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

- Every call that passes is stored whole: request, response, headers that matter, timing.
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
