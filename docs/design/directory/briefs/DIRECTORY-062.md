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
> - ADR-128 — The runner applies OS containment from the same Lys policy and reports native denials — Compile one Lys policy into Seatbelt on macOS and Landlock with a private network namespace on Linux. A policy-bound egress service enforces hostnames while OS rules prevent direct bypass. A runner applies containment before untrusted exec and records its policy digest and session incarnation. Native kernel events feed the same authenticated refusal stream as051, with distinct provenance from tool and proxy denials. Missing enforcement or required audit support refuses launch. gaps are visible and end affected sessions through the existing ownership mechanism. A writable outside-root fixture that succeeds without confinement is the filesystem control. Never infer sandbox enforcement from a failure to write /etc/x. Both native platforms require real tests and receipts.
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

Tom's sandboxing request, relayed by Waffles at 07:14 Melbourne on 29 September 2026, card 11VnzLRp. DIRECTORY-051 owns policy and tool-boundary enforcement. This card extends that same authority to the process boundary, so an agent's shell cannot bypass it. Waffles corrected the /etc/x demonstration at 07:16: an ordinary permission failure is not proof of sandbox enforcement. Use a writable outside-root control and authentic OS denial evidence. Archie reviewed the first head at 07:22. Waffles ruled at 07:23 that host preparation is a separate page-driven act with native OS authorisation, and that 205 is the Linux proof venue. These corrections preserve the full two-platform completion requirement.

## Task

Apply Seatbelt on macOS and Landlock plus a network namespace on Linux before any runner-owned agent executes. Bind enforcement to the same Lys policy, deliver native denials to the existing refusal page, and prove both platforms with visible controlled probes.

## Requirements

### R1: One Lys policy becomes a bound containment plan

Behavioural. Extend DIRECTORY-051's Lys-owned enforcement policy with its process-containment projection. The server derives that projection from the same policy and granted authority, never from a runner-local allow list or an agent's settings. The plan names the agent, runner, session incarnation, policy revision and digest, isolated home and workspace, declared read-only runtime inputs, allowed destination hosts and required enforcement/reporting capabilities. The authenticated runner verifies the binding before launch and records the exact plan it enforces. Reject a missing or stale plan, a mismatched session/runner, or an unsupported policy operation by name before the agent runs. Interpret writable roots as directory objects, not string prefixes. Temporary files and caches belong beneath those roots. Keep credentials, policy inputs, audit transport and runner control endpoints inaccessible to the agent except through their existing explicitly granted interfaces. A capability probe reports actual OS enforcement and audit support. An unsupported host or missing privilege produces containment_unavailable naming the missing capability, with no uncontained fallback. A tightened policy stops the affected session through the existing runner lifetime mechanism and obtains its exit evidence before launching under a replacement plan. A policy update never silently leaves a session on a revoked grant. Record unavailable, prepared, enforced, ending and ended states from evidence, and never call prepared enforced.

DIRECTORY-051 at 17eeac97 defines the versioned agent policy in agent_policy_store.rs but has no digest. This row adds a versioned, canonical policy encoding and its digest there and in agent_policy_api.rs, with stable encoding fixtures. The digest includes every enforcement-relevant policy member and is verified from the received policy, not trusted as a caller label. The plan travels in the existing server-signed launch act. The Lys identity server Ed25519 key signs that act through runner_client.rs. The runner verifies it against the public key supplied by its existing --server-key provisioning and socket::Options.server_key, never a key carried by the plan. Preserve the connection challenge and runner-address binding. Name a new protocol version if the signed schema changes. Old signed bytes remain verifiable without being re-encoded.

**Acceptance:**
- A plan carries the same revision and digest as the DIRECTORY-051 policy used for the session.
- A modified plan signed by no admitted authority is refused before a child is spawned.
- A plan bound to another runner or session incarnation is refused before a child is spawned.
- A stale policy revision is refused before a child is spawned.
- An unsupported capability returns containment_unavailable naming that capability and the child-spawn counter remains zero.
- A policy tightening records the old session exit before a replacement session is reported enforced.
- A sibling path sharing a textual prefix with the workspace receives no writable-root permission.
- Changing an enforcement rule changes the canonical policy digest.
- Supplying a verification key inside a plan cannot replace the provisioned server key.
- Replaying a signed plan on a different connection challenge is refused.

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
- modify: crates/lys-identity-server/src/agent_policy_store.rs
- modify: crates/lys-identity-server/src/agent_policy_api.rs
- modify: crates/lys-identity-server/tests/agent_policy.rs
- modify: crates/lys-identity-server/src/runner_client.rs
- modify: crates/lys-runner/src/socket.rs

**Checklist:**
- C425 — One Lys policy becomes a bound containment plan (DIRECTORY-062 R1).

