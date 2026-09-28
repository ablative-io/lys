---
type: brief
id: DIRECTORY-062
cluster: directory
title: Every runner-owned agent is contained by the operating system and its denials are visible
---

# DIRECTORY-062: Every runner-owned agent is contained by the operating system and its denials are visible

> **Cluster:** directory
> **Depends on:** DIRECTORY-051
> **Design anchor:**
> - ADR-128 — The runner applies OS containment from the same Lys policy and reports native denials — Compile one Lys policy into Seatbelt on macOS and Landlock with a private network namespace on Linux. A policy-bound egress service enforces hostnames while OS rules prevent direct bypass. A runner applies containment before untrusted exec and records its policy digest and session incarnation. Native kernel events feed the same authenticated refusal stream as051, with distinct provenance from tool and proxy denials. Missing enforcement or required audit support refuses launch; gaps are visible and end affected sessions through the existing ownership mechanism. A writable outside-root fixture that succeeds without confinement is the filesystem control. Never infer sandbox enforcement from a failure to write /etc/x. Both native platforms require real tests and receipts.
> **Checklist:**
> - C425 — One Lys policy becomes a bound containment plan (DIRECTORY-062 R1).
> - C426 — macOS applies Seatbelt before the harness can run (DIRECTORY-062 R2).
> - C427 — Linux applies Landlock and a network namespace before exec (DIRECTORY-062 R3).
> - C428 — Kernel evidence feeds the existing refusal stream (DIRECTORY-062 R4).
> - C429 — The agent page states the sandbox and the evidence (DIRECTORY-062 R5).
> - C430 — A person watches real native denials and an allowed control (DIRECTORY-062 R6).
> **Stories:**
> - S170 (Person running an agent under a policy, Operates and inspects a runner-owned session) — As the person responsible for an agent, I want its operating-system sandbox to enforce the same policy as Lys, so a shell or child process cannot bypass my limits.
> - S171 (Person running an agent under a policy, Operates and inspects a runner-owned session) — As a person watching an agent, I want to see its actual sandbox and OS-backed refusals, so I can distinguish enforced restrictions from missing coverage.

## Purpose

Tom's sandboxing request, relayed by Waffles at 07:14 Melbourne on 29 September 2026, card 11VnzLRp. DIRECTORY-051 owns policy and tool-boundary enforcement. This card extends that same authority to the process boundary, so an agent's shell cannot bypass it. Waffles corrected the /etc/x demonstration at 07:16: an ordinary permission failure is not proof of sandbox enforcement. Use a writable outside-root control and authentic OS denial evidence.

## Task

Apply Seatbelt on macOS and Landlock plus a network namespace on Linux before any runner-owned agent executes. Bind enforcement to the same Lys policy, deliver native denials to the existing refusal page, and prove both platforms with visible controlled probes.

## Requirements

### R1: One Lys policy becomes a bound containment plan

Behavioural. Extend DIRECTORY-051's Lys-owned enforcement policy with its process-containment projection. The server derives that projection from the same policy and granted authority, never from a runner-local allow list or an agent's settings. The plan names the agent, runner, session incarnation, policy revision and digest, isolated home and workspace, declared read-only runtime inputs, allowed destination hosts and required enforcement/reporting capabilities. The authenticated runner verifies the binding before launch and records the exact plan it enforces. Reject a missing or stale plan, a mismatched session/runner, or an unsupported policy operation by name before the agent runs. Interpret writable roots as directory objects, not string prefixes. Temporary files and caches belong beneath those roots. Keep credentials, policy inputs, audit transport and runner control endpoints inaccessible to the agent except through their existing explicitly granted interfaces. A capability probe reports actual OS enforcement and audit support. An unsupported host or missing privilege produces containment_unavailable naming the missing capability, with no uncontained fallback. A tightened policy stops the affected session through the existing runner lifetime mechanism and obtains its exit evidence before launching under a replacement plan. A policy update never silently leaves a session on a revoked grant. Record unavailable, prepared, enforced, ending and ended states from evidence, and never call prepared enforced.

