---
type: brief
id: DIRECTORY-087
cluster: directory
title: Issue an AI its own product identity with its signed responsible person
---

# DIRECTORY-087: Issue an AI its own product identity with its signed responsible person

> **Cluster:** directory
> **Depends on:** DIRECTORY-067, DIRECTORY-079, DIRECTORY-081
> **Blocked by:** Cambium CORE-047 owns consuming the typed AI identity, its responsible-person association and migration of existing seats. Its owner must agree the signed claim and live-binding contract before implementation is accepted end to end; this brief changes no Cambium code., A real managed-seat proof depends on DIRECTORY-064's launched-binary qualification pin. That work is owned by its qualification box; no DIRECTORY-064 pair, runner transport or qualification list is edited here.
> **Design anchor:**
> - ADR-011 — An identity is registered, active, suspended or retired — An identity is in one of four states: registered (exists in the directory, no grants, no credential handle, may not act), active (may act within its grants), suspended (kept whole, grants kept but not effective) and retired (permanent, history kept, never reactivated; a new identity is made instead). Register, activate, suspend, reinstate and retire are the only transitions, each one signed audit record naming the authenticated actor and their provenance, the identity, from, to, when and reason. Having a grant or a credential is a fact beside the state, not a state. A person is registered by first sign-in; an agent is registered by a signed-in person, who carries it as its responsible person for life and may cause every transition of their own agents. Source: docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:17-44 and docs/design/identity/LIFECYCLE-STATES-2026-09-22.md:71-95.
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> - ADR-116 — Every app registers with Lys through one published API; Lys depends on no app — An app is a record in Lys: an id, a name, its sign-in client, and a permission schema it owns (resource kinds under the app's own prefix, each kind's actions, relations carrying actions, and parent kinds whose relations flow down). An app registers and changes its schema only through the API, is approved by an administrator on a Lys screen before it has any effect, and cannot touch another app's kinds. Lys's own model is the schema of the app 'lys'. The API is described by one OpenAPI document generated from the routes and their types, never written by hand. The MCP server is a face over that same API with three tools, the caller's own identity on every call, and no credential or authority of its own. Lys depends on no app. It holds no app's name, kind, schema or code; it never calls an app, waits on one or reads one's store. Every app depends on Lys through this API alone, and Lys runs the same with no apps registered as with twenty.
> - ADR-126 — Applications have connector identities; people register them with explicit permission — An application connector is its own third identity and grant-holder kind. Apps do not act as agents and do not use service accounts, which are accounts people use to service things. Only a signed-in person with an explicit ordinary register_app grant registers an app. The super administrator may grant that permission to a person; an agent, connector or service account cannot register, even if presented with such a grant. Approval creates a connector identity and binds the app to it. No app authority is implicit in approval, binding or ownership of a kind prefix: every permission is an explicit grant with a chain tracing to the super administrator. Own-kind checks, schema acts, batch and which use that connector identity and the ordinary grant engine. Extend IdentityId, grant admission and the permission-store holder, plus event and snapshot representation with compatibility tests. Preserve historical person/agent bytes and signatures; version a representation when necessary, never rewrite history or coerce identities. This supersedes DIRECTORY-048 wording allowing a service account or connector holding register_app to register, and forbids a separate registrar credential as substitute authority.
> **Checklist:**
> - C449 — An approved app's client is a provider client (DIRECTORY-067 R1).
> - C450 — A retired app's codes and tokens stop at once (DIRECTORY-067 R2).
> - C488 — An app given the profile scope and asking for it receives the person's display name as the name claim; one not asking receives the subject only; one asking for a scope it was not given is refused by name (DIRECTORY-079 R3).
> **Stories:**
> - S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen.
> - S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.
> - S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

## Purpose

Lys's provider currently authorises only a person and signs only that person's subject. A product must distinguish an AI from the person responsible for it, verify both identities and enforce the AI's own rights. On source bf158badec3a9454064eaac3e509c61ecee1492b, provider/endpoints.rs:204 calls own_person; provider.rs Grant stores PersonId; endpoints.rs:490–503 has no responsible-person claim; userinfo parses a profile subject as PersonId. caller_admission.rs already resolves Agent provenance and requires its responsible person Active. C449/C450/C488 and S152/S179/S180 are deliberately split with the existing provider briefs: they establish person/app admission and profile; this brief adds the typed AI principal and its ownership assertion without changing person subjects.

