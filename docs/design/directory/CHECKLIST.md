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

- [ ] **C38** — The state is answered only by folding the identity's transition records in log order over a type of exactly four values; nothing sets a state; a record the table refuses makes the read refuse lifecycle_fold_invalid at its coordinate; reads never write the log (d009_r3_ac1 to d009_r3_ac5).
- [ ] **C39** — An access check is the conjunction, active and grant exists and grant fresh, state first: a registered, suspended, retired or unknown identity is refused by state with the grant answerer called zero times, the grant legs pass through DIRECTORY-006 R4's decision with its names, and an answered check appends nothing (d009_r4_ac1 to d009_r4_ac5).
- [ ] **C40** — At sign-in and at every refresh the directory service refuses a suspended or retired identity by name before any grant question, calls Rauthy to revoke nothing, rolls back no admitted action, and admits an active identity at refresh whatever grants it holds (d009_r5_ac1 to d009_r5_ac5).
- [ ] **C41** — One typed read answers an identity's state, the record that put it there, the actions available in that state, its full history and provisioned as a view, and one typed list is filterable by state and kind; a retired identity reads exactly as a live one and no operation deletes, hides, redacts or truncates a record (d009_r6_ac1 to d009_r6_ac5).

## Certificate revocation folded from the log

- [ ] **C65** — docs/design/identity/CERTIFICATE-REVOCATION.md states the certificate-log leaves, the one-claim certificate as the revocable unit, the issuing-authority signer, permanence, revocation_before_issuance, the fold, N and the tolerance, history, the no-log forms and the refusal table.
- [ ] **C66** — The issuance leaf, the revocation leaf and the attestation entry encode and decode under lys-identity/certificate-log/v1, and a revocation verifies only under the issuing authority's key for the log's origin.
- [ ] **C67** — The live set is folded from the certificate log with its folded size; revocations not signed by the issuing authority, revocations of a certificate with no earlier issuance and reinstatements are refused by name at their index, and a leaf the fold cannot read blocks every permit.
- [ ] **C68** — A revocation is one leaf appended at the log's extent through lys-log-store, and the LeafStore trait gains no delete, rewrite, truncate, fork or merge.
- [ ] **C69** — Revocation-aware verification takes N and a tolerance with no default, carries the folded size in every answer, and refuses a revoked certificate naming its revocation leaf.
- [ ] **C70** — A revoked certificate's inclusion and consistency proofs and issuance record still verify, an attestation by its key verifies only when its own entry precedes the revocation leaf, and an attestation by a certificate not revoked needs no entry.
- [ ] **C71** — lys ca verify keeps its meaning and its help says verification without a log does not check revocation, with lys-core unchanged.

## The enduring agent and its session credentials

- [ ] **C49** — docs/design/identity/DIRECTORY-CONTRACT.md states road adjustment 3 as the directory's contract, citing docs/design/identity/STATEMENT-2026-09-22.md:162-167: the agent record holds no session credential, a session is a separate record that points at its agent, starting a session never creates an agent, and a second session of the same agent presents the same enduring identity with a new session credential.
- [ ] **C50** — An agent registered with no session appears in the directory under its responsible person, and its read carries no session credential.

## Sign-in identities belong to people (conformance row 1.2)

- [ ] **C313** — Binding to an agent a provider account linked to a person is refused by name and writes nothing, while an agent's own machine account, linked to no person, is accepted as its issuer-subject binding.
- [ ] **C314** — A person's link of a provider account the directory already binds to an agent is refused by name, stating that the account is an agent's own account; the agent's binding stands, and the agent is named in the directory's history to its responsible person and a directory administrator, never to the person trying to link.
- [ ] **C315** — Delegating from a sign-in identity is refused by name for an agent recipient and for a person recipient, while a service-access grant consented through the same provider account is admitted by the grant rules alone.
- [ ] **C316** — The explanation seam lists each of a person's sign-in identities on the cannot-give list with the reason 'sign-in identity', whoever the recipient is.
- [ ] **C317** — A sign-in identity refusal shows the provider and subject to the identity's owner and a directory administrator only; anyone else sees the act, the recipient and that a sign-in identity is involved.
- [ ] **C318** — Each refusal of an act that would give an agent a sign-in identity leaves the log and projection unchanged, and one counted test over the store after the two directory refusals finds no agent record carrying a sign-in identity.

## Lifecycle conformance rows 3.1 and 3.2 (DIRECTORY-019)

- [ ] **C123** — docs/design/identity/LIFECYCLE-CONTRACT.md maps CONFORMANCE rows 3.1 and 3.2 to named tests and states the needs-a-new-person rule, that ADR-011 is not revised, and that the state is authority only.
- [ ] **C124** — Registering an agent whose register request names no responsible person is refused lifecycle_agent_without_person and appends nothing.
- [ ] **C125** — An agent's lifecycle read carries needs_a_new_person, worked out on read, true on every one of a person's agents once that person is retired and false while the person is suspended, with no record appended and each agent's state unchanged.
- [ ] **C126** — A walk of all 20 origin-by-target changes of state admits exactly the 6 in the table and refuses the other 14 by name with nothing appended.
- [ ] **C127** — A change of state starts and stops no process, and no key of the typed lifecycle read says anything about running.

## The Rauthy readiness gate leg (DIRECTORY-009)

- [ ] **C36** — A test run by a round leg searches the leg's complete output for every secret value the runtime received, asserts how many it searched for, and finds none.
- [ ] **C37** — .land/gates.sh is unchanged and no round-cadence leg is added.

## What a person cannot give (DIRECTORY-024)