**Acceptance:**
- A plan carries the same revision and digest as the DIRECTORY-051 policy used for the session.
- A modified plan signed by no admitted authority is refused before a child is spawned.
- A plan bound to another runner or session incarnation is refused before a child is spawned.
- A stale policy revision is refused before a child is spawned.
- An unsupported capability returns containment_unavailable naming that capability and the child-spawn counter remains zero.
- A policy tightening records the old session exit before a replacement session is reported enforced.
- A sibling path sharing a textual prefix with the workspace receives no writable-root permission.

**Files:**
- create: crates/lys-runner/src/containment_policy.rs
- create: crates/lys-runner/src/containment.rs
- create: crates/lys-runner/tests/containment_policy.rs
- create: crates/lys-identity-server/src/containment_policy.rs
- create: crates/lys-identity-server/tests/containment_policy.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-runner/src/session.rs
- modify: crates/lys-runner/src/state.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/runner_api.rs
- modify: crates/lys-identity-server/src/runner_sessions.rs

**Checklist:**
- C425 — One Lys policy becomes a bound containment plan (DIRECTORY-062 R1).

**Stories:**
- S170 (Person running an agent under a policy, Operates and inspects a runner-owned session) — As the person responsible for an agent, I want its operating-system sandbox to enforce the same policy as Lys, so a shell or child process cannot bypass my limits.

### R2: macOS applies Seatbelt before the harness can run

Behavioural. Compile the bound plan to a Seatbelt profile and launch the harness through the platform sandbox entry point from the runner's PTY path. The first untrusted instruction, its shell tools and all descendants run under that profile. Deny writes outside the isolated home and workspace. Allow only the declared runtime reads and required PTY/stdio operations. Close inherited descriptors that could provide a write, network or control bypass. The agent cannot replace the profile or launch an unsandboxed helper. A narrowly owned egress service evaluates destination host names from the same policy, including name resolution, redirects and each new connection. Seatbelt permits network access only to that service's bound endpoint; direct IPv4/IPv6, alternative proxies, resolver traffic and local sockets cannot bypass it. This avoids treating a shared IP address as proof of an allowed hostname. Runtime dependencies and the egress endpoint are explicitly part of the bound plan, not blanket file or network exceptions. Kernel denial collection is R4. Refuse launch if this OS cannot enforce the profile or supply its required evidence. A backend probe may check feature support, but a test double cannot establish native Seatbelt enforcement. Apply the existing runner's process ownership and cancellation rules during preparation, start, rotation, stop and restart.

**Acceptance:**
- On macOS the same writable outside-root fixture succeeds without the sandbox and is denied inside it.
- A write inside the isolated workspace succeeds under Seatbelt.
- A descendant shell is subject to the same outside-root denial.
- A symlink from the workspace to the outside fixture does not permit writing the target.
- An inherited writable descriptor to the outside fixture is not available to the harness.
- A direct connection to the controlled denied endpoint is refused by Seatbelt.
- An allowed hostname succeeds through the egress service.
- A denied hostname sharing an address with an allowed hostname is refused by the egress policy.
- Removing the Seatbelt entry point causes a named start refusal with no harness execution.

**Files:**
- create: crates/lys-runner/src/containment_macos.rs
- create: crates/lys-runner/src/containment_egress.rs
- create: crates/lys-runner/tests/containment_macos.rs
- create: crates/lys-runner/tests/containment_egress.rs
- modify: crates/lys-runner/src/containment.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/pty.rs
- modify: crates/lys-runner/src/session/lifecycle.rs
- modify: crates/lys-runner/Cargo.toml
- modify: Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C426 — macOS applies Seatbelt before the harness can run (DIRECTORY-062 R2).

**Stories:**
- S170 (Person running an agent under a policy, Operates and inspects a runner-owned session) — As the person responsible for an agent, I want its operating-system sandbox to enforce the same policy as Lys, so a shell or child process cannot bypass my limits.

### R3: Linux applies Landlock and a network namespace before exec