**Stories:**
- S170 (Person running an agent under a policy, Operates and inspects a runner-owned session) — As the person responsible for an agent, I want its operating-system sandbox to enforce the same policy as Lys, so a shell or child process cannot bypass my limits.

### R2: macOS applies Seatbelt before the harness can run

Behavioural. Compile the bound plan to a Seatbelt profile and launch the harness through /usr/bin/sandbox-exec from the runner's PTY path. The first untrusted instruction, its shell tools and all descendants run under that profile. Deny writes outside the isolated home and workspace. Allow only the declared runtime reads and required PTY/stdio operations. Close inherited descriptors that could provide a write, network or control bypass. The agent cannot replace the profile or launch an unsandboxed helper. A narrowly owned egress service evaluates destination host names from the same policy, including name resolution, redirects and each new connection. Seatbelt permits network access only to that service's bound endpoint. Direct IPv4/IPv6, alternative proxies, resolver traffic and local sockets cannot bypass it. This avoids treating a shared IP address as proof of an allowed hostname. Runtime dependencies and the egress endpoint are explicitly part of the bound plan, not blanket file or network exceptions. Kernel denial collection is R4. Refuse launch if this OS cannot enforce the profile or supply its required evidence. A backend probe may check feature support, but a test double cannot establish native Seatbelt enforcement. Apply the existing runner's process ownership and cancellation rules during preparation, start, rotation, stop and restart.

Do not call sandbox_init through FFI or add unsafe Rust. Native log collection requires the host to grant the audit collector access to the unified sandbox log. Establish that access through R3 host preparation, including the administrator-group privilege required by log stream, rather than elevating the agent or assuming that an ordinary runner can read it. An unsupported sandbox-exec or unreadable native log refuses launch.

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

Behavioural. Implement the lys containment launch helper as a separate single-threaded process. It prepares its own private mount and network namespaces, removes inherited capabilities, sets no_new_privs, restricts itself with Landlock and only then execs the harness. Do not use Command::pre_exec or unsafe fork callbacks in the multithreaded runner. Admit only the bound plan's writable directory objects and declared read-only runtime inputs. Restrict namespace escape, inherited descriptors and IPC bypasses. Before exec, verify the actual running Landlock ABI is at least 7 and includes every required filesystem right and audit logging for subsequent execs. Kernel 6.15 introduced ABI 7. A version string alone proves no capability. Missing rights or CONFIG_AUDIT support refuse by name.

The private network namespace has loopback and no host or external route. A named in-namespace forwarder listens only on that namespace's loopback and relays to the host egress service over a dedicated pathname Unix socket bind-mounted into its private mount namespace. The host service alone resolves hostnames and opens external connections after applying the same Lys policy. The socket and forwarder are bound to the admitted plan and session, cannot request arbitrary host operations and cannot select another policy. No implicit veth, host route, privileged slirp process or inherited external socket supplies egress. A direct bypass attempt reaches the namespace's policy-owned reject and log rule. Linux LOG reporting outside the initial network namespace requires net.netfilter.nf_log_all_netns=1. Probe that capability explicitly. Missing log access is not a successful denial receipt. Landlock port rules never stand in for hostname policy.

Host preparation is a separate, named, one-time privileged act, initiated from the Lys agent page and authorised through the operating system's own UI. It is not part of ordinary installation. On macOS use the native administrator authorisation dialog. On Linux use the distribution's graphical authorisation agent through polkit. No terminal command or password field in Lys is required. Without the preparation receipt and a successful capability recheck, return containment_preparation_required naming the host and capability and spawn no agent. Cancelled or denied OS authorisation returns containment_preparation_refused. Absence of a graphical authorisation agent is a named unavailable capability. The helper accepts only the signed preparation operation and its fixed, reviewed resource manifest, never caller-selected commands or arbitrary paths. Record each owned resource and its prior value. Repeated preparation is idempotent and interrupted preparation resumes from that record. It must not restore or overwrite resources subsequently changed by another owner.

Linux preparation gives the isolated audit collector CAP_AUDIT_READ or a narrowly scoped root helper and verifies actual access. Namespace preparation must work under the distribution's policy. On Ubuntu 24.04, account explicitly for kernel.apparmor_restrict_unprivileged_userns with a helper-specific AppArmor profile. Do not disable that restriction globally. The explicitly authorised network logging sysctl is named in the preparation screen and receipt. It cannot change silently during ordinary install or agent start. The privileged helper itself does not become the harness. The final agent has no setup or audit privilege. The launch helper waits for authenticated readiness before untrusted exec is released. Failure or cancellation obtains child exit evidence and removes only the session resources it owns.

