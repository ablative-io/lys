---
type: brief
id: HOME-037
cluster: home
title: The launch carries every profile field to Claude Code and our Codex build, or refuses it by name
---

# HOME-037: The launch carries every profile field to Claude Code and our Codex build, or refuses it by name

> **Cluster:** home
> **Depends on:** HOME-002, DIRECTORY-051, DIRECTORY-065
> **Blocked by:** DIRECTORY-051 and DIRECTORY-065 landed on main (the Tool policy store and the pinned Codex contract with its policy render): the build starts only when `git log --oneline origin/main --grep=DIRECTORY-065` names its last requirement's commit.
> **Design anchor:**
> - ADR-133 — The launch is everything Lys records for the seat, for Claude Code and our Codex build — A provisioning profile declares its harness build; the launch renders every field for that build or the profile is refused by name when it is recorded. Command MCP servers carry secrets only as handles. Claude Code permissions come from the profile and the Tool policy; Codex permissions are DIRECTORY-065's render of the same policy.
> - ADR-131 — Codex policy comes from Lys and refusal provenance follows the real harness contract — Render the same Lys policy into isolated native Codex settings. Use064's one transport owner and051's one refusal store. Distinguish Codex-reported rejection, Lys judge denial and062 OS denial. Required unrepresentable policy refuses launch. Coverage is capability-derived, never a blanket claim.
> **Checklist:**
> - C188 — The harness and its build are declared in Lys, never in code (HOME-037 R1).
> - C189 — Every model the profile lists is carried or refused by name (HOME-037 R2).
> - C190 — Skills reach the session's own config directory (HOME-037 R3).
> - C191 — Command-started MCP servers with typed settings, secrets only as handles, and their channel policy (HOME-037 R4).
> - C192 — The settings file carries the permissions from the profile and the Tool policy (HOME-037 R5).
> - C193 — A Codex launch renders for our declared Codex build (HOME-037 R6).
> - C194 — Proof: a real seat's setup recorded, rendered and compared (HOME-037 R7).
> **Stories:**
> - S78 (Seat operator, Starts an agent session from what Lys records for it) — As the operator who starts a seat from Lys, I want everything I set for it (its harness build, models, skills, MCP servers and permissions) to be everything Claude Code or our Codex build is given, so that a seat started from Lys is the seat I configured and nothing is silently dropped.

## Purpose

What an operator sets in Lys must be everything the harness gets (Tom, 29 September 11:57 and 11:58). Today launch_template.rs is hard-wired to Claude Code; only the first model becomes --model and the rest are reported and dropped; skills are reported as having no slot and dropped; MCP servers can only be addresses, so no seat's real command-started servers can be recorded; the settings file carries environment only, with tools as a flat --allowedTools and the agent's Tool policy never reaching it. The Codex seats cannot speak to anybody on stock Codex, so the Codex launch must render for our own declared build.

## Task

Carry every provisioning field into the launch of the declared harness, Claude Code or our Codex build, and refuse by name, when the profile is recorded, any field the declared harness cannot carry, so no field is ever dropped with a note.

## Requirements

### R1: The harness and its build are declared in Lys, never in code

Behavioural. A provisioning profile names its harness as a declared build: the kind (claude_code or codex), the program path the operator declares, and the package or build identity that build is verified against. A version string never proves a build: the installed codex-channels package answers --version with 0.0.0, and a PATH codex-cli answers 0.36.0 without being our build. For Codex the declared build reuses DIRECTORY-065 R1's trusted package record and its capability check: the declared program is resolved through its wrapper (the installed cdx resolves the codex-channels symlink, then runs the package's bin/codex), the resolved artifact is verified against its package record, and the native effective configuration it reads back is compared with the render. For Claude Code the declared build is verified the same way against its own install record. launch_template.rs takes the harness from the profile and no longer imports claude_code::HARNESS or names a program, wrapper, config directory, package, version or build anywhere in code; nothing adds a fixed path or a new build. Rendering a profile with no declared harness is refused by name (HarnessUndeclared); a start whose resolved artifact is not the declared package or build is refused by name (HarnessBuildDiffers) before anything is launched, and an unknown Codex build is refused by DIRECTORY-065's codex_policy_contract_unsupported. The Provisioning screen records the declared build and shows the refusal names.