- [ ] **C181** — The delegation form's answer lists, for the chosen recipient, every grant in force the person holds on any resource that they cannot give, every relation on the source grant's resource that no grant they hold covers in any standing, their sign-in identity when the recipient is an agent, and each service account they hold to which a reason applies, marks the source grant when it is listed, and lists nothing they can give and nothing not in force.
- [ ] **C182** — Each item carries exactly one reason, the first applicable in the order sign_in_identity, above_what_you_hold, lent_to_you, use_only, people_only, agents_only; lent_to_you and use_only are told apart by whether the person's use-only grant names a source, and coverage is decided by the model's action sets, never by a rank of names.
- [ ] **C183** — The cannot-give answer comes from the one authenticated grant seam, is byte-identical for API, tool and browser callers, refuses an unknown reason on the typed contract, and discloses nothing the person cannot discover.
- [ ] **C184** — The delegation screen shows exactly the items and reasons the answer lists, asks again when the recipient changes and discards a superseded answer, and refuses an answer carrying an unknown reason by name, showing no item and no blank.
- [ ] **C185** — Conformance row 2.4 is carried by acceptance lines that name it, and its Brief column names DIRECTORY-024.

## Road step 2: SpiceDB answers every permission check (DIRECTORY-025)

- [ ] **C186** — The identity server reaches SpiceDB through one pure-Rust gRPC client that maps a fresh store's NOT_FOUND schema answer to a typed no-schema result, tests run against a disposable SpiceDB, and crates/lys/src/identity/ gains no SpiceDB check or write.
- [ ] **C187** — SpiceDB's schema is written only by the identity server at start-up, and its relationships only by projecting committed signed grant events; revoking a root refuses every grant derived from it and no unrelated grant; a delegation or a committed delegation event whose expiry passes its source grant's is refused by name, never clamped; every grant is its own grant object of 3 relationships, and two grants of one holder, action and resource stand apart; a root revoke deletes the revoked grant's standing relationship first and its remaining and derived relationships in writes within SpiceDB's configured update cap, resuming on replay after a crash.
- [ ] **C188** — Every grant and permission check the identity server makes, from browser, API and agent routes, is answered by SpiceDB through one evaluator behind DIRECTORY-006 R4's decision, at least as fresh as the projection, and an engine outage refuses by name; the administrator's admission by configured issuer and subject is the one check that never asks SpiceDB, and the install guide says so beside SpiceDB's unchanged step-1 sentence. An expired grant is refused by the caveat's time from the named clock, and an answer missing that context is refused, never admitted.
- [ ] **C189** — A call after a committed revoke is never admitted: before the projection catches up only a check that depends on the unapplied grant event is refused as not yet current, naming the grant, and unrelated authority stays usable; after it catches up the refusal names the withdrawn grant.
- [ ] **C190** — An agent whose permission is withdrawn between the two steps of a task is refused on its next call to the identity server.
- [ ] **C191** — The server answers why an identity can and why it cannot do a thing from the one evaluator's traced verdict, with the path to a responsible person, the named reason and the policy revision, and answers who can act on a resource through SpiceDB at the same revision, so the forward and reverse answers agree.
- [ ] **C192** — The screen shows the server's why answer for a permitted and a refused question and never decides a permission in the browser.

## Starting an agent (CONFORMANCE section 5)

- [ ] **C229** — No start path in the library, the route or the CLI spawns a process, and a test that reads the start files and gives a start with a marker-writing executable proves it (CONFORMANCE 5.1).
- [ ] **C230** — Before a command is given the five named checks run (the agent is active, its profile version is reviewed, the machine is allowed for the role, its virtual credentials are valid, the machine may reach what the profile needs), a failed check names itself in words, and no command is given (CONFORMANCE 5.2).
- [ ] **C231** — While a check's owning record does not exist, a start is refused by name, naming the check and the card that makes the record (Ink1H1Os, SECRETS-002, network row 8.5), and nothing is faked to let it through.
- [ ] **C232** — No credential value is on the command line or the clipboard, and a test reads both (CONFORMANCE 5.3).
- [ ] **C233** — A launch record naming the machine, the executable, the working directory, the profile version and the credential ids is kept for every command given, reads back after a restart, and a start given again from it is a new record naming the one it was copied from (CONFORMANCE 5.4).
- [ ] **C234** — A start request names only the agent, the profile version and the machine; the command given reads env LYS_AGENT_ID=<agent id> LYS_LAUNCH_RECORD=<launch record id> LYS_CREDENTIAL_IDS=<comma-separated credential ids> before the reviewed profile version's own executable and its recorded arguments, unchanged and in order, with the recorded working directory, each value shell-quoted and refused by name outside its id grammar; it is not a lys subcommand, and a request that sets the executable or the working directory is refused by name.
- [ ] **C235** — An agent shows as running only when its verified signed report names its launch record, and copying the command changes no state (CONFORMANCE 5.5).
- [ ] **C236** — With no report a start reads unconfirmed, never not started; the screen says the request stands and warns against asking elsewhere, and a second start for the agent is refused as start_unconfirmed (CONFORMANCE 5.6).
- [ ] **C237** — A withdrawal by the giver or anyone holding the same right records who and when and that the request no longer stands, never that the agent did not start, and a report arriving afterwards shows running with the withdrawal beside it.
- [ ] **C238** — Only the agent's responsible person and a directory administrator (in step 1, the admitted administrator) have a working Start; anyone else is refused by name, naming the agent and the right they lack.
- [ ] **C239** — The Start drawer and the unconfirmed notice are built on DIRECTORY-005's surface, and DIRECTORY-005's R1 and boundary name Start as the one control that works in step 1.
- [ ] **C240** — The start route gives, gives again, withdraws and reads a start through the library and holds no start logic of its own, and the CLI answer lys identity start-command, the one subcommand this card adds to DIRECTORY-002's identity group, prints exactly what the route answers for the same three inputs and starts nothing.
- [ ] **C241** — A start resolves its agent to the enduring agent record the provision brief DIRECTORY-011 keeps and is refused by name, writing nothing, when there is none; a start never creates or changes an agent record and writes no session record, and a start of an agent already running is given as a new launch record naming the same agent, whose session record the report makes in the sessions brief's store.

## Row 07: gate, install and demonstrate the release (DIRECTORY-030)

