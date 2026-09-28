---
type: brief
id: DIRECTORY-038
cluster: directory
title: Refuse by name every directory act that would give a sign-in identity to an agent
---

# DIRECTORY-038: Refuse by name every directory act that would give a sign-in identity to an agent

> **Cluster:** directory
> **Depends on:** DIRECTORY-003, DIRECTORY-004, DIRECTORY-006
> **Blocked by:** DIRECTORY-003 landed on lys main, its manifest entry checked by one command. In a clone of https://github.com/ablative-io/lys.git with main checked out, from the repository root, run: grep -cE '^[[:space:]]*"crates/lys-identity(-server)?",?$' Cargo.toml. Expected once landed: it prints 2 and exits 0. At 7b536253 it prints 0 and exits 1, so this brief is not dispatched., R3's source path reconciled against DIRECTORY-003's reviewed manifest, checked by one command. In a clone of https://github.com/ablative-io/lys.git with main checked out, from the repository root, run: git ls-files --error-unmatch crates/lys-identity/src/bindings.rs. Expected once landed: it prints crates/lys-identity/src/bindings.rs and exits 0. At 7b536253 it exits 1, so this brief is not dispatched. If DIRECTORY-003's reviewed manifest names its R1 binding module under another path, R3's modify path and its structure row are reconciled to that path before dispatch (CN9), and the refusal never goes in crates/lys-identity/src/lib.rs., DIRECTORY-004 landed on lys main, its fork-side check by one command. In a clone of https://github.com/ablative-io/lys.git with main checked out, from the repository root, run: git ls-tree HEAD vendor/rauthy | grep -vc dd61ac3c84d6b238108dc8438b53043b5177a662. Expected once landed: it prints 1 and exits 0, because the pin names a gated fork commit other than dd61ac3c84d6b238108dc8438b53043b5177a662. At 7b536253 it prints 0 and exits 1, so this brief is not dispatched., DIRECTORY-004's link report on lys main, checked by one command. In a clone of https://github.com/ablative-io/lys.git with main checked out, from the repository root, run: grep -ohE 'ID001_LINK_(PAIR|REFUSAL|AUDIT|MIGRATION)' docs/design/identity/reports/IDENTITY-001-links.md | sort -u. Expected once landed: it prints the four lines ID001_LINK_AUDIT, ID001_LINK_MIGRATION, ID001_LINK_PAIR and ID001_LINK_REFUSAL and exits 0. At 7b536253 the file does not exist, so this brief is not dispatched., DIRECTORY-006 landed on lys main, checked by one command. In a clone of https://github.com/ablative-io/lys.git with main checked out, from the repository root, run: ls crates/lys-identity/src/grants/admission.rs crates/lys-identity-server/src/grants.rs crates/lys-identity-server/src/routes.rs. Expected once landed: it prints the three paths and exits 0. At 7b536253 none exists, it exits non-zero, and this brief is not dispatched.
> **Design anchor:**
> - ADR-001 — Secrets are held behind a handle the door swaps for the credential — A seat holds a short-lived handle bound to its identity. The real credential sits in the door's encrypted store and never leaves the server. The door's proxy checks SpiceDB, swaps the handle for the credential, forwards the call and writes one audit line. Built in Rust inside the door; no OpenBao unless credentials minted on demand are later needed.
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-004 — Manifold is optional and every project stands alone — The engine that starts or ends a seat is whichever one runs the agent: manifold, aion, or a customer's own. Each project in the stack works without the others; an engine without the broker reads its own pool file as it does today.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-105 — Sign-in identities belong to people only; the harness login token is the one named exception — A sign-in identity, a provider account linked to a person in the directory, belongs to that person only: lys never links, delegates or issues from it to an agent, and refuses each such act of its own by name. An agent's own machine account, bound to no person, is a service account and stays allowed. An issuer and subject is bound to one holder, whichever came first. The harness login token recorded at docs/design/identity/STATEMENT-2026-09-22.md:43 and :167 and docs/design/identity/PROVISIONING-2026-09-22.md:18 is the one named exception; it never passes through anything lys issues, links or delegates, and it is not redefined. Rejected: letting a person lend a sign-in identity to their own agent, and redefining the login token so the rule could be claimed without an exception.
> **Checklist:**
> - C313 — Binding to an agent a provider account linked to a person is refused by name and writes nothing, while an agent's own machine account, linked to no person, is accepted as its issuer-subject binding.
> - C314 — A person's link of a provider account the directory already binds to an agent is refused by name, stating that the account is an agent's own account; the agent's binding stands, and the agent is named in the directory's history to its responsible person and a directory administrator, never to the person trying to link.
> - C315 — Delegating from a sign-in identity is refused by name for an agent recipient and for a person recipient, while a service-access grant consented through the same provider account is admitted by the grant rules alone.
> - C316 — The explanation seam lists each of a person's sign-in identities on the cannot-give list with the reason 'sign-in identity', whoever the recipient is.
> - C317 — A sign-in identity refusal shows the provider and subject to the identity's owner and a directory administrator only; anyone else sees the act, the recipient and that a sign-in identity is involved.
> - C318 — Each refusal of an act that would give an agent a sign-in identity leaves the log and projection unchanged, and one counted test over the store after the two directory refusals finds no agent record carrying a sign-in identity.
> **Stories:**
> - S136 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want the directory to refuse every act that would give that account to an agent, so that no agent can ever hold my sign-in.
> - S137 (Person who signs in, Keeps their sign-in identities to themselves) — As a person delegating to an agent or another person, I want my sign-in identities listed as things I cannot give with the reason 'sign-in identity', so that I know why they are never offered.
> - S138 (Person who signs in, Keeps their sign-in identities to themselves) — As an agent's responsible person, I want my agent's own machine account accepted as its binding and kept as the agent's, so that the agent has its own service account without holding anyone's sign-in.
> - S139 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want refusals shown to other callers never to reveal my provider or subject, so that my sign-in account is not disclosed through someone else's refused request.
> - S140 (Directory administrator, Resolves a refused act) — As a directory administrator, I want a refusal to show me the provider and subject involved, and which agent holds a binding a person tried to link, so that I can tell which account and which agent a refused act touched.

