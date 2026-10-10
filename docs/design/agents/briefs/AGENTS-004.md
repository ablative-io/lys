---
type: brief
id: AGENTS-004
cluster: agents
title: Keep supervised seats alive across terminal, service and upgrade changes
---

# AGENTS-004: Keep supervised seats alive across terminal, service and upgrade changes

> **Cluster:** agents
> **Depends on:** AGENTS-002, DIRECTORY-064, DIRECTORY-084
> **Blocked by:** AGENTS-002 R1 must implement and bind the seat record to StartSeat, and R3 must implement freshly authorised deliberate start/stop/restart. This brief extends their owner and lifecycle contracts; it does not invent a second registry or its own admission., DIRECTORY-064 crates/lys-runner/src/harness_control/process.rs:26 has QUALIFIED = &[] at 71e44c78360d2b2228657dc5213026f920526fb9. Amendment 37 non-fixture binary qualification and its final pin are owed before a real supervised harness can be launched. No fixture or survival helper qualifies that pin., DIRECTORY-084 must bound retained operation-id guards without losing retry-window deduplication before the history-independent recovery and scale claims can qualify.
> **Design anchor:**
> - ADR-118 — Lys owns running and monitoring its agents and enforcing values; Argus reads data for analytics — Lys owns everything about running and monitoring an agent it starts: installing its hooks/status line, reading its session stream and recording tokens, context and time. Lys holds budgets, goals, expectations and deliverables and enforces them through its runner with notices, compaction requests or stops as the value says. The context-watch notices previously driven by Argus become Lys-driven acts. Lys exposes the resulting data for Argus, which keeps analytics and visualisation; Lys does not depend on it. Lys screens are plain controls/state plus the authoritative tool-policy and grant refused-acts list on each agent page. Context compaction, goals and reminders are Lys-owned background acts. Argus may read explicitly permitted Lys variables alongside its data stream; no secret or arbitrary environment export follows from this. DIRECTORY-051 creates the missing tool-boundary policy and its browser editor. It does not claim an OS sandbox. The runner proves socket peer identity in peer.rs and makes grantable decisions against the live grant authority, denying uncertainty. Codex pre-tool refusal coverage is explicitly unavailable. OS process containment is the separate card11VnzLRp.
> - ADR-130 — Managed harness channels carry automatic context and reminder acts — 064 adds one runner-owned typed control/event channel per managed Claude or Codex session. It reuses 051 operation and usage owners, serialises all inputs at proved boundaries, requires actual harness evidence and retains uncertain outcomes without automatic resend. Legacy PTY sessions remain manual and cannot claim this capability. 065 extends the same Codex event reader for refusal coverage.
> - ADR-135 — An agent Lys starts carries a pass for that run and may be granted any action a person can take, except the responsibilities a person keeps — Every non-public action is a grantable permission. Lys issues a pass per run at spin-up, written with the Lys MCP address into the seat's config, ended when the run ends; every route takes it as the agent and judges each call against live grants. A refusal names who can grant, walking up the chain, and the agent asks them; the answer is once, for a while, ongoing or no. Grants gain one-time and transfer-and-return. A small named set of responsibilities is never passed to an agent.
> - ADR-136 — Permissions across the stack: Lys is the one authority; products decide hot routes from a signed pass and ask Lys for deliberate ones; a grant carries its mode — Lys holds every identity and grant; a product permission model is data in its app schema (ADR-116). Every product route is classified once as hot (decided offline from the pass) or deliberate (asked of Lys live, never falling back). The pass is the access token with a rights claim for its audience, short-lived, signed with the key Lys publishes. A grant carries one mode: outright, by_draft or by_two; a held act is a Lys draft the product executes after approval by reading Lys. Reach is by placement, never a pattern; a placement may be restricted. Roles are named bundles in the product schema; a widening by an app waits for the administrator. A machine is the fifth identity. One client library, lys-pass, with conformance fixtures every product gate runs. Availability is stated on each product screen; every deliberate act is receipted in Lys.
> - ADR-137 — Argus's remaining functions move into Lys as data beside goals, and a seat is a Lys record launched by the runner through the proxy; no shared library — Words (five slots, four layers, templates), variables (per agent and session, revision, author, expiry), schedules and seats are identity-server records in Lys's log, rendered and acted on at delivery through DIRECTORY-064's dispatcher with the contributing revisions in the receipt. No shared library: Lodestone is a read-only query engine over a store and holds nothing; haematite is the store under Lys already; a library would be a second place for the same data. Seat start, stop and restart are deliberate acts under Lys rights (ADR-136). Argus's alarms, rules, launches and queue are not moved into Lys: they become liminal services and aion work (ADR-136 section 3). A seat moves off herdr and Argus one at a time, Waffles's first.
> **Checklist:**
> - C711 — Supervised seats have an independent Lys process/descriptor owner and survive attaching-terminal exit or crash; manual Launch semantics remain unchanged (AGENTS-004 R1).
> - C712 — Runner and identity-server restarts rebind the same live seat/session/generation, preserve accepted hooks and uncertainty, and name unavailable authority (AGENTS-004 R2).
> - C713 — An upgrade hands an active turn, control descriptors and proxy stream over with one writer; refusal keeps the old live owner and never falls back to stop/start (AGENTS-004 R3).
> - C714 — Versioned owner/custody records migrate actual installed v3 state atomically and recover from an indexed bounded projection and tail (AGENTS-004 R4).
> - C715 — Proxy, hook, delivery, registry, reconnect and handover have measured deterministic count ratchets correlated with wall-clock work, with zero idle polling (AGENTS-004 R5).
> - C716 — Every survival behaviour has an observed main red and exact-change green, explicit fault/ready/exit signals and complete counts; tests over two seconds are defects (AGENTS-004 R6).
> - C717 — The actual roster N and 3N fixture keep per-seat work and memory bounded and unrelated seats responsive through one seat failure or handover (AGENTS-004 R7).
> - C718 — Intentional stop/restart retain fresh authority and stop-key retries; the registry reports proven owner/liveness/unknown state and the real install survives all four transitions (AGENTS-004 R8).
> **Stories:**
> - S406 (An operator, a person running the estate) — As an operator, I want a supervised seat to keep working when its terminal, runner or identity service exits, so that a viewing or service failure does not end its conversation.
> - S407 (An operator, a person running the estate) — As an operator, I want to upgrade Lys while an active seat streams and receives control replies, so that the same seat continues under one proven owner.
> - S408 (An operator, a person running the estate) — As an operator, I want stored ownership and uncertain work to recover without replay and every failure to be proved by a signal-driven test, so that recovery does not invent success.
> - S409 (An operator, a person running the estate) — As an operator, I want counted work per seat to stay flat at three times the real roster, so that supervision remains cheap as the team grows.
> - S410 (An operator, a person running the estate) — As an operator, I want a deliberate stop to win over handover and the screen to name current ownership and unknown authority, so that I can trust what Lys says is running.