## Task

Extend the existing approved-app authorization-code provider to authenticate a registered AI through its own verified Lys run/pass or signed-agent provenance, issue a token whose subject is that AI, carry the responsible person's issuer/subject in a signed versioned claim, and expose a live binding the product can verify. Keep the existing person flow and product credential custody. Write public conformance fixtures for the Cambium owner; implementation there remains CORE-047. Include the installed token-table migration, restart survival, deterministic count ratchets, observed behavioural reds and a threefold seat fixture. This is a brief-only publication box, not a provider deployment, a new credential, issuer linking or a change to DIRECTORY-064.

## Requirements

### R1: Authenticate the AI as its own registered principal

WHEN a registered AI asks the existing /oauth/authorize endpoint for an approved product, THE SYSTEM SHALL resolve its canonical AgentId from the existing verified run-pass/signed-agent admission owner and bind the authorization to that exact live run/session and provenance. It SHALL NOT require a person's browser cookie, substitute the responsible person's subject, infer identity from a display name/email, accept a caller-supplied AgentId as proof or combine credentials. Use caller_admission's Active-agent/Active-responsible-person checks, and check the product's approved client, registered redirect, scopes, PKCE S256 and the AI's live authority to sign in to that app. Registration remains an explicit existing person-authorised directory act; an unregistered seat is refused and no new Person or Agent is silently minted by OAuth. A person continues through the existing person flow with the same canonical subject. Service accounts/connectors are refused by type unless separately specified. Typed provider grants name PersonSession or AgentRun and never treat a runner session id as a browser session-store id. Authorization-code exchange remains the existing approved-product credential flow; no client-credentials, refresh-token or impersonation grant is introduced. An agent's run proof is sent only to Lys, never forwarded to a product redirect or put in a URL/log.

**Acceptance:**
- provider_agent_has_own_subject authorises an Active registered agent with its own valid run proof and an approved app/PKCE request, then exchanges the code. The signed sub is exactly agent.to_string(), differs from its responsible person's canonical id, and records the agent run provenance. This is a behaviourally runnable red on the base: own_person refuses the Agent before a code is issued.
- provider_agent_admission_refuses_wrong_principal supplies an unregistered agent, an inactive agent, an inactive responsible person, a missing owner, a mixed credential, another run's proof and an app/redirect without authority in separate cases. Each refuses by name before granting a code/token, creates no identity and exposes no credential value.
- provider_person_subject_unchanged runs the existing person flow and asserts the same issuer/sub/client/PKCE rules and subject-only versus profile/name behaviour. Agent admission does not borrow the Registered-person own-account exception or reinterpret a service account as an AI.

**Files:**
- create: crates/lys-identity-server/src/provider/principal.rs
- create: crates/lys-identity-server/tests/provider_agent_identity.rs
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/provider/endpoints.rs
- modify: crates/lys-identity-server/src/provider/exchange.rs
- modify: crates/lys-identity-server/src/error_provider.rs

**Checklist:**
- C449 — An approved app's client is a provider client (DIRECTORY-067 R1).
- C450 — A retired app's codes and tokens stop at once (DIRECTORY-067 R2).

**Stories:**
- S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen.
- S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.
- S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

### R2: Sign the responsible-person binding and preserve old installs