- [ ] **C242** — The identity-release leg is registered once, as a demand-cadence leg in docs/design/project.json running scripts/identity-gates/release.sh, named in one line of CLAUDE.md and mirrored in the directory design's gate, and .land/gates.sh is unchanged.
- [ ] **C243** — Every required venue leg is recorded green on the exact pushed Lys, Rauthy and Cambium refs in IDENTITY-001-commands.jsonl and the release report, and a missing leg is named as a blocker.
- [ ] **C244** — The row 03 and row 05 live install receipts are verified against the release refs and each is marked test-keyed.
- [ ] **C245** — The staged install runs the tested refs on the node the operator names under a test service key, with its backups and a rollback that never launches an older Rauthy binary against a forward-only migrated database recorded.
- [ ] **C246** — Standalone acceptance on the staged install is recorded before any cutover, with the staging statement at the demonstration's opening, who the two providers resolve to and the agent's recorded creation.
- [ ] **C247** — Every requirement not met is named in the release report, preserved Cambium identities among them until the cutover, the three reserved acts are named, and no health check is counted as completion.

## Road step 2: the agent capability claim

- [ ] **C249** — docs/design/identity/CAPABILITY-CLAIM.md proposes lys/agent-capability/v1 under the proposed OID 1.3.6.1.4.1.66364.2.1, alongside the unchanged .1 extension transport, stating its transport, assertion, encoding, issuer key identifier (the 20-byte RFC 7093 method 1 keyid, a new issuer-key fingerprint), scope, verifier check, rendering consumer, revocation, issuer and anchor, and status; docs/design/WIRE-FORMATS.md carries it as PROPOSED and both of its ratification sentences name the owning lead with a second reader, with D1 to D6 unchanged; and the .2 row of docs/PEN-REGISTRATION.md keeps .2 a family arc and records .2.1 for the typed capability claim.
- [ ] **C250** — docs/design/identity/CAPABILITY-CLAIM-REVIEW.md records an adversarial review of the draft by a party other than its author, holding at least the forgery, cross-format confusion, malleability, transposition, issuer substitution, downgrade, replay and expiry attacks, each built as bytes and defeated by a named refusal of the draft, and a timing-oracle entry that, for each comparison the verifier makes, builds a timing attack and names what defeats it or states why both operands are public.
- [ ] **C251** — Once the owning lead and a second reader have ratified lys/agent-capability/v1 on the card, the format's row in docs/design/WIRE-FORMATS.md's decision log reads RATIFIED with the date, both names and the reasons the card records, and the .2 row of docs/PEN-REGISTRATION.md reads In use (.2.1), with no other line of either file changed.
- [ ] **C252** — lys-identity encodes the claim of holder id and every grant held at issuance, each with its grant id and its window as it stood then, the list empty when the agent held none and a grant's end absent when it has none, to the vectors pinned in the draft, refuses every non-canonical, unordered, duplicated, unknown-member, missing-member and trailing-byte payload with claim_malformed, and refuses another content type with claim_version_unknown.
- [ ] **C253** — The operator enrols one Ed25519 public key per agent, keyed by the agent's directory id and held beside the agent record, once, as one signed directory event; a second enrolment is refused and no key is taken from an issuance call.
- [ ] **C254** — When the operator calls the directory's issuance route with an active agent's certificate-signing request, the directory issues that enduring agent a certificate, subject its directory id, carrying exactly one lys/agent-capability/v1 claim listing every grant it holds at issuance, an empty list when it holds none, under an issuer key held in the service's custody and named in the certificate's Authority Key Identifier by its 20-byte issuer-key fingerprint so that `openssl verify -CAfile` finds the issuer, for the window asked for or the configured default of 30 days and never past the configured maximum of 90 days, appended to the certificate log as the revocation card's issuance leaf, as one directory event naming the certificate it follows; it refuses an agent that is not active, an agent holding a current certificate, one not expired, not under a replaced key and not held revoked by the revocation fold (naming it and its expiry), an agent with no enrolled key (agent_key_not_enrolled), a request whose key does not match the enrolled key (key_mismatch) and an issuance whose leaf is not appended; nothing issues when a grant is given.
- [ ] **C255** — Replacing an agent's enrolled key is one audited directory event naming the old and new key, never an overwrite; an old key that is not the enrolled one is refused with key_replacement_mismatch, and a certificate under the replaced key is no longer current.
- [ ] **C256** — verify_agent_capability finds the signing key in a set of trusted issuer keys before it reads the claim and refuses an Authority Key Identifier that is absent or names another key (issuer_key_mismatch), a malformed claim, an unknown version, a holder other than the subject, a grant the claim does not list, and an instant outside the certificate's own window, each by a named refusal, with the refusal legs counted; a listed grant's window is never checked as live.

## Sessions: the session record and its credential (DIRECTORY-026)

- [ ] **C193** — docs/design/identity/DIRECTORY-CONTRACT.md has a section '## Sessions and their credentials' giving the session record's seven members, the credential as 32 random bytes kept only as its SHA-256, the challenge issued for one agent id and one launch record id with its 60-second lifetime, the lys/session-start/v1 message over the agent's directory id, the directory's identifier, the launch record id and the challenge with a 109-byte worked example, the signature checked first, the state-revealing refusals in their order, the two acts that end a session, and the fourteen named refusals.
- [ ] **C194** — docs/design/identity/IDENTITY-EVENTS.md extends lys/identity-event/v1 under a fresh joint review without changing its byte layout: session_started and session_ended are event kinds naming the agent and its responsible person, session_started carrying the launch record id, agent_reported is the agent's accepted report itself, and a session ended by the agent is an agent_reported followed by a session_ended naming it; no event carries a credential's value; the actor gains method codes 2, 3 and 4 with their issuer and subject stated, and no event is written under method 4 in this card, the change kinds gain 7 to 9, and every code is only ever appended.
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

## Roles and their versions