## Purpose

Give a supervised seat a Lys-owned process and descriptor lifetime independent of its terminal, runner and identity service, so component restarts and upgrades preserve ongoing work. The source at bf158badec3a9454064eaac3e509c61ecee1492b currently ends recovered runner sessions, refuses live sessions during upgrade and stops the proxy. This brief replaces that supervised-seat behaviour with authenticated single-writer custody and proves never down, never slow, never break and flat per-seat scale. It preserves deliberate stop authority and manual-session semantics.

## Task

Implement an independent Lys seat owner, authenticated recovery and active-turn/stream handover through install and upgrade. Preserve the same harness, session, conversation, descriptors and operation ids across terminal crash, runner restart, identity-server restart and upgrade. Complete the entire slice and red matrix before qualification. Deliver versioned storage and old-install migration, counted performance ratchets, event-driven faults and actual-roster 3N proofs. No herdr ownership, control replay, stop/start upgrade fallback, new admission semantics or automatic fleet move.

## Requirements

### R1: Never down: give each supervised seat an independent Lys owner

WHEN AGENTS-002 R1 starts a supervised seat, THE SYSTEM SHALL establish an independent Lys-owned supervisor process through a typed runner owner command before acknowledging launch. That supervisor indexes one owner per seat/session/generation and owns the harness process, process-start identity, control pipes, terminal descriptors, collector and receipt cursor. It is detached from the attaching terminal and from the runner and identity-server process groups; closing or killing those clients does not close its descriptors or signal its harness. The runner and identity server connect as authenticated clients of that owner; they do not each launch a second harness. An exclusive owner lease and monotonic generation fence allow only one writer for a session. Keep the existing DIRECTORY-050 manual Launch lifecycle unchanged: this survival contract applies to an explicitly bound AGENTS-002 supervised seat, never by guessing that a manual session is a seat. No herdr, screen, manifold or external supervisor is in the ownership or recovery path. An unexpected harness exit is recorded as that exit, not concealed by replaying a turn or silently creating a new session.