For an AI, THE SYSTEM SHALL sign a lys_identity claim inside the existing EdDSA ID token: {version:1,kind:agent,responsible_person:{iss:<this exact Lys issuer>,sub:<canonical PersonId>},directory_revision:<committed directory checkpoint tree size>}. The outer sub is the canonical AgentId, never the responsible person; the two identifiers are parsed as their declared types. The binding is read from the current directory projection, with both identities Active, and the checkpoint revision is captured with that same projection before signing. The request never supplies this binding. Recheck the live run and ownership at code exchange; ownership/lifecycle changes between authorize and exchange refuse IdentityBindingMoved or the existing lifecycle refusal, not a token with stale or relabelled ownership. The claim is essential for an AI regardless of profile/name scope; no email, person name, key, cookie or run pass is included. Discovery advertises lys_identity and its versioned contract. Person tokens keep their existing subject and optional name semantics; absence of this new claim on an old person token remains the explicitly supported person shape, while an AI token missing/malformed/unknown-version ownership is refused. userinfo returns the same typed identity for the app-bound opaque access token only while its run/person session, approved app and recorded ownership remain current; it never parses an AgentId as PersonId or silently updates the owner under an old token. The persisted Access table gains a versioned typed principal and its session/run binding through the shared migration owner, with a migration from the actual previously installed table bytes. Preserve valid old person tokens and their digest/app/scope/expiry; no unknown row is dropped, no old format is refused merely for being old, and malformed data fails by name. Respect the upgrade-intent fence before writing the new shape; an intent-read failure refuses rather than guessing. No rollback compatibility is claimed without the old-install proof.

**Acceptance:**
- provider_agent_claim_is_signed verifies the token under the public JWKS and independently parses sub, lys_identity.kind/version, responsible_person.iss/sub and directory_revision. Changing the owner, agent subject or revision without resigning fails signature validation; missing/unknown-version fields refuse as an AI token. The base emits no such claim, so the required assertion runs red there.
- provider_agent_binding_changes injects a directory ownership/lifecycle transition between authorize and exchange and before userinfo through explicit events. Each stale token/code is refused naming the changed binding; a newly authorised token names the new confirmed owner, with the AI's enduring subject unchanged.
- provider_agent_access_upgrade opens actual old-install person token-table bytes under the shared migration owner, migrates, issues one AI token and restarts. All old unexpired person tokens retain their original digest/app/scope/expiry and identity; the new token retains its typed AgentRun/binding. Pending upgrade intent and intent-read failure prevent incompatible writes. A malformed row is a named failure, never a dropped-token success.

**Files:**
- create: crates/lys-identity-server/src/provider/identity_claim.rs
- create: crates/lys-identity-server/src/provider_access_migration.rs
- create: crates/lys-identity-server/tests/provider_agent_claim.rs
- create: crates/lys-identity-server/tests/provider_agent_upgrade.rs
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/provider/endpoints.rs
- modify: crates/lys-identity-server/src/provider_tokens.rs

**Checklist:**
- C449 — An approved app's client is a provider client (DIRECTORY-067 R1).
- C450 — A retired app's codes and tokens stop at once (DIRECTORY-067 R2).
- C488 — An app given the profile scope and asking for it receives the person's display name as the name claim; one not asking receives the subject only; one asking for a scope it was not given is refused by name (DIRECTORY-079 R3).

**Stories:**
- S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen.
- S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.
- S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

### R3: Give Cambium a verifiable AI and person contract

THE SYSTEM SHALL supply a public, versioned conformance fixture and consumer contract for the approved Cambium client. Before admitting an AI, the consumer verifies EdDSA/signing key, exact configured issuer, audience, nonce/state, expiry/authentication provenance and the typed lys_identity claim. It checks the signed responsible person's issuer equals the token's approved Lys issuer, parses that sub as PersonId separately from the AgentId outer sub, and reads the live app-bound userinfo binding through the token's own authenticated context. Any mismatch, inactive/unknown binding, unsupported version, source outage or wrong audience refuses by name; no unsigned field, display name, requested owner or bare bearer substitutes for the signed assertion. This is an identity/ownership assertion, not a delegation of the person's roles or grants. Cambium CORE-047 owns associating its AI Seat principal with the verified responsible Person and its existing-seat migration; it never creates a human Person for an AI subject, remints an existing person by email/name or treats ownership as administrator authority. Its later door/wire permission checks ask the AI's own rights live and refuse a revoked binding/grant on the next act; offline ID-token validation alone is not fresh authority and cannot promise immediate revocation. Keep person OIDC issuer/sub semantics and the separate explicit issuer-linking owner unchanged. The contract names each verifier/refusal and includes synthetic signed positive and adversarial fixtures; private or live token/key/store data is never copied into them. Consumer agreement and a real Lys-to-Cambium agent admission remain required qualification evidence, not a claim made by the fixture.

