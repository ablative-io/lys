# Seat supervision: the independent owner of a supervised seat

AGENTS-004. Written 10 October 2026 with the code, before its first build, on Tom's word that every repository is written whole first.

## What runs where

A seat AGENTS-002 starts as a supervised seat is owned by a process of its own: `lys runner seat-owner`, started by the runner in a session and process group of its own (`setsid`), standard input closed, its words in `owners/<session>/owner.log` under the runner's state directory. The owner is a runner of its own serving one managed session on `owners/<session>/owner.sock`, so every ordinary act (input, read, operate, status, stop) reaches the harness through the runner protocol unchanged.

The runner that started the owner, the terminal that attaches to it and the identity server that admits its acts are clients. Each can close or die without closing one of the owner's descriptors or signalling its harness. A manual `Launch` is never a seat: the survival contract is selected by the typed binding member `owner` on a managed launch, and by nothing else.

## What is on disk, per owner

| File | What it is |
| --- | --- |
| `owners/<session>/launch.json` | The plan the owner read at start (0600): the binding, the managed launch, the person responsible, credential references by name, and the intent ids of its establish and harness records. Never a credential's bytes. |
| `owners/<session>/server.pub` | The server's public key the owner verifies signed acts with, as 64 hexadecimal characters. |
| `owners/<session>/seat-owners.json` | The indexed projection of the owner's record: seat, session, conversation, harness start identity, lease (generation, holder, when), custody stage, cursors. Format `lys-runner-seat-owners/v1`. Replaced whole at each checkpoint. |
| `owners/<session>/seat-owners.journal` | The bounded recovery tail: one intent per line, appended and synced once per transition, at most 256 lines past the checkpoint. |
| `owners/<session>/endpoint.json` | The owned seat as the runner recorded it once the owner was ready: binding, socket, the owner's runner id, and the owner process's pid and start identity. |
| `owners/<session>/runner/` | The owner's own runner state (its one session). |

The installed `lys-runner-sessions/v3` record is not changed by any of this: an old install opens with no owner, its manual sessions keep their meaning, and the operations store is not opened by the owner record. A record of an unknown version, a torn line or an oversized member is refused by name, never skipped and never read as empty.

## How a client binds

A client sends `Act::Owner { command: hello }` on the owner's socket naming the seat, session, conversation and generation it expects, which client it is (runner, identity server, successor) and its own process-start identity. The owner reads the peer's pid and uid from the socket (`SO_PEERCRED`), asks the kernel for that pid's start identity, and answers `bound` with its view only when all three agree. The refusals, each by name, with the owner serving on:

- `seat_owner_client_unproved`: another user, another pid, or a start identity the kernel does not confirm.
- `seat_owner_binding_mismatch`: another seat, session or conversation.
- `seat_owner_generation_stale`: another lease generation.

The identity server signs its acts for the owner's runner id, which the bound view carries. The parent runner forwards nothing: it holds no signing key.

## What survives what

| Event | What happens |
| --- | --- |
| The attaching terminal exits or is killed | Nothing: it held a client connection. |
| The runner exits or is killed | The owner and harness go on. The replacement runner reads `owners/*/endpoint.json` (one record per owner, never a journal), proves each owner's pid and start identity at the kernel, indexes the live ones and lists the others as unreachable (`exited`, `pid_reused`, `unproved`). It restarts nothing and replays nothing: a dead owner is unknown authority, not proof that the seat is offline. |
| The identity server exits or is killed | The owner and harness go on. The replacement binds with `hello` at the same generation. Deliberate acts that need the server's authority are held at admission until it is back (the server's `AuthorityUnavailable`); an accepted hook moved the owner's hook cursor once, under an intent derived from the cursor it reached, and is not charged again. |
| A reply is lost | The same intent id sent again is answered `already` with no second journal line. A different id is a different intent. A cursor that moves back is `seat_owner_cursor_regression`. |
| A deliberate stop | Recorded as `stopping` before the harness is ended, then as `exited` with who and why. A stop fences any handover: `seat_owner_stop_fenced`. |

## Counted, never timed

`tests/seat_cost_ratchet.json` holds, per hot path (proxy forward, hook ingest, control delivery, registry read, reconnect, custody transfer), the ceiling for calls, keyed record visits, copied bytes, journal appends, physical syncs and idle wakes. A change is accepted at equal or lower counts only; the file's `provenance` member says whether a ceiling was measured or derived by reading. One owner transition is one sync; a duplicate readback is none; a registry read is one keyed lookup and no file; a reconnect visits the live owners plus at most the tail. The wait for an owner is a blocking read on its ready pipe: no timer, no poll, zero wakes while nothing arrives.

## Upgrading the owner binary

An owner is never handed a live harness (AGENTS-004 amendment 1, Waffles 10 October 2026 16:1x, on Tom's word). One owner per seat means an upgrade of the owner binary moves no descriptor: the owner process a seat started with keeps serving that seat until the seat ends, and every seat started after the upgrade is served by the new binary. Each owner announces its build on its ready line; the runner records it in the owned seat (`endpoint.json`, member `build`) and the owner in its lease, and `GET /seats/owned` shows it per seat, so the screen says which seats still run the old build. `SCM_RIGHTS` and `kqueue EVFILT_PROC` are not in Lys. The custody fence (prepare, transfer at the next generation, release) stays recorded and tested at the store for a successor that is a new owner process of the same seat.

## Reading an owner

```
lys runner status --socket <state>/owners/<session>/owner.sock      # its one session
# Act::Owner { command: { "command": "status" } }                   # its record
# Act::Owned on the parent runner                                   # every owner it started, live and unreachable
```
