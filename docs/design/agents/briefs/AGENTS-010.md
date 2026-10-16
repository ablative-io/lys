---
type: brief
id: AGENTS-010
cluster: agents
title: A seat-program profile names the model wire it speaks, and Lys binds its account through the proxy as it does for a native run
---

# AGENTS-010: A seat-program profile names the model wire it speaks, and Lys binds its account through the proxy as it does for a native run

> **Cluster:** agents
> **Depends on:** AGENTS-009
> **Blocked by:** Found 17 Oct 2026 04:1x AEDT with Apollo, setting up haematite's assistant as a seat-program/v1 seat running codex-acp in a home of its own (haematite ASSIST-004 R2, because codex-acp loaded every MCP server in the machine's ~/.codex). Read at e857556e: crates/lys-identity-server/src/model_account_start.rs routed() (lines 89-101) knows providers only for claude-code/template-v1 and codex/template-v1, and the account variable (lines 203-215) refuses every other contract with LaunchUnrenderable "account harness unsupported". crates/lys-home/src/harness/seat_program.rs already gives a seat-program run both ANTHROPIC_BASE_URL and OPENAI_BASE_URL keyed to its run (lines 235-245), and the runner forwards them (lys-runner harness_control/seat_program/launch.rs:118); only the account is missing, so a seat-program run that uses a model has no way to authenticate except a login kept in its own home, outside Lys.

## Purpose

Tom's rulings of 3 October: accounts for Lys-started AIs are accounts Tom registers in Lys, Lys records who draws on which and how much, and every model call goes through Lys's proxy, which holds the credential. A seat-program seat (the haematite and liminal assistants, and any harness Lys does not template natively) is a Lys-started AI too, so it gets its account the same way, never by someone running a login in the seat's home with their own account. That login would be a credential outside Lys that nothing accounts for and that breaks when the seat moves machines.

## Task

Authoring START 2026-10-17 04:1x AEDT; source base e857556ecf7cd4982af3b71bd09d7876af6cbd4e / tree bd083ff42d4f02f9738dbf81b7b8d3c1a3038e71. Waffles, from Apollo's question. Decisions taken here by Tom's rules, with their reasons: (1) the seat-program description gains one reviewed field, model_wire, either "openai-responses" or "anthropic-messages": a seat program is opaque to Lys, so the person who reviews it is told which wire it speaks rather than Lys guessing from its arguments. (2) The wire maps to exactly what the native template of that wire gets: openai-responses takes the codex/template-v1 providers and CODEX_API_KEY; anthropic-messages takes the claude-code/template-v1 provider and ANTHROPIC_AUTH_TOKEN, with CLAUDE_CODE_OAUTH_TOKEN and ANTHROPIC_API_KEY emptied. One mapping, two callers, so a native run and a seat run can never drift. (3) A seat-program profile that names an account rotation and no model_wire is refused when the profile is set, by name, never at start.

## Requirements

### R1: The seat-program description names its model wire

THE SYSTEM SHALL accept an optional model_wire field on the seat_program description in crates/lys-home/src/harness/seat_program.rs (SeatProgram and its TryFrom), whose only values are "openai-responses" and "anthropic-messages", refusing any other value by name; and the Provisioning page (surface/identity/src/features/provisioning/) SHALL show it beside the executable and seat file the person reviews.

**Acceptance:**
- Red: a seat_program with model_wire "openai-responses" parses and round-trips; today the field is refused as unknown.
- Red: model_wire "gpt" is refused naming the two allowed values.
- Red (vitest): the Provisioning review of a seat-program profile shows its model wire.

**Files:**
- create: crates/lys-home/src/harness/seat_program_wire_tests.rs
- modify: crates/lys-home/src/harness/seat_program.rs
- modify: surface/identity/src/features/provisioning/

### R2: Lys binds the account for a seat-program run as it does for the native run of its wire

THE SYSTEM SHALL, in crates/lys-identity-server/src/model_account_start.rs, resolve the providers in routed() and the account variable from one mapping keyed by the run's wire: codex/template-v1 and seat-program/v1 with model_wire openai-responses take the OpenAI providers and CODEX_API_KEY; claude-code/template-v1 and seat-program/v1 with model_wire anthropic-messages take the Anthropic provider and ANTHROPIC_AUTH_TOKEN with CLAUDE_CODE_OAUTH_TOKEN and ANTHROPIC_API_KEY emptied; and SHALL bind the proxy and set LYS_ACCOUNT_VARIABLE exactly as for the native run.

**Acceptance:**
- Red: a seat-program/v1 run with model_wire openai-responses and an account rotation gets LYS_ACCOUNT_VARIABLE=CODEX_API_KEY and CODEX_API_KEY=lys-account.<first account>, and the proxy binding for its run; today it is refused "account harness unsupported".
- Red: the same with anthropic-messages gets ANTHROPIC_AUTH_TOKEN with the two Anthropic variables emptied.
- Red: every existing native-run account test passes unchanged: the mapping did not move a native run.

**Files:**
- create: crates/lys-identity-server/src/model_account_wire_tests.rs
- modify: crates/lys-identity-server/src/model_account_start.rs
- modify: crates/lys-identity-server/src/lib.rs

### R3: A seat-program profile with an account and no wire is refused when it is set

THE SYSTEM SHALL refuse, in crates/lys-identity-server/src/provisioning_api/validation.rs, a seat-program/v1 profile that names a model account rotation and no model_wire, by a name that says the wire is missing, when the profile is set; a seat-program profile with no account rotation SHALL be accepted without a wire as today.

**Acceptance:**
- Red: POST /agents/{id}/provisioning with a seat-program harness, an account rotation and no model_wire is refused by the wire-missing name; today it is accepted and refused only at start.
- Red: the same without an account rotation is accepted as today.

**Files:**
- modify: crates/lys-identity-server/src/provisioning_api/validation.rs
- modify: crates/lys-identity-server/src/seat_program_review_tests.rs

## Boundaries

- SHALL NOT let a seat program carry a model credential of its own, read one from its home, or be handed the account's secret: the run gets lys-account.<id>, which only Lys's proxy resolves.
- SHALL NOT change a native claude-code or codex run's environment or arguments.
- SHALL NOT guess a seat program's wire from its executable or arguments.
- SHALL NOT be built or tested until every line is written (the 3 Oct rule); one battery, one install.

## Verification

- sh scripts/design/gate.sh at the committed source; parse the validate, coverage and rendered-comparison lines, keeping only failures, never the exit code alone.
- cargo nextest run -p lys-home -p lys-identity-server at the head, counts stated per crate; cargo test --doc; clippy pedantic -D warnings; cargo fmt; ast-grep; file-length gate 500 lines (ADR-111); vitest for surface/identity.
- Walk on the install: haematite's assistant seat (seat-program/v1, model_wire openai-responses, Tom's registered Codex account) starts under Lys and answers its first turn, and Lys's usage shows the draw against that account.
- Worst credible failure if this were used in a hospital: a seat program handed a real model credential that outlives its run and is copied off the machine, so model calls on patient context are made outside Lys's record; it could affect every person whose data that seat reads, so the run only ever holds lys-account.<id>, which the proxy alone resolves.