- [ ] **C169** — Roles, versions and holdings are typed records whose every act is one signed directory event naming the authenticated actor and its capacity, and a committed version never changes.
- [ ] **C170** — Project ownership is the explicit owner relation on the project in the directory's authorization model, given only by the directory's configured administrator or an existing owner of that project, and never inferred from a name, label or rank.
- [ ] **C171** — Every role-version check is answered through one server-side seam, SpiceDB's evaluator once it is on the main branch, and no request body supplies an actor, a capacity or a permission.
- [ ] **C172** — Assigning a role, by the person the agent answers to or an owner of the role's project, makes a holding that names its role and version with grants copied from that version's templates, each admitted under DIRECTORY-006 for the assigning actor (conformance 4.7, ADR-089).
- [ ] **C173** — Editing a role's grant templates makes the next version and leaves every holder on the version it held with its grants unchanged; editing its title is recorded and makes no version; a newly made role's default is move at the next renewal (conformance 4.2).
- [ ] **C174** — Who holds a role, and on which version, is answered as a query over holdings.
- [ ] **C175** — A move by hand shows what it adds and removes before it is taken, is admitted only for the person the holder answers to or an owner of the role's project, and is one signed record naming the actor, its capacity and the timing chosen (conformance 4.3).
- [ ] **C176** — A move at the holder's next start leaves a running session on the version it started with, and a move now, taken only by the admitted operator, stops every open session of the agent first and names in the move's record every session its listing showed, each with its outcome.
- [ ] **C177** — A renewal is a new grant of the holding with a new end date by the person the holder answers to or an owner of the role's project, at the role's current version under a move at the next renewal and at the held version otherwise, recorded naming the actor, the holding and the version; an unrenewed holding lapses and does not move (conformance 4.4).
- [ ] **C178** — Each holding shows its policy; one with an end date under a move at the next renewal shows its end date, that its next renewal moves it to the current version, and who can stop the move; one with no end date shows it moves only by a deliberate act; a holding's policy and a role's default change only by recorded acts that move no holder (conformance 4.4, ADR-077).
- [ ] **C179** — A provisional holding's end date is on its grants and is the same after a role edit and after a move (the roles half of conformance 4.5).
- [ ] **C180** — The role screen shows versions, holders on each version, move dates, policies and the move preview from the server's answers, shows Assign only to the person the holder answers to and the owners of the role's project, and offers an act only where the server says it is permitted.

## Row 6.4: issuance entered in the log (DIRECTORY-031)

- [ ] **C209** — `lys ca issue` refuses to run without `--log` and `--leaf-out`, and on both issuance paths enters the certificate in the lys-log-store log as one leaf whose bytes are the certificate's DER and nothing else before any file is written.
- [ ] **C210** — With `--log` and `--leaf-out` and no log key, `lys ca issue` writes the certificate and the leaf and reports the log, the leaf index, the tree size and the root in base64, signing nothing and writing no artifact; the log's operator makes the lys/log-inclusion-proof/v1 artifact with `lys log prove inclusion`.
- [ ] **C211** — When the log refuses the entry, `lys ca issue` exits 1 with the log's refusal by name, writes no certificate and no leaf, and the log's leaves are unchanged.
- [ ] **C212** — `lys ca issuer-cert --key <path> --out <path>` writes the issuer's self-signed CA certificate as one PEM block carrying no key material, and it and `lys ca issue --issuer-out` write the same bytes on every call, from one issuer certificate stored beside the CA key when it is first built.
- [ ] **C213** — Holding only the issuer certificate, the issued certificate, the leaf and the operator's artifact, with no lys binary on PATH, `openssl verify -CAfile` accepts the certificate, `scripts/verify_inclusion.py` exits 0 under the reported root, and a leaf changed by one byte makes it exit 2.
- [ ] **C214** — docs/design/directory/PROOF-ISSUANCE.md records the stranger's check run with test keys and a test log: each command, its exit code and its output, and otherwise hashes, counts and paths only.
- [ ] **C215** — The directory design's non-goal for road step 2 no longer excludes capability certificates, cites CONFORMANCE rows 6.1 to 6.4 as the reason, and still excludes anchoring in production.

## Row 8.3: the access graph (DIRECTORY-034)

