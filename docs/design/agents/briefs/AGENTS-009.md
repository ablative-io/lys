---
type: brief
id: AGENTS-009
cluster: agents
title: Lys starts a reviewed seat program as an agent, with its own reviewed arguments, stdin held to EOF, the model only through Lys's proxy and the launch identity and proof, and the agent reads the answer to its drafts on an event
---

# AGENTS-009: Lys starts a reviewed seat program as an agent, with its own reviewed arguments, stdin held to EOF, the model only through Lys's proxy and the launch identity and proof, and the agent reads the answer to its drafts on an event

> **Cluster:** agents
> **Depends on:** AGENTS-008
> **Blocked by:** Apollo, 16 Oct 2026 04:3x: haematite's assistant (`haematite assistant run --config <seat.json>`, an ACP host) cannot be started by Lys at all, and liminal's `liminal agent run` seat is expected to have the same gap, so this is one Lys contract for both products, not two. Haematite's half is written: docs/design/assistant/profiles/README.md at haematite 259b5e9 names what the contract must bind; ASSIST-001 R7 is blocked on this brief., Source base d594733e. crates/lys-home/src/harness/rendering.rs:186-190 resolves only `claude-code/template-v1` and `codex/template-v1`; rendering_launch.rs:105 accepts the same two and builds only their native flags. crates/lys/src/commands/launch.rs:233-241 refuses any `lys run` argument as launch_arguments_unavailable. crates/lys-runner/src/harness_control/process/qualified.rs:13-26 qualifies Claude Code 2.1.296 and codex-cli 0.161.0-alpha.3 only. Lys has no route that reads one product draft (product_drafts.rs:48-50 serve list, create, approve and refuse), and /changes admits only a signed-in person (changes.rs:113, signed_in), so an agent can learn its draft's answer only by paging the list again and again.

## Purpose

A product that ships its own agent host (haematite's assistant today, liminal's agent next) is started by Lys like any other agent: reviewed once by a person, launched by the runner on the person's computer with exactly the reviewed program, arguments and seat file, its model calls only through Lys's proxy with no provider credential anywhere in its environment, carrying the same launch identity and proof a native Claude Code or Codex run carries, and its stdin held open for the life of the run. When that agent asks for something only a person may decide, it is woken when the decision is taken and reads the answer, without polling. Lys stays product-agnostic: nothing in it names haematite or liminal.

## Task

Authoring START 2026-10-16 04:4x AEDT; source base d594733e (Lys origin/main). Waffles, from Apollo's 04:3x request and haematite 259b5e9. Decisions taken by Tom's rules, with reasons: (1) ONE contract, `seat-program/v1`, never a product-named one such as the proposed `haematite-assistant/seat-v1`, because Tom's golden rule of 15 Oct keeps every product independent and Apollo expects liminal to need the same contract. (2) The program's arguments are reviewed state on the harness description, not a per-run input: `lys run` keeps refusing free arguments (launch.rs:233), because an argument nobody reviewed must not reach a process that holds a person's authority; its refusal words change to say where reviewed arguments live. (3) The seat file is named by absolute path and bound by its sha256 at review, like the executable: Lys does not write or parse a product's seat file; the model choice, the words and the one MCP server live in the seat file the product's renderer made, and Lys binds the reviewed bytes. (4) A reviewed executable or seat file whose bytes have since changed (a product upgrade) is started and MARKED in Lys with both digests and a one-press re-review, never refused, matching AGENTS-008 decision (3) and Tom's 13 Oct rule that a wrong refusal is the failure that cannot happen; a description naming an unknown contract is still refused by name. (5) The draft answer travels as Lys's existing coalescing /changes signal followed by an ordinary authorised read of the one draft, because changes.rs already carries no records in the signal and bounds its waiters (PLACES = 128); no new feed and no timer. (6) haematite.database's agent_permission action is admitted to drafts by haematite registering it in its app schema; Lys's admit_action (product_drafts.rs:192) already admits any registered action, so Lys's part is a test that proves it, not new code.

## Requirements

### R1: A harness description can declare the seat-program contract, reviewed by a person

