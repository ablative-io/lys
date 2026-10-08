# Agents — Checklist

## Words, variables and schedules

- [ ] **C701** — The five message slots (context warning, preparation, compaction, wake-up, scheduled reminder) resolve built-in, then workspace, then agent, then session, with named templates; an empty layer inherits (AGENTS-001 R1).
- [ ] **C702** — An agent's and a session's variables are JSON values with a revision, an author and an optional expiry; a stale revision is refused by name; the agent reads and sets its own through lys and the Lys MCP server (AGENTS-001 R2).
- [ ] **C703** — Delivery renders {{vars.key | fallback}}, {{goals}} and {{time_left}} at the moment of delivery and captures the rendered text with every contributing revision in the operation's receipt (AGENTS-001 R3).
- [ ] **C704** — A schedule has at, until, interval, max occurrences and recipients; missed intervals coalesce to one send; an uncertain delivery stops it for inspection; every state survives restart (AGENTS-001 R4).
- [ ] **C705** — The Usage screen shows and sets words, variables and schedules with named refusals (AGENTS-001 R5).

## Seats

- [ ] **C706** — A seat is a Lys record (name, harness, profile, home, machine) launched by the runner with the proxy and Lys's hooks and status line in its provisioned home (AGENTS-002 R1).
- [ ] **C707** — The Sessions screen shows every seat online, offline, not seen by the runner, or seen but unregistered, from the runner's own liveness, never from a terminal program (AGENTS-002 R2).
- [ ] **C708** — Start, stop and restart are deliberate acts under Lys rights; stop goes through the harness's control channel and the runner, never typed keys (AGENTS-002 R3).
- [ ] **C709** — lys attach shows a seat's live terminal to its responsible person or an administrator and nobody else (AGENTS-002 R4).
- [ ] **C710** — A seat moves off herdr and Argus one at a time with the same profile, proved by the first context warning arriving from Lys and Argus's delivery for that seat off (AGENTS-002 R5).

## Seat survival and handover

- [ ] **C711** — Supervised seats have an independent Lys process/descriptor owner and survive attaching-terminal exit or crash; manual Launch semantics remain unchanged (AGENTS-004 R1).
- [ ] **C712** — Runner and identity-server restarts rebind the same live seat/session/generation, preserve accepted hooks and uncertainty, and name unavailable authority (AGENTS-004 R2).
- [ ] **C713** — An upgrade hands an active turn, control descriptors and proxy stream over with one writer; refusal keeps the old live owner and never falls back to stop/start (AGENTS-004 R3).
- [ ] **C714** — Versioned owner/custody records migrate actual installed v3 state atomically and recover from an indexed bounded projection and tail (AGENTS-004 R4).
- [ ] **C715** — Proxy, hook, delivery, registry, reconnect and handover have measured deterministic count ratchets correlated with wall-clock work, with zero idle polling (AGENTS-004 R5).
- [ ] **C716** — Every survival behaviour has an observed main red and exact-change green, explicit fault/ready/exit signals and complete counts; tests over two seconds are defects (AGENTS-004 R6).
- [ ] **C717** — The actual roster N and 3N fixture keep per-seat work and memory bounded and unrelated seats responsive through one seat failure or handover (AGENTS-004 R7).
- [ ] **C718** — Intentional stop/restart retain fresh authority and stop-key retries; the registry reports proven owner/liveness/unknown state and the real install survives all four transitions (AGENTS-004 R8).

## Counted seat-loop performance

- [ ] **C719** — Every one of 41 census paths has an actual-code benchmark, reproducible counter vector, native-time correlation and non-increasing case ratchet; unmeasured values remain unqualified (AGENTS-005 R1/R10).
- [ ] **C720** — Capture admission, frames, worker, completion, usage and drain remove repeated history/journal work while preserving every byte and durable completion stage (AGENTS-005 R2).
- [ ] **C721** — Peer, hook, bind, status and source follow use indexed current identities/cursors instead of fleet, date and rotation history scans (AGENTS-005 R3).
- [ ] **C722** — Delivery/reminder and lifecycle costs are indexed and bounded, with existing DIRECTORY-064 and AGENTS-004 durability/retention contracts preserved (AGENTS-005 R4/R9).
- [ ] **C723** — Usage/feed enforcement touches only affected indexed memberships/standing and current immutable configuration, preserving all crossing actions (AGENTS-005 R5).
- [ ] **C724** — Registry/liveness pages use current indexed fields and proven owner events instead of copying history and polling all seats; exact totals/freshness remain unchanged (AGENTS-005 R6).
- [ ] **C725** — Terminal/pattern/transport paths count complete work, preserve every byte and match, and remove prefix re-search and shared network-wait holds (AGENTS-005 R7).
- [ ] **C726** — Approval, connector, MCP refusal and nonce paths remove per-request global scans/copies/thread growth without stale grants or weakened replay/audit (AGENTS-005 R8).
