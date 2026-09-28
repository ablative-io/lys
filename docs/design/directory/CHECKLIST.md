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
- [ ] **C13** — Only the configured administrator mutates the directory in step 1; unauthenticated, same-email and wrong issuer-subject callers are refused without effect (ID001_ADMIN).
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

## Lifecycle states and transitions (DIRECTORY-009)

- [ ] **C36** — docs/design/identity/LIFECYCLE-CONTRACT.md carries the four states with provisioned as a view, the five transitions each with who may cause it and no policy actor, the record rule, the fold rule, the check conjunction, the next-check rule with a refresh as a check, the retired rule, what the screen shows, the six named refusals with their acts, and what IDENTITY-EVENTS.md must carry before the code rows start.
- [ ] **C37** — A transition is admitted only from the administrator or, for an agent, its responsible person as its registration record names them; every other actor, and an unattested one, is refused lifecycle_actor_not_permitted with nothing appended (d009_r2_ac1 to d009_r2_ac4).
- [ ] **C38** — The state is answered only by folding the identity's transition records in log order over a type of exactly four values; nothing sets a state; a record the table refuses makes the read refuse lifecycle_fold_invalid at its coordinate; reads never write the log (d009_r3_ac1 to d009_r3_ac5).
- [ ] **C39** — An access check is the conjunction, active and grant exists and grant fresh, state first: a registered, suspended, retired or unknown identity is refused by state with the grant answerer called zero times, the grant legs pass through DIRECTORY-006 R4's decision with its names, and an answered check appends nothing (d009_r4_ac1 to d009_r4_ac5).
- [ ] **C40** — At sign-in and at every refresh the directory service refuses a suspended or retired identity by name before any grant question, calls Rauthy to revoke nothing, rolls back no admitted action, and admits an active identity at refresh whatever grants it holds (d009_r5_ac1 to d009_r5_ac5).
- [ ] **C41** — One typed read answers an identity's state, the record that put it there, the actions available in that state, its full history and provisioned as a view, and one typed list is filterable by state and kind; a retired identity reads exactly as a live one and no operation deletes, hides, redacts or truncates a record (d009_r6_ac1 to d009_r6_ac5).
