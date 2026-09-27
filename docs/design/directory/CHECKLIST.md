# Directory — Checklist

## The revised directory briefs

- [ ] **C1** — The directory design records the outcome, the shared contract as principles, the constraints and the non-goals of IDENTITY-001, revised for the grant ruling.
- [ ] **C2** — The project decision ledger holds the identity decisions with their authority and quote: the maintained Rauthy fork, one PostgreSQL service and database (ADR-005, already recorded), the product accents, and the lifecycle states as working team decisions marked proposed.
- [ ] **C3** — Each open row (02, 04, 03, 05) is a design-system brief, DIRECTORY-002 to DIRECTORY-005, in dependency order, with its wall as files, its ID001 acceptance identifiers kept, and its estimate in its task.
- [ ] **C4** — The grant path (create an agent under a person, grant it a project, the action is allowed, revoke or suspend, the same action is refused) is a requirement with acceptance criteria in the row that owns it, and row 02 states what SpiceDB enforces in step 1 and what it does not.
- [ ] **C5** — Every decision still open for Tom is recorded as open: the grant representation, suspension semantics, the service name, the anchor, and nightly versus waiting for the upstream release.
- [ ] **C6** — The rendered markdown of this cluster matches its JSON and coverage is clean.

## Row 02: the standalone service dependencies (DIRECTORY-002)

- [ ] **C7** — Rauthy, SpiceDB and one PostgreSQL service and database are packaged and installed for development, with separate roles and schema namespaces, the database address as configuration, and readiness, restart, named refusals, the shared database and restore proved (ID001_DEPLOY, ID001_DEPLOY_REFUSAL, ID001_SHARED_DB, ID001_PIN_CLONE).
- [ ] **C8** — lys identity prepare, configure and health exist in the CLI exactly as revision 5's module manifest names them, register the platform and Cambium clients idempotently, and keep every secret out of Git, logs, errors and health output.
- [ ] **C9** — Both Rauthy client themes carry Aion's neutral vocabulary with each product's own accent, Cambium green and the identity orange (ID001_THEME).
- [ ] **C10** — SpiceDB's step-1 role is written down and held: installed and checked, asked for no decision, holding no grant.

## Row 04: the directory and its signed changes (DIRECTORY-003)

- [ ] **C11** — People and agents are registered with enduring identifiers and issuer-subject bindings, each agent under the signed-in person responsible for it, and registration issues no login, credential or certificate (ID001_DIRECTORY).
- [ ] **C12** — Every identity change is one signed event through lys-log-store under a jointly reviewed envelope, every answered projection equals replay across the crash boundaries, and a receipt verifies independently (ID001_AUDIT_FAULTS, ID001_RECEIPT).
- [ ] **C13** — Only the configured administrator mutates the directory in step 1; unauthenticated, same-email and wrong issuer-subject callers are refused without effect (ID001_ADMIN). Under the lead's ruling of 27 September 2026 (DIRECTORY-026), one exception: an agent authenticated by its enrolled key may create and end its own session record, and nothing else; every other mutation by a caller who is not an administrator, including an agent acting on another agent's session, is still refused by name.
- [ ] **C14** — The link-audit receiver is built and proved against fixtures before row 03 needs it (ID001_RECEIVER).
- [ ] **C15** — Each identity's lifecycle state is recorded as ADR-011 proposes, every transition one signed event, and the grant path's row is recorded open for Tom with its criteria drafted.

## Row 03: two providers, one person (DIRECTORY-004)

- [ ] **C16** — The pinned fork commit links Google and GitHub to one unchanged Rauthy person, refuses every collision and replay, migrates both storages and audits each link atomically (ID001_LINK_PAIR, ID001_LINK_REFUSAL, ID001_LINK_MIGRATION, ID001_LINK_AUDIT).
- [ ] **C17** — The provider-link contract and the links report are written, and the report names the gated fork commit the pin moves to.

## Row 05: the standalone screen journey (DIRECTORY-005)

- [ ] **C18** — The standalone journey (sign in, the directory, a record, registering and editing an agent, linked accounts, signed history) works with Cambium and Manifold absent, each agent showing its responsible person (ID001_STANDALONE).
- [ ] **C19** — Expired login, unauthorized edit, provider collision, audit pending and backend outage each have a visible, actionable outcome and no optimistic completed state (ID001_SCREEN_REFUSAL).
- [ ] **C20** — The screens follow Aion's appearance with the identity product's own orange accent and no build dependency on Cambium or Aion (ADR-010).

## Human-rooted grant implementation