## Purpose

Conformance row 1.2 says a sign-in identity is never lent to or held by an agent. A sign-in identity is a person's link to a provider, such as a Google or GitHub account, through which that person signs in. IDENTITY-001 was to state this and never did, and no brief tests it. The directory holds sign-in identities beside people (DIRECTORY-004 links two providers to one person) and gives grants to agents (DIRECTORY-006), so the refusal belongs at the point where anything would attach a sign-in identity to an agent. In the directory a sign-in identity is a provider account, an issuer and subject (P1), linked to a person, and only that is refused; sign-in identities belong to people only (ADR-105). The directory enforces the two acts it owns, linking a provider account to an agent and delegating a grant whose source is a sign-in identity, each refused by name, each writing nothing, and one counted test over the store after those two directory refusals finds no agent record carrying a sign-in identity. The third act, issuing an agent a credential derived from a person's sign-in session, is the broker's (ADR-001): this brief states it as a finding against SECRETS-002 and does not enforce it. This brief names row 1.2 as the row it passes. Row 1.2 is tested in part by this brief's card, and is met in full only when the broker's line (finding 1 in the task) passes on the broker's card.

## Task

Build R1 to R7 in order, once DIRECTORY-003, DIRECTORY-004 and DIRECTORY-006 have landed on lys main (blocked_by gives the one command that checks each). R1 classifies an issuer-subject pair from the directory's signed history; R2 is the one refusal value and its two views; R3 refuses a person's provider account as an agent's binding and accepts an agent's own machine account; R4 refuses a person's link of a provider account the directory already binds to an agent; R5 refuses delegating from a sign-in identity at DIRECTORY-006 admission; R6 serves the reason 'sign-in identity' on the cannot-give list and chooses the refusal's view at the grant seam; R7 is the one test over the store. Every path is relative to the repository root (CN3).

