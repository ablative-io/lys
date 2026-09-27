---
type: brief
id: DIRECTORY-009
cluster: directory
title: Refuse by name every directory act that would give a sign-in identity to an agent
---

# DIRECTORY-009: Refuse by name every directory act that would give a sign-in identity to an agent

> **Cluster:** directory
> **Depends on:** DIRECTORY-004, DIRECTORY-006
> **Blocked by:** Sign-off of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until it has been reviewed)., DIRECTORY-004 landed on lys main, checked by a command anyone can run: git clone https://github.com/ablative-io/lys.git lys-check && cd lys-check && git ls-tree origin/main vendor/rauthy prints a commit other than dd61ac3c84d6b238108dc8438b53043b5177a662, and git ls-tree -r --name-only origin/main -- docs/design/identity/PROVIDER-LINK-CONTRACT.md docs/design/identity/reports/IDENTITY-001-links.md prints both paths. Until both hold, this brief is not dispatched., DIRECTORY-006 landed on lys main, checked by a command anyone can run: in the same clone, git ls-tree -r --name-only origin/main -- crates/lys-identity/src/grants/admission.rs crates/lys-identity-server/src/grants.rs crates/lys-identity/tests/grant_delegation.rs crates/lys-identity-server/tests/grant_explanations.rs prints all four paths. Until it does, this brief is not dispatched., The binding registration path of DIRECTORY-003 R1 and its link-audit receiver (R4) are named by DIRECTORY-003's reviewed file manifest, which does not exist yet; R1, R3 and R4 below reconcile their call sites to that manifest before dispatch, and a missing seam is a named dispatch blocker, never a new seam invented here., Finding against DIRECTORY-004 (recorded in this brief's task): its link path has no call that asks lys-identity by issuer-subject pair before the fork links. R4 builds the lys-identity side of that call; the fork-side call and its acceptance line are carried by DIRECTORY-004 and the fork-owned brief it waits on.
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-019 — Sign-in identities belong to people only; the harness login token is the one named exception — A sign-in identity, a provider account linked to a person, belongs to that person only: lys refuses by name every act of its own that would give one to an agent or to another person, and an account once linked to a person stays refused for agents after it is unlinked. The harness login token recorded at docs/design/identity/STATEMENT-2026-09-22.md:43 and :167 and docs/design/identity/PROVISIONING-2026-09-22.md:18 is the one named exception; it never passes through anything lys issues, links or delegates, and it is not redefined. Rejected: letting a person lend a sign-in identity to their own agent, dropping the refusal once an account is unlinked, and redefining the login token so the rule could be claimed without an exception.
> **Checklist:**
> - C31 — Binding to an agent a provider account that is, or once was, linked to a person is refused by name and writes nothing, while an agent's own machine account, linked to no person, is accepted as its issuer-subject binding.
> - C32 — A person's link of a provider account already bound to an agent as its service account is refused by name at the lys-identity check the link path asks, naming the withdrawal that answers it, and no binding is withdrawn.
> - C33 — Delegating a sign-in identity is refused by name for an agent recipient and for a person recipient, while a service-access grant consented through the same provider account is admitted by the grant rules alone.
> - C34 — The explanation seam lists each of a person's sign-in identities on the cannot-give list with the reason 'sign-in identity', whoever the recipient is.
> - C35 — A sign-in identity refusal shows the provider and subject to the identity's owner and a directory administrator only; anyone else sees the act, the recipient and that a sign-in identity is involved.
> - C36 — Every sign-in identity refusal leaves the log and projection unchanged, and one counted test over the store finds no agent record carrying a sign-in identity.
> **Stories:**
> - S13 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want the directory to refuse every act that would give that account to an agent, so that no agent can ever hold my sign-in.
> - S14 (Person who signs in, Keeps their sign-in identities to themselves) — As a person delegating to an agent or another person, I want my sign-in identities listed as things I cannot give with the reason 'sign-in identity', so that I know why they are never offered.
> - S15 (Person who signs in, Keeps their sign-in identities to themselves) — As the responsible person, I want my agent's own machine account accepted as its binding, so that the agent can have its own service account without holding anyone's sign-in.
> - S16 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want refusals shown to other callers never to reveal my provider or subject, so that my sign-in account is not disclosed through someone else's refused request.
> - S17 (Directory administrator, Resolves a refused act) — As a directory administrator, I want a sign-in identity refusal to show me the provider and subject involved, so that I can tell which account a refused act touched.