**Acceptance:**
- A test renders two profiles that differ only in their declared harness and gets a Claude Code and a Codex template; grep finds no harness name, program or wrapper path, package, version or build literal in launch_template.rs or launch_harness.rs.
- A profile with no harness is refused HarnessUndeclared at render; a declared Codex build whose wrapper resolves to an artifact that answers the declared --version but is not the declared package is refused HarnessBuildDiffers, and one whose effective configuration read back differs from the render is refused by name, each red on the base.

**Files:**
- create: crates/lys-identity-server/src/launch_harness.rs
- create: crates/lys-identity-server/tests/launch_harness.rs
- modify: crates/lys-identity-server/src/provisioning_store.rs
- modify: crates/lys-identity-server/src/provisioning_api.rs
- modify: crates/lys-identity-server/src/launch_template.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: surface/identity/src/features/provisioning/Provisioning.tsx

**Checklist:**
- C188 — The harness and its build are declared in Lys, never in code (HOME-037 R1).

**Stories:**
- S78 (Seat operator, Starts an agent session from what Lys records for it) — As the operator who starts a seat from Lys, I want everything I set for it (its harness build, models, skills, MCP servers and permissions) to be everything Claude Code or our Codex build is given, so that a seat started from Lys is the seat I configured and nothing is silently dropped.

### R2: Every model the profile lists is carried or refused by name

Behavioural. model_access is carried whole: the first model is the session model and every further model goes into the field the declared harness's pinned contract names for further models. Where that contract has no such field, recording the profile is refused by name (ModelUnrepresentable, naming the harness and the model), so nothing is reported as left out and dropped. left_out() and its report of models after the first are removed.

**Acceptance:**
- A profile of three models renders all three for each harness whose contract carries them, and is refused ModelUnrepresentable at record time for a harness whose contract does not; the base drops the second and third, so the test is red there.

**Files:**
- create: crates/lys-identity-server/src/launch_fields.rs
- create: crates/lys-identity-server/tests/launch_models.rs
- modify: crates/lys-identity-server/src/launch_template.rs
- modify: crates/lys-identity-server/src/lib.rs

**Checklist:**
- C189 — Every model the profile lists is carried or refused by name (HOME-037 R2).

**Stories:**
- S78 (Seat operator, Starts an agent session from what Lys records for it) — As the operator who starts a seat from Lys, I want everything I set for it (its harness build, models, skills, MCP servers and permissions) to be everything Claude Code or our Codex build is given, so that a seat started from Lys is the seat I configured and nothing is silently dropped.

### R3: Skills reach the session's own config directory

Behavioural. Each skill the profile names is a skill Lys keeps by name and content hash, rendered into the session's isolated config directory where the declared harness reads skills (for Claude Code the CLAUDE_CONFIG_DIR the render sets; for Codex the location DIRECTORY-065 R1's contract pins), and recorded in the render manifest and the lys.given entry by path, length and SHA-256. A skill Lys does not keep is refused at record time by name (SkillUnknown). No skill is written into the operator's own home.

**Acceptance:**
- A profile naming two kept skills renders both into the session's config directory with hashes in the manifest, twice identical; an unknown skill is refused SkillUnknown when the profile is recorded; the base reports skills as having no template slot.

**Files:**
- create: crates/lys-home/src/harness/skills.rs
- create: crates/lys-home/src/harness/skills_tests.rs
- create: crates/lys-identity-server/tests/launch_skills.rs
- modify: crates/lys-home/src/harness/mod.rs
- modify: crates/lys-home/src/harness/claude_code/render_write.rs
- modify: crates/lys-identity-server/src/launch_fields.rs
- modify: crates/lys-identity-server/src/provisioning_store.rs

**Checklist:**
- C190 — Skills reach the session's own config directory (HOME-037 R3).

**Stories:**
- S78 (Seat operator, Starts an agent session from what Lys records for it) — As the operator who starts a seat from Lys, I want everything I set for it (its harness build, models, skills, MCP servers and permissions) to be everything Claude Code or our Codex build is given, so that a seat started from Lys is the seat I configured and nothing is silently dropped.

### R4: Command-started MCP servers with typed settings, secrets only as handles, and their channel policy