The rule. The directory refuses, by name, every act that would give an agent a sign-in identity. The refusal names the act, the agent and the sign-in identity, and states that sign-in identities belong to people only. Nothing an agent holds or presents is ever a person's sign-in session or token. A sign-in identity is a provider account linked to a person in the directory, and only that is refused. An agent's own machine account, bound to the agent and to no person, is a service account under row 1.3's separate list, and registering it as the agent's issuer-subject binding under DIRECTORY-003 R1 is allowed. A provider account already linked to a person is refused for an agent by name. An issuer and subject is bound to one holder in the directory, whichever came first: when a person links a provider account whose issuer and subject the directory already binds to an agent, the person's link is refused by name, stating that the account is an agent's own account, the agent's binding stands, and nothing is removed or flagged. That refusal names the agent to the agent's responsible person and to directory administrators, and withholds it from the person trying to link: the person linking is told that the account is an agent's own account and may not be linked to a person, and no agent id, name or responsible person is in that refusal. The refusal is recorded in the directory's history, and there the agent is named, so the responsible person and an administrator read which agent holds the binding. The words' 'each refusal writes nothing to the directory' governs the two acts that would give an agent a sign-in identity; the reverse-order refusal is a person's act and is recorded.

Acceptance, as corrected by the lead's ruling. Linking a provider account to an agent is refused by name; delegating from a sign-in identity to an agent is refused by name; each refusal writes nothing to the directory; and no agent record in the directory ever carries a sign-in identity, checked by one test over the store after the two directory refusals. The third refusal, a credential request for an agent that presents a person's sign-in token refused by name, is the broker's finding (finding 1).

The named exception. The exception stays: it is the standing ruling this brief does not overturn, recorded at docs/design/identity/STATEMENT-2026-09-22.md:43 and docs/design/identity/STATEMENT-2026-09-22.md:167 and docs/design/identity/PROVISIONING-2026-09-22.md:18 (SECRETS-002 R5), and it is the one named exception. The harness login token reaches a seat's process under that ruling, and it never passes through anything lys issues, links or delegates. The rule governs everything lys itself gives an agent. The token is not redefined, and the brief does not pretend it is something other than what it is.

Finding 1, against SECRETS-002, carried by the broker's card. Issuing credentials is the broker's, so the third refusal is a rule the broker enforces. Its acceptance line, for the broker's card to carry: the broker refuses to issue an agent any credential derived from a person's sign-in session; a credential request for an agent that presents a person's sign-in token is refused by name and issues nothing. Row 1.2 is met in full only when that line passes.

Finding 2, against DIRECTORY-004, carried by DIRECTORY-004 and the fork-owned brief it waits on. DIRECTORY-004's ID001_LINK_REFUSAL sees only Rauthy, so it cannot see an agent's binding in the directory. The finding: the link path consults the directory's bindings before the fork links, by asking R4's check with the issuer-subject pair. The acceptance line for the reverse order is carried here, by R4.

The screen. The server's reason is this brief's, and the screen is row 2.4's. Apollo's card jAfmblAP carries the cannot-give list with its four reasons, sign-in identity among them, on the delegation screen. This brief asserts that the server returns the reason 'sign-in identity' for such an entry (R6), and names jAfmblAP as where a person sees it. No screen work is duplicated here. The reason is a property of the source, not of the recipient: a sign-in identity is not a thing anyone is given, person or agent, so the server returns it whoever the recipient is.

In: the classification, the refusal and its two views, the binding refusal with the agent's own account accepted, the reverse-order link refusal, the delegation refusal, the cannot-give reason and the store test. Out: the broker's refusal (finding 1), the fork-side call (finding 2), any screen, any change to docs/design/identity/CONFORMANCE.md, to IDENTITY-001 or to SECRETS-002, and any change to lys-core or a published wire format.

## Requirements

### R1: Classify an issuer-subject pair from the directory's signed history