- [ ] **C272** — The graph module under surface/identity/src/features/graph/ imports nothing from the permission engine except the generated Access client (its other imports are only React, the surface's router package, the shell modules the test lists by file name, the surface's stylesheets and its own files, followed transitively, the surface's API client refused by name wherever it is reached), declares none of the mock-up's rule functions and references no mutation, and a test that counts what it checked proves it (conformance 8.3).
- [ ] **C273** — Every grant edge and every reachability the graph draws equals an Access answer from the real evaluator for the same question, over every identity, resource and declared action of a fixture directory: paths as returned, a no as Access's reason, a does not stand edge only for a grant a forward refusal names, with Access's reason, a grant Access no longer returns gone on the next draw, standing never inferred, and an outage shown as a named refusal.
- [ ] **C274** — A read-only route in the identity server serves the permission model's resources with their parents, declared actions and model version, recording nothing; a caller sees a resource only where they hold a grant on it or on an ancestor, a directory administrator sees every resource, pass-on authority widens nothing, and a hidden parent is served as withheld without its name.
- [ ] **C275** — Before any question the graph shows the people, agents and resources the person may see, with containment edges from the served parents, a parent withheld mark where the parent is withheld, answers-to edges from the records and no grant edge, under three toggles: Grants, Containment and Who answers for whom, none of which asks Access again.
- [ ] **C276** — Choosing a person or agent draws the forward answer for each resource and action its visible grants carry, choosing a resource draws the reverse answer, an administrator holding the reverse question's visibility permission sees the whole directory, a reverse question the person may not ask is refused by name, no hidden grant is drawn or hinted at, and an incomplete reverse answer is marked incomplete.
- [ ] **C277** — Every grant and does not stand edge shows the model version of the Access answer it came from and every containment edge that of the resource route's answer, a draw of mixed versions names each above the graph, the draw time shows beside the version, and the graph asks Access again only on navigation, reload or its refresh control.
- [ ] **C278** — The graph is reached by the deep links #/graph and #/graph/<id> and by a Show in graph link on each screen present at build time that shows one identity (the directory's preview drawer, the identity record screen and the You page) and on the grant explanation, whose link opens the graph on the holder's node with that grant's edge selected, none on sign-in or the delegation form, with no rail entry or g h shortcut of its own.

## Grant conformance rows named by their tests

- [ ] **C319** — Each of conformance rows 1.4, 1.5 (its agents and grants clauses), 2.1, 2.2, 2.3, 2.5, 2.6 and 8.4 names the test that passes it with a stated command that runs that test alone, and every crates/lys-identity test of the relation bound, affirmative pass-on, inherited expiry, cascading revocation and last used carries its row in its name.
- [ ] **C320** — The grant view the service answers carries each grant's effective standing and effective end, judged over its chain on the server, and the screens render those two fields and walk no chain of their own.
- [ ] **C321** — The screens meet the rows' words from the service's answer and from nothing else: What you hold shows each grant's source and whether it may be passed on and drops the grants derived from a revoked one, as does each agent's holdings on You; an agent's Access tab keeps them marked void; the delegation form shows the source grant, its actions, may-pass-on and the effective end it ends no later than; a grant card shows last used, its source and its window, and not seen is never shown as never used.
- [ ] **C322** — An administrator's People screen shows other people while You shows only the signed-in person's own agents and grants; row 1.5's personal-secrets clause stays unmet and You keeps its not-built panel naming SECRETS-002.
- [ ] **C323** — Each of the seven drift injections (a grant shown without its source, may-pass-on read from a missing prohibition, a derived grant surviving its source's revocation, a derived grant surviving its source's end past admission's own check, the same past the hop check at commit and on replay, the same past the hop check at exercise, not seen rendered as never used) makes exactly one test fail, and it is the named test of that row.
- [ ] **C324** — A surface leg registered in docs/design/project.json installs surface/identity's dependencies from its lockfile and runs its vitests, so every land runs the screen tests.
- [ ] **C325** — CONFORMANCE.md's Brief cell on each of the eight rows names this brief's tests in the '<brief id> <leg> <test name>' entry form after the kept planning text, and DIRECTORY-006 R6 names the screen files that exist in place of the four it names that do not, with no structure path of the directory design named twice.

## The identity surface shell (conformance 9.1 to 9.3)

- [ ] **C303** — .land/gates.sh runs the surface's test command as a leg that fails by name when npm is missing, the project setup's trees measure surface/identity by npm ci and npm test, and the design's gate array is its parent's with the one surface leg appended and nothing else changed.
- [ ] **C304** — One route table names the 14 rail screens and the 30 hash-routed tabs with a built column, and the tests fail by name on a built row the shell does not serve.
- [ ] **C305** — The rail toggles by its button and by [, kept under iam.labels across a reload, and the dock side switches by the Configuration Layout segment, by the palette act and by \, kept under iam.dock, with the layout flipped.
- [ ] **C306** — The palette Go to entries and the fourteen g go-to letters reach every rail screen.
- [ ] **C307** — Every element with a click action is a button or a link, or has tabindex 0 and dispatches one click on Enter and Space, palette rows included, with no caller under surface/identity/src/features changed.
- [ ] **C308** — j and k move focus with the cursor without replacing the screen, and Enter opens the focused row.
- [ ] **C309** — The help overlay places one numbered mark per explained element and states the count, swallows its dismissing click, and returns focus on Escape to the element that had it.
- [ ] **C310** — A Tab walk over every built route, driven by @testing-library/user-event, reaches every element with a click action other than those in a closed layer (the closed palette's rows, walked instead with the palette open) and the two dismiss backdrops, and activates each with Enter and, for non-links, Space, with a non-zero count asserted.
- [ ] **C311** — index.v6.html sits beside index.v5.html and differs from it by the focus fix and the keyboard fix only, each proved against v6 and shown failing on v5.
- [ ] **C312** — CONFORMANCE.md line 3 names index.v6.html as the reference, with v5 as the prior, and pins index.v6.html by its path and sha256, the first pin of the mock-up.

## Rendered brief list fields

- [ ] **C31** — scripts/design/render-brief.py is byte-identical to the design-system method's scripts/render-brief.py at the method commit that lands the prose-list rule, and scripts/design/SOURCE.md names that commit for render-brief.py.
- [ ] **C32** — A brief's blocked_by, and any other list field whose entries are not all bare ids, renders as its label followed by one Markdown list item per entry, in JSON order, each item's text byte-equal to its entry.
- [ ] **C33** — A list field whose entries are all bare ids (ADR-, RM-, C, S and brief ids) renders on one ', '-joined line exactly as before, and render-brief.py states the rule that tells the two kinds of list apart in the docstring of is_bare_id, which render_list_field applies to every list field.
- [ ] **C34** — Every brief markdown that render-brief.py produces from a brief JSON in a cluster with a design.json is re-rendered in the same change, sh scripts/design/gate.sh exits 0, and docs/design/identity/briefs/IDENTITY-001.md and CONTEXT-001.md are unchanged.
- [ ] **C35** — scripts/design/tests/test_render_brief.py proves a two-entry prose blocked_by renders two list items byte-equal to their entries and a bare-id list renders on one line, and scripts/design/gate.sh runs it so the design leg fails when it fails.

## Grant decisions do the work once

- [ ] **C326** — One decision per check (DIRECTORY-042 R1), proved by a counting test that fails at the base.
- [ ] **C327** — A decision reads only the relationships of the grants on its path (DIRECTORY-042 R2), proved by a counting test that fails at the base.
- [ ] **C328** — Holders and changes are indexed, not rebuilt (DIRECTORY-042 R3), proved by a counting test that fails at the base.
- [ ] **C329** — Use events do not round-trip the mirror one by one (DIRECTORY-042 R4), proved by a counting test that fails at the base.
- [ ] **C330** — Revocation derives its deletions from the book (DIRECTORY-042 R5), proved by a counting test that fails at the base.
- [ ] **C331** — Who-holds answers from one snapshot (DIRECTORY-042 R6), proved by a counting test that fails at the base.
- [ ] **C332** — The SpiceDB schema is read when it can have changed, not before every call (DIRECTORY-042 R7), proved by a counting test that fails at the base.
- [ ] **C333** — SpiceDB calls use a pooled async client, outside the directory lock, with no timeout (DIRECTORY-042 R8), proved by a counting test that fails at the base.

## The identity server's hot paths do each piece of work once

- [ ] **C334** — Runtime state is indexed (DIRECTORY-043 R1), proved by a counting test that fails at the base.
- [ ] **C335** — Snapshots serialise by reference (DIRECTORY-043 R2), proved by a counting test that fails at the base.
- [ ] **C336** — Directory writes never block an async worker (DIRECTORY-043 R3), proved by a counting test that fails at the base.
- [ ] **C337** — A session listing locks once and clones nothing it does not return (DIRECTORY-043 R4), proved by a counting test that fails at the base.
- [ ] **C338** — A memory view reads each session once, off the async worker (DIRECTORY-043 R5), proved by a counting test that fails at the base.
- [ ] **C339** — A receipt is rebuilt from the stored coordinate, not by re-reading leaves (DIRECTORY-043 R6), proved by a counting test that fails at the base.

## Waits on signals, never on a clock

- [ ] **C340** — Waiting for a service to answer waits on its readiness event, not a one-second sleep (DIRECTORY-044 R1).
- [ ] **C341** — Stopping a service waits on the platform's exit notification (DIRECTORY-044 R2).
- [ ] **C342** — Loopback, Rauthy and health exchanges carry no timeout (DIRECTORY-044 R3).
- [ ] **C343** — The Explain view re-measures on a layout signal (DIRECTORY-044 R4).

## A running install names its build and upgrades in place

- [ ] **C344** — Every Lys binary answers --version with its build commit (DIRECTORY-045 R1).
- [ ] **C345** — `lys identity upgrade` swaps the binaries and screens and returns to the previous build on failure (DIRECTORY-045 R2); the configuration and compose files move with the binaries, an install made before it is adopted, and an upgrade stopped part-way is finished or put back (DIRECTORY-045 R5 to R7).
- [ ] **C346** — The install and the server say which build is running (DIRECTORY-045 R3).
- [ ] **C347** — Prove the upgrade on a scratch install in the round; the live install is upgraded and recorded after landing (DIRECTORY-045 R4).

## Hot paths do their work once (DIRECTORY-046)

- [ ] **C348** — GET /reviews filters by viewer first and finds each latest decision by index (DIRECTORY-046 R1).
- [ ] **C349** — A request is found by id, not by scanning every request (DIRECTORY-046 R2).
- [ ] **C350** — Certificate issue uses the service key the server already holds (DIRECTORY-046 R3).
- [ ] **C351** — Served screens never block an async worker and index.html is read once (DIRECTORY-046 R4).
- [ ] **C352** — A record is serialised without cloning it (DIRECTORY-046 R5).
- [ ] **C353** — The grants screens ask for reach in one concurrent batch (DIRECTORY-046 R6).
- [ ] **C354** — The screens choose the right route first and fetch independent reads together (DIRECTORY-046 R7).

## Lys is the only sign-in anyone sees (DIRECTORY-047)

- [ ] **C355** — First run asks for the administrator on Lys's setup page without machine defaults; Lys owns the password policy, writes it during preparation, and shows that same value (DIRECTORY-047 R1).
- [ ] **C356** — Password sign-in stays on Lys; valid browser callbacks set a session and redirect 303 to /, proven with the current OIDC flow (DIRECTORY-047 R2).
- [ ] **C357** — Every product, Cambium first, is a client of Lys at Lys's origin; no product configuration names the issuer (DIRECTORY-047 R3).
- [ ] **C358** — Provider setup shows the server-supplied redirect and checks the provider; the build uses a fake-provider round trip, and the landing lead verifies real Google after landing (DIRECTORY-047 R4).
- [ ] **C359** — Account changes use Lys screens; reachable issuer admin paths explicitly refuse host browsers while required loopback calls work, and upgrade handles rights and recorded network choices before this card lands after 045 (DIRECTORY-047 R5).
- [ ] **C360** — Nothing a person reads (screens, install output, refusals, page titles) names the issuer (DIRECTORY-047 R6).

## Every app registers with Lys through a published API (DIRECTORY-048)

- [ ] **C361** — Only a person holding an explicit register_app grant registers an app; approval creates its distinct application-connector identity and binding, and every app permission follows an explicit grant chain back to the super administrator (ADR-126, DIRECTORY-048 R1).
- [ ] **C362** — An app's schema declares kinds under its own prefix, their actions, relations carrying actions and parent kinds; an invalid schema is refused naming the line of the fault (DIRECTORY-048 R2).
- [ ] **C363** — An app cannot define, change or grant on another app's kinds; the refusal names the owning prefix (DIRECTORY-048 R2).
- [ ] **C364** — A schema change that would strand standing grants is refused naming the relation and the count; a dry run answers the same without writing (DIRECTORY-048 R3).
- [ ] **C365** — Lys's own model is the schema of the app 'lys'; existing grants and checks answer the same after the move (DIRECTORY-048 R4).
- [ ] **C366** — Apps check many permissions and list permitted resources as their own application connector, under explicit grants and their own-kind boundary; neither binding nor prefix ownership grants authority (ADR-126, DIRECTORY-048 R5).
- [ ] **C367** — One OpenAPI document, generated from the routes and types, describes every route; a route without an entry fails the build (DIRECTORY-048 R6).
- [ ] **C368** — Lys holds no app's name or schema in code or configuration and makes no call to any app (DIRECTORY-048 R7).
- [ ] **C386** — A person builds an app's whole permission template on the Apps screen from templates, tests it on example people and resources against the real check, and saves it; built and uploaded schemas are the same record (DIRECTORY-048 R8).

## Lys MCP server, secure and compact (DIRECTORY-049)

- [ ] **C369** — The MCP server offers exactly three tools over the published API; the tool list is under 2 KB (DIRECTORY-049 R1, R5).
- [ ] **C370** — Every MCP call is a person's Lys token obtained through the MCP authorization flow, or an agent's signed request; there is no key, shared secret or unauthenticated mode (DIRECTORY-049 R2).
- [ ] **C371** — Every MCP call runs the same handler and grant check as the HTTP route, as the caller; no MCP answer returns a secret's value (DIRECTORY-049 R3, R4).
- [ ] **C372** — A destructive write through MCP needs the target's id repeated in confirm; every write leaves a receipt naming the caller and 'mcp' (DIRECTORY-049 R4, R6).
- [ ] **C373** — An agent's launch renders 'lys mcp' into its MCP configuration, signing with its own certificate key held by handle (DIRECTORY-049 R7).

## Lys runs and drives the agents it starts (DIRECTORY-050)

- [ ] **C374** — Lys ships a runner that holds each started agent in its own pseudo-terminal in the background, surviving the screen closing (DIRECTORY-050 R1).
- [ ] **C375** — A published runner protocol lets another tool be a machine's runner; Lys needs no particular runner (DIRECTORY-050 R2).
- [ ] **C376** — Starting an agent on a machine with a runner runs it there and reports it running, from the screen, the API and the MCP (DIRECTORY-050 R3).
- [ ] **C377** — A session can be typed into, sent keys, read, waited on for a pattern, resized and compacted through Lys, each under a grant and with a receipt (DIRECTORY-050 R4).
- [ ] **C378** — A session that prints its usage-limit words moves to the next account in its list, by handle, never by value (DIRECTORY-050 R5).
- [ ] **C379** — A message to an agent wakes its session; the emergency stop ends sessions through the runner and reports each confirmed (DIRECTORY-050 R6).
- [ ] **C380** — A Sessions screen shows every running agent, its terminal read live, with type, keys and stop (DIRECTORY-050 R7).

## Lys tracks tokens, context, time, budgets and goals (DIRECTORY-051)

- [ ] **C381** — Lys installs its own session hooks/status line and incrementally follows the runner-local stream; usage is durable, authenticated and exported for optional Argus analytics (DIRECTORY-051 R1).
- [ ] **C382** — Budgets for context, tokens and time are set on an agent, a team or a person, and inherited downward (DIRECTORY-051 R2).
- [ ] **C383** — A reached budget compacts, stops or tells, as the budget says, once, with a receipt (DIRECTORY-051 R3).
- [ ] **C384** — Goals on an agent carry a deadline and reminders delivered into its session (DIRECTORY-051 R4).
- [ ] **C385** — Plain controls set and read budgets and goals and show reached or uncertain state; analytics stays in Argus (DIRECTORY-051 R5).
- [ ] **C421** — Lys creates an editable tool-boundary policy and records runner/grant denials on the agent page; Codex pre-tool coverage is explicitly unavailable (DIRECTORY-051 R6).

## Provision a working team in one act (DIRECTORY-052)

- [ ] **C387** — A team plan names its purpose, total budget, deliverables with their evidence, and each member's profile, memories, opening conversation, budget share, goals and checker (DIRECTORY-052 R1).
- [ ] **C388** — Provisioning a plan creates every agent, grant, home, budget and goal in one all-or-nothing act and starts them (DIRECTORY-052 R2).
- [ ] **C389** — Each member starts with its chosen memories and its opening conversation already in its session (DIRECTORY-052 R3).
- [ ] **C390** — A deliverable is met only when its checker accepts it with the named evidence; the team's spend is held to its total (DIRECTORY-052 R4).
- [ ] **C391** — A Teams screen builds a plan from a template, provisions it, and shows each member's state, spend, goals and deliverables (DIRECTORY-052 R5).
- [ ] **C392** — Accounts and secrets are stored once in the broker and assigned to members by handle; values are never shown again (DIRECTORY-052 R6).

## Go back after an upgrade, one build stamp (DIRECTORY-053)

- [ ] **C393** — `lys identity upgrade --back` returns a running install to the build kept in bin.previous (and surface.previous), stopped, swapped, started and waited on for ready exactly as an upgrade is, and records the build now running (DIRECTORY-053 R1).
- [ ] **C394** — Every Lys binary takes its build stamp from one shared build-support crate; no build.rs is copied (DIRECTORY-053 R2).

## Install Lys by opening an app (DIRECTORY-054)

- [ ] **C395** — `lys package app` builds a signed, notarised Lys.app and disk image holding every Lys binary and the screens package, each stamped with its build; with no signing identity it is refused by name (DIRECTORY-054 R1).
- [ ] **C396** — Opening Lys.app runs the install service in the process, shows each step on a Lys page in the browser in plain words, and hands over to first-run setup (DIRECTORY-054 R2).
- [ ] **C397** — A missing or stopped container engine is a Lys page with what to do, and the install continues by itself when the engine appears, on its socket's event (DIRECTORY-054 R3).
- [ ] **C398** — Lys starts at login, opening the app again opens Lys, a newer app upgrades through DIRECTORY-045, and uninstalling is a Lys screen that keeps data unless the person chooses otherwise (DIRECTORY-054 R4).
- [ ] **C399** — On a fresh macOS account, a person goes from the downloaded disk image to signed in with no terminal process started and nothing naming the issuer (DIRECTORY-054 R5).

## The exit lock lives in the service only (DIRECTORY-057)

- [ ] **C400** — The three ways to keep the exit lock out of the starter are compared in ADR-121 and the holder command is chosen, with no unsafe code (DIRECTORY-057 R1).
- [ ] **C401** — A holder command opens the exit lock, takes it and becomes the service by exec, keeping its pid; a program that cannot run is refused by name (DIRECTORY-057 R2).
- [ ] **C402** — The starter never opens the exit lock; a service that ends at once beside other starts is seen ended every time (DIRECTORY-057 R3).

## A receipt's checkpoint is signed (DIRECTORY-058)

- [ ] **C403** — The receipts route answers its checkpoint as a note signed by the service key, signed at open, at each committed append and at each settle that adopts leaves, under the log's own origin (DIRECTORY-058 R1).
- [ ] **C404** — One function verifies a receipt answer against a pinned key with a named refusal for each failure; a forged tree around a genuine event is refused (DIRECTORY-058 R2).

## Installed audit sender

- [ ] **C405** — The service completes sender enrolment as part of browser setup after install returns, resumes the original intent after interruption, and never requires another install invocation.
- [ ] **C406** — Sender configuration and seed are private, stable, and mounted read-only without exposing the directory private key.
- [ ] **C407** — A dedicated TLS-only authority key issues the receiver certificate, distinct from capability, event, receiver and sender keys; the receiver listens only on its declared restricted address.
- [ ] **C408** — Generated sender configuration names separate TLS trust and receipt trust and a declared positive response-body bound.
- [ ] **C409** — Scratch installation proves signed observation acceptance and named transport, identity and response refusals.
- [ ] **C410** — DIRECTORY-045 R5 retains sender credentials and swaps/restores sender configuration, trust material and mounts with the chosen binaries.

## Lys needs no app and every app can find it (DIRECTORY-059)

- [ ] **C411** — A start reads its credentials from Lys's own broker, and no route reaches an app for them (DIRECTORY-059 R1).
- [ ] **C412** — Lys writes a discovery record any app can find (DIRECTORY-059 R2).
- [ ] **C413** — Any registered app reads Lys's people, seats and agents (DIRECTORY-059 R3).
- [ ] **C414** — Signing out of Lys signs the person out at the issuer, and the issuer tells every registered app (DIRECTORY-059 R4).
- [ ] **C415** — An app asks for its registration, an administrator approves it on one screen, and the app receives its secret by a one-time code (DIRECTORY-059 R5).

## The runner holds each session's holder key (DIRECTORY-060)

- [ ] **C418** — A runner session's handles are issued to a key the runner made for it in memory, before anything is spawned (DIRECTORY-060 R1).
- [ ] **C419** — The runner signs a presentation only for a peer proved by credentials and ancestry to be the session the handle was issued to, on macOS and Linux (DIRECTORY-060 R2).
- [ ] **C420** — The harness and lys mcp get every presentation from the runner and hold no key (DIRECTORY-060 R3).

## A SpiceDB call ends on its answer or its caller (DIRECTORY-061)

- [ ] **C422** — No SpiceDB connect, write or read in Lys waits on a clock, and a cancelled call returns at once (DIRECTORY-061 R1).
- [ ] **C423** — Every grants section runs off the async workers, and a request that leaves ends its SpiceDB wait and lets the grants lock go (DIRECTORY-061 R2).
- [ ] **C424** — A section that takes the grants lock after its request left calls no SpiceDB (DIRECTORY-061 R2).

## Native process containment (DIRECTORY-062)

- [ ] **C425** — One Lys policy becomes a bound containment plan (DIRECTORY-062 R1).
- [ ] **C426** — macOS applies Seatbelt before the harness can run (DIRECTORY-062 R2).
- [ ] **C427** — Linux applies Landlock and a network namespace before exec (DIRECTORY-062 R3).
- [ ] **C428** — Kernel evidence feeds the existing refusal stream (DIRECTORY-062 R4).
- [ ] **C429** — The agent page states the sandbox and the evidence (DIRECTORY-062 R5).
- [ ] **C430** — A person watches real native denials and an allowed control (DIRECTORY-062 R6).

## Unknown API paths and the health route (DIRECTORY-063)

- [ ] **C431** — Every unknown path under /api answers 404 with a named JSON refusal, never the page (DIRECTORY-063 R1).
- [ ] **C432** — GET /api/health answers that the service is serving, with its name and build, asking no other service (DIRECTORY-063 R2).

## Managed context and goal delivery (DIRECTORY-064)

- [ ] **C433** — One managed harness channel, with proved turn boundaries (DIRECTORY-064 R1).
- [ ] **C434** — Use Claude and Codex control protocols, never terminal typing (DIRECTORY-064 R2).
- [ ] **C435** — Enforce context thresholds at the owned boundary (DIRECTORY-064 R3).
- [ ] **C436** — Deliver current goal and reminder words at turn boundaries (DIRECTORY-064 R4).
- [ ] **C437** — Reconcile uncertain delivery with existing operation receipts (DIRECTORY-064 R5).
- [ ] **C438** — Plain controls and a real managed-session proof (DIRECTORY-064 R6).

## Placed and running runner builds (DIRECTORY-066)

- [ ] **C445** — Place the runner binary without restarting a live runner (DIRECTORY-066 R1).
- [ ] **C446** — Record running and placed builds as different facts (DIRECTORY-066 R2).
- [ ] **C447** — Restart only through an explicit session-aware operation (DIRECTORY-066 R3).
- [ ] **C448** — Show the same pending restart on the page and CLI (DIRECTORY-066 R4).

## Codex policy and refusal coverage (DIRECTORY-065)

- [ ] **C439** — Pin the actual Codex executable and its supported policy contract (DIRECTORY-065 R1).
- [ ] **C440** — Render native Codex permissions from the bound Lys policy (DIRECTORY-065 R2).
- [ ] **C441** — Bind Codex pre-tool policy checks to the existing Lys judge (DIRECTORY-065 R3).
- [ ] **C442** — Record native Codex rejections with honest provenance (DIRECTORY-065 R4).
- [ ] **C443** — The Codex agent page shows measured policy and refusal coverage (DIRECTORY-065 R5).
- [ ] **C444** — Prove config enforcement and denial delivery through the real harness (DIRECTORY-065 R6).

## Approved apps sign in and inactive people are refused (DIRECTORY-067)

- [ ] **C449** — An approved app's client is a provider client (DIRECTORY-067 R1).
- [ ] **C450** — A retired app's codes and tokens stop at once (DIRECTORY-067 R2).
- [ ] **C451** — A suspended or retired person gets no code, no token and no answer (DIRECTORY-067 R3).
- [ ] **C452** — A code presented twice ends the tokens it bought (DIRECTORY-067 R4).

## Sign-out ends app access and apps may read email (DIRECTORY-068)

- [ ] **C453** — A code or token dies with the Lys session it came from (DIRECTORY-068 R1).
- [ ] **C454** — An app signs the person out through Lys's end-session endpoint (DIRECTORY-068 R2).
- [ ] **C455** — An app that asks for the email scope receives the person's email (DIRECTORY-068 R3).

## The message service is set on the screen (DIRECTORY-069)

- [ ] **C456** — The setting is Lys's own record, set on the Connections screen (DIRECTORY-069 R1).
- [ ] **C457** — An install's configured setting becomes the first line, once (DIRECTORY-069 R2).
- [ ] **C458** — The screen shows whether the service answers (DIRECTORY-069 R3).