Behavioural. An MCP server is either an address (http or https, as today) or a command: the program, its arguments, its working directory, and environment whose every member is typed. A setting that is not a secret (a seat file path, a transport address, a mode flag) is a typed literal, a string, number or boolean, carried unchanged; a secret is a use-only handle id that the launch sets and the lys-secrets broker resolves at start, never a value. Each server also carries its channel policy explicitly, off or wake: whether messages the server delivers wake an idle seat. The Codex render writes it as the server's channel member that the fork's McpServerChannelPolicy reads (config/src/mcp_types.rs in the source DIRECTORY-065 R1 pins); the Claude Code render writes that build's channel enablement for the server. The legacy notify hook is outbound turn-complete only and is never taken for inbound wake. The Claude Code mcp slot renders a command server as its stdio entry (command, args, cwd, env of literals and handle variables); the Codex render writes it where the pinned contract names. Recording a server whose program, argument, literal, address or environment carries a credential (a secret-shaped literal, a bearer or key shape, a userinfo URL) is refused by name (McpCredentialInline), naming the server and the member, never echoing the value; a secret is never accepted as a literal. A handle binding the declared build or its launcher cannot resolve is refused by name before launch (McpHandleUnsupported), naming the server, the member and the resolution owner; a channel policy the declared build cannot carry is refused by name when the profile is recorded (McpChannelUnrepresentable). The Provisioning screen records both kinds, their typed settings and their channel policy.

**Acceptance:**
- The five command servers the proof records (meridian, manifold, hammerbarn, cambium-mcp, excalidraw) each render as a stdio entry naming their exact program, arguments and working directory, their nonsecret settings as typed literals, their secrets as handle variables and their channel policy; a render of the same profile is byte identical.
- An argument or literal carrying a key is refused McpCredentialInline without the value appearing in the refusal, a log line or the response; a handle the declared build cannot resolve is refused McpHandleUnsupported before launch; a wake policy on a build without channels is refused McpChannelUnrepresentable; the base refuses a server without a url.

**Files:**
- create: crates/lys-identity-server/src/launch_mcp.rs
- create: crates/lys-identity-server/tests/launch_mcp.rs
- modify: crates/lys-identity-server/src/provisioning_store.rs
- modify: crates/lys-identity-server/src/provisioning_api.rs
- modify: crates/lys-identity-server/src/launch_template.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-home/src/harness/claude_code/template.rs
- modify: docs/design/home/launch-template.schema.json
- modify: surface/identity/src/features/provisioning/Provisioning.tsx

**Checklist:**
- C191 — Command-started MCP servers with typed settings, secrets only as handles, and their channel policy (HOME-037 R4).

**Stories:**
- S78 (Seat operator, Starts an agent session from what Lys records for it) — As the operator who starts a seat from Lys, I want everything I set for it (its harness build, models, skills, MCP servers and permissions) to be everything Claude Code or our Codex build is given, so that a seat started from Lys is the seat I configured and nothing is silently dropped.

### R5: The settings file carries the permissions from the profile and the Tool policy