**Acceptance:**
- seat_survives_terminal_crash starts a qualified supervised harness, waits for its ready event, then kills the test-owned attaching terminal PID. The same harness process-start identity, session, conversation and owner generation remain live; a correlated control reply and proxy call complete. Terminal detach and terminal SIGKILL are separate cases.
- seat_owner_is_unique submits two starts for the same seat binding concurrently behind a ready barrier. Exactly one harness and owner lease are created; the other result is the same receipted operation or a named ownership refusal, never a second session. A forged owner credential or foreign process-start identity is refused without touching that process.
- seat_owner_preserves_manual_lifecycle runs the existing manual Launch restart/stop assertions unchanged and proves the new survival behaviour is selected only by the typed supervised-seat binding. No executable lookup or process-control call reaches herdr.

**Files:**
- create: crates/lys-runner/src/seat_owner.rs
- create: crates/lys-runner/src/seat_owner/protocol.rs
- create: crates/lys-runner/src/seat_owner/process.rs
- create: crates/lys-runner/tests/seat_survival.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/socket.rs
- modify: crates/lys-runner/src/session.rs
- modify: crates/lys-runner/src/session/lifecycle/managed.rs
- modify: crates/lys/src/cli/runner.rs
- modify: crates/lys/src/commands/runner.rs
- modify: crates/lys/src/identity/install.rs
- modify: crates/lys/src/identity/install/services.rs

**Checklist:**
- C711 — Supervised seats have an independent Lys process/descriptor owner and survive attaching-terminal exit or crash; manual Launch semantics remain unchanged (AGENTS-004 R1).

**Stories:**
- S406 (An operator, a person running the estate) — As an operator, I want a supervised seat to keep working when its terminal, runner or identity service exits, so that a viewing or service failure does not end its conversation.

### R2: Reconnect after runner and identity-server restarts without ending the seat

WHEN the runner or identity server exits or restarts, THE SYSTEM SHALL leave the independent owner and supervised harness alive and rebind the replacement client by authenticated seat/session/generation and process-start identity. Preserve the conversation, run key, active admitted turn, control operation id, receipt cursor and proxy correlation. Recovery reads the indexed current projection and a bounded tail; it does not replay all finished operations. Required authority that is unavailable is named AuthorityUnavailable and holds new deliberate operations at their admission boundary; an already admitted turn retains its existing scoped grant until its existing validity boundary, with no invented extension. Authority recovery resumes held work only after the required live checks succeed. A request already possibly sent stays uncertain under DIRECTORY-064: no automatic resend, new id or fabricated success. Hook producers connect to the stable owner endpoint, with a bounded durable cursor and correlated duplicate suppression, so a service gap neither loses an accepted hook nor charges its usage twice. A dead service is unknown authority, not proof that the seat is offline. Key references remain references, and process ownership is proved by the kernel and authenticated owner, never a PID file alone.