Behavioural. A single-threaded launch helper enters a private user/mount/network isolation setup, drops inherited capabilities, sets no_new_privs, applies Landlock filesystem restrictions from the bound plan, and only then execs the harness. It admits only the plan's writable directory objects and declared read-only runtime inputs. The network namespace has no direct host or external route. Its only permitted network path reaches the R2 egress service, which enforces the same hostname policy. Landlock's port rules are not a hostname allow list. Restrict namespace escape, mounts, inherited sockets/descriptors and IPC paths that could ask an unrestricted process to act for the agent. The helper's privileged setup, where required by the host, accepts only authenticated runner requests tied to the plan and exact child incarnation, never arbitrary commands or caller-selected policy files. Installation configures those narrowly required privileges and audit access explicitly. Missing host support refuses the start; it does not omit Landlock, the namespace or auditing. Query the running kernel's Landlock ABI and required rights instead of assuming support from a version string. Denial reporting requires kernel audit support and Landlock logging for subsequent execs. Retain every required restriction; do not intersect away unsupported rights and call that complete. The helper waits on an authenticated readiness handshake so an agent is not reported running before confinement succeeds. Failure or cancellation during setup obtains child exit evidence and removes only that session's owned setup resources.

**Acceptance:**
- On Linux the writable outside-root fixture succeeds without isolation and is denied by Landlock inside it.
- A write inside the allowed workspace succeeds under Landlock.
- A child exec retains the filesystem and network restrictions.
- A symlink or rename attempt cannot provide access to the outside fixture.
- A direct IPv4 connection to the denied endpoint fails inside the network namespace.
- A direct IPv6 connection to the denied endpoint fails inside the network namespace.
- An alternative DNS or proxy path cannot reach the denied endpoint.
- A missing required Landlock right refuses the start with no harness execution.
- Missing namespace or kernel-audit permission refuses the start by name.
- Cancellation before readiness produces a child-exit record and no running-session receipt.

**Files:**
- create: crates/lys-runner/src/containment_linux.rs
- create: crates/lys-runner/src/containment_helper.rs
- create: crates/lys-runner/tests/containment_linux.rs
- create: crates/lys/src/cli/containment.rs
- modify: crates/lys-runner/src/containment.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/pty.rs
- modify: crates/lys-runner/src/session/lifecycle.rs
- modify: crates/lys-runner/Cargo.toml
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/cli/runner.rs
- modify: crates/lys/src/identity/install.rs
- modify: Cargo.toml
- modify: Cargo.lock

**Checklist:**
- C427 — Linux applies Landlock and a network namespace before exec (DIRECTORY-062 R3).

**Stories:**
- S170 (Person running an agent under a policy, Operates and inspects a runner-owned session) — As the person responsible for an agent, I want its operating-system sandbox to enforce the same policy as Lys, so a shell or child process cannot bypass my limits.

### R4: Kernel evidence feeds the existing refusal stream

Behavioural. Collect native denial events using the host's authenticated OS audit/log source: macOS sandbox violation records and Linux Landlock audit plus namespace network-filter audit records. Where the namespace drops network traffic, install a policy-bound reject/log path so the denial is observable, rather than inferring it from a hung curl. Attribute events using OS process lifetime evidence and the runner's admitted descendant lineage, not a reusable PID alone or an agent-supplied label. Bind each report to agent, session incarnation, policy digest, native source event identity, action/resource and timestamp. Feed DIRECTORY-051's authoritative refusal store and cursor delivery. Preserve kernel filesystem/network denials and egress-policy denials as different named sources; a proxy refusal is not a kernel report. A Bash exit status, stderr string or agent explanation is never kernel evidence. Keep native source cursors and deduplication durable across reconnect/reopen with checkpoint-plus-tail reads. If a native source cannot provide complete coverage, loses events, or becomes inaccessible, emit an explicit reporting-gap state and end affected sessions through the runner; never answer an empty all-clear or synthesize a missing event. At start, establish audit capability before untrusted execution. Filter to this runner's admitted processes and safe resource descriptions; no secrets, other users' events or full command credentials enter records. Failure to persist a refusal is visible as evidence unavailable and cannot turn a denial into permission.