Behavioural. For Claude Code, the file --settings names carries permissions: allow and deny rules, the permission mode and the additional directories, from the profile's tools and the agent's Tool policy (DIRECTORY-051's policy store), in place of the flat --allowedTools flag. A rule the settings file cannot express is refused at record time by name (PolicyUnrepresentable, naming the rule). For Codex the permissions are the ones DIRECTORY-065 R2 renders from the same policy; this brief adds no second Codex policy render.

**Acceptance:**
- A profile with allow, deny, a mode and two directories, and a Tool policy with one deny, renders a settings file whose permissions equal the union, twice identical, and no --allowedTools flag; an inexpressible rule is refused PolicyUnrepresentable; the base writes environment only.

**Files:**
- create: crates/lys-identity-server/src/launch_permissions.rs
- create: crates/lys-identity-server/tests/launch_permissions.rs
- modify: crates/lys-identity-server/src/launch_template.rs
- modify: crates/lys-identity-server/src/lib.rs
- modify: crates/lys-home/src/harness/claude_code/launch_env.rs
- modify: crates/lys-home/src/harness/claude_code/template.rs
- modify: docs/design/home/launch-template.schema.json

**Checklist:**
- C192 — The settings file carries the permissions from the profile and the Tool policy (HOME-037 R5).

**Stories:**
- S78 (Seat operator, Starts an agent session from what Lys records for it) — As the operator who starts a seat from Lys, I want everything I set for it (its harness build, models, skills, MCP servers and permissions) to be everything Claude Code or our Codex build is given, so that a seat started from Lys is the seat I configured and nothing is silently dropped.

### R6: A Codex launch renders for our declared Codex build

Behavioural. A profile whose declared harness is codex renders through lys-home's Codex harness into an isolated CODEX_HOME: the config file the pinned contract names, carrying the models, the MCP servers of R4 with their typed settings, handle variables and channel policy, the skills of R3 and DIRECTORY-065 R2's permissions, and a launch line that runs the declared program. launch_template.rs dispatches on the declared kind; nothing assumes stock Codex, and a Codex build the contract has not pinned is refused by DIRECTORY-065's codex_policy_contract_unsupported.

**Acceptance:**
- A codex profile renders a CODEX_HOME whose config names every model, server, channel policy and skill of the profile and whose launch line names the declared program; the declared build reads that config back as its effective configuration with each server's channel policy as rendered; a second render is byte identical; the base refuses to render anything but claude_code.

**Files:**
- create: crates/lys-home/src/harness/codex/launch.rs
- create: crates/lys-home/src/harness/codex/launch_tests.rs
- create: crates/lys-identity-server/tests/launch_codex.rs
- modify: crates/lys-home/src/harness/codex/mod.rs
- modify: crates/lys-identity-server/src/launch_template.rs

**Checklist:**
- C193 — A Codex launch renders for our declared Codex build (HOME-037 R6).

**Stories:**
- S78 (Seat operator, Starts an agent session from what Lys records for it) — As the operator who starts a seat from Lys, I want everything I set for it (its harness build, models, skills, MCP servers and permissions) to be everything Claude Code or our Codex build is given, so that a seat started from Lys is the seat I configured and nothing is silently dropped.

### R7: Proof: a real seat's setup recorded, rendered and compared

Proof. A gate script records, in a scratch Lys, a real seat's setup: its models, its skills, its Tool policy and its five command MCP servers (meridian, manifold, hammerbarn, cambium-mcp, excalidraw), with every nonsecret setting as a typed literal, every secret as a handle and each server's channel policy. It renders the Claude Code launch and the Codex launch and compares them with what that seat runs today, exactly: each server's name, program, full argument list, working directory, literal settings, handle variable names and handle identities, channel policy, and the permission rules, never a secret value. It then measures, not reads: it starts each render in a scratch session, lists the tools every server offers there, and delivers a Cambium mention to the idle seat and records that the seat woke and answered it, on our Codex build and on Claude Code. PROOF-LAUNCH-ALL.md records the commit, the declared builds with their package identities, each comparison, each tool list, each delivered mention and wake, and each refusal by name.

**Acceptance:**
- scripts/identity-gates/launch-all.sh exits 0 at the card's head, and PROOF-LAUNCH-ALL.md shows every server, argument, setting, model, skill and rule of the seat present and equal in both renders, or refused by name, with no line left out.
- The proof shows each server's tools listed from the running render and an idle seat on our Codex build and on Claude Code woken by a delivered Cambium mention; a render with a server's channel policy off shows the same mention not waking it.

**Files:**
- create: scripts/identity-gates/launch-all.sh
- create: docs/design/home/PROOF-LAUNCH-ALL.md
- modify: docs/design/project.json

**Checklist:**
- C194 — Proof: a real seat's setup recorded, rendered and compared (HOME-037 R7).

**Stories:**
- S78 (Seat operator, Starts an agent session from what Lys records for it) — As the operator who starts a seat from Lys, I want everything I set for it (its harness build, models, skills, MCP servers and permissions) to be everything Claude Code or our Codex build is given, so that a seat started from Lys is the seat I configured and nothing is silently dropped.

## Boundaries

- SHALL NOT hard-code a harness, program or wrapper path, config directory, package, version or build, and SHALL NOT take a --version answer as proof of a build: each comes from the declared build in the profile, verified against its package record, and the Codex contract is the one DIRECTORY-065 R1 pins.
- SHALL NOT carry a secret value anywhere: a secret in environment, arguments or addresses is a handle id only, a secret is never accepted as a literal, and no refusal, log line, Debug form, response or rendered file holds a value; nonsecret settings are typed literals.
- SHALL NOT drop a field with a note: every field is carried, or refused by name when the profile is recorded, or, for a start whose declared build no longer matches, refused by name before launch.
- SHALL NOT render Codex policy a second way: Codex permissions are DIRECTORY-065 R2's render of the same Lys policy.
- SHALL NOT write into the operator's own home or a live harness home: every rendered file is under the session's isolated config directory or --out.
- SHALL NOT add a timeout, deadline, sleep, poll interval, #[allow], #[ignore], let _ =, unsafe or any bypass; every file stays under 500 lines of code and ast-grep stays at zero hits.

## Verification

- The full Lys gate and the surface checks exit 0 at the card's head, measured by the card round.
- Each requirement's test is red on the base and green at the head, and scripts/identity-gates/launch-all.sh exits 0 with PROOF-LAUNCH-ALL.md written from its run.