**Acceptance:**
- seat_survives_runner_restart waits for an active-turn event and SIGKILLs only the test-owned runner PID. The owner reports the same live harness throughout; the replacement runner binds the same session and generation, accepts the next correlated control request, and launches zero replacement harnesses.
- seat_survives_identity_restart kills only the test-owned identity-server PID after a stream frame and hook acceptance are signalled. The active admitted turn completes through the owner/proxy, a new deliberate act is held with AuthorityUnavailable, and restarting the server permits admission without changing the session or charging the accepted hook twice.
- seat_reconnect_keeps_uncertainty loses the reply after the owner records possibly-sent, then restarts both clients. The same operation remains uncertain, exactly one harness send is counted, and the new clients cannot mark it acknowledged without its actual receipt.
- seat_reconnect_refuses_foreign_generation supplies a reused PID, stale lease generation and wrong public identity in separate cases. Each names the mismatch; the live owner and unrelated processes remain untouched.

**Files:**
- create: crates/lys-runner/src/seat_owner/recovery.rs
- create: crates/lys-runner/tests/seat_restart.rs
- create: crates/lys-identity-server/src/seat_supervision.rs
- create: crates/lys-identity-server/tests/seat_supervision.rs
- modify: crates/lys-runner/src/collector.rs
- modify: crates/lys-runner/src/harness_control/context.rs
- modify: crates/lys-runner/src/harness_control/events.rs
- modify: crates/lys-runner/src/operations/store.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/runtime_state/control.rs

**Checklist:**
- C712 — Runner and identity-server restarts rebind the same live seat/session/generation, preserve accepted hooks and uncertainty, and name unavailable authority (AGENTS-004 R2).

**Stories:**
- S406 (An operator, a person running the estate) — As an operator, I want a supervised seat to keep working when its terminal, runner or identity service exits, so that a viewing or service failure does not end its conversation.
- S407 (An operator, a person running the estate) — As an operator, I want to upgrade Lys while an active seat streams and receives control replies, so that the same seat continues under one proven owner.

### R3: An upgrade of the owner binary moves no harness

