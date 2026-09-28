---
type: brief
id: DIRECTORY-052
cluster: directory
title: Provision a working team in one act: agents with memories, an opening conversation, a budget, goals, deliverables and a checker
---

# DIRECTORY-052: Provision a working team in one act: agents with memories, an opening conversation, a budget, goals, deliverables and a checker

> **Cluster:** directory
> **Depends on:** DIRECTORY-050, DIRECTORY-051
> **Design anchor:**
> - ADR-119 — A team is provisioned in one act: agents with memories, an opening conversation, a budget, goals and deliverables, and a checker — A team plan is a record: its purpose, a total budget, its deliverables with the evidence each needs, and its members, each with a profile, the memories it starts with, an opening conversation, its share of the budget, its goals and reminders, and a checker (a named person or agent who must accept each deliverable). Provisioning a plan creates the agents, grants, homes, budgets and goals in one act that is all or nothing, and starts them through the runner. A deliverable is met only when its checker accepts it with the evidence.
> **Checklist:**
> - C387 — A team plan names its purpose, total budget, deliverables with their evidence, and each member's profile, memories, opening conversation, budget share, goals and checker (DIRECTORY-052 R1).
> - C388 — Provisioning a plan creates every agent, grant, home, budget and goal in one all-or-nothing act and starts them (DIRECTORY-052 R2).
> - C389 — Each member starts with its chosen memories and its opening conversation already in its session (DIRECTORY-052 R3).
> - C390 — A deliverable is met only when its checker accepts it with the named evidence; the team's spend is held to its total (DIRECTORY-052 R4).
> - C391 — A Teams screen builds a plan from a template, provisions it, and shows each member's state, spend, goals and deliverables (DIRECTORY-052 R5).
> **Stories:**
> - S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

## Purpose

Tom, 28 September 2026 19:55, on Dot: 'I want you to be able to provision new agents so we can provision them off with the right memories, with the right few-shot kind of prompting to start it off, and to be able to send them off to do work without having to know that they've got a budget, know that they've got reminders, know that it's actually going to happen, know that things are actually being checked. So I could say, Waffles, we need a team to work on this. And you could provision them, give them a certain budget, and we can start to budget out things properly.'

## Task

Add team plans: a record that names a team's purpose, budget, deliverables and members; provision it in one all-or-nothing act that creates agents, grants, homes with their starting memories and opening conversation, budgets and goals, and starts them through the runner; hold deliverables to a checker's acceptance with evidence; build, provision and watch plans on a Teams screen and through the API.

## Requirements

### R1: A team plan record

Behavioural. A plan names its purpose, its responsible person, its total budget (tokens and time), its deliverables (each with words, a deadline and the evidence that proves it), and its members; each member names a provisioning profile, the memories it starts with (lantern notes or a predecessor's letter from a named home), an opening conversation (turns authored as lys-home fewshot writes them), its share of the budget, its goals with reminders, and its checker (a person or another member). A plan is refused plan_invalid naming the field when shares exceed the total, a member has no checker, or a deliverable names no evidence.

**Acceptance:**
- A plan whose member shares exceed its total is refused naming the total.
- A member with no checker is refused naming the member.

**Files:**
- create: crates/lys-identity-server/src/team_plans_api.rs
- create: crates/lys-identity-server/src/team_plans_state.rs
- create: crates/lys-identity-server/tests/team_plans.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C387 — A team plan names its purpose, total budget, deliverables with their evidence, and each member's profile, memories, opening conversation, budget share, goals and checker (DIRECTORY-052 R1).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

### R2: Provision in one all-or-nothing act

Behavioural. POST /team-plans/{id}/provision creates, under the responsible person's grants: each agent, its grants as the profile names, its home, its budgets (DIRECTORY-051 R2) and goals (R4), then starts each through the runner (DIRECTORY-050 R3). If any step is refused, everything it created is undone and the refusal names the member and the step; a second provision under the same operation id answers what was done and does nothing twice.

**Acceptance:**
- Provisioning a three-member plan leaves three running agents with their budgets and goals.
- A refusal at the third member leaves nothing of the first two, naming the member and step.
- Sending the same operation twice does nothing the second time.

**Files:**
- create: crates/lys-identity-server/src/team_plans_provision.rs
- modify: crates/lys-identity-server/src/team_plans_api.rs

**Checklist:**
- C388 — Provisioning a plan creates every agent, grant, home, budget and goal in one all-or-nothing act and starts them (DIRECTORY-052 R2).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

### R3: Starting memories and opening conversation

Behavioural. Each member's home is created with the memories the plan names copied in as its inherited memory, and its session starts by resuming the opening conversation written for it, so its first turn already holds them. The render records both in the session's given record (lys.given).

**Acceptance:**
- A member's first answer uses a fact only its starting memory holds.
- The given record lists the memories and the opening conversation by hash.

**Files:**
- modify: crates/lys-home/src/record/given.rs
- modify: crates/lys-identity-server/src/team_plans_provision.rs

**Checklist:**
- C389 — Each member starts with its chosen memories and its opening conversation already in its session (DIRECTORY-052 R3).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

### R4: Deliverables met only by the checker, with evidence; spend held to the total

Behavioural. A member marks a deliverable ready with its evidence (a commit on a named branch, a document, a check that passed); it is met only when its checker accepts it, and the checker may send it back with words. The team's spend is the sum of its members'; when it reaches the plan's total, every member is stopped or the responsible person told, as the plan says.

**Acceptance:**
- A deliverable marked ready without evidence is refused evidence_missing.
- Only the named checker can accept it.
- The team total reached stops every member when the plan says stop.

**Files:**
- modify: crates/lys-identity-server/src/goals_api.rs
- modify: crates/lys-identity-server/src/budgets_act.rs

**Checklist:**
- C390 — A deliverable is met only when its checker accepts it with the named evidence; the team's spend is held to its total (DIRECTORY-052 R4).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

### R5: Teams screen

Behavioural. A Teams screen builds a plan from a template (for example: a builder and a reviewer; a lead with three builders and a checker), provisions it, and shows each member running or stopped, its spend against its share, its goals and the deliverables with their state, styled from the design tokens with no default controls.

**Acceptance:**
- A plan built from a template on the screen provisions and appears with every member running.
- The screen shows spend, goals and deliverables per member.
- Surface tests cover building, provisioning and the refusals.

**Files:**
- create: surface/identity/src/features/team-plans/TeamPlans.tsx
- create: surface/identity/src/features/team-plans/team-plans.css

**Checklist:**
- C391 — A Teams screen builds a plan from a template, provisions it, and shows each member's state, spend, goals and deliverables (DIRECTORY-052 R5).

**Stories:**
- S158 (Person running a team of agents, Starts, watches, talks to and stops agents from Lys) — As a person running agents, I want to ask for a team for a piece of work and have it provisioned with the right memories, opening conversation, budget, goals and checker, so that I can send it off knowing it will be done and checked within what I can afford.

## Boundaries

- SHALL NOT start any member without a budget and a checker.
- SHALL NOT leave a partly provisioned team: all or nothing.
- SHALL NOT mark a deliverable met without its checker's acceptance and the evidence.
- SHALL NOT add a timeout, deadline on a request, poll, #[allow], #[ignore] or any bypass.
- SHALL NOT add a silent fallback: every failure is a named refusal.

## Verification

- The full Lys gate, ast-grep scan and the surface checks exit 0 at the card's head, measured by the card round.
- On a scratch install: build a two-member plan from a template, provision it, see both running with budgets, mark and accept a deliverable.