**Acceptance:**
- provider_agent_consumer_conformance uses an independent verifier with only synthetic fixtures and the public JWKS: a valid AI resolves to an AI principal plus its separate responsible-person identity; wrong issuer/audience/nonce, an expired assertion, a tampered owner, missing/unknown-version ownership and a PersonId in the agent sub each refuse with the declared reason.
- provider_agent_live_binding_required changes or suspends the responsible-person binding after a signed assertion, and separately makes userinfo unavailable. Admission refuses until a fresh valid binding/assertion is obtained; it never admits using the offline token alone. The live check adds one declared bounded provider request at admission, not a history scan.
- provider_agent_cambium_seam has the CORE-047 owner read the exact contract/fixtures and run its consumer tests: AI and responsible Person are distinct, an existing Person retains its identifier, ownership grants no inherited role, and the next act after revocation refuses. Unrun consumer tests, seat migration and real product admission are explicitly marked owed.

**Files:**
- create: crates/lys-identity-server/tests/provider_agent_consumer.rs
- create: crates/lys-identity-server/tests/fixtures/provider_agent_identity_v1.json

**Checklist:**
- C449 — An approved app's client is a provider client (DIRECTORY-067 R1).
- C450 — A retired app's codes and tokens stop at once (DIRECTORY-067 R2).
- C488 — An app given the profile scope and asking for it receives the person's display name as the name claim; one not asking receives the subject only; one asking for a scope it was not given is refused by name (DIRECTORY-079 R3).

**Stories:**
- S152 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As an administrator, I want to approve each app and see its schema on a Lys screen before it takes effect, so that no app gives itself power I have not seen.
- S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.
- S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

### R4: Never down: preserve identity through restart and upgrade

THE SYSTEM SHALL preserve the AI's canonical subject, responsible-person association, provider signing key/issuer and valid persisted typed access binding through a provider restart and upgrade. A provider restart is not a seat stop or identity registration; the running seat survives under the never-down supervision slice. An authorization code whose existing volatile state was lost returns the named expired/unknown-code refusal and the same AI may authorise again; it SHALL NOT mint a second identity, repeat a completed product registration or turn an AI into its person. The upgrade owner fences token-table migration and preserves recoverable intent on failure. Missing signing key, unreadable token state or unavailable live run/ownership is a named unavailable state; no fresh random key, empty table, stale-owner assertion or online default is an availability fallback. This requirement makes no promise that a service can answer while its process is absent; it proves preservation and event-driven reattachment without damaging the seat.

**Acceptance:**
- provider_agent_restart_identity issues a fixture AI assertion/access token, signals provider exit and restarts from its owned state. Public key/issuer, canonical AI sub, responsible Person and valid access binding remain equal; the independently held seat child/session is unchanged and reconnects on an explicit ready event.
- provider_agent_upgrade_faults injects migration-intent read failure, unavailable key/state and a lost authorization response separately. Each gives a named refusal or recoverable intent; no duplicate registration/key or AI-to-person conversion occurs, and the running seat is not killed.

**Files:**
- create: crates/lys-identity-server/tests/provider_agent_lifecycle.rs

**Checklist:**
- C449 — An approved app's client is a provider client (DIRECTORY-067 R1).
- C450 — A retired app's codes and tokens stop at once (DIRECTORY-067 R2).

**Stories:**
- S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.
- S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

### R5: Never slow: count authorization, exchange and verification

THE SYSTEM SHALL provide deterministic count fixtures for AI authorize, code exchange, userinfo/live binding and independent consumer verification. Count identity/run/app lookups, lock acquisitions, signature operations, broker/HTTP calls, writes/syncs, whole-state clones and history visits; use instruction counts where supported and name unavailable counting capability instead of inventing it. Current identity and responsible-person reads use the existing projection index, and run/access/code lookup uses its indexed owner. Never hold directory/provider/pass-store locks across a broker or network await; never clone the entire directory, run table or token history to issue or verify one AI assertion. Mutation persistence uses bounded operation records with explicit sync counts; reads and signature verification perform zero disk syncs and loop over zero history rows. Any stored rewrite replacement includes R2's installed-shape migration. Check in the measured initial ceilings only after proving the selected counts track wall-clock improvement for the same workload/build; wall-clock measurements are evidence, not the primary verdict, and other builds are never paused. The accepted count ratchet only goes down; a raised limit, hidden extra call or smaller fixture cannot turn a regression green. A test exceeding two seconds is a defect, not a reason to raise a timer.