**Acceptance:**
- On Linux the same user successfully writes the outside-root fixture without isolation.
- The control file is removed before the Linux denial attempt.
- Landlock denies the sandboxed write to that fixture.
- A write inside the allowed workspace succeeds under Landlock.
- A child exec retains the filesystem restriction.
- A child exec retains the network restriction.
- A symlink cannot provide write access to the outside fixture.
- A rename cannot provide write access to the outside fixture.
- A direct IPv4 connection is rejected inside the network namespace.
- A direct IPv6 connection is rejected inside the network namespace.
- An alternative DNS path cannot reach the denied endpoint.
- An alternative proxy cannot reach the denied endpoint.
- An allowed hostname succeeds through the namespace forwarder and the host Unix socket.
- The forwarder refuses a request bound to another session.
- A missing required Landlock right refuses start before harness execution.
- A running ABI below 7 refuses start naming Landlock audit support.
- Missing namespace permission refuses start by name.
- Missing kernel audit access refuses start by name.
- Disabled noninitial-namespace LOG reporting refuses start by name.
- Without host preparation, start returns containment_preparation_required.
- Cancelling native OS authorisation returns containment_preparation_refused.
- Ordinary install makes no privileged host-preparation change.
- Repeating successful preparation creates no duplicate owned resource.
- Interrupting preparation then resuming preserves another owner's changed resource.
- Ubuntu namespace restrictions remain enabled after helper-specific preparation.
- Cancellation before readiness produces a child-exit record.
- Cancellation before readiness produces no running-session receipt.

**Files:**
- create: crates/lys-runner/src/containment_linux.rs
- create: crates/lys-runner/src/containment_helper.rs
- create: crates/lys-runner/tests/containment_linux.rs
- create: crates/lys/src/cli/containment.rs
- create: crates/lys-runner/src/containment_forwarder.rs
- create: crates/lys-runner/src/containment_prepare.rs
- create: crates/lys-runner/src/containment_prepare_macos.rs
- create: crates/lys-runner/src/containment_prepare_linux.rs
- create: crates/lys-runner/tests/containment_prepare.rs
- create: crates/lys-runner/tests/containment_forwarder.rs
- create: docs/CONTAINMENT-PREPARATION.md
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

Behavioural. Collect native denial events using the host's authenticated OS audit/log source: macOS sandbox violation records and Linux Landlock audit plus namespace network-filter audit records. Where the namespace drops network traffic, install a policy-bound reject/log path so the denial is observable, rather than inferring it from a hung curl. Attribute events using OS process lifetime evidence and the runner's admitted descendant lineage, not a reusable PID alone or an agent-supplied label. Bind each report to agent, session incarnation, policy digest, native source event identity, action/resource and timestamp. Feed DIRECTORY-051's authoritative refusal store and cursor delivery. Preserve kernel filesystem/network denials and egress-policy denials as different named sources. a proxy refusal is not a kernel report. A Bash exit status, stderr string or agent explanation is never kernel evidence. Keep native source cursors and deduplication durable across reconnect/reopen with checkpoint-plus-tail reads. If a native source cannot provide complete coverage, loses events, or becomes inaccessible, emit an explicit reporting-gap state and end affected sessions through the runner. never answer an empty all-clear or synthesize a missing event. At start, establish audit capability before untrusted execution. Filter to this runner's admitted processes and safe resource descriptions. no secrets, other users' events or full command credentials enter records. Failure to persist a refusal is visible as evidence unavailable and cannot turn a denial into permission.

Native macOS logs may coalesce repeated violations. Preserve the source report identity and any reported duplicate count as one aggregate report. Never fabricate one record per attempted syscall. An uncounted suppression or a coalesced report without sufficient identity is an explicit reporting gap. A counted aggregate still marks per-attempt coverage incomplete and follows the gap/end-session rule above. Linux audit read capability and namespace LOG access are checked before launch and on loss. The collector filters privileged source data before forwarding it to the unprivileged runner.

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
- A native report with a duplicate count retains that count in one aggregate refusal.
- Replaying the same aggregate source identity creates no second aggregate.
- A coalesced report marks per-attempt coverage incomplete.
- An uncounted suppression emits a named reporting gap.
- An aggregate never invents individual syscall identities.

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

When host preparation is absent, display Containment unavailable with a Prepare this computer action for an authorised person. That action invokes R3 through the trusted installed local helper and the native OS authorisation UI. Bind the browser request to the signed-in person, selected host and a fresh preparation intent. Enforce the same origin and existing CSRF controls. A remote browser cannot authorise another computer's privileged change. Show the exact requested privileges before the OS dialog and retain an advanced read-only resource manifest. Denial leaves the page unavailable. Completion rechecks capability before changing the state. This adds no unsandboxed start path.