- [ ] **C21** — The proposed application grant schema separates exercise and pass-on authority; its additional fields require independent review before dispatch and do not change published crypto formats.
- [ ] **C22** — Delegation requires an explicit permission to delegate to the requested subject kind; use-only, absent policy and owner labels cannot create it.
- [ ] **C23** — Every grant is bounded by a live, acyclic ancestry to a responsible person; scope is evaluated by model actions/resources, never display-name rank.
- [ ] **C24** — Grant changes and their audit share one durable authoritative event; retries and uncertain outcomes preserve operation identity and replay-equivalent projections.
- [ ] **C25** — Revocation stops derived authority while independent authority remains usable; the proposed revision-freshness mechanism is reviewed before dispatch.
- [ ] **C26** — Inherited expiry and provisional end dates are enforced at admission; role changes and reinstatement cannot resurrect expired or revoked authority.
- [ ] **C27** — Typed browser/API/agent seams use one authenticated evaluator; forward and reverse explanations name the same authority path and revision without private-record leakage.
- [ ] **C28** — Observed grant usage names its source and time; not seen is not reported as never used.
- [ ] **C29** — The You and delegation screens show real server authority, separate service accounts from sign-in identities, and preserve personal versus administrator visibility.
- [ ] **C30** — The accepted mock-up conformance is tested through actual requests, refusals, pending outcomes, keyboard paths and durable read-back rather than simulated success.

## Sessions: the session record and its credential (DIRECTORY-026)

- [ ] **C193** — docs/design/identity/DIRECTORY-CONTRACT.md has a section '## Sessions and their credentials' giving the session record's seven members, the credential as 32 random bytes kept only as its SHA-256, the challenge issued for one agent id and one launch record id with its 60-second lifetime, the lys/session-start/v1 message over the agent's directory id, the directory's identifier, the launch record id and the challenge with a 109-byte worked example, the signature checked first, the state-revealing refusals in their order, the two acts that end a session, and the fourteen named refusals.
- [ ] **C194** — docs/design/identity/IDENTITY-EVENTS.md extends lys/identity-event/v1 under a fresh joint review without changing its byte layout: session_started and session_ended are event kinds naming the agent and its responsible person, session_started carrying the launch record id, and neither carries a credential's value; the actor gains method codes 2 and 3, the change kinds gain 7 to 9, and every code is only ever appended.
- [ ] **C195** — docs/design/WIRE-FORMATS.md carries one decision-log row proposing lys/session-start/v1, PROPOSED and not ratified, naming its fields (the tag, the agent id, the launch record id, the nonce, the audience and the freshness) and stating that no production agent signs under it until the log shows it ratified.
- [ ] **C196** — docs/design/identity/SESSION-CREDENTIAL-REVIEW.md records an adversarial review of the session credential and the report-back proof, by a party other than their author, with each of its thirteen attacks defeated, including replay, cross-directory, cross-agent, launch-record swap, cross-protocol use and unauthenticated probing, before any session code starts.
- [ ] **C197** — A challenge is issued once, for one agent id and one launch record id, with the same answer whether or not the directory holds the agent: a proof arriving 60 seconds after its issue is admitted, and one arriving 61 seconds after is refused challenge_expired.
- [ ] **C198** — A report-back from an agent whose certificate is issued, signed by its enrolled key, marks the challenge used in the same act that creates one session record pointing at the agent's directory id, commits one session_started event and returns the credential's value once, the directory keeping only its SHA-256.
- [ ] **C199** — Starting a session never creates an agent: a report-back naming an agent the directory does not hold is refused proof_invalid, and the directory's agent count is unchanged by any number of session starts.
- [ ] **C200** — The report-back's signature is verified before anything else: an unsigned or badly signed report-back, or one naming an agent the directory does not hold or one with no enrolled key, is refused proof_invalid with one and the same body and leaves the challenge store byte-identical, and only a correctly signed proof receives the state-revealing refusals, in their order.
- [ ] **C201** — A correctly signed report-back is refused by name, committing no event and creating no session, for a reused challenge (challenge_reused), an expired one (challenge_expired), one issued to another agent (challenge_foreign_agent), one never issued (challenge_unknown), a proof naming another directory (wrong_directory), a launch record id that differs from the challenge's (launch_record_mismatch), a retired agent (agent_retired) and an agent whose certificate is not issued (certificate_not_issued, naming the act of issuing it); a suspended agent is not refused.
- [ ] **C202** — A presented session credential is answered with the agent directory id and session id from its session record, and a presentation commits no event.
- [ ] **C203** — A second session of the same agent, started while the first is open, presents the same agent directory id with a different credential, and after the agent reports the first session's end its credential is refused session_ended, naming that session, while the second is still accepted.
- [ ] **C204** — The directory's configured administrator stops a session as one signed event naming the administrator as actor, after which its credential is refused session_ended, naming the session; any other caller's stop is refused by DIRECTORY-003's admission.
- [ ] **C205** — The directory service answers the challenge, the report-back, the presentation, the agent's end and the administrator's stop on five routes, answers every proof_invalid case with the same status and body bytes, and no log line, trace or answer other than the report-back's carries a credential's value.
- [ ] **C206** — An agent authenticated by its enrolled key creates and ends its own session record and nothing else: its session credential presented on another agent's session stop and on DIRECTORY-003's agent registration is refused by DIRECTORY-003's admission, and a report-back naming another agent signed by its key is refused proof_invalid.
- [ ] **C207** — An agent's sessions are listed by its directory id, in the order they started, with each session's state and how it ended, on a route only the directory's configured administrator is answered on, by a command written out in full, and every other caller is refused by DIRECTORY-003's admission.
- [ ] **C208** — A report-back from an agent whose certificate has been revoked is refused certificate_revoked, creating no session, and the agent's open sessions stay open.