**Acceptance:**
- provider_agent_count_ratchet measures the declared counts on the exact source/change, adds one named lookup/signature operation through the fixture seam and goes red, then removes it and passes. Checked-in ceilings cannot increase and the fixture volume cannot shrink.
- provider_agent_counts_no_history runs with equal active principals and increasing expired/retained token history. Reads/signature verification have zero syncs/whole-state clones/history visits, indexed identity/run lookup counts remain within the same ceiling and no lock survives the fixture's broker/network-await signal.
- provider_agent_count_latency_evidence records a real counted-work reduction and its wall-clock comparison under unchanged concurrent estate work. An uncorrelated metric is rejected before becoming a ratchet; no idle-machine or quiet-slot condition is introduced.

**Files:**
- create: crates/lys-identity-server/tests/provider_agent_counts.rs
- create: crates/lys-identity-server/tests/provider_agent_counts.json
- modify: crates/lys-identity-server/src/provider_tokens.rs
- modify: crates/lys-identity-server/src/provider/exchange.rs

**Checklist:**
- C449 — An approved app's client is a provider client (DIRECTORY-067 R1).
- C450 — A retired app's codes and tokens stop at once (DIRECTORY-067 R2).

**Stories:**
- S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.
- S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

### R6: Never break: prove behaviour with reds and explicit faults

Every functional acceptance behaviour SHALL have its named test observed red on pinned main before its implementation, then green on the exact change, with commands, exit codes and full case counts. A missing test target, compile error, copied implementation or unrun scenario is not a behavioural red. Tests use public provider/admission/persistence boundaries and independent claim verification. Ownership change, revocation, provider exit/readiness, denied persistence, failed migration and lost replies are injected by explicit signals/events, never clock sleeps, deadlines or watchdogs. Expiry is asserted by supplied clock data, not waiting for time to pass. No allow/ignore/timeout bypass, default-empty state, discarded token row or generic person fallback may hide a failure. The whole agreed slice is written before its build qualification; focused nextest results never stand for the full assembled gate or the installed Cambium path.

**Acceptance:**
- provider_agent_behaviour_red_matrix records every functional criterion in R1–R5 and R7 with the same runnable test, source/change commits, observed behavioural outcomes, commands/exits and full counts. It refuses missing or compile-only reds and makes every still-unrun consumer/old-install/real-seat leg explicit.
- provider_agent_signal_faults drives all faults/ready transitions under competing load through owned event channels. Every failure is named, expiry changes only supplied clock data and each test remains below the two-second defect bar without added sleeps or timers.

**Files:**
- create: crates/lys-identity-server/tests/provider_agent_faults.rs
- create: crates/lys-identity-server/tests/provider_agent_red_matrix.json

**Checklist:**
- C449 — An approved app's client is a provider client (DIRECTORY-067 R1).
- C450 — A retired app's codes and tokens stop at once (DIRECTORY-067 R2).

**Stories:**
- S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.
- S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

### R7: Scales: three times the seats with flat per-AI work

THE SYSTEM SHALL capture the current unique seat count N from the authenticated roster and run a reproducible equal-work fixture at N and 3N (about 25/75 is a planning estimate, not the count). Each seat gets its own AgentId/run binding and the same bounded claim, app, authorization/exchange/userinfo/verification workload; no repeated person record or shared token stands in for different AI identities. Compare exact total and per-AI lookup/signature/network/write/sync counts and peak owned bytes. Per-AI counts and bounded live state remain flat at the N ceiling; retained history adds no per-read work and an unavailable roster is unknown, never N=0. Expired code/token cleanup is indexed and bounded by the expired entries removed, not a complete-table retain scan per authorization. No process-global lock is held across product/broker calls and one revoked AI cannot delay, stop or lend its responsible person's rights to the other seats.

