---
type: brief
id: AGENTS-012
cluster: agents
title: lys claude starts your agent when Lys is not answering: the device keeps the last launch it was given, runs the model proxy itself, and hands the run's record to Lys when Lys next answers
---

# AGENTS-012: lys claude starts your agent when Lys is not answering: the device keeps the last launch it was given, runs the model proxy itself, and hands the run's record to Lys when Lys next answers

> **Cluster:** agents
> **Design anchor:**
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-012 — A harness launch template is kept in the home by hash, and each render is recorded on the session beside its context path — A launch template per harness is a JSON object with named slots (transcript, mcp, env, secrets, instructions) plus flags, stored in the home under templates/ by its SHA-256; lys-home renders a template and a session into files and runtime variables with command mappings in text, prints the launch line and never runs it, and records each render as a sixth lys.harness_event kind, template_render, hung as a side leaf beside the context path with the written paths in a manifest block named by hash. Rejected: a transcript converter or adapter protocol per harness, a template kept outside the home (a seat document of another tool), and a render event that advances the head, which would change the session head hash between two renders of the same session.
> - ADR-086 — Every command given keeps its own signed launch record, and running is tied to it through lys/session-start/v1 — Every command given keeps one launch record, a signed directory event naming the machine, the executable, the working directory, the profile version and the credential ids. Its id is carried in lys/session-start/v1, whose signed message then holds five things: the tag, the agent's directory id, the directory's identifier, the launch record id and the directory-issued challenge. The agent reads the id from LYS_LAUNCH_RECORD and its report names it, and running is tied to the command given through that id. A start given again copies a kept record into a new record with its own id that names the record it was copied from. Rejected: tying running to the agent id alone, re-issuing one record for every command given, and keeping the launch facts only in the command line.
> - ADR-113 — The identity install and its clients wait on signals, never on a clock — A wait ends on the event it is waiting for or on the failure that makes the event impossible: a service is ready when it says so or when a single connection to it succeeds after a readiness event (its log gaining its listening line, observed through the platform's file-change notification), a stopped process is gone when the platform reports its exit (kqueue EVFILT_PROC on macOS, pidfd on Linux), a peer that refuses or closes is an error at once, and nothing carries a timeout. The screen re-measures on a ResizeObserver or animation-frame signal. Rejected: shorter sleeps, which still wait on a clock; keeping timeouts as a safety net, which the rule forbids and which hides a stuck peer's cause.
> - ADR-133 — The launch is everything Lys records for the seat, for Claude Code and our Codex build — A provisioning profile declares its harness build; the launch renders every field for that build or the profile is refused by name when it is recorded. Command MCP servers carry secrets only as handles. Claude Code permissions come from the profile and the Tool policy; Codex permissions are DIRECTORY-065's render of the same policy.
> **Checklist:**
> - C752 — Every server-rendered launch is kept on the device for its agent with the template hash, harness, proxy identity, run pass and the pass's expiry, written atomically, shown by lys agents (AGENTS-012 R1).
> - C753 — When the server's connect fails as the OS or TLS reports it, lys claude, lys codex and lys run start the agent from the kept launch with the proxy on the device recording every call, say so in the first line, and refuse by name with no kept launch or an expired pass; no clock anywhere (AGENTS-012 R2).
> - C754 — A run started while Lys was unreachable is handed to Lys whole on the next command that reaches it, accepted by launch hash and pass validity at start, idempotent by run id, never dropped, refused by name (AGENTS-012 R3).
> - C755 — The surface shows a handed-over session as started while Lys was not answering, from its device, counted in cost like any other (AGENTS-012 R4).
> - C756 — On a bare machine the first lys command asks for the Lys address once and keeps it, never answers config_unreadable; the first launch writes the device's Claude Code settings and lys agents lists them by path (AGENTS-012 R5).
> **Stories:**
> - S426 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person starting an agent on my own computer, I want lys claude to start my agent when the Lys console is off or unreachable, telling me plainly that it is, so that the one word works every time and I never have to open a terminal to work out why it did not.
> - S427 (An operator, a person running the estate) — As an operator, I want every run that started while Lys was unreachable to appear under its person and agent with its calls and cost the moment Lys answers again, so that nothing an agent did in the storm case is missing from the record.

## Purpose

Tom, 18 Oct 2026 08:4x and 10:31: starting an agent outside Lys must work when Lys is down; "I don't think Lys works as a server... people are not going into their Claude Code settings and setting up a proxy"; the thing a person types is one word and it works on any machine, every time. Today `lys claude` (crates/lys/src/commands/launch.rs) asks the server for the device, the agent catalog, the rendered launch and the terminal attachment, and stops with NotSignedIn or a transport error when the server does not answer, so the storm case (the Lys host off, the person at home with their laptop) is the case where the one word fails. Lys as a launcher and a pass only holds if the launcher does not need the console. Every project stands alone (ADR-004); the launch template is kept by hash (ADR-012) and the run is tied to a signed launch record (ADR-086), which is exactly what a device can keep and replay. If this code were used in a hospital, the worst credible failure is a run started while Lys was unreachable that is never recorded under the person's name, so an act an agent took on a ward has no owner in the record; the second is a kept launch replayed after the person's pass was taken back. Both are prevented by the kept launch carrying the pass it was given and its expiry, by refusing a kept launch whose pass has expired by name, and by handing every offline run to Lys whole the moment Lys answers. Tom, 10:55: "When you install Lys on your device, it will configure your rules"; no business Claude account; and 09:25 he ran `lys identity setup-code` on a laptop with no configuration and got config_unreadable: the first run on a bare machine must ask for the Lys address once, never fail on a missing file.

## Task

Make `lys claude`, `lys codex` and `lys run` start the agent from the launch the server last rendered for it on this device when the server does not answer, with the model proxy running on the device and recording every call, and hand those runs to the server as sessions under the person's name on the next launch or sign-in that reaches it; every refusal by name, no clock anywhere.

## Requirements

### R1: Every launch the server renders is kept on the device, by the agent, with the pass it carried

THE SYSTEM SHALL, after every launch the server renders (`started()` in launch.rs), write under the sign-in state root a kept launch for that agent on this device: the rendered template and its recorded hash (ADR-012), the harness name, the agent id and name, the machine id, the model proxy identity the server supplied, the run pass as rendered into the seat's configuration, that pass's expiry as the pass states it, the server's base, and the time it was kept. THE SYSTEM SHALL overwrite the previous kept launch for the same agent and device atomically (write to a sibling, rename) and SHALL keep nothing a person did not already have on the device: the pass written is the one already rendered into the seat's own configuration by the existing launch. `lys agents` SHALL show, beside each agent, when its launch was last kept on this device and when its pass expires.

**Acceptance:**
- After one `lys claude` against an answering server, the kept launch exists under the sign-in state root for that agent and device, and `lys agents --json` carries kept_at and pass_expires_at for it
- A second launch replaces the kept launch; a crash between write and rename leaves the previous one intact (test writes the sibling and asserts the named file is the old one)
- The kept launch's template hash equals the hash the server recorded for the session (read back from the server in the test fixture)
- No secret is written that is not already in the seat's rendered configuration; the test diffs the kept launch's fields against the rendered mcp.json or LYS_AGENT_PASS value

**Files:**
- create: crates/lys/src/commands/launch/kept.rs
- create: crates/lys/src/commands/launch/kept_tests.rs
- modify: crates/lys/src/commands/launch.rs
- modify: crates/lys/src/commands/launch_context.rs

**Checklist:**
- C752 — Every server-rendered launch is kept on the device for its agent with the template hash, harness, proxy identity, run pass and the pass's expiry, written atomically, shown by lys agents (AGENTS-012 R1).

**Stories:**
- S426 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person starting an agent on my own computer, I want lys claude to start my agent when the Lys console is off or unreachable, telling me plainly that it is, so that the one word works every time and I never have to open a terminal to work out why it did not.

### R2: When the server does not answer, the one word starts the agent from the kept launch, says so, and runs the proxy on the device

THE SYSTEM SHALL, when `lys claude`, `lys codex` or `lys run` cannot reach the server (the connect is refused, reset, unresolved or the TLS handshake fails, as the operating system or the TLS library reports it; never a bound chosen by Lys, ADR-113), start the agent from the kept launch for that agent on this device if one exists and its pass has not expired, with the harness arguments the person typed, and SHALL print as its first line, before the terminal attaches: "Lys at <base> is not answering (<the error as reported>); started <agent> from the launch kept <when>; this run is recorded here and handed to Lys when it answers". THE SYSTEM SHALL start `lys proxy serve` on the device for that run, bound on loopback, with the kept proxy identity, recording every call whole under a run id minted on the device (ADR-086's launch record signed with the agent's key), and SHALL point the harness at that proxy exactly as the server-rendered launch would. THE SYSTEM SHALL refuse by name when there is no kept launch for the agent on this device ("no launch kept for <agent> on this computer; run `lys claude --agent <agent>` once while Lys answers") and when the kept pass has expired ("the pass kept for <agent> expired <when>; sign in while Lys answers"), and SHALL never fall back to another agent, another device's launch or an unsigned run.

**Acceptance:**
- With the server stopped (its pid gone) and a kept launch present, `lys claude` starts the harness, the first line is the one above with the real error text, and the proxy on the device records the first model call whole (the test's fake provider sees it; the record under the run id has it)
- With the server stopped and no kept launch, `lys claude` refuses with the named refusal and starts nothing (no harness, no proxy process; asserted by pid)
- With the server stopped and a kept pass whose expiry is in the past, `lys claude` refuses with the named refusal and starts nothing
- No sleep, timeout, retry loop or poll is added; the reach failure is the operating system's or TLS library's error, asserted by its name in the first line
- With the server answering, behaviour is unchanged: the server renders the launch and the kept launch is updated (R1)

**Files:**
- create: crates/lys/src/commands/launch/offline.rs
- create: crates/lys/src/commands/launch/offline_tests.rs
- modify: crates/lys/src/commands/launch.rs
- modify: crates/lys/src/commands/launch_context.rs
- modify: crates/lys/src/cli/proxy.rs

**Checklist:**
- C753 — When the server's connect fails as the OS or TLS reports it, lys claude, lys codex and lys run start the agent from the kept launch with the proxy on the device recording every call, say so in the first line, and refuse by name with no kept launch or an expired pass; no clock anywhere (AGENTS-012 R2).

**Stories:**
- S426 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person starting an agent on my own computer, I want lys claude to start my agent when the Lys console is off or unreachable, telling me plainly that it is, so that the one word works every time and I never have to open a terminal to work out why it did not.

### R3: A run started while Lys was not answering is handed to Lys whole the next time Lys answers, under the person's name

THE SYSTEM SHALL keep, on the device, every run started from a kept launch (its launch record, its session-start statement, the proxy's record of every call, its start and end) until the server has accepted it. THE SYSTEM SHALL, on the next `lys claude`, `lys codex`, `lys run`, `lys agents` or `lys sign-in` that reaches the server, post each such run to the server as a session of that agent under the person's name, marked started_while_unreachable with the kept launch's hash and the device's run id, and SHALL remove the device's copy only on the server's acceptance of that run by id. The server SHALL accept such a run when the kept launch's hash matches a launch it rendered for that agent and device and the pass named was valid at the run's start, SHALL record it as a session with its calls, and SHALL refuse by name otherwise ("launch hash unknown for <agent> on <machine>", "pass <id> was taken back at <when>, before this run started"), leaving the device's copy in place and the refusal printed. THE SYSTEM SHALL never drop a run: a refused run stays on the device, listed by `lys agents` with its refusal.

**Acceptance:**
- Start a run offline (R2), end it, start the server, run `lys agents`: the server lists the session under the agent with started_while_unreachable and its calls; the device's copy is gone
- The same with the pass revoked on the server between the offline start and the hand-over: the server refuses by name, the device keeps the run, `lys agents` shows it with the refusal
- A hand-over interrupted after the server accepted but before the device removed its copy is idempotent: the second hand-over is accepted again by id and nothing is duplicated on the server (test asserts one session)
- The hand-over is one request per run with its record whole; no call is dropped (count of calls recorded on the device equals the count on the server)

**Files:**
- create: crates/lys/src/commands/launch/handover.rs
- create: crates/lys/src/commands/launch/handover_tests.rs
- create: crates/lys-identity-server/src/sessions_unreachable.rs
- create: crates/lys-identity-server/src/sessions_unreachable_tests.rs
- modify: crates/lys/src/commands/launch.rs
- modify: crates/lys-identity-server/src/routes_startup.rs

**Checklist:**
- C754 — A run started while Lys was unreachable is handed to Lys whole on the next command that reaches it, accepted by launch hash and pass validity at start, idempotent by run id, never dropped, refused by name (AGENTS-012 R3).

**Stories:**
- S427 (An operator, a person running the estate) — As an operator, I want every run that started while Lys was unreachable to appear under its person and agent with its calls and cost the moment Lys answers again, so that nothing an agent did in the storm case is missing from the record.

### R4: The surface shows a run that started while Lys was not answering for what it is

THE SYSTEM SHALL show, on the agent's file and on Running agents, a session handed over under R3 with the words "started while Lys was not answering, from <device>, handed over <when>" beside its start time, and its calls and cost like any other session. It SHALL NOT be shown as an error and SHALL NOT be hidden from any list a session appears in.

**Acceptance:**
- The agent's sessions list and Running agents show the handed-over session with those words (vitest, against the server's answer shape)
- Cost and usage totals include it (the dashboard's week total moves by its cost in the test)

**Files:**
- modify: surface/identity/src/features/file/sections.tsx
- modify: surface/identity/src/features/sessions/Sessions.tsx
- modify: surface/identity/src/features/runtime/Sessions.tsx

**Checklist:**
- C755 — The surface shows a handed-over session as started while Lys was not answering, from its device, counted in cost like any other (AGENTS-012 R4).

**Stories:**
- S427 (An operator, a person running the estate) — As an operator, I want every run that started while Lys was unreachable to appear under its person and agent with its calls and cost the moment Lys answers again, so that nothing an agent did in the storm case is missing from the record.

### R5: The first run on a bare machine asks once and keeps the answer; install writes the device's Claude Code rules

THE SYSTEM SHALL, when `lys sign-in`, `lys claude`, `lys codex` or `lys run` runs on a machine with no sign-in state and no --server, ask for the Lys address on the terminal once ("Where is your Lys? (an https address)"), keep it under the sign-in state root, and continue into sign-in; it SHALL NOT answer config_unreadable or any file-not-found to a person. THE SYSTEM SHALL, on the first successful launch on a device, write the device's Claude Code settings that the launch needs (the model proxy address, the Lys MCP entry, the hooks the reviewed profile names) into the seat's own configuration as today, and SHALL state in `lys agents` which settings it wrote and where; enforcement is by those written settings and the pass, never by an enterprise or business account.

**Acceptance:**
- On a machine with no sign-in state, `lys sign-in` asks for the address, keeps it, and the next `lys sign-in` does not ask again
- `lys identity setup-code` and every launch command on a bare machine answer a named next act, never config_unreadable (the 18 Oct 09:25 case on Dean's laptop, reproduced in a test with an empty state root)
- `lys agents --json` lists the settings written for the device with their paths; no secret value in the listing

**Files:**
- modify: crates/lys/src/commands/launch_context.rs
- modify: crates/lys/src/cli/sign_in.rs
- modify: crates/lys/src/commands/launch.rs

**Checklist:**
- C756 — On a bare machine the first lys command asks for the Lys address once and keeps it, never answers config_unreadable; the first launch writes the device's Claude Code settings and lys agents lists them by path (AGENTS-012 R5).

**Stories:**
- S426 (A person starting an agent on their own computer, types one word and expects the agent to start, whether or not the Lys console is up) — As a person starting an agent on my own computer, I want lys claude to start my agent when the Lys console is off or unreachable, telling me plainly that it is, so that the one word works every time and I never have to open a terminal to work out why it did not.

## Boundaries

- SHALL NOT change what the server renders or how a launch is reviewed; the kept launch is a copy of what the server already gave this device.
- SHALL NOT keep any credential on the device that the existing launch does not already render into the seat's own configuration, and SHALL NOT print a pass, a key or a token in any line.
- SHALL NOT add a daemon, a background sync, a timer or a poll: hand-over happens on the next command that reaches the server, and the reach failure is the operating system's own error.
- SHALL NOT start a kept launch for an agent whose pass has expired, nor another agent's or another device's launch, nor an unsigned run.
- SHALL NOT touch the canvas or the terminal front; attachment of an offline run uses the existing managed-frames or pseudo-terminal path on the device.

## Verification

- cargo nextest run -p lys -p lys-identity-server at the head on the Mac and the 205, with the new tests named; cargo clippy --workspace --all-targets -- -D warnings; cargo fmt --all; file-length gate 500; the surface's tsc and vitest for R4.
- By hand on the Mac after install: stop the installed server by pid, run `lys claude` as a person with a kept launch, see the first line and a model call recorded on the device; start the server, run `lys agents`, see the session under the agent with started_while_unreachable. Posted with the lines read.
- sh scripts/design/gate.sh at the committed source; parse the validate, coverage and rendered-comparison lines, keeping only failures, never the exit code alone.
- Worst credible failure if this were used in a hospital: a run an agent made while Lys was unreachable has no owner in the record, or a kept launch runs after its pass was taken back; about everyone a ward's agents act for. Prevented by the kept pass's expiry checked by name, the hand-over of every run whole, and no run ever dropped.