THE SYSTEM SHALL classify an issuer-subject pair, from the directory's signed events, as exactly one of: a sign-in identity (linked to a person, naming that person), an agent's own account (bound to an agent and to no person, naming that agent), or unbound. A person's registration binding and every provider link the DIRECTORY-003 R4 receiver has recorded count as linked to a person; a link the receiver records as removed no longer does. WHILE the receiver holds a link operation naming the pair that is not yet reconciled into a signed event, THE SYSTEM SHALL classify the pair as unreconciled, never as unbound (P5). The classification SHALL be derived from the replayed log, so it is the same after reopen. It SHALL NOT use email, display name or provider-side account labels to decide anything (P1), and SHALL NOT read any state outside the directory's log and the receiver's pending operations.

**Acceptance:**
- With issuer https://accounts.google.com and subject g-100 linked to person P, the classifier returns sign-in identity naming P, and after the store is closed and reopened it still returns sign-in identity naming P.
- With issuer https://accounts.google.com and subject svc-a bound to agent A and to no person, the classifier returns agent's own account naming A.
- A pair with issuer https://accounts.google.com and subject g-999 that no event names returns unbound, including when P's recorded email is the email of the g-999 account in the fixture.
- With the receiver holding a received, unreconciled link operation of issuer https://github.com subject gh-300 to person P, the classifier returns unreconciled.
- The test counts the four classes it exercised and asserts the count is 4.

**Files:**
- create: crates/lys-identity/src/sign_in_identity.rs
- create: crates/lys-identity/tests/sign_in_classification.rs
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C313 — Binding to an agent a provider account linked to a person is refused by name and writes nothing, while an agent's own machine account, linked to no person, is accepted as its issuer-subject binding.

**Stories:**
- S136 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want the directory to refuse every act that would give that account to an agent, so that no agent can ever hold my sign-in.

### R2: Define the sign-in identity refusal and its two views

THE SYSTEM SHALL define one refusal for acts that would give a sign-in identity away. It names the act (link to an agent, or delegate), the recipient and its kind (agent or person), and the sign-in identity (provider and subject), and it states the literal sentence 'sign-in identities belong to people only'. WHEN the refusal is returned to the sign-in identity's owner or to a directory administrator (DIRECTORY-003 R3), THE SYSTEM SHALL show the provider and subject. WHEN it is returned to anyone else, THE SYSTEM SHALL show the act, the recipient and its kind, and that a sign-in identity is involved, and SHALL state the literal sentence 'sign-in identities belong to people only'. The view for anyone else SHALL NOT carry the provider or the subject in any form: not in its text, its Display, its Debug, its serialised response or its error code, which SHALL be the same whichever provider and subject are involved, as DIRECTORY-006 R5 requires of a why-refused response.

