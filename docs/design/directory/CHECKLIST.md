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

## Road step 2: the agent capability claim

- [ ] **C58** — docs/design/identity/CAPABILITY-CLAIM.md proposes lys/agent-capability/v1 under the proposed OID 1.3.6.1.4.1.66364.2.1, alongside the unchanged .1 extension transport, stating its transport, assertion, encoding, issuer key identifier (the 20-byte RFC 7093 method 1 keyid, a new issuer-key fingerprint), scope, verifier check, rendering consumer, revocation, issuer and anchor, and status; docs/design/WIRE-FORMATS.md carries it as PROPOSED and both of its ratification sentences name the owning lead with a second reader, with D1 to D6 unchanged; and the .2 row of docs/PEN-REGISTRATION.md keeps .2 a family arc and records .2.1 for the typed capability claim.
- [ ] **C59** — docs/design/identity/CAPABILITY-CLAIM-REVIEW.md records an adversarial review of the draft by a party other than its author, holding at least the forgery, cross-format confusion, malleability, transposition, issuer substitution, downgrade, replay and expiry attacks, each built as bytes and defeated by a named refusal of the draft, and a timing-oracle entry that, for each comparison the verifier makes, builds a timing attack and names what defeats it or states why both operands are public.
- [ ] **C193** — Once the owning lead and a second reader have ratified lys/agent-capability/v1 on the card, the format's row in docs/design/WIRE-FORMATS.md's decision log reads RATIFIED with the date, both names and the reasons the card records, and the .2 row of docs/PEN-REGISTRATION.md reads In use (.2.1), with no other line of either file changed.
- [ ] **C60** — lys-identity encodes the claim of holder id and every grant held at issuance, each with its grant id and its window as it stood then, to the vector pinned in the draft, refuses every non-canonical, unordered, duplicated, unknown-member, missing-member and trailing-byte payload with claim_malformed, and refuses another content type with claim_version_unknown.
- [ ] **C61** — The operator enrols one Ed25519 public key per agent, keyed by the agent's directory id and held beside the agent record, once, as one signed directory event; a second enrolment is refused and no key is taken from an issuance call.
- [ ] **C62** — When the operator calls the directory's issuance route with an active agent's certificate-signing request, the directory issues that enduring agent a certificate, subject its directory id, carrying exactly one lys/agent-capability/v1 claim listing every grant it holds at issuance, under an issuer key held in the service's custody and named in the certificate's Authority Key Identifier by its 20-byte issuer-key fingerprint so that `openssl verify -CAfile` finds the issuer, for the window asked for or the configured default of 30 days and never past the configured maximum of 90 days, appended to the certificate log as the revocation card's issuance leaf, as one directory event naming the certificate it follows; it refuses an agent that is not active, an agent holding a current certificate (naming it and its expiry), an agent with no enrolled key (agent_key_not_enrolled), a request whose key does not match the enrolled key (key_mismatch) and an issuance whose leaf is not appended; nothing issues when a grant is given.
- [ ] **C63** — Replacing an agent's enrolled key is one audited directory event naming the old and new key, never an overwrite; an old key that is not the enrolled one is refused with key_replacement_mismatch, and a certificate under the replaced key is no longer current.
- [ ] **C64** — verify_agent_capability finds the signing key in a set of trusted issuer keys before it reads the claim and refuses an Authority Key Identifier that is absent or names another key (issuer_key_mismatch), a malformed claim, an unknown version, a holder other than the subject, a grant the claim does not list, and an instant outside the certificate's own window, each by a named refusal, with the refusal legs counted; a listed grant's window is never checked as live.