**Acceptance:**
- The agent page shows enforced only after the matching runner readiness receipt.
- The page displays the native backend and applied policy digest returned by the server.
- A missing capability is shown by name on the page.
- A reporting gap is visible beside the refusal list.
- A user without access to the agent cannot read its containment state or streamed changes.
- Reconnecting resumes the cursor without duplicate denial rows.
- Keyboard navigation reaches policy details and the refusal source explanation.
- An unprepared agent page says Containment unavailable.
- An authorised person reaches native host preparation from the page without a terminal.
- A denied OS authorisation leaves containment unavailable.
- A preparation request for another host is refused.
- An unauthorised browser request never invokes the privileged helper.

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

Evidence. Provide a repeatable demonstration on scratch macOS and Linux installations using the real runner, native containment and a deterministic shell harness, with no model call. The same user first writes a harmless named outside-root fixture successfully without containment. inside the sandbox the agent's Bash touch of that fixture fails, an allowed workspace touch succeeds, and curl to a controlled denied network destination fails. Both denied acts appear live on that agent's page with OS provenance. Do not use /etc/x as proof: ordinary filesystem permissions alone can deny it, as Waffles corrected at 07:16 Melbourne. The denied host is a test-owned listener outside the allowed egress path, not a public or sensitive endpoint. Record each attempted case and its actual receipt, so zero attempts cannot pass. Register required native macOS and Linux containment legs in the repository gate declarations, routed to matching capable venues. Neither platform is proven by cross-compilation, a mocked event, a tool hook or a test on the other platform. Missing capability is a failed/blocked leg, never an ignored test or green result. The card is complete only when both native legs and the ordinary full battery pass on the same commit. The demonstration report records commit, OS/kernel and runtime capabilities, applied policy digest, control result, each denial's source identity, screen evidence and cleanup of owned fixtures. No production policy, host /etc path or real user's files are changed for the test.

Route the Linux capability and native test legs to venue 205 through the existing gate workflow. Route the macOS native leg to Dean's macOS venue. Record the actual venue identity, kernel release, Landlock ABI, CONFIG_AUDIT availability, audit reader privilege, namespace policy and network LOG capability from the gate receipt. Do not probe 205 by hand. If 205 lacks kernel 6.15 or the required ABI/audit access, create a separate named venue-prerequisite card linked to 062 and keep its Linux leg blocked. The macOS leg may pass first. That partial pass does not complete or authorise landing 062. The final combined proof reruns any stale native leg on the exact integrated head. Host preparation for the scratch tests uses the same separately authorised page flow and resource manifest, never an implicit privileged test setup.

**Acceptance:**
- The macOS native leg records a successful unsandboxed write to the outside-root fixture.
- The macOS native leg records a denied sandboxed write to that same fixture.
- The macOS native leg records a successful sandboxed write inside the workspace.
- The macOS native leg records a kernel denial for the controlled direct network probe.
- The Linux native leg records a successful allowed workspace write.
- The Linux native leg records a native denial for the controlled direct network probe.
- The browser shows the source identity of each native refusal.
- The report names every attempted case and both native leg receipts at the exact card commit.
- An unavailable native capability makes its leg fail by name.
- Every owned test fixture is removed after the probe.
- The Linux leg on 205 records a successful unsandboxed write to the outside-root fixture.
- The Linux leg on 205 records a denied sandboxed write to that fixture.
- The macOS browser receives the filesystem denial through the authenticated refusal route.
- The macOS browser receives the direct network denial through the authenticated refusal route.
- The Linux browser receives the filesystem denial through the authenticated refusal route.
- The Linux browser receives the direct network denial through the authenticated refusal route.
- The 205 gate receipt names the actual kernel and Landlock ABI.
- A missing Linux prerequisite leaves the Linux leg blocked even when macOS passed.

**Files:**
- create: scripts/identity-gates/containment-macos.sh
- create: scripts/identity-gates/containment-linux.sh
- create: crates/lys-runner/tests/containment_probe.rs
- create: surface/identity/tests/acceptance/containment.spec.ts
- create: docs/design/directory/PROOF-CONTAINMENT.md
- create: scripts/identity-gates/containment-capabilities.sh
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
- SHALL NOT change host resources during ordinary install or agent start. The separately OS-authorised host preparation may change only its displayed fixed manifest. Per-session firewall and namespace resources remain owned by the admitted plan.
- SHALL NOT treat a PID alone as process identity or claim a requested stop as a confirmed exit.
- SHALL NOT change any existing signed wire bytes in place. Version new protocol members where the existing signed contract requires it.
- SHALL NOT count physical lines instead of ADR-111 code lines, grow grants.rs, or evade the file-length gate.
- SHALL NOT claim either native platform passed when its matching venue or required kernel evidence is unavailable.

## Verification

- The full card and landing chain, including Jev, fmt, both strict Clippy configurations, workspace tests, documentation, ast-grep, surface and file-length legs, passes on the exact integrated head.
- Both named native containment legs pass on that same head and their machine receipts are included in PROOF-CONTAINMENT.md.
- The real-browser containment acceptance passes against scratch services and real native enforcement on both platforms.