**Acceptance:**
- For act delegate, recipient agent A, sign-in identity issuer https://accounts.google.com subject g-100 owned by P: the view returned to P contains https://accounts.google.com, g-100 and the literal sentence 'sign-in identities belong to people only', and the view returned to the configured administrator contains https://accounts.google.com and g-100.
- The same refusal returned to person Q contains the act delegate, agent A, the kind agent, the words sign-in identity and the literal sentence 'sign-in identities belong to people only', and neither https://accounts.google.com nor g-100 appears in any of its Display, its Debug and its serialised JSON.
- The views returned to Q of two refusals that differ only in the sign-in identity (issuer https://accounts.google.com subject g-100, and issuer https://github.com subject gh-200) serialise to byte-identical JSON with the same error code.

**Files:**
- create: crates/lys-identity/src/sign_in_refusal.rs
- create: crates/lys-identity/tests/sign_in_refusal_views.rs
- modify: crates/lys-identity/src/lib.rs

**Checklist:**
- C317 — A sign-in identity refusal shows the provider and subject to the identity's owner and a directory administrator only; anyone else sees the act, the recipient and that a sign-in identity is involved.

**Stories:**
- S139 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want refusals shown to other callers never to reveal my provider or subject, so that my sign-in account is not disclosed through someone else's refused request.
- S140 (Directory administrator, Resolves a refused act) — As a directory administrator, I want a refusal to show me the provider and subject involved, and which agent holds a binding a person tried to link, so that I can tell which account and which agent a refused act touched.

### R3: Refuse a person's provider account as an agent's binding and accept an agent's own machine account

WHEN a caller asks the DIRECTORY-003 R1 binding API to bind an issuer-subject pair to an agent, and R1 classifies the pair as a sign-in identity, THE SYSTEM SHALL refuse with R2's refusal for the act link to an agent, naming the agent and the sign-in identity, and SHALL write no event. This holds whether the agent's responsible person is the sign-in identity's owner or anyone else. IF R1 classifies the pair as unreconciled, THEN THE SYSTEM SHALL refuse by name that the pair's link state is not yet reconciled, and SHALL write no event (P5). WHEN R1 classifies the pair as unbound, THE SYSTEM SHALL accept it as the agent's own machine account, a service account under row 1.3's separate list, as DIRECTORY-003 R1 does. The refusal sits in the binding module DIRECTORY-003 R1 lands, where the binding is admitted, and not in the crate root. THE SYSTEM SHALL NOT withdraw, move or alter any existing binding to make room, and SHALL NOT manufacture a human login for the agent (P3).

**Acceptance:**
- Binding issuer https://accounts.google.com subject svc-a, linked to no person, to agent A is accepted: the log gains exactly one event and R1 then returns agent's own account naming A.
- Binding issuer https://accounts.google.com subject g-100, linked to person P, to agent A whose responsible person is P is refused with the sign-in identity refusal naming act link to an agent, agent A and the sign-in identity, and the log's length and the projection's digest are unchanged.
- The same binding to agent B whose responsible person is Q is refused the same way, and the log's length and the projection's digest are unchanged.
- Binding issuer https://github.com subject gh-300, whose link to P the receiver holds unreconciled, to agent A is refused by name as not yet reconciled, and the log's length and the projection's digest are unchanged.
- The test counts one accepted case and three refused cases, and asserts both counts.

**Files:**
- create: crates/lys-identity/tests/sign_in_agent_binding.rs
- modify: crates/lys-identity/src/bindings.rs

**Checklist:**
- C313 — Binding to an agent a provider account linked to a person is refused by name and writes nothing, while an agent's own machine account, linked to no person, is accepted as its issuer-subject binding.

**Stories:**
- S136 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want the directory to refuse every act that would give that account to an agent, so that no agent can ever hold my sign-in.
- S138 (Person who signs in, Keeps their sign-in identities to themselves) — As an agent's responsible person, I want my agent's own machine account accepted as its binding and kept as the agent's, so that the agent has its own service account without holding anyone's sign-in.

### R4: Refuse a person's link of a provider account the directory binds to an agent

WHEN the link path asks the directory, by issuer-subject pair, whether a person may link that provider account, and R1 classifies the pair as an agent's own account, THE SYSTEM SHALL refuse by name for the act link to a person, stating that the account is an agent's own account and may not be linked to a person, and SHALL record the refusal as one signed entry in the directory's history that names the person trying to link, the issuer-subject pair and the agent holding the binding. THE SYSTEM SHALL show which agent holds the binding only to that agent's responsible person and to a directory administrator (DIRECTORY-003 R3), the same split DIRECTORY-006 R5 makes for a sign-in identity's provider and subject. The refusal returned for the person trying to link SHALL NOT carry the agent's id, the agent's name or the agent's responsible person in any form, and the history entry read as that person SHALL NOT name the agent. THE SYSTEM SHALL NOT remove, flag, move or alter the agent's binding, SHALL NOT write any event other than that one history entry, and SHALL answer only a caller authenticated as the link-audit source the DIRECTORY-003 R4 receiver already admits. WHEN the pair is bound to no agent, THE SYSTEM SHALL answer that the directory holds no agent binding for it, SHALL write no event, and SHALL NOT decide anything the fork's own refusals decide (DIRECTORY-004 R1). The check is served by the standalone identity server beside the receiver; the fork-side call that asks it is finding 2 in the task.

**Acceptance:**
- With issuer https://github.com subject gh-7 bound to agent B, named build-bot and responsible person Q, as B's own account, the check asked for person P returns the refusal naming act link to a person and stating that the account is an agent's own account and may not be linked to a person; the log gains exactly one entry, the refusal's history entry, and R1 still returns agent's own account naming B.
- A search of the body returned for P in that refusal finds neither B's agent id nor the name build-bot nor Q's id: zero matches for each.
- The refusal's history entry read as the configured administrator names agent B, and the same entry read as Q names agent B.
- The same history entry read as P finds neither B's agent id nor the name build-bot: zero matches for each.
- The check for issuer https://github.com subject gh-8, bound to no agent, returns that the directory holds no agent binding for it, and the log's length is unchanged.
- An unauthenticated caller is refused by name, receives no answer about gh-7, and the log's length is unchanged.
- A caller authenticated as the configured administrator, which is not the link-audit source, is refused by name, receives no answer about gh-7, and the log's length is unchanged.
- The test counts the four checks it drove (gh-7 for P, gh-8, unauthenticated, administrator) and the three history reads (administrator, Q, P), and asserts the counts are 4 and 3.

**Files:**
- create: crates/lys-identity-server/src/sign_in_link_check.rs
- create: crates/lys-identity-server/tests/sign_in_link_check.rs
- modify: crates/lys-identity-server/src/routes.rs

**Checklist:**
- C314 — A person's link of a provider account the directory already binds to an agent is refused by name, stating that the account is an agent's own account; the agent's binding stands, and the agent is named in the directory's history to its responsible person and a directory administrator, never to the person trying to link.

**Stories:**
- S138 (Person who signs in, Keeps their sign-in identities to themselves) — As an agent's responsible person, I want my agent's own machine account accepted as its binding and kept as the agent's, so that the agent has its own service account without holding anyone's sign-in.
- S140 (Directory administrator, Resolves a refused act) — As a directory administrator, I want a refusal to show me the provider and subject involved, and which agent holds a binding a person tried to link, so that I can tell which account and which agent a refused act touched.

### R5: Refuse delegating from a sign-in identity at grant admission, for every recipient

WHEN a delegation request reaches DIRECTORY-006 R2 admission and its offered source resolves in the directory to a person's sign-in identity rather than to a grant, THE SYSTEM SHALL refuse with R2's refusal for the act delegate, naming the recipient, the recipient's kind and the sign-in identity, before any mutation, whether the recipient is an agent or a person. The offered source is the value in the request's source grant member of DIRECTORY-006 R1's contract. Admission SHALL resolve that value before its unknown-parent refusal: a value that names a grant resolves to that grant, and a value that is an issuer-subject pair, in the form DIRECTORY-003 R1 uses for an external binding, which R1 of this brief classifies as a sign-in identity, resolves to that sign-in identity. IF R1 classifies the offered issuer-subject pair as unreconciled, THEN THE SYSTEM SHALL refuse by name that the pair's link state is not yet reconciled, before any mutation, and SHALL NOT answer it as unbound (P5). WHEN R1 classifies the pair as unbound or as an agent's own account, the pair is not a sign-in identity and names no grant, so admission SHALL refuse it with DIRECTORY-006's own unknown-parent refusal, before any mutation, and SHALL NOT refuse it with the sign-in identity refusal. THE SYSTEM SHALL NOT add a source kind or any member to DIRECTORY-006 R1's grant contract to do this. A service-access grant consented through the same provider account the person signs in with is a separate record (SECRETS-002 R4); THE SYSTEM SHALL admit or refuse it by DIRECTORY-006's own rules only, and SHALL NOT refuse it because it shares an account with a sign-in identity.

**Acceptance:**
- Person P, who signs in with issuer https://accounts.google.com subject g-100, sends a delegation request to P's agent A whose source grant member holds issuer https://accounts.google.com subject g-100: refused with the sign-in identity refusal naming act delegate, recipient A and kind agent; the log's length and the projection's digest are unchanged.
- P sends the same request to person Q: refused with the sign-in identity refusal naming act delegate, recipient Q and kind person; the log's length and the projection's digest are unchanged.
- P sends a delegation request to agent A whose source grant member holds issuer https://github.com subject gh-300, whose link to P the receiver holds unreconciled: refused by name as not yet reconciled, not with the sign-in identity refusal; the log's length and the projection's digest are unchanged.
- P sends a delegation request to agent A whose source grant member holds issuer https://accounts.google.com subject svc-a, bound to agent A as its own account: refused with DIRECTORY-006's unknown-parent refusal, not with the sign-in identity refusal; the log's length and the projection's digest are unchanged.
- P holds a Google Drive service-access grant consented through issuer https://accounts.google.com subject g-100, with pass-on authority for agents; P delegates it to agent A: accepted, exactly one grant event is written, and A's new grant names the Drive grant as its source.
- The test counts four refusals (two sign-in identity, one not yet reconciled, one unknown parent) and one acceptance, and asserts both counts.

**Files:**
- create: crates/lys-identity/tests/grant_sign_in_identity.rs
- modify: crates/lys-identity/src/grants/admission.rs

**Checklist:**
- C315 — Delegating from a sign-in identity is refused by name for an agent recipient and for a person recipient, while a service-access grant consented through the same provider account is admitted by the grant rules alone.

**Stories:**
- S136 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want the directory to refuse every act that would give that account to an agent, so that no agent can ever hold my sign-in.
- S137 (Person who signs in, Keeps their sign-in identities to themselves) — As a person delegating to an agent or another person, I want my sign-in identities listed as things I cannot give with the reason 'sign-in identity', so that I know why they are never offered.

### R6: Serve the reason 'sign-in identity' on the cannot-give list and choose the refusal's view at the grant seam

WHEN a person asks DIRECTORY-006 R5's explanation seam what they cannot give to a recipient, THE SYSTEM SHALL list each of that person's sign-in identities with the reason 'sign-in identity', whoever the recipient is, agent or person, because the reason is a property of the source and not of the recipient. THE SYSTEM SHALL NOT list another person's sign-in identities to them. WHEN the grant seam returns R5's refusal, THE SYSTEM SHALL choose R2's view by the caller: the owner and a directory administrator see the provider and subject, and anyone else sees the view without them. The list is shown to a person by Cambium card jAfmblAP on the delegation screen; THE SYSTEM SHALL NOT add or change any screen file.

**Acceptance:**
- Person P with sign-in identities issuer https://accounts.google.com subject g-100 and issuer https://github.com subject gh-200 asks what P cannot give to agent A: the response lists exactly those two, each with reason sign-in identity.
- The same question with recipient person Q lists the same two, each with reason sign-in identity.
- Q's sign-in identity issuer https://accounts.google.com subject g-300 appears in neither of P's responses.
- Agent A, holding pass-on authority from P, asks the grant seam to delegate from P's sign-in identity g-100 to agent B: the body A receives contains neither https://accounts.google.com nor g-100; the same refusal read by P contains both, and the same refusal read by the configured administrator contains both.
- The brief's diff changes no file under surface/.

**Files:**
- create: crates/lys-identity-server/tests/grant_cannot_give.rs
- modify: crates/lys-identity-server/src/grants.rs

**Checklist:**
- C316 — The explanation seam lists each of a person's sign-in identities on the cannot-give list with the reason 'sign-in identity', whoever the recipient is.
- C317 — A sign-in identity refusal shows the provider and subject to the identity's owner and a directory administrator only; anyone else sees the act, the recipient and that a sign-in identity is involved.

**Stories:**
- S137 (Person who signs in, Keeps their sign-in identities to themselves) — As a person delegating to an agent or another person, I want my sign-in identities listed as things I cannot give with the reason 'sign-in identity', so that I know why they are never offered.
- S139 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want refusals shown to other callers never to reveal my provider or subject, so that my sign-in account is not disclosed through someone else's refused request.
- S140 (Directory administrator, Resolves a refused act) — As a directory administrator, I want a refusal to show me the provider and subject involved, and which agent holds a binding a person tried to link, so that I can tell which account and which agent a refused act touched.

### R7: Prove over the store that no agent record carries a sign-in identity

After the two directory refusals, linking a provider account to an agent (R3) and delegating from a sign-in identity to an agent (R5), one test over the store SHALL enumerate every agent record, from the projection and again from a replay of the log after reopen, and SHALL find that no agent's binding is classified by R1 as a sign-in identity and that no grant an agent holds has a sign-in identity as its source. The test SHALL NOT pass over zero agents or zero refusals: it counts the agent records it inspected and the refusals it drove, and asserts both.

**Acceptance:**
- The test drives three refusals: binding issuer https://accounts.google.com subject g-100, linked to person P, to agent A (responsible person P); binding it to agent B (responsible person Q); and P delegating from it to agent A. Around each it records the log's length and the projection's digest, finds the two values equal, and asserts it counted 3 refusals.
- After the three refusals and R3's accepted binding of issuer https://accounts.google.com subject svc-a to agent A, the test inspects every agent record, asserts it inspected 2, and finds for each agent zero bindings classified as a sign-in identity and zero held grants whose source is a sign-in identity.
- After the store is closed and reopened, the same enumeration over the replayed log inspects 2 agent records and finds the same zero counts.

**Files:**
- create: crates/lys-identity/tests/sign_in_store.rs

**Checklist:**
- C318 — Each refusal of an act that would give an agent a sign-in identity leaves the log and projection unchanged, and one counted test over the store after the two directory refusals finds no agent record carrying a sign-in identity.

**Stories:**
- S136 (Person who signs in, Keeps their sign-in identities to themselves) — As a person who signs in with a provider account, I want the directory to refuse every act that would give that account to an agent, so that no agent can ever hold my sign-in.

## Boundaries

- SHALL NOT change docs/design/identity/briefs/IDENTITY-001.json, docs/design/identity/briefs/IDENTITY-001.md or docs/design/identity/CONFORMANCE.md (CN1).
- SHALL NOT change SECRETS-002 or the harness login token exception, and SHALL NOT redefine that token as anything other than what it is.
- SHALL NOT build the broker's refusal (finding 1) or any credential issuance; issuance is the broker's (ADR-001).
- SHALL NOT add a seam that exists only to refuse: the refusals sit in the binding seam DIRECTORY-003 owns, the admission and explanation seams DIRECTORY-006 owns, and the directory's side of the link path, whose fork side is finding 2.
- SHALL NOT add or change any screen file; the cannot-give list is shown by Cambium card jAfmblAP.
- SHALL NOT change any file under vendor/rauthy.
- SHALL NOT change lys-core, its published wire formats or lys/delegation/v1, and SHALL NOT add a member or source kind to DIRECTORY-006 R1's grant contract.
- SHALL NOT refuse an agent's own machine account, linked to no person, as its DIRECTORY-003 R1 binding.
- SHALL NOT withdraw, move, flag or alter any existing binding or grant to make a refused act succeed.
- SHALL NOT change any existing brief's id, requirements, estimates or dependency order.

## Verification

- From the repository root: sh scripts/design/gate.sh exits 0.
- At implementation time, from the exact revision, each command exits 0: cargo fmt --all; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps. Report each refusal case and each counted leg the tests exercised.
- Drift injection, each failing exactly one test file, the one built for the check, and no other. First: make R2's view for anyone else carry an error code that varies with the sign-in identity's provider and subject without containing either; exactly one test file fails, crates/lys-identity/tests/sign_in_refusal_views.rs (its byte-identical case), because no other test compares those views across two sign-in identities. Second: make R4's refusal returned for the person trying to link carry the agent's id; exactly one test file fails, crates/lys-identity-server/tests/sign_in_link_check.rs, because no other test drives R4. Revert both.
- Before reporting row 1.2 passed, confirm the broker's card carries finding 1's acceptance line and that it passes; until then row 1.2 is reported as tested in part by this card and awaiting the broker.