## Purpose

State and test conformance row 1.2 of docs/design/identity/CONFORMANCE.md: a sign-in identity is never lent to or held by an agent. A sign-in identity is a provider account (issuer plus subject, P1) linked to a person in the directory, through which that person signs in; sign-in identities belong to people only (ADR-019). IDENTITY-001 was to state this and never did, and no brief tests it; this brief carries the statement and its tests instead of amending IDENTITY-001 (CN1). The directory holds sign-in identities beside people (DIRECTORY-004) and gives grants to agents (DIRECTORY-006), so the refusal sits at the two acts the directory owns: linking a provider account to an agent, and delegating a sign-in identity. Each is refused by name, writes nothing, and one counted test over the store then finds no agent record carrying a sign-in identity. The third act, issuing an agent a credential derived from a person's sign-in session, is the broker's (ADR-001): this brief states it as a finding against SECRETS-002 and does not enforce it. This brief names row 1.2 as the row it passes; row 1.2 is met in full only when the broker's line (task, finding 1) also passes. The one named exception is the harness login token, which reaches a seat's process under the ruling recorded at docs/design/identity/STATEMENT-2026-09-22.md:43 and docs/design/identity/STATEMENT-2026-09-22.md:167 and docs/design/identity/PROVISIONING-2026-09-22.md:18; it never passes through anything lys issues, links or delegates, it is not redefined here, and the rule governs everything lys itself gives an agent.

## Task

Build R1 to R7 in order, after DIRECTORY-004 and DIRECTORY-006 have landed on lys main (blocked_by gives the command that checks each). R1 classifies an issuer-subject pair from the directory's signed history; R2 is the one refusal value and its two views; R3 refuses a sign-in identity as an agent's binding and accepts an agent's own machine account; R4 is the lys-identity check the link path asks before a person links a provider account; R5 refuses delegating a sign-in identity at DIRECTORY-006 admission; R6 serves the reason 'sign-in identity' on the cannot-give list and the two refusal views at the grant seam; R7 proves every refusal writes nothing and that no agent record carries a sign-in identity. Every path is relative to the repository root (CN3).

The rule. Sign-in identities belong to people only. Nothing an agent holds or presents is ever a person's sign-in session or token. A sign-in identity is never a thing anyone can give, to an agent or to another person; row 1.2 is the agent case, and the person-recipient case follows from the same rule. An agent's own machine account, bound to the agent and to no person, is a service account under row 1.3's separate list and stays allowed as its DIRECTORY-003 R1 issuer-subject binding. A provider account once linked to a person stays refused as any agent's binding after it is unlinked, whether the agent is that person's or anyone else's; the directory keeps the unlink on the record, and the refusal names the account as a former sign-in identity. When an agent's own machine account is bound first and a person later tries to link that same provider account, the agent's binding stands and the person's link is refused by name; nothing is ever withdrawn silently to make room. A service-access grant (for example Google Drive consented through the same account the person signs in with) is a separate record under SECRETS-002 R4, delegated under DIRECTORY-006's own rules; delegating it gives the agent that service's access, never the person's sign-in or session, and this brief does not refuse it.

The named exception. The harness login token reaches a seat's process under the ruling recorded at docs/design/identity/STATEMENT-2026-09-22.md:43 and docs/design/identity/STATEMENT-2026-09-22.md:167 and docs/design/identity/PROVISIONING-2026-09-22.md:18 (SECRETS-002 R5). It is the one named exception, it never passes through anything lys issues, links or delegates, and this brief neither overturns nor redefines it.

Finding 1, against SECRETS-002, carried by the broker's card. Issuing credentials is the broker's (ADR-001), so the third refusal is a rule the broker enforces, not a directory seam. Its acceptance line, for the broker's card to carry: the broker refuses to issue an agent any credential derived from a person's sign-in session; a credential request for an agent that presents a person's sign-in token is refused by name, the refusal names the act, the agent and the sign-in identity, and the request issues nothing. Row 1.2 is met in full only when that line passes.