**Acceptance:**
- provider_agent_threefold_scale pins roster source/revision/N and reports exact completed/refused case counts, per-AI deterministic costs and peak owned bytes at N and3N. Each issued sub/run is distinct, owner associations are correct, per-AI costs stay within the N ceiling and zero read-history scans occur.
- provider_agent_isolated_revocation revokes one AI while all others complete their correlated public provider flows via explicit completion events. Only the revoked identity refuses, nobody receives another AI/person's subject or token, and no full-table cleanup blocks the others.

**Files:**
- create: crates/lys-identity-server/tests/provider_agent_scale.rs
- create: crates/lys-identity-server/tests/provider_agent_scale.json
- modify: crates/lys-identity-server/src/provider.rs
- modify: crates/lys-identity-server/src/provider_tokens.rs

**Checklist:**
- C449 — An approved app's client is a provider client (DIRECTORY-067 R1).
- C450 — A retired app's codes and tokens stop at once (DIRECTORY-067 R2).

**Stories:**
- S179 (Developer of an app that signs in with Lys, Builds a product that uses Lys for sign-in and permissions without Lys knowing about it) — As the developer of an app an administrator approved, I want people to sign in to it through Lys with the client I was issued, so that approval is all my app needs.
- S180 (Administrator, Suspends, reinstates and retires identities, and reads why a check refused) — As an administrator, I want a person I suspend and an app I retire to stop signing in at once, so that my decision takes effect everywhere.

## Boundaries

- Only this DIRECTORY brief pair is written in the current box. No provider/Cambium runtime code, installed state, key, issuer/origin, role policy, source register or live session is changed.
- No DIRECTORY-064 pair/runner/control-channel/pin change. A collision with its in-flight work is a stop naming the exact path; live managed-seat qualification stays its owner's prerequisite.
- Use the existing approved-app authorization-code and virtual credential owners. No new credential-copy path, person impersonation, refresh/client-credentials grant, service-account policy or default agent-to-person conversion.
- Cambium CORE-047 owns consumer implementation, existing-seat migration and door/wire role enforcement. Waffles approved the signed claim shape in 1c273c623302e0aa4ad10b07d0b5a11d46c77acff40a601fa2c3a11c002a5e1a; consumer agreement and implementation qualification remain owed; no conformance fixture claims live product support. The consumer label follows Waffles' confirmed re-split in 843594db47dc548a06285b00b468908d3084de8e9dbaaf0d3cc59560ba343859.
- Person subjects and explicit issuer linking remain their existing owners. Email/display names are not identity, ownership is not inherited authority and a signed offline assertion is not a live grant check.

## Verification

- Publish the handwritten JSON and actual rendered Markdown under the next free DIRECTORY id from verified origin/main (7e094d014f727b1f252cd3f50fd4cd801e6bcb46; highest DIRECTORY-086). Verify every declared modify path with git ls-tree at source bf158badec3a9454064eaac3e509c61ecee1492b. Run the Python-only scripts/design/gate.sh on this Mac under Waffles' current amendment, retaining its exit code and complete parsed failures/warnings; failures must be zero.
- Commit only this pair by pathspec straight to existing main. Push remains held under Waffles until the underlying main qualification/publication is released. Read back exact local commit/tree/bytes and record this distinction; no remote publication is claimed from a local commit.
- Implementation qualification uses observed behavioural reds first, actual formatter, strict touched-crate Clippy and focused nextest only on the authorised remote seat. Write the whole agreed slice before builds; the lead owns the one assembled gate, old-install migration qualification, real Cambium consumer proof and install. No Mac compilation or focused-test/full-suite substitution.
- Handback names exact commits/trees, every command/exit and full counts, changed hot-path locks, whole-state clones, syncs per call and history loops; mark unrun source/change, consumer agreement, persisted-shape migration, roster/count baselines and real-seat legs explicitly.
- Hospital line: a specification does not prove identity or ownership enforcement. The worst credible unresolved failure is a product treating an AI as its responsible person or accepting stale ownership, giving the AI that person's access; signed-claim, live-binding, migration and real-consumer evidence are required before that path is trusted.