**Acceptance:**
- A real macOS filesystem denial produces one refusal with the native event identity and the correct session incarnation.
- A real Linux Landlock denial produces one refusal with the native event identity and the correct session incarnation.
- A denied direct network probe produces a native network refusal rather than a parsed command error.
- Replaying the same source event after reopening adds no duplicate refusal.
- An event from a reused PID outside the admitted process lifetime is not attributed to the old agent.
- A kernel-log gap marks reporting incomplete and records the affected session ending.
- A forged terminal line resembling a kernel denial creates no refusal.
- An event containing a secret-bearing command does not persist or display that secret.
- A restart reads no refusal event before its verified checkpoint.

**Files:**
- create: crates/lys-runner/src/containment_audit.rs
- create: crates/lys-runner/src/containment_audit_macos.rs
- create: crates/lys-runner/src/containment_audit_linux.rs
- create: crates/lys-runner/tests/containment_audit.rs
- modify: crates/lys-runner/src/lib.rs
- modify: crates/lys-runner/src/state.rs
- modify: crates/lys-runner/src/protocol.rs
- modify: crates/lys-runner/src/refusals.rs
- modify: crates/lys-runner/tests/refusals.rs
- modify: crates/lys-identity-server/src/refusals_api.rs
- modify: crates/lys-identity-server/src/refusals_store.rs
- modify: crates/lys-identity-server/tests/refusals.rs

**Checklist:**
- C428 — Kernel evidence feeds the existing refusal stream (DIRECTORY-062 R4).

**Stories:**
- S171 (Person running an agent under a policy, Operates and inspects a runner-owned session) — As a person watching an agent, I want to see its actual sandbox and OS-backed refusals, so I can distinguish enforced restrictions from missing coverage.

### R5: The agent page states the sandbox and the evidence

Behavioural. Add a plain containment section to the existing agent page beside DIRECTORY-051's refusal list. It names the OS backend, applied policy revision, allowed write locations and destination hosts, the enforcement state and audit coverage. Explain unavailable or ending states with the actual capability or failure and the person authorized to change the relevant policy. Do not offer an unsandboxed start switch or imply ordinary grants can override an unavailable OS protection. The server exposes these states from runner receipts, not from a browser calculation or profile checkbox. Read visibility and streamed updates use the existing agent authorization boundary. Each refusal links to its applied policy and distinguishes kernel, egress and tool-boundary sources in plain language. Changes arrive through existing push/cursor delivery. The UI keeps the same components, typography, keyboard access and reduced-motion behavior as the agent page. Runtime state must remain truthful after a reconnect, a stop, an audit gap or a policy change.

**Acceptance:**
- The agent page shows enforced only after the matching runner readiness receipt.
- The page displays the native backend and applied policy digest returned by the server.
- A missing capability is shown by name on the page.
- A reporting gap is visible beside the refusal list.
- A user without access to the agent cannot read its containment state or streamed changes.
- Reconnecting resumes the cursor without duplicate denial rows.
- Keyboard navigation reaches policy details and the refusal source explanation.

**Files:**
- create: crates/lys-identity-server/src/containment_api.rs
- create: crates/lys-identity-server/tests/containment.rs
- create: surface/identity/src/features/file/AgentContainment.tsx
- create: surface/identity/tests/agent-containment.test.tsx
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/runner_api.rs
- modify: surface/identity/src/features/file/IdentityFile.tsx
- modify: surface/identity/src/features/file/AgentRefusals.tsx
- modify: surface/identity/src/features/file/sections.tsx
- modify: surface/identity/src/api.ts

**Checklist:**
- C429 — The agent page states the sandbox and the evidence (DIRECTORY-062 R5).

**Stories:**
- S171 (Person running an agent under a policy, Operates and inspects a runner-owned session) — As a person watching an agent, I want to see its actual sandbox and OS-backed refusals, so I can distinguish enforced restrictions from missing coverage.

### R6: A person watches real native denials and an allowed control