THE SYSTEM SHALL resolve the rendering contract `seat-program/v1` beside the two native contracts. A `seat-program/v1` description SHALL carry: the absolute executable path and its sha256; the ordered fixed arguments, each a literal string, among which exactly one is the placeholder `{seat_file}`; the absolute seat-file path and its sha256; and the adapter the runner speaks to the program on stdin and stdout (an ACP adapter name). The review form on #/provisioning SHALL show each of those fields read-only with both digests, and the person's review SHALL record them. A description that names `seat-program/v1` but lacks any field, gives a relative path, carries no or two `{seat_file}` placeholders, or names a native-only field (confinement, CLAUDE_CONFIG_DIR, native flags) SHALL be refused by name naming the field.

**Acceptance:**
- Red (nextest, lys-home): `registered("seat-program/v1")` is true; today it is false.
- Red (nextest, lys-home): a complete seat-program description renders a launch plan whose argv is exactly [executable, fixed arguments with {seat_file} replaced by the seat-file path]; nothing is appended and nothing dropped.
- Red (nextest, lys-home): each malformed description (missing field, relative path, zero or two placeholders, a native-only field) is refused naming that field.
- Red (vitest, surface/identity): the review form shows executable, arguments, seat file and both digests for a seat-program description, and the native fields are absent.

**Files:**
- create: crates/lys-home/src/harness/seat_program.rs
- create: crates/lys-home/src/harness/seat_program_tests.rs
- create: surface/identity/src/features/provisioning/seat-program-review.test.tsx
- modify: crates/lys-home/src/harness/mod.rs
- modify: crates/lys-home/src/harness/rendering.rs
- modify: crates/lys-home/src/harness/rendering_launch.rs
- modify: crates/lys-identity-server/src/provisioning_api/validation.rs
- modify: surface/identity/src/features/provisioning/Provisioning.tsx

### R2: The runner starts the seat program with the run's identity, proof and proxy, and no provider credential

THE SYSTEM SHALL start a seat-program run on the person's computer with exactly the reviewed argv, the login environment Lys already gives every run (Tom, 3 Oct), and the same Lys launch variables a native run receives (LYS_RUN, LYS_AGENT, LYS_MACHINE, LYS_RUNNER_SOCKET, LYS_PROVISIONING_VERSION and the agent pass file named by LYS_AGENT_PASS), plus the keyed proxy base addresses for Anthropic and OpenAI that rendering.rs already derives for native runs. Before starting, the runner SHALL remove from the child's environment every provider credential and every credential-shaped variable by the same rule the native launch uses (claude_code/launch_env.rs), and SHALL refuse the start by name if the seat file itself is not owner-only. The run SHALL be recorded with the executable and seat-file digests it actually started.

**Acceptance:**
- Red (nextest, lys-runner): a fixture seat program that prints its argv and environment names (never values) to its stdout shows the reviewed argv exactly, every Lys launch variable, both proxy base addresses carrying the run key, and no ANTHROPIC_API_KEY, OPENAI_API_KEY or other credential-shaped name even when the runner's own environment holds them.
- Red (nextest, lys-runner): a seat file with mode 0644 refuses the start naming the path and the mode; nothing is started.
- Red (nextest, lys-runner): the run record names the started executable and seat-file sha256.

**Files:**
- create: crates/lys-runner/src/session/seat_program_launch_tests.rs
- modify: crates/lys-runner/src/session/launch.rs
- modify: crates/lys-home/src/harness/claude_code/launch_env.rs

### R3: Stdin is held open for the life of the run and closed only to end it

THE SYSTEM SHALL hold the seat program's stdin open from start until the run is ended, speak the declared adapter over stdin and stdout, and end the run by closing stdin and waiting for the process's exit event; a program that exits on its own SHALL end the run with its exit status recorded. No step SHALL wait a set time: readiness is the adapter's first answer, ending is the exit event.

**Acceptance:**
- Red (nextest, lys-runner): a fixture program that exits 0 the moment its stdin reaches EOF stays alive while the run is live, and exits 0 when the run is ended; the recorded status is 0.
- Red (nextest, lys-runner): a fixture that exits 3 by itself ends the run with status 3 recorded and named.
- Red: an ast-grep or grep over the files this requirement touches finds no sleep, timeout-as-readiness or poll interval.

**Files:**
- create: crates/lys-runner/src/session/seat_program_stdin_tests.rs
- modify: crates/lys-runner/src/session/launch.rs

### R4: A changed program or seat file is started and marked, with a one-press re-review; an unknown contract is refused

