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

## Audit receipts (DIRECTORY-007)

- [ ] **C31** — docs/design/identity/AUDIT-RECEIPT.md carries IDENTITY-001's receipt contract word for word with the test tag as its one ruled addition, names exactly one emitter for every operation, states the step-1 sign-in boundary, the changes-only rule and the test-receipt rule, and lists what IDENTITY-EVENTS.md must carry before the code rows start.
- [ ] **C32** — crates/lys-receipt reads a receipt of the shared shape and verifies it from a LeafStore and a key in the order tag, signature, coordinate, commitment, with every cryptographic or structural failure one refusal class and the test-tag and shape refusals named on their own (rcpt_accept, rcpt_tamper, rcpt_leaf, rcpt_testtag, rcpt_shape, rcpt_redact).
- [ ] **C33** — lys log verify receipt verifies offline from the receipt, the log directory and the key strings alone, keeps the published CLI's refusal discipline, writes nothing to the log and leaves inclusion and consistency verification unchanged (rcli_accept, rcli_tamper, rcli_precrypto, rcli_test, rcli_readonly).
- [ ] **C34** — The directory service leaves a sign-in receipt for every sign-in it records and a refusal receipt carrying the refusal's name as its reason for every sign-in it refuses, registers nobody on a first sign-in, decides no refusal on state itself, and leaves no receipt for a read or an answered check (signin_ok, signin_refused, signin_first, signin_refused_reason).
- [ ] **C35** — A development install emits test receipts under a test key and a test tag and the whole path is proved end to end: verified with the test key, refused without it by name, refused under another key, with no private key in any document.