Evidence. Provide a repeatable demonstration on scratch macOS and Linux installations using the real runner, native containment and a deterministic shell harness, with no model call. The same user first writes a harmless named outside-root fixture successfully without containment; inside the sandbox the agent's Bash touch of that fixture fails, an allowed workspace touch succeeds, and curl to a controlled denied network destination fails. Both denied acts appear live on that agent's page with OS provenance. Do not use /etc/x as proof: ordinary filesystem permissions alone can deny it, as Waffles corrected at 07:16 Melbourne. The denied host is a test-owned listener outside the allowed egress path, not a public or sensitive endpoint. Record each attempted case and its actual receipt, so zero attempts cannot pass. Register required native macOS and Linux containment legs in the repository gate declarations, routed to matching capable venues. Neither platform is proven by cross-compilation, a mocked event, a tool hook or a test on the other platform. Missing capability is a failed/blocked leg, never an ignored test or green result. The card is complete only when both native legs and the ordinary full battery pass on the same commit. The demonstration report records commit, OS/kernel and runtime capabilities, applied policy digest, control result, each denial's source identity, screen evidence and cleanup of owned fixtures. No production policy, host /etc path or real user's files are changed for the test.

**Acceptance:**
- The macOS native leg records a successful unsandboxed write to the outside-root fixture.
- The macOS native leg records a denied sandboxed write to that same fixture.
- The macOS native leg records a successful sandboxed write inside the workspace.
- The macOS native leg records a kernel denial for the controlled direct network probe.
- The Linux native leg records the corresponding outside-root control and sandbox denial as separate results.
- The Linux native leg records a successful allowed workspace write.
- The Linux native leg records a native denial for the controlled direct network probe.
- The real browser receives both denied acts through the authenticated refusal route on each platform.
- The browser shows the source identity of each native refusal.
- The report names every attempted case and both native leg receipts at the exact card commit.
- An unavailable native capability makes its leg fail by name.
- Every owned test fixture is removed after the probe.

**Files:**
- create: scripts/identity-gates/containment-macos.sh
- create: scripts/identity-gates/containment-linux.sh
- create: crates/lys-runner/tests/containment_probe.rs
- create: surface/identity/tests/acceptance/containment.spec.ts
- create: docs/design/directory/PROOF-CONTAINMENT.md
- modify: docs/design/project.json
- modify: docs/design/directory/design.json
- modify: .land/gates.sh
- modify: surface/identity/vite.config.ts
- modify: surface/identity/package.json

**Checklist:**
- C430 — A person watches real native denials and an allowed control (DIRECTORY-062 R6).

**Stories:**
- S171 (Person running an agent under a policy, Operates and inspects a runner-owned session) — As a person watching an agent, I want to see its actual sandbox and OS-backed refusals, so I can distinguish enforced restrictions from missing coverage.

## Boundaries

- SHALL NOT start before DIRECTORY-051 is complete and green and its actual policy, refusal and harness integration is in the base tree. Recheck every touched file against that exact prerequisite head.
- SHALL NOT implement a second authority policy in the runner, proxy or browser.
- SHALL NOT fall back to an uncontained process, omit unsupported rights or claim audit coverage from command output.
- SHALL NOT add a timeout, deadline, polling loop, watchdog or timer. Native event waits end on source events or cancellation.
- SHALL NOT add unsafe, lint suppressions, ignored tests or underscore renames. Use reviewed safe OS interfaces and named helpers.
- SHALL NOT modify live host firewall or namespace rules outside the per-session resources owned by the admitted plan.
- SHALL NOT treat a PID alone as process identity or claim a requested stop as a confirmed exit.
- SHALL NOT change any existing signed wire bytes in place. Version new protocol members where the existing signed contract requires it.
- SHALL NOT count physical lines instead of ADR-111 code lines, grow grants.rs, or evade the file-length gate.
- SHALL NOT claim either native platform passed when its matching venue or required kernel evidence is unavailable.

## Verification

- The full card and landing chain, including Jev, fmt, both strict Clippy configurations, workspace tests, documentation, ast-grep, surface and file-length legs, passes on the exact integrated head.
- Both named native containment legs pass on that same head and their machine receipts are included in PROOF-CONTAINMENT.md.
- The real-browser containment acceptance passes against scratch services and real native enforcement on both platforms.