THE SYSTEM SHALL compare the executable's and seat file's sha256 at start with the reviewed digests. When either differs the run SHALL start and SHALL be marked in Lys (the agent's row and the run) with the reviewed and found digests and a Re-review action that records the found digests as reviewed by the person who presses it. A description whose rendering contract is not registered SHALL be refused by name, as today.

**Acceptance:**
- Red (nextest, lys-runner): with the seat file changed after review, the run starts and its record carries reviewed and found digests; today no digest is compared.
- Red (vitest, surface/identity): the agent row shows the mark with both digests and a Re-review button; pressing it posts the found digests and the mark clears on the next read.
- Red (nextest, lys-home): an unregistered contract is refused naming it.

**Files:**
- create: crates/lys-runner/src/harness_control/process/reviewed_digests.rs
- modify: crates/lys-runner/src/harness_control/process/qualified.rs
- modify: surface/identity/src/features/provisioning/ProfileCopies.tsx

### R5: `lys run` says where reviewed arguments live

THE SYSTEM SHALL keep refusing free `lys run` arguments as launch_arguments_unavailable, and the refusal words SHALL say that a program's arguments are reviewed on its harness description in Lys and name the #/provisioning page, so a person is told the way forward instead of a dead end.

**Acceptance:**
- Red (nextest, lys): `lys run <agent> -- --config x` is refused with launch_arguments_unavailable and words naming the harness description and #/provisioning; nothing is started.

**Files:**
- modify: crates/lys/src/commands/launch.rs

### R6: An agent reads the answer to its own product draft, woken by the change signal

THE SYSTEM SHALL serve GET /product-drafts/{id} to the identity that requested the draft and to a person who may decide it, answering its state (held, approved, refused), the decider, the decision time and the target (kind, id, action), and never the words of another draft; any other caller is refused by name. /changes SHALL admit an agent connection's pass as well as a signed-in person, under the same waiter bound (PLACES), so an agent waits on the signal and re-reads its draft when the generation moves. The decision SHALL move the generation.

**Acceptance:**
- Red (nextest, lys-identity-server): an agent holding a draft waits on /changes with its pass; a person approves the draft; the agent's wait returns and its GET /product-drafts/{id} reads approved with the decider. No timer appears in the test.
- Red: another agent's GET of the same draft is refused by name; a person who may not decide it is refused by name.
- Red: today GET /product-drafts/{id} is 404 and /changes refuses an agent pass.

**Files:**
- create: crates/lys-identity-server/src/product_drafts_read.rs
- create: crates/lys-identity-server/src/product_drafts_read_tests.rs
- modify: crates/lys-identity-server/src/product_drafts.rs
- modify: crates/lys-identity-server/src/changes.rs

### R7: A registered app action such as agent_permission is admitted to drafts

THE SYSTEM SHALL admit to product drafts every action an app's registered schema declares, with no product name in Lys. Haematite registering `haematite.database/agent_permission` is haematite's change, not Lys's.

**Acceptance:**
- Red-or-green-first (nextest, lys-identity-server): with a fixture app schema declaring `haematite.database` and the action `agent_permission`, POST /product-drafts for that action under a held grant is admitted; with the action absent from the schema it is refused by name. If this passes on today's code, it is kept as the proof and stated as already true, not reworded as a fix.

**Files:**
- create: crates/lys-identity-server/src/product_drafts_registered_action_tests.rs

## Boundaries

- SHALL NOT name haematite, liminal or any product in Lys source; the contract is `seat-program/v1` for every product.
- SHALL NOT put a provider credential or credential-shaped variable in a seat program's environment, and SHALL NOT print, log or answer any pass, token or key.
- SHALL NOT pass an unreviewed argument to any program.
- SHALL NOT write, parse or alter a product's seat file; Lys binds its path and digest only.
- SHALL NOT wait a set time anywhere: readiness, ending and draft answers are events (Tom, 15 Oct).
- SHALL NOT change the two native contracts' behaviour; their existing tests pass unchanged.

## Verification

- sh scripts/design/gate.sh at the committed source; parse the validate, coverage and rendered-comparison lines, never the exit code alone.
- cargo nextest run -p lys -p lys-home -p lys-runner -p lys-identity-server, counts stated per crate; cargo test --doc for the same crates; cargo clippy -p each -- -D warnings -W clippy::pedantic; ast-grep with the repository's rules; the 500-line file gate.
- surface/identity: npm run typecheck and npm test, parsed.
- Live, after install: register haematite's assistant as a seat-program harness on Tom's Mac, start it through Lys, and show its first adapter answer, the eleven haematite tools listed, one real call, and the run record's digests; then the same for liminal's agent seat when Hermes has it.