Amendment 1 (10 October 2026, Waffles 16:1x on Tom's word; the descriptor transfer this requirement first asked for is not written, by design, and that is the ruling, not a gap). WHEN the installed Lys binary is upgraded with supervised seats alive, THE SYSTEM SHALL leave every live seat with the owner process it started with: one owner per seat means no harness descriptor, partial frame or child is ever moved between processes. The owner a seat started with keeps serving that seat until the seat ends; every seat started after the upgrade is served by the new binary. Each owner announces its build on its ready line; the runner records it in the owned seat's endpoint record and the owner in its lease; the owned-seats readback (Act::Owned, GET /seats/owned and the Seats screen) names the build per seat so the screen can say which seats still run the old binary. SCM_RIGHTS descriptor passing and kqueue EVFILT_PROC are not in Lys. The custody fence (prepare, transfer at the next generation, release) stays recorded and refused by name at the store.

**Acceptance:**
- seat_started_before_an_upgrade_keeps_its_owner starts a seat with the owner binary announcing one build, replaces the runner, starts a second seat with the binary announcing a newer build, and proves the first seat is still served by its original owner process (same pid and start identity, same build) while the second names the new build; the replacement runner's recovery lists both live with their builds in one visit each.

**Files:**
- create: crates/lys-runner/tests/seat_upgrade.rs
- modify: crates/lys-runner/src/seat_owner/protocol.rs
- modify: crates/lys-runner/src/seat_owner/spawn.rs
- modify: crates/lys-runner/src/seat_owner/record.rs
- modify: crates/lys-runner/src/seat_owner/rules.rs
- modify: crates/lys-runner/src/seat_owner/process.rs
- modify: crates/lys-identity-server/src/seat_supervision.rs
- modify: surface/identity/src/features/sessions/Owned.tsx
- modify: docs/ops/SEAT-SUPERVISION.md

**Checklist:**
- C713 — An upgrade hands an active turn, control descriptors and proxy stream over with one writer; refusal keeps the old live owner and never falls back to stop/start (AGENTS-004 R3).

**Stories:**
- S407 (An operator, a person running the estate) — As an operator, I want to upgrade Lys while an active seat streams and receives control replies, so that the same seat continues under one proven owner.

### R4: Version ownership records and migrate actual installed state

THE SYSTEM SHALL persist versioned supervised-owner, lease, custody and receipt-cursor records with one durable authority per session. Existing lys-runner-sessions/v3 bytes are an installed shape: any change to them ships a migration from an old install and preserves ended sessions, manual-session meaning, operation ids and uncertainty. New AGENTS-002 seat records that have never been installed are identified as new records, not described as an invented legacy format. Persisted records contain public identity and credential references only. A migration or handover intent fences all participating writers; an uncertain append is resolved by reading its stable intent/operation id, never by accepting a different id. A failed or interrupted migration leaves the previous committed readable state and running owner authoritative until the new checkpoint is durable. Reopening validates every shape, bounded field and process identity, with named errors; a malformed record is not skipped and an unknown old version is not silently treated as empty. Keep a compact indexed current projection plus bounded recovery tail; retirement and id-dedup retention use DIRECTORY-084 without forgetting ids whose retry window remains open. No recovery loop over all permanent session or delivery history is permitted.

**Acceptance:**
- seat_owner_migrates_installed_v3 opens a captured old-install fixture with its v3 session file containing live manual and ended state, and its separate operation store containing uncertain control ids, upgrades and reopens it. All old meanings and ids remain readable; manual state is not promoted to a supervised seat, and new supervised records have their explicit version.
- seat_owner_migration_is_atomic injects exit and append-reply loss at each migration/custody intent barrier. Readback identifies the same intent, exposes exactly one committed owner and preserves the old checkpoint until its replacement is durable. No frame/control is automatically replayed.
- seat_owner_store_refuses_named_errors supplies malformed version, oversized handover member, missing credential reference and durable-write refusal separately. Each returns the member/stage-specific refusal without secret bytes, partial success or killing a running harness.
- seat_owner_recovery_is_bounded grows retired session/control history while keeping the same live projection and open retry window. Recovery record visits/bytes remain within the checked-in projection/tail bound; retry ids inside the window still deduplicate after reopening.

**Files:**
- create: crates/lys-runner/src/seat_owner/store.rs
- create: crates/lys-runner/tests/seat_owner_store.rs
- create: crates/lys-runner/tests/fixtures/seat_owner_installed_v3.json
- modify: crates/lys-runner/src/state.rs
- modify: crates/lys-runner/src/durable.rs
- modify: crates/lys-runner/src/operations/store.rs

**Checklist:**
- C714 — Versioned owner/custody records migrate actual installed v3 state atomically and recover from an indexed bounded projection and tail (AGENTS-004 R4).

**Stories:**
- S407 (An operator, a person running the estate) — As an operator, I want to upgrade Lys while an active seat streams and receives control replies, so that the same seat continues under one proven owner.
- S408 (An operator, a person running the estate) — As an operator, I want stored ownership and uncertain work to recover without replay and every failure to be proved by a signal-driven test, so that recovery does not invent success.

### R5: Never slow: count hot-path work and ratchet it down

THE SYSTEM SHALL ship deterministic counted benchmarks for proxy forward, hook ingest, control delivery, registry read, reconnect and custody transfer. Count instructions or named calls, keyed record visits, copied bytes, journal appends and physical disk syncs; fixture inputs and counter definitions are checked in with the measured baseline. Prove the chosen instruction/call measure tracks wall-clock work with paired workloads that increase and then remove a known source of work; retain both counts and durations as diagnostic evidence, never make elapsed time the performance gate. The checked-in ratchet permits equal or lower counts only; raising a ceiling or changing the counter to hide work fails. Each changed hot path declares locks held and their scope, whole-state clones, syncs per call and loops over history. No estate/session-history clone, fleet lock held across disk/network/kernel waits, or scan of all history is allowed. Shared maps serve indexed per-seat lookups; ownership waits use readiness/exit/socket events, with zero recurring work while idle and no polling timer. One logical owner transition uses at most one owner-journal sync, with duplicate readback using zero; combined hook/control/usage persistence costs are all counted, not excluded as work owned elsewhere. Handover buffers, frame lengths, pending operations and retained tails use explicit checked-in bounds drawn from the qualified protocol; overflow is a named refusal before acceptance, and already accepted streams use backpressure instead of discarding bytes.

**Acceptance:**
- seat_cost_ratchet records every named counter at fixed inputs on the accepted baseline and change. All six paths fit their checked-in ceilings; a positive-control extra lookup/copy/sync raises its counter and fails the ratchet. The control is removed for the final change.
- seat_cost_tracks_duration records paired count and wall-clock observations for the same increased/reduced-work workloads on the qualification machine without waiting for it to be quiet. The observations establish the selected counts track work; inconclusive correlation is reported as unqualified, never excused by a wider wall-clock limit.
- seat_owner_idle_is_event_driven holds all ready/exit/data signals closed, reads the event-work counters, then releases one ready signal. Before release the recurring wake/work count stays zero; afterwards exactly its declared work occurs, with no sleep, watchdog or timeout in the proof.
- seat_cost_counts_complete_transition measures the full successful transition, accepted hook/delivery and duplicate readback through their durable owners. It reports every physical sync and copied byte, proves no map guard spans the instrumented wait, and refuses an excess bound before accepting that input.

**Files:**
- create: crates/lys-runner/src/seat_owner/counts.rs
- create: crates/lys-runner/tests/seat_cost.rs
- create: crates/lys-runner/tests/seat_cost_ratchet.json

**Checklist:**
- C715 — Proxy, hook, delivery, registry, reconnect and handover have measured deterministic count ratchets correlated with wall-clock work, with zero idle polling (AGENTS-004 R5).

**Stories:**
- S409 (An operator, a person running the estate) — As an operator, I want counted work per seat to stay flat at three times the real roster, so that supervision remains cheap as the team grows.

### R6: Never break: prove red then green with explicit fault signals

THE SYSTEM SHALL provide a complete matrix linking every R1-R8 behaviour and acceptance case to a failing test observed on the pinned main before implementation and a passing test at the exact changed commit. Fault injection uses explicit process-ready, active-turn, frame, durable-append, custody and exit barriers; tests wait on a signal or kernel exit, never a timer, sleep, watchdog or raised timeout. Component failures use only PIDs created and held by the fixture. Include descriptor/lease refusal, partial frame, reply loss, successor crash, migration interruption, authority outage and ownership mismatch. Every error is handled or propagated by name. A test slower than two seconds is a defect to diagnose and fix while retaining its assertions; counts include all slow tests, not a tail. Preserve existing control uncertainty, charge-once, stop, installed-state and manual-session tests. No stub, TODO, catch-all error fallback or skipped/ignored test satisfies the contract. A red must compile and fail its named behaviour assertion at an existing observable entry point; a missing new API compile error, unqualified adapter fixture or skipped prerequisite is not an observed behavioural red.

**Acceptance:**
- seat_survival_red_matrix retains the full list of main/change commits, test names, inputs, expected refusals, exits, passed/failed/skipped counts and durations. Every acceptance case has an observed main red and exact-change green; tests not run remain explicitly unconfirmed.
- seat_survival_fault_matrix releases each named fault barrier in the ownership/upgrade/migration/recovery path. The harness and accepted bytes remain accounted for, refusals identify the stage and generation, and no accepted operation is fabricated or resent.
- seat_survival_assertions_stay_intact compares the red/green assertion manifest and executes the retained manual/control/stop/store proofs. No assertion is dropped to reach green, no test uses clock waits, and every case over two seconds is listed as a defect rather than a pass of the delivery bar.

**Files:**
- create: crates/lys-runner/tests/seat_survival_faults.rs
- create: crates/lys-runner/tests/seat_survival_red_matrix.json

**Checklist:**
- C716 — Every survival behaviour has an observed main red and exact-change green, explicit fault/ready/exit signals and complete counts; tests over two seconds are defects (AGENTS-004 R6).

**Stories:**
- S408 (An operator, a person running the estate) — As an operator, I want stored ownership and uncertain work to recover without replay and every failure to be proved by a signal-driven test, so that recovery does not invent success.

### R7: Scales: hold per-seat cost flat at three times the actual roster

THE SYSTEM SHALL capture the actual authenticated unique seat roster and its revision/count N, then run equal-work fixtures at N and 3N. The estimate is about 25/75 seats; it is not an observed roster and cannot substitute for reading it. An unavailable roster is named unknown, never N=0. Compare the deterministic counters from R5 for each path and peak owned bytes per seat at equal bounded frame/queue/history populations. Per-seat counted work must not increase with unrelated seats; total work and owned memory are proportional to seats plus explicitly counted once-per-batch setup. Use keyed routing and per-seat queues without a fleet-wide stop, rescan or lock across waits. Inject one runner reconnect, service outage, handover and explicit seat stop at 3N while the other seats continue correlated hook, proxy, delivery and registry work. Long retired history must not increase per-seat lookup/recovery work; DIRECTORY-084 retention is a prerequisite, not an unbounded seen-id set hidden from the measurement. All queue/byte bounds and overflow refusals are reported with complete accepted/refused/completed counts.

**Acceptance:**
- seat_survival_threefold_scale retains roster source/revision/N, fixture seed and full N/3N counter tables. Each equal-work path has the same per-seat ceiling at N and 3N, total work is proportional and peak owned bytes obey the declared bound; a failed roster read refuses by name.
- seat_survival_scale_isolates_one_seat parks one seat at a handover barrier at 3N and deliberately stops it through an authorised action. Every other seat completes its correlated work, no other harness is signalled, and every admitted/refused/completed operation is counted.
- seat_survival_scale_ignores_retired_history grows retired history with a fixed live roster and retry window. Indexed reads/reconnects keep the same record-visit and copy counts; a bound-exceeding pending input is refused by name without delaying unrelated ready seats.

**Files:**
- create: crates/lys-runner/tests/seat_scale.rs
- create: crates/lys-runner/tests/seat_scale_fixture.json

**Checklist:**
- C717 — The actual roster N and 3N fixture keep per-seat work and memory bounded and unrelated seats responsive through one seat failure or handover (AGENTS-004 R7).

**Stories:**
- S409 (An operator, a person running the estate) — As an operator, I want counted work per seat to stay flat at three times the real roster, so that supervision remains cheap as the team grows.

### R8: Keep intentional stop distinct and show proven ownership

THE SYSTEM SHALL keep AGENTS-002 R3 start/stop/restart as deliberate freshly authorised acts. A deliberate seat restart ends the old session and starts the authorised new one; a runner/service restart or upgrade preserves the old session. An explicit emergency stop may preempt a handover, retains its stop operation key for retries, and cannot be undone by a successor adopting a stale manifest. Both old and incoming owners observe the durable stop fence and return the actual harness exit proof; process-name kills and typed terminal keys are forbidden. Ordinary stop retains the existing active-turn refusal unless its authorised force path applies. Registry and Sessions screen report the current owner generation, proven harness liveness, detached viewer, authority-held state, handover stage and named unavailable cause from the owner, with unknown distinguished from offline. No service health 200 or dormant helper means a seat is live. Operational instructions record exact install/build/owner identities before and after recovery, the rollback refusal and the real one-seat migration proof. Attach stays a rights-checked viewer whose exit never ends a supervised seat.

**Acceptance:**
- seat_stop_preempts_handover parks custody transfer, sends one authorised emergency stop and retries its same key after a client restart. The same stop receipt and kernel exit proof return, both owners reject stale adoption, and no new session is launched.
- seat_restart_is_deliberate proves a runner restart preserves session identity, while an authorised seat restart produces one ended old session and one new session under its fresh grant. Unauthorised restart and ordinary mid-turn stop refuse by name without signalling the harness.
- seat_registry_names_owner_state supplies live-detached, authority-held, handing-over, owner-unreachable and exited proofs. API and surface display the matching generation/state/cause; unreachable is unknown, not offline, and attach requires the responsible person or administrator.
- seat_real_install_survival runs one qualified non-fixture seat under its actual harness binary and existing credential references. Terminal SIGKILL, runner restart, identity-server restart and upgrade each preserve its same active conversation and deliver a correlated receipt/stream. Exact binaries/versions/commits, process-start identities, commands/exits and before/after owner proof are retained; no credentials are copied.

**Files:**
- create: crates/lys-runner/tests/seat_stop_handoff.rs
- create: surface/identity/tests/seat_supervision.test.tsx
- create: docs/ops/SEAT-SUPERVISION.md
- modify: crates/lys-runner/src/session/stop.rs
- modify: crates/lys-runner/src/socket.rs
- modify: surface/identity/src/api.ts
- modify: surface/identity/src/features/sessions/Sessions.tsx

**Checklist:**
- C718 — Intentional stop/restart retain fresh authority and stop-key retries; the registry reports proven owner/liveness/unknown state and the real install survives all four transitions (AGENTS-004 R8).

**Stories:**
- S410 (An operator, a person running the estate) — As an operator, I want a deliberate stop to win over handover and the screen to name current ownership and unknown authority, so that I can trust what Lys says is running.

## Boundaries

- AGENTS-002 owns seat identity/launch/attach and deliberate acts; DIRECTORY-064 owns qualified control and uncertainty. Extend those contracts, never populate QUALIFIED from a fixture or bypass admission. AGENTS-001 words/variables/schedules and AGENTS-003 import keep their existing owners.
- No unrelated crate rewrite, identity/admission redesign, credential extraction/copy, external supervisor, new rule engine, automatic seat move or source cleanup. A harness process failure is named; this brief does not claim that a dead process kept the same PID alive or permit an automatic replay of its work.
- Manual DIRECTORY-050 sessions retain their meaning and explicit lifecycle. Only typed AGENTS-002 supervised bindings receive independent ownership. New record declarations and the seven AGENTS-004 structure additions do not change existing declarations.
- Writing this brief changes no installed process. Implementation, non-fixture qualification, the lead-owned one battery/install and the one-seat move remain separate work. A fixture, pushed commit, service health or design gate is not hospital readiness.

## Verification

- Brief box: verify every modify path with git ls-tree at the pinned local-main tree, verify create paths are absent and preserve existing checklist/story/design declarations. Render with scripts/design/render-cluster.py. Under Waffles gate amendment babdb0dd473296b573e87f08c1f5f21e35e544e6b003734f18bf03f7703b7efe, run scripts/design/gate.sh on this Mac in an exact scratch candidate from bf158badec3a9454064eaac3e509c61ecee1492b plus the eight owned JSON/Markdown paths, excluding concurrent foreign drafts (Python design checks only). Retain its exit code and full parsed validation/coverage/failure/warning counts; exit 0 and zero parsed failures are required. Commit those exact paths on top of local main, record commit/tree and hold push until Waffles releases publication.
- Implementation: record observed main reds, write all R1-R8 code/tests/migrations/count ratchets before any assembled gate, run the real formatter, focused nextest for every touched behaviour and strict touched-crate Clippy in the assigned remote seat checkout only. Retain command exits and complete counts. No compilation or tests on this Mac, no full suite or partial gate by the writer. The lead runs the one assembled qualification after everything is written.
- Done for implementation requires exact pushed/remote-read-back commit and tree, all focused cases green, all required Clippy configurations exit 0, complete counted ratchets and N/3N tables, old-install migration proof, and the lead-owned terminal qualification/battery/install with real active-turn and stream survival. A subset is reported by its full counts, never as a workspace pass. Tests over two seconds remain defects until fixed without removing assertions.
- Every runtime handback names each changed hot path, locks held, whole-state clones, physical disk syncs per call and loops over history, plus versions, process-start identities, commands/exits, full test/load counts, open prerequisites and what was not run. Missing evidence is unconfirmed, not a silent fallback.
- Stop and report the exact path if implementation requires an unmade product/authority decision or a change outside Lys; continue independent in-scope work. Named incompatible handover or unavailable authority preserves the old live owner and refuses that transition. Only a freshly authorised deliberate stop may end the seat.
- Hospital line: this is a written supervision contract, not evidence of safe continuous operation with patient records; safety and performance claims require the exact installed non-fixture survival, authority, migration and counted-load proofs.