Finding 2, against DIRECTORY-004, carried by DIRECTORY-004 and the fork-owned brief it waits on. DIRECTORY-004's link path has no call that asks lys-identity by issuer-subject pair before the fork links, and the fork knows only Rauthy users, so it cannot see an agent's binding. Its acceptance line: with an agent's own machine account bound to issuer https://github.com and subject gh-7 in lys-identity, a person's attempt to link that account in the fork is refused by name with R4's refusal, and the fork creates no link.

The screen. The cannot-give list is row 2.4's screen, carried by Cambium card jAfmblAP on the delegation screen; this brief supplies only the server's reason (R6) and does no screen work.

In: the classification, the refusal and its views, the two directory refusals, the pre-link check, the cannot-give reason and the store test. Out: the broker's refusal (finding 1), the fork-side call (finding 2), any screen, any change to CONFORMANCE.md or to IDENTITY-001, and any change to lys-core or a published wire format.

## Requirements

### R1: Classify an issuer-subject pair from the directory's signed history

THE SYSTEM SHALL classify an issuer-subject pair, from the directory's signed events, as exactly one of: a current sign-in identity (linked to a person now, naming that person), a former sign-in identity (linked to a person once and since unlinked, naming that person), an agent's service account (bound to an agent and never to a person, naming that agent), or unbound. A person's registration binding and every provider link the DIRECTORY-003 R4 receiver records count as linked to a person; an unlink the receiver records moves the pair to former and never back to unbound. The classification SHALL be derived from the replayed log, so it is the same after reopen. It SHALL NOT use email, display name or provider-side account labels to decide anything (P1), and SHALL NOT drop a former sign-in identity from the projection on unlink, retirement of the person, or replay.

**Acceptance:**
- With issuer https://accounts.google.com and subject g-100 linked to person P, the classifier returns current sign-in identity naming P.
- After the receiver records P's unlink of that pair, the classifier returns former sign-in identity naming P; after the store is closed and reopened, it still returns former sign-in identity naming P.
- With issuer https://accounts.google.com and subject svc-a bound to agent A and to no person, the classifier returns agent's service account naming A.
- A pair with issuer https://accounts.google.com and subject g-999 that no event names returns unbound, including when P's recorded email is the email of the g-999 account in the fixture.
- The test counts the four classes it exercised and asserts the count is 4.

**Files:**
- create: crates/lys-identity/src/sign_in_identity.rs
- create: crates/lys-identity/tests/sign_in_classification.rs
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C31 — Binding to an agent a provider account that is, or once was, linked to a person is refused by name and writes nothing, while an agent's own machine account, linked to no person, is accepted as its issuer-subject binding.

**Stories:**
- S13 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want the directory to refuse every act that would give that account to an agent, so that no agent can ever hold my sign-in.

### R2: Define the sign-in identity refusal and its two views

THE SYSTEM SHALL define one refusal for acts that would give a sign-in identity away. It names the act (link to an agent, or delegate), the recipient and its kind (agent or person), and the sign-in identity (provider and subject, current or former), and it states the literal sentence 'sign-in identities belong to people only'. WHEN the refusal is returned to the sign-in identity's owner or to a directory administrator (DIRECTORY-003 R3), THE SYSTEM SHALL show the provider and subject. WHEN it is returned to anyone else, THE SYSTEM SHALL show the act, the recipient and its kind, and that a sign-in identity is involved, and SHALL state the literal sentence 'sign-in identities belong to people only'. The view for anyone else SHALL NOT carry the provider or the subject in any form: not in its text, its Display, its Debug, its serialised response or its error code, which SHALL be the same whichever provider and subject are involved. This is what DIRECTORY-006 R5 requires of a why-refused response: it names the blocking condition without disclosing another identity's protected records.

**Acceptance:**
- For act delegate, recipient agent A, sign-in identity issuer https://accounts.google.com subject g-100 owned by P: the owner view shown to P contains https://accounts.google.com and g-100, and so does the administrator view shown to the configured administrator.
- The same refusal shown to person Q contains the act delegate, agent A, the kind agent, the words sign-in identity and the literal sentence 'sign-in identities belong to people only', and neither https://accounts.google.com nor g-100 appears in any of its Display, its Debug and its serialised JSON.
- The redacted views of two refusals that differ only in the sign-in identity (issuer https://accounts.google.com subject g-100, and issuer https://github.com subject gh-200) serialise to byte-identical JSON with the same error code.
- A refusal for a former sign-in identity shown to its owner names it as a former sign-in identity.

**Files:**
- create: crates/lys-identity/src/sign_in_refusal.rs
- create: crates/lys-identity/tests/sign_in_refusal_views.rs
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C35 — A sign-in identity refusal shows the provider and subject to the identity's owner and a directory administrator only; anyone else sees the act, the recipient and that a sign-in identity is involved.

**Stories:**
- S16 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want refusals shown to other callers never to reveal my provider or subject, so that my sign-in account is not disclosed through someone else's refused request.
- S17 (Directory administrator, Resolves a refused act) — As a directory administrator, I want a sign-in identity refusal to show me the provider and subject involved, so that I can tell which account a refused act touched.

### R3: Refuse a sign-in identity as an agent's binding and accept an agent's own machine account

WHEN a caller asks the DIRECTORY-003 R1 binding API to bind an issuer-subject pair to an agent, and R1 classifies the pair as a current or former sign-in identity, THE SYSTEM SHALL refuse with R2's refusal for the act link to an agent, naming the agent and the sign-in identity, and SHALL write no event. This holds whether the agent's responsible person is the sign-in identity's owner or anyone else. WHEN the pair is unbound, THE SYSTEM SHALL accept it as the agent's own machine account, a service account under row 1.3's separate list, as DIRECTORY-003 R1 does today. THE SYSTEM SHALL NOT withdraw, move or alter any existing binding to make room, and SHALL NOT manufacture a human login for the agent (P3).

**Acceptance:**
- Binding issuer https://accounts.google.com subject svc-a, linked to no person, to agent A is accepted: the log gains exactly one event and R1 then returns agent's service account naming A.
- Binding issuer https://accounts.google.com subject g-100, linked to person P, to agent A whose responsible person is P is refused with the sign-in identity refusal naming act link to an agent, agent A and the sign-in identity, and the log's length is unchanged.
- The same binding to agent B whose responsible person is Q is refused the same way, and the log's length is unchanged.
- After P links then unlinks issuer https://github.com subject gh-200, binding that pair to agent A is refused by name as a former sign-in identity, and binding it to agent B is refused the same way; the log's length is unchanged by both.
- The test counts one accepted case and four refused cases, and asserts both counts.

**Files:**
- create: crates/lys-identity/tests/sign_in_agent_binding.rs
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C31 — Binding to an agent a provider account that is, or once was, linked to a person is refused by name and writes nothing, while an agent's own machine account, linked to no person, is accepted as its issuer-subject binding.

**Stories:**
- S13 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want the directory to refuse every act that would give that account to an agent, so that no agent can ever hold my sign-in.
- S15 (Person who signs in, Keeps their sign-in identities to themselves) — As the responsible person, I want my agent's own machine account accepted as its binding, so that the agent can have its own service account without holding anyone's sign-in.

### R4: Answer the link path's question before a person links a provider account

WHEN the link path asks lys-identity, by issuer-subject pair, whether a person may link that provider account, and R1 classifies the pair as an agent's service account, THE SYSTEM SHALL refuse by name. The refusal SHALL say the provider account is bound to an agent as that agent's service account, and SHALL name the act that answers it: the agent's responsible person or a directory administrator withdraws that binding first, and then the person links. THE SYSTEM SHALL NOT withdraw the agent's binding, SHALL NOT write any event, and SHALL answer only a caller authenticated as the link-audit source the DIRECTORY-003 R4 receiver already admits. WHEN the pair is bound to no agent, THE SYSTEM SHALL answer that lys-identity holds no agent binding for it, and SHALL NOT decide anything the fork's own refusals decide (DIRECTORY-004 R1). The check is served by the standalone identity server beside the receiver; the fork-side call is finding 2 in the task.

**Acceptance:**
- With issuer https://github.com subject gh-7 bound to agent A as its service account, the check for person P linking that pair returns the refusal: it states the account is bound to an agent as that agent's service account and names the act that answers it: the binding withdrawn first by one of the two who may withdraw it, the agent's responsible person and a directory administrator, and then the person's link; the log's length is unchanged and R1 still returns agent's service account naming A.
- The check for issuer https://github.com subject gh-8, bound to no agent, returns that lys-identity holds no agent binding for it, and the log's length is unchanged.
- An unauthenticated caller and a caller authenticated as anything other than the link-audit source are each refused by name, and neither receives an answer about gh-7.
- The test counts the four cases it drove and asserts the count is 4.

**Files:**
- create: crates/lys-identity-server/src/sign_in_link_check.rs
- create: crates/lys-identity-server/tests/sign_in_link_check.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C32 — A person's link of a provider account already bound to an agent as its service account is refused by name at the lys-identity check the link path asks, naming the withdrawal that answers it, and no binding is withdrawn.

**Stories:**
- S15 (Person who signs in, Keeps their sign-in identities to themselves) — As the responsible person, I want my agent's own machine account accepted as its binding, so that the agent can have its own service account without holding anyone's sign-in.

### R5: Refuse delegating a sign-in identity at grant admission, for every recipient

WHEN a delegation request reaches DIRECTORY-006 R2 admission and its offered source resolves in the directory to a person's sign-in identity rather than to a grant, THE SYSTEM SHALL refuse with R2's refusal for the act delegate, naming the recipient, the recipient's kind and the sign-in identity, before any mutation, whether the recipient is an agent or a person. THE SYSTEM SHALL NOT add a source kind or any member to DIRECTORY-006 R1's grant contract to do this. A service-access grant consented through the same provider account the person signs in with is a separate record (SECRETS-002 R4); THE SYSTEM SHALL admit or refuse it by DIRECTORY-006's own rules only, and SHALL NOT refuse it because it shares an account with a sign-in identity.

**Acceptance:**
- Person P, who signs in with issuer https://accounts.google.com subject g-100, delegates that sign-in identity to P's agent A: refused with the sign-in identity refusal naming act delegate, recipient A and kind agent, and zero grant events are written.
- P delegates the same sign-in identity to person Q: refused naming act delegate, recipient Q and kind person, and zero grant events are written.
- P holds a Google Drive service-access grant consented through issuer https://accounts.google.com subject g-100, with pass-on authority for agents; P delegates it to agent A: accepted, exactly one grant event is written, and A's new grant names the Drive grant as its source.
- The test counts two refusals and one acceptance, and asserts both counts.

**Files:**
- create: crates/lys-identity/tests/grant_sign_in_identity.rs
- modify: crates/lys-identity/src/grants/admission.rs

**Checklist:**
- C33 — Delegating a sign-in identity is refused by name for an agent recipient and for a person recipient, while a service-access grant consented through the same provider account is admitted by the grant rules alone.

**Stories:**
- S13 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want the directory to refuse every act that would give that account to an agent, so that no agent can ever hold my sign-in.
- S14 (Person who signs in, Keeps their sign-in identities to themselves) — As a person delegating to an agent or another person, I want my sign-in identities listed as things I cannot give with the reason 'sign-in identity', so that I know why they are never offered.

### R6: Serve the reason 'sign-in identity' on the cannot-give list and the two views at the grant seam

WHEN a person asks DIRECTORY-006 R5's explanation seam what they cannot give to a recipient, THE SYSTEM SHALL list each of that person's current sign-in identities with the reason 'sign-in identity', whoever the recipient is. THE SYSTEM SHALL NOT list another person's sign-in identities to them. WHEN the grant seam returns R5's refusal, THE SYSTEM SHALL choose R2's view by the caller: the owner and a directory administrator see the provider and subject, and anyone else sees the redacted view. The list is shown to a person by Cambium card jAfmblAP on the delegation screen, which uses this reason; THE SYSTEM SHALL NOT add or change any screen file.

**Acceptance:**
- Person P with sign-in identities issuer https://accounts.google.com subject g-100 and issuer https://github.com subject gh-200 asks what P cannot give to agent A: the response lists exactly those two, each with reason sign-in identity.
- The same question with recipient person Q lists the same two, each with reason sign-in identity.
- Q's sign-in identity issuer https://accounts.google.com subject g-300 appears in neither of P's responses.
- Agent A, holding pass-on authority from P, asks the grant seam to delegate P's sign-in identity g-100 to agent B: A receives the redacted view, whose body contains neither https://accounts.google.com nor g-100; the same refusal read by P and by the configured administrator contains both.
- No file under surface/ changes in this brief's diff.

**Files:**
- create: crates/lys-identity-server/tests/grant_cannot_give.rs
- modify: crates/lys-identity-server/src/grants.rs

**Checklist:**
- C34 — The explanation seam lists each of a person's sign-in identities on the cannot-give list with the reason 'sign-in identity', whoever the recipient is.
- C35 — A sign-in identity refusal shows the provider and subject to the identity's owner and a directory administrator only; anyone else sees the act, the recipient and that a sign-in identity is involved.

**Stories:**
- S14 (Person who signs in, Keeps their sign-in identities to themselves) — As a person delegating to an agent or another person, I want my sign-in identities listed as things I cannot give with the reason 'sign-in identity', so that I know why they are never offered.
- S16 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want refusals shown to other callers never to reveal my provider or subject, so that my sign-in account is not disclosed through someone else's refused request.
- S17 (Directory administrator, Resolves a refused act) — As a directory administrator, I want a sign-in identity refusal to show me the provider and subject involved, so that I can tell which account a refused act touched.

### R7: Prove every refusal writes nothing and no agent record carries a sign-in identity

WHEN any refusal of R3, R4 or R5 is returned, THE SYSTEM SHALL leave the directory's log length and its projection unchanged. After those refusals, one test over the store SHALL enumerate every agent record, from the projection and again from a replay of the log after reopen, and SHALL find that no agent's binding is classified by R1 as a current or former sign-in identity and that no grant an agent holds has a sign-in identity as its source. The test SHALL NOT pass over zero agents: it counts the agent records it inspected and the refusals it drove, and asserts both.

**Acceptance:**
- Before and after each refusal case of R3, R4 and R5 the test records the log's length and a digest of the projection; for each of the seven refusal cases (the four of R3, the agent-bound refusal of R4 and the two of R5) the two values are equal, and the test asserts it counted 7 refusals.
- After the seven refusals and R3's accepted service-account binding, the store holds agents A and B; the test inspects every agent record, asserts it inspected 2, and finds for each agent zero bindings classified as a current sign-in identity, zero bindings classified as a former sign-in identity and zero held grants whose source is a sign-in identity.
- After the store is closed and reopened, the same enumeration over the replayed log inspects 2 agent records and finds the same zero counts.

**Files:**
- create: crates/lys-identity/tests/sign_in_store.rs

**Checklist:**
- C36 — Every sign-in identity refusal leaves the log and projection unchanged, and one counted test over the store finds no agent record carrying a sign-in identity.

**Stories:**
- S13 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want the directory to refuse every act that would give that account to an agent, so that no agent can ever hold my sign-in.

## Boundaries

- SHALL NOT change docs/design/identity/briefs/IDENTITY-001.json, docs/design/identity/briefs/IDENTITY-001.md or docs/design/identity/CONFORMANCE.md (CN1).
- SHALL NOT change SECRETS-002 R5 or the harness login token exception, and SHALL NOT redefine that token as anything other than what it is.
- SHALL NOT build the broker's refusal (finding 1) or any credential issuance; issuance is the broker's (ADR-001).
- SHALL NOT add a seam that exists only to refuse: the refusals sit in the binding, link-check and admission seams DIRECTORY-003, DIRECTORY-004 and DIRECTORY-006 already own.
- SHALL NOT add or change any screen file; the cannot-give list is shown by Cambium card jAfmblAP.
- SHALL NOT change lys-core, its published wire formats or lys/delegation/v1, and SHALL NOT add a member or source kind to DIRECTORY-006 R1's grant contract.
- SHALL NOT refuse an agent's own machine account, linked to no person, as its DIRECTORY-003 R1 binding.
- SHALL NOT withdraw, move or alter any existing binding or grant to make a refused act succeed.
- SHALL NOT change any existing brief's id, requirements, estimates or dependency order.

## Verification

- From the repository root: sh scripts/design/gate.sh exits 0.
- At implementation time, from the exact revision: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps. Report each refusal case and each counted leg the tests exercised.
- Drift injection: make R3's check accept a former sign-in identity; exactly one test fails, and it is the former-sign-in-identity case of crates/lys-identity/tests/sign_in_agent_binding.rs. Make R2's redacted view carry the subject; exactly one test file fails, crates/lys-identity/tests/sign_in_refusal_views.rs. Revert both.
- Before reporting row 1.2 passed, confirm the broker's card carries finding 1's acceptance line and that it passes; until then row 1.2 is reported as met by the directory and awaiting the broker.
