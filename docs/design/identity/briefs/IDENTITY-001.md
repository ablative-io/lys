# IDENTITY-001 — Standalone identity directory and two-provider sign-in

Rendered from `IDENTITY-001.json`; edit the JSON source, then regenerate this file.

Revision 5. Status: **row_01_closed_revision_5_awaiting_row_02_recheck**. Owner: Chippy. Reviewer: Waffles. Date: 2026-09-22 Australia/Melbourne.

Authority: [../STATEMENT-2026-09-22.md](../STATEMENT-2026-09-22.md).

An operator installs the product without Cambium or Manifold, signs in, explicitly links Google and GitHub to one person, registers an agent before it runs, and inspects signed identity-change history. Cambium then uses this issuer while retaining existing participant IDs, memberships and content attribution.

## Authority

- Tom 14:35-14:42: maintained Rauthy fork; standalone product beyond login; home in Lys.
- Tom 15:01: seven-step road plus four adjustments accepted; Chippy steps 1-3 and Cambium; Archie steps 4-5; lifecycle screen shared.
- Waffles 15:03 and 15:04:12, Cambium posts 23288c37ba8d29a01db477e738bdca659ac971d56d55ca788c8ac8885c02e0a9 and 95beb81d0590b794b3ccd72269d673e4ee99131fa7fce677868ff9aa6edac385: complete brief reviewed before any row, including fork creation; fork, service deployment, standalone directory with audit, then Cambium integration.
- Waffles review delivered 15:15:58 Melbourne (message 547f599c630df31282d33912c99a8342062cfbf637031437a5ba89f13d5e472d, signed 15:17): reorder 01,02,04,03,05,06,07; inspect native events before enlarging fork; one PostgreSQL; Rauthy themes; per-row hours and live demonstrations. His earlier 15:05 wording is corrected to the door timestamp 15:04:12.
- Tom 15:18 and Waffles 15:19 in Dot: consistent appearance across products, Aion's browser appearance preferred, Cambium not yet the reference. Identity screens follow Aion colours/type/spacing without a Cambium surface build dependency; Rauthy uses the same colours. Shared design-system extraction remains open and outside this brief.
- Tom 15:20, clarified by Waffles 15:21 in Dot: shared design, distinct product accents; Cambium green, Aion blue/black, Argus light blue, Haematite mustard. The new product's accent is orange, confirmed by Tom at 15:21:43. Do not silently make every product Aion-blue.
- Waffles approved revision 2 at 15:22:45, Cambium post a1d511ca4ce5ed95cfd577c5d89341a6f16387c6b38dad037b2419b83b65f519: rows 01 and 02 start; ceiling 48 hours, one implementer, one row/gate at a time. The subsequent colour update comes from Tom and Waffles and does not expand implementation scope.
- Waffles at door time 15:30:50, post d7cb4eb8f7463169da3395617b17b4d7f0b0882a96c7604e69679f209aa4cdbf: maintained fork branch ablative from v0.36.2; main is untouched upstream mirror; release-tag rebases only with their own gated row; no cherry-picks. Row03 adds users API type, migration inserts and admin UserInfo paths. Commit/push docs and pin without a gate; venue recursive-clone proof moves to row02 gate. Archie retains step04; Chippy rows pause if usage ends. Argus gate timeout belongs to Heimdall; no gate submission until resolved.
- Waffles ruling at door time15:36:25, post6591ea53d3ee6890121409cad43a0ec8a4770a9a451dc8aa34cf22cafef3b323: row01 accepted; isolated development installs and test-account demonstrations in rows02–05 may use v0.36.2. IDENTITY-001-UPSTREAM-AUTH-STATE binds real sign-in row06 and install row07. Add a separately gated rebase row onto an upstream release carrying #1696 and #1728; if no release by row05 showing, Tom decides nightly versus waiting. Release/advisory check before every install. Recursive-clone proof is due at row02 gate, not a row02 start blocker.
- Waffles15:38:49, postb02ebd241e1a19d45446218e372dcf504db44c3b3559e137b852c14051ec4098: no Python/provisioning shell; PostgreSQL init SQL, existing lys CLI prepare/configure/health subcommands, Rust integration tests; name exact module manifest before writes. Replace health.sh and external tests/identity_deployment. Re-estimate row02 and escalate any total above48 hours.
- Waffles at door time15:40:29, post4df1345f7fa1aee5880a1c46640ba591147284abaf7904d015f4e2e25b64c7ef: release rebase becomes separate future brief IDENTITY-002, opened only when a suitable upstream release exists; its4-hour estimate is recorded beside the blocker. IDENTITY-001 row02 is8 hours, total44.5 against48 ceiling,3.5 contingency. Rows06/07 depend on IDENTITY-002. Commit revision5/module manifest for re-check before row02 source starts.

## Ceiling

- Estimate:44.5 focused implementer hours (original40 minus row02 original3.5 plus row02 Rust implementation8); ceiling48 hours, contingency3.5. The release-rebase work is a separate future brief IDENTITY-002, estimated4 hours outside this ceiling, opened only when a suitable upstream release exists. Report a row overrun as soon as known; Waffles takes any required ceiling change to Tom. External queue/release/operator/review waits are reported separately.
- One implementer, Chippy; no builders or additional sub-agents. Archie owns a separate concurrent memory/context lane. Shared contract edits are coordinated before either author writes them.
- One row in implementation and one gate invocation at a time in this lane. Gate/build venue scheduling is coordinated with Waffles, not assumed exclusive across the estate.
- Edit main in the main Mac checkout. No worktree, tree copy to a laptop, or SSH build. Release builds, checks and tests run through the gate workflow on a laptop fetching exact pushed refs. At most one light local cargo check, with no heavy work beside it.
- Seven ordered rows below; real sign-in/production rows06 and07 additionally depend on separate future brief IDENTITY-002. A row that needs any file outside its wall stops and names it; reviewer approves a brief revision before that file is edited. Directory walls for wholly new modules allow only the named responsibility and require an exact file manifest before their row starts.
- Step 1 creates directory records, sign-in and minimum identity audit. Capability certificates, arbitrary grants, session launch/stop, runtime adapters, credential handles, memory, context assembly, lanterns and production anchoring remain subsequent briefs.
- SpiceDB and its durable database are installed and checked in row 2 as Waffles requested. Step 2 still owns live capability policy and its enforcement; merely running SpiceDB is not permission enforcement.
- No production Cambium auth cutover or restart before the scratch acceptance, review and Gypsy's coordinated install. No upstream contribution is a prerequisite. No product name is selected by technical crate or directory names.
- Live installed demonstrations to Tom are required at the end of row 03 and row 05, before proceeding beyond those milestones; row 07 is the final combined release proof, not their first showing.

## Design choices and reviewed rulings

- Maintained fork is ablative-io/rauthy branch ablative, created and pushed from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662. Fork main remains an untouched upstream mirror. Lys vendor/rauthy pins an exact commit on ablative; upgrades rebase onto upstream release tags only, each in its own gated row. No reset of main and no cherry-picking unpublished security changes.
- Use one PostgreSQL service and one durable database for Rauthy and SpiceDB, with isolated roles/schema namespaces and verified migration/search-path behavior. Pin supported releases and image digests in row 02. Rauthy v0.36.2 database.rs supports PostgreSQL; its internal Hiqlite cache/node still starts in that mode and is not a second authoritative identity database. Document required cache/key/config files and restore behavior rather than claiming a SQL dump alone backs up the whole product. No Hiqlite identity datastore in the installed configuration.
- Keep people authenticated by Rauthy; register enduring agents as first-class directory principals. Registration is not issuance of a Rauthy login, a runtime credential or a capability certificate. Row 1 records the evidence on Rauthy's machine-principal facilities for step 2.
- Bootstrap the initial directory administrator by an explicitly configured issuer/subject, never by email or first visitor. Step-1 directory maintenance has this bounded administrator policy; general assignment and why-access views arrive in step 2. Reject unauthenticated and non-administrator mutations.
- Propose S256 for the platform's own confidential OIDC client. Cambium S256 is a separately conditional addition in row 6: only include it if review explicitly selects that client configuration. Rauthy v0.36.2 does not require PKCE merely because a client is confidential; Client.challenge selects allowed methods. The existing confidential Cambium code flow is not itself evidence of incompatibility.
- Use a versioned domain event envelope outside lys-core. One signed committed directory event is both the identity change and its audit record; rebuild the directory projection from those events. Do not implement a database change followed by a best-effort log append.
- The service signs an attestation naming the authenticated human actor and authentication provenance. Do not claim the human personally signed bytes with a private key they do not hold. Agent registration by an administrator likewise records the administrator, not an invented agent signature.
- Native Rauthy events were inspected before retaining a link-audit addition: no provider-link/unlink EventType exists; mutation paths do not emit such an event; existing persistence is asynchronous and severity-filtered, with age-based cleanup. The stock stream cannot establish the required durable outcome. Retain a narrowly scoped same-transaction link audit record/outbox with stable operation IDs and explicit pending/acknowledged state, subject to row-01 exact-source inventory and review of the minimal diff. Reuse native transport where it can meet replay/acknowledgement requirements; do not build a general replacement event service or audit unrelated Rauthy operations.
- Use Aion's appearance as the reference for structure, typography, spacing and interaction, while preserving each product's own accent. Configure the Cambium Rauthy client green; give the identity client its own Ablative accent (orange, confirmed by Tom at 15:21:43). Exact accessible light/dark accent values are declared and reviewed before theme acceptance. Rauthy's fonts and page layout remain upstream's. Neither client nor the standalone screens require a build dependency on Cambium or Aion; extracting a shared design-system package stays a separate open decision.
- Estate guide now includes identity at ablative-docs 385916e (Waffles 15:24:18): accent #D4975A, deep #A86B2E, wash #3D2A17, separately keyed from Manifold copper. Rows 02 and 05 read those values and the shared foundation from docs/design-system-v2/palette/estate-colour-tokens.json; estate-colour-family.html renders the reference. Use Aion for layout/type/interaction, not a copied application dependency. Do not copy its remaining purple status token: the estate guide explicitly bans purple.
- Archie and Chippy agreed at 15:31 on one reviewed versioned envelope with typed audit/context payloads, log coordinate in the returned receipt outside the leaf and service-attested human actions. Exact encoding and fields remain joint-review work. Name the commitment hash explicitly; do not confuse SHA-256 attestation commitments with BLAKE3 content addresses. Row04 currently means lys-log-store file storage, not an already-proved Haematite backend.
- Development isolation is explicit: rows02–05 use only disposable test identities and test provider registrations, no live Cambium participant migration, production tokens or real business sign-in. Each development install checks current releases/advisories and records the accepted v0.36.2 exception. The upstream-auth-state blocker still binds06/07; no implicit acceptance of a nightly base.

## Contract shared with the context lane

- An enduring identity ID is stable through provider additions, key rotation and later multiple sessions. Issuer plus subject identifies an external login; email and display name never establish identity equivalence.
- A person or agent may be registered before any session exists. No synthetic running state or fabricated execution credential is created by registration.
- A future execution/fork ID refers to its enduring identity and, for a fork, its parent execution. Step 1 reserves that distinction in the contract but does not implement session history or launch.
- Audit receipts carry a version, stable operation ID, actor, affected identity, operation, payload commitment and resulting log coordinate/checkpoint. Secrets and whole context objects are excluded. Exact signed encoding is reviewed before use, not frozen by this draft.
- An uncertain append is reconciled using the existing Lys log's reopen rules before its projection answers as current. No retry under a fresh operation ID; no success before durable evidence. Named pending/refused outcomes stay visible until resolved.

## External dependencies

- **IDENTITY-002** (not_opened_waiting_for_upstream_release, estimated 4 hours): blocks rows 06, 07. Rebase ablative onto an upstream release carrying #1696 and #1728, exact-ref gate and three named account-status/redirect/logout guards. Open the brief only when that release exists; see RAUTHY-BASELINE.md. No nightly/cherry-pick substitution without Tom ruling.

## Source evidence

- [https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/events/event.rs](https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/events/event.rs) — lines 141-168,403-412,538-577. EventType has no provider-linked or provider-unlinked variant. Event has id, timestamp, severity/type, IP, numeric data and text, not a typed provider-subject link payload. Its insertion is a separate database call.
- [https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/entity/auth_providers.rs](https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/entity/auth_providers.rs) — lines 1167-1180. Explicit linking assigns the user provider ID and federation UID for a later save. That branch emits no link event.
- [https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/entity/users.rs](https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/entity/users.rs) — lines 801-817. provider_unlink clears the two federation fields and saves the user; no unlink event is emitted.
- [https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/events/listener.rs](https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/events/listener.rs) — lines 39-53,105-119,182-205. A received event is handled in a spawned task; persistence is conditional on configured severity. SSE replay comes from a bounded latest-event queue, not an acknowledged durable consumer cursor.
- [https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/api/src/events.rs](https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/api/src/events.rs) — lines 70-101. The authenticated SSE endpoint registers a client using level and latest count. This is not proof of a crash-safe link audit stream.
- [https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/schedulers/src/events.rs](https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/schedulers/src/events.rs) — lines 11-45. Old events are deleted by timestamp according to cleanup_days, with no Lys acknowledgement condition.
- [https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/database.rs](https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/database.rs) — lines 68-91,226-239. PostgreSQL is supported and has its own migration runner. Hiqlite starts internally even in PostgreSQL mode for cache/node functions; this does not require keeping the identity records in a separate embedded database.
- [https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/entity/theme.rs](https://raw.githubusercontent.com/sebadob/rauthy/v0.36.2/src/data/src/entity/theme.rs) — lines 19-28,93-100. Theme configuration is per client with light/dark values and border radius. The supported palette mapping does not replace fonts or layout.
- [/Users/tom/Developer/ablative/stack/aion/apps/aion-ops-console/src/index.css](/Users/tom/Developer/ablative/stack/aion/apps/aion-ops-console/src/index.css) — lines 1-180; last file commit 3b622599a. Read the clean source: self-hosted DM Sans and JetBrains Mono, radius and spacing ladders, explicit dark and light palettes with strategy-blue action accent and terracotta attention. Use this as the visual reference, not an endorsement of all Aion frontend implementation choices. Record the full source ref when deriving themes.
- [/Users/tom/Developer/ablative/docs/design-system-v2/palette/estate-colour-tokens.json](/Users/tom/Developer/ablative/docs/design-system-v2/palette/estate-colour-tokens.json) — lines 1-28. Verified updated guide at ablative-docs 385916e: identity orange #D4975A, deep #A86B2E and wash #3D2A17 are now an explicit entry alongside the shared foundation. Waffles owns this addition; the brief consumes it without rewriting the guide.

## Row 01 — Establish the maintained fork and exact source baseline

Estimated focused hours: **1.5**. Depends on: approved brief.

Provenance: Statement: Not decided (Rauthy decision 14:35-14:38); Waffles 15:03 fork instruction.

### File wall

**lys**

- `.gitmodules`
- `vendor/rauthy`
- `docs/design/identity/briefs/IDENTITY-001.json`
- `docs/design/identity/briefs/IDENTITY-001.md`
- `docs/design/identity/RAUTHY-BASELINE.md`

**external_action**

- `Create ablative-io/rauthy fork of sebadob/rauthy; pin v0.36.2 commit; no other repository created.`

### Work

- Verify the tag's commit, upstream licence, security/advisory status and supported toolchain; record fork maintenance and security-update ownership. Stop by name if the proposed baseline has an unresolved security blocker.
- Replace the partial reading extract as implementation authority with the actual pinned checkout. Inventory every link/login/unlink/admin/import/export consumer of the single provider fields and both supported migration paths.
- Record machine-principal facilities, confidential-client authentication and challenge configuration from source. Record the exact existing upstream files row 3 must touch in the brief before row 3 starts.
- Verify the native-events findings below against the fork commit; compare a minimal extension of existing event storage/transport with a dedicated link outbox. Record the least invasive design that atomically retains each committed link/unlink through receiver outages; no table is added just because the first draft named one.
- Review later security fixes against deployed paths and preserve IDENTITY-001-UPSTREAM-AUTH-STATE. Waffles accepted row01 after the pin, branch and findings were recorded. The defect blocks real sign-in/production rows06/07; development rows02–05 continue under the15:36:25 ruling. No cherry-picks; an upstream release containing both hardening commits must pass separate IDENTITY-002 before row06.

### Acceptance

- The pin and branch are verified locally and remotely. Fresh recursive-clone acceptance is carried to the row02 venue gate by Waffles15:36:25; it does not hold the start of row02.
- The baseline report names release, commit, licence, maintenance owner, policy for security updates and every legacy provider-field consumer; no assumption that a tree SHA is a commit SHA.

### Acceptance greps (locators, not test verdicts)

- rg -n 'commit|licence|security|challenge|confidential|machine|consumer' docs/design/identity/RAUTHY-BASELINE.md

## Row 02 — Install the standalone service dependencies with durable storage

Estimated focused hours: **8**. Depends on: 01.

Provenance: Statement: Permissions; Two rules from Tom; Road adjustment 1; Waffles 15:04:12 deployment order (SpiceDB deployment brought forward, capability enforcement remains step 2).

### File wall

**lys**

- `deploy/identity/compose.yaml`
- `deploy/identity/versions.json`
- `deploy/identity/config.example.toml`
- `deploy/identity/README.md`
- `docs/design/identity/reports/IDENTITY-001-deployment.md`
- `deploy/identity/rauthy-themes.json`
- `deploy/identity/theme-map.md`
- `deploy/identity/postgres-init.sql`
- `Cargo.toml`
- `Cargo.lock`
- `crates/lys/Cargo.toml`
- `crates/lys/src/main.rs`
- `crates/lys/src/cli.rs`
- `crates/lys/src/commands/error.rs`
- `crates/lys/src/identity/mod.rs`
- `crates/lys/src/identity/cli.rs`
- `crates/lys/src/identity/config.rs`
- `crates/lys/src/identity/credentials.rs`
- `crates/lys/src/identity/private_files.rs`
- `crates/lys/src/identity/prepare.rs`
- `crates/lys/src/identity/configure.rs`
- `crates/lys/src/identity/rauthy.rs`
- `crates/lys/src/identity/themes.rs`
- `crates/lys/src/identity/health.rs`
- `crates/lys/src/identity/error.rs`
- `crates/lys/tests/identity_deploy.rs`
- `crates/lys/tests/identity_refusals.rs`
- `crates/lys/tests/identity_theme.rs`
- `crates/lys/tests/identity_shared_db.rs`
- `crates/lys/tests/identity_restart.rs`
- `crates/lys/tests/identity_support/mod.rs`
- `crates/lys/tests/identity_support/fixtures.rs`
- `crates/lys/tests/identity_support/server.rs`
- `crates/lys/tests/identity_support/compose.rs`

### Module responsibilities

- `mod.rs`: Declarations and re-exports only.
- `cli.rs`: Identity subcommand argument declarations.
- `config.rs`: Typed deployment configuration and validation; no secret values in diagnostics.
- `credentials.rs`: Zeroizing/redacted credential material and stable reuse.
- `private_files.rs`: Restricted-mode durable file creation and outcome reconciliation.
- `prepare.rs`: Validate inputs and materialise the declared private deployment artifacts.
- `configure.rs`: Idempotent client/theme reconciliation with stable operation identifiers.
- `rauthy.rs`: Typed Rauthy API requests/responses, named status errors and uncertain-outcome read-back.
- `themes.rs`: Read the declared estate palette mapping and validate both client themes.
- `health.rs`: Named readiness checks for the declared services.
- `error.rs`: Typed errors carrying operation/resource/path, never secret bytes.

### Work

- Prepare and install an isolated development instance on the Mac, using test identities only. Keep release builds/checks/tests on the laptop workflow; do not use the development exception to bypass the gate path. Check upstream releases and advisories before every install and record the security exception.
- Package maintained Rauthy and SpiceDB against one PostgreSQL service and one durable database, with separate least-privilege roles/schema namespaces, explicitly configured credentials and documented local/TLS origins. Verify both migration runners and connection search paths coexist before claiming one-database readiness. These are three dependency processes beside the platform door; Cambium, Manifold, Argus and Aion servers are not runtime dependencies.
- Define readiness, migration order, named configuration failures, stop/start and backup/restore checks. Keep generated credentials out of Git and logs; health output excludes secrets.
- Register separate platform and Cambium OIDC clients with exact redirect URIs and RS256 where the Cambium verifier requires it. Google/GitHub federation credentials belong to Rauthy; Google API consent remains a different client purpose.
- Configure both Rauthy client themes using Aion's pinned neutral/text/radius design vocabulary and each client's own product accent: Cambium green, identity product accent separately declared (orange confirmed by Tom). Map text, text_high, bg, bg_high, action, accent and error into HSL for light and dark, plus button text colour and border radius. Record every source token and colour conversion; verify readable contrast. Colours mirror each product, not a universal Aion-blue. Fonts and page layout remain Rauthy's; no font/layout patch or cross-application build dependency enters the fork.
- Implement lys identity prepare/configure/health in the existing CLI using the module manifest below. Preparation validates declared config and materialises private files with restricted modes; configure reconciles two clients and themes with stable operation IDs and read-back after uncertain transport outcomes; health names service/database readiness failures. PostgreSQL runs postgres-init.sql through its own init directory. No Python or shell provisioning engine.

### Acceptance

- ID001_DEPLOY: from a fresh venue directory, dependencies become ready with no other Ablative service running; restart preserves issuer identity and database contents.
- ID001_DEPLOY_REFUSAL: missing secret, invalid issuer/redirect and unavailable database produce named failures, never a substitute identity or development database.
- Dependency artifact digests, versions, resolved configuration without secrets and backup/restore result are recorded. This row does not claim the product directory exists yet.
- ID001_THEME: inspect both client login pages in light and dark mode; all seven HSL fields, button text and border radius match the declared mapping, persist after restart and retain readable contrast. Record the source-token ref and Rauthy theme export without credentials.
- ID001_SHARED_DB: prove Rauthy and SpiceDB both write/reopen against the same PostgreSQL database, migrations do not collide and the service roles cannot read the other schema. Restore the database plus documented key/config/cache dependencies and repeat readiness.
- ID001_PIN_CLONE: the venue fetches the exact pushed Lys ref with recursive submodules and verifies the expected Rauthy commit on ablative without using the Mac reading copy.

### Acceptance greps (locators, not test verdicts)

- rg -n 'ID001_DEPLOY|ID001_DEPLOY_REFUSAL|ID001_THEME|ID001_SHARED_DB|ID001_PIN_CLONE' crates/lys/tests/identity_*.rs

## Row 04 — Build the directory contract and signed authoritative changes

Estimated focused hours: **10**. Depends on: 02.

Provenance: Statement: Road adjustments 1-3; Where it lives: lys; lifecycle distinction between identity authority and each session.

### File wall

**lys**

- `Cargo.toml`
- `Cargo.lock`
- `crates/lys-identity/`
- `crates/lys-identity-server/`
- `docs/design/identity/DIRECTORY-CONTRACT.md`
- `docs/design/identity/IDENTITY-EVENTS.md`
- `tests/identity_contract/`
- `deploy/identity/compose.yaml`
- `deploy/identity/config.example.toml`
- `deploy/identity/README.md`

### Work

- Add domain crates for directory records, typed API, OIDC session handling, administrator admission and event projection. Keep lys-core, its cryptographic primitives and published formats unchanged. Name every new module in the manifest before this row starts.
- Define stable person/agent identifiers, external issuer-subject bindings, registration and profile changes, explicit provenance and operation-id retry semantics. Register an agent without pretending it is running or manufacturing a human login for it.
- Review the versioned event envelope jointly with Archie before it signs durable bytes. Commit signed directory changes through lys-log-store and rebuild projections at open; reconcile uncertain writes before answering affected reads.
- Provide a read-only receipt and inclusion-verification path. Authenticate the initial administrator by configured issuer/subject; fail closed for other mutation callers and make the limited step-1 authority visible.
- Build and test the authenticated link-audit receiver before row 03 needs it, against the reviewed typed contract and fixtures. It consumes the minimal durable source selected in row 01, deduplicates stable source operation IDs and returns verifiable receipts. Separate issuer observations from human-signed claims; audit actor provenance survives replay.

### Acceptance

- ID001_DIRECTORY: register a person and an agent, edit the display profile, list/read both and reopen; enduring IDs and signed history remain unchanged.
- ID001_AUDIT_FAULTS: enumerate append/pin/projection crash boundaries and count exercised cases; every answered projection equals replay; uncertain operations retain identity and resolve once without silent loss or double application.
- ID001_ADMIN: an unauthenticated caller, a non-admin with the same email, and wrong issuer/subject are refused; denied calls cannot mutate state.
- ID001_RECEIPT: independently verify a recorded change against a checkpoint/key; changed actor, payload, sequence or signature fails verification. Secret-redaction tests cover debug, errors and serialized public responses.
- ID001_RECEIVER: fixture delivery, duplicate delivery, lost acknowledgement, receiver restart and unauthorized source exercise the reviewed link-audit contract without requiring the future multi-provider fork. Count each leg and prove one logical event per source operation.

### Acceptance greps (locators, not test verdicts)

- rg -n 'ID001_DIRECTORY|ID001_AUDIT_FAULTS|ID001_ADMIN|ID001_RECEIPT|ID001_RECEIVER' crates/lys-identity tests/identity_contract

## Row 03 — Link two upstream providers to one Rauthy person

Estimated focused hours: **10**. Depends on: 04.

Provenance: Statement: Rauthy decision and Road step 1; existing source refuses user is already federated.

### File wall

**rauthy**

- `src/api/src/auth_providers.rs`
- `src/api_types/src/auth_providers.rs`
- `src/data/src/entity/auth_providers.rs`
- `src/data/src/entity/users.rs`
- `src/data/src/entity/mod.rs`
- `src/data/src/entity/identity_links.rs`
- `src/data/src/entity/identity_link_audit.rs`
- `src/service/src/oidc/auth_providers/login_finish.rs`
- `src/service/src/oidc/auth_providers/login_start.rs`
- `frontend/src/api/types/auth_provider.ts`
- `frontend/src/api/types/user.ts`
- `frontend/src/lib/account/AccMain.svelte`
- `frontend/src/lib/account/AccOther.svelte`
- `frontend/src/lib/account/AccLinkedProviders.svelte`
- `frontend/src/routes/providers/callback/+page.svelte`
- `migrations/hiqlite/32_identity_links.sql`
- `migrations/postgres/V27__identity_links.sql`
- `tests/identity_links/`
- `src/api_types/src/users.rs`
- `src/data/src/migration/inserts.rs`
- `frontend/src/lib/admin/users/UserInfo.svelte`

**lys**

- `vendor/rauthy`
- `docs/design/identity/PROVIDER-LINK-CONTRACT.md`
- `docs/design/identity/reports/IDENTITY-001-links.md`

### Work

- Replace the single provider pair with a link relation unique on provider and subject; migrate existing links transactionally without changing Rauthy user IDs. Row 1 must verify migration filenames and complete this exact wall before code starts.
- Require an authenticated person, fresh reauthentication and a single-use target-bound linking intent. Bind state, nonce, provider and callback; reject identity collision instead of merging by matching email.
- Update every reader found in row 1, including administrative deletion/export/import where present. List links; unlink only after confirming the identity retains a usable authentication/recovery method. Preserve upstream account security controls.
- Use the minimal durable link-audit design selected from the native-events evidence in row 01. Commit link/unlink and audit provenance atomically; an acknowledged Lys receipt from the already-built row-04 receiver is required before the platform labels the audit complete. Duplicate delivery cannot create another identity event.

### Acceptance

- ID001_LINK_PAIR: Google then GitHub, and GitHub then Google, resolve to one unchanged user subject; sign out and back in through either provider and reopen storage.
- ID001_LINK_REFUSAL: same email with different subject, already-owned provider identity, replayed/cross-account intent, CSRF/nonce mismatch and final-login unlink are refused with no unintended link.
- ID001_LINK_MIGRATION: both supported storage migrations preserve IDs and links, restart safely after interruption and refuse incompatible schema versions by name.
- ID001_LINK_AUDIT: crash after database commit but before audit acknowledgement retains the outbox; delivery/retry yields one logical signed event. Audit outage is visible, never a false completed receipt.
- ID001_LINK_LIVE: install the exact gated fork and show Tom two linked providers resolving to one person in Rauthy's own account page; sign in through both. Post a separate install/showing receipt to Tom with Melbourne pass time, fork ref, artifact hash and observed result before proceeding to row 05. A venue test or screenshot alone does not replace the live demonstration.

### Acceptance greps (locators, not test verdicts)

- rg -n 'ID001_LINK_PAIR|ID001_LINK_REFUSAL|ID001_LINK_MIGRATION|ID001_LINK_AUDIT|ID001_LINK_LIVE' vendor/rauthy/tests/identity_links

## Row 05 — Complete the standalone screen journey

Estimated focused hours: **6**. Depends on: 03.

Provenance: Statement: beyond just a login page; Road adjustment 1; lifecycle screen running through every release.

### File wall

**lys**

- `surface/identity/`
- `crates/lys-identity-server/src/assets.rs`
- `crates/lys-identity-server/src/routes.rs`
- `deploy/identity/README.md`
- `docs/design/identity/reports/IDENTITY-001-standalone.md`

### Work

- Build sign-in, directory of people and agents, record detail, register/edit agent, linked-account entry point and signed change history. Generate boundary types from the server schema. Preserve shared lifecycle extension points without implementing Archie's screens.
- Follow Aion's structure, DM Sans/JetBrains Mono typography, radii, spacing and browser interaction conventions, with the identity product's own accent instead of inheriting Aion-blue. Keep pinned provenance for neutral design values and separately declared accent values, without a build dependency on Cambium or Aion. Central design-system extraction is outside this brief. Show provider-link audit pending, unavailable services and named refusals accurately.
- Registration states what has been created. Controls for launch, live permissions, secrets and memory are not presented as working in step 1.

### Acceptance

- ID001_STANDALONE: with Cambium and Manifold absent, an operator installs, signs in, links two providers, creates an agent, returns after restart and finds the same records and inspectable change history.
- ID001_SCREEN_REFUSAL: expired login, unauthorized edit, provider collision, audit-pending and backend outage have actionable visible outcomes; no optimistic completed state after refusal.
- Use the native browser against the installed venue artifact; record screenshots and observed actions plus artifact hashes. A mockup or frontend build alone is not this acceptance.
- ID001_DIRECTORY_LIVE: install the gated standalone directory and complete its journey with Tom in a live browser. Post a separate line to Tom with the Melbourne pass time, installed refs/hashes and what was demonstrated before starting row 06; no deferral to row 07.

### Acceptance greps (locators, not test verdicts)

- rg -n 'ID001_STANDALONE|ID001_SCREEN_REFUSAL|ID001_DIRECTORY_LIVE' surface/identity

## Row 06 — Connect Cambium without changing its people or Google API ownership

Estimated focused hours: **4**. Depends on: 05, IDENTITY-002.

Provenance: Statement: Cambium first connected customer, independent operation retained; DOOR-TRANSITION-2026-09-21 explicit linking, read with today's issuer decision.

### File wall

**cambium**

- `crates/cambium-door/src/http/google.rs`
- `crates/cambium-door/src/http/google_client.rs`
- `crates/cambium-door/src/http/mod.rs`
- `crates/cambium-door/tests/google.rs`
- `crates/cambium-door/tests/google_walk.rs`
- `crates/cambium-door/tests/identity_link/cases.rs`
- `crates/cambium-door/tests/signin_worlds.rs`
- `crates/cambium-door/tests/common/harness.rs`
- `crates/cambium-server/src/config/types.rs`
- `crates/cambium-server/src/config/defaults.rs`
- `crates/cambium-server/src/config/file.rs`
- `crates/cambium-server/src/config/keys.rs`
- `crates/cambium-server/src/boot/settings.rs`
- `crates/cambium-server/src/boot/run.rs`
- `crates/cambium-server/src/boot/check.rs`
- `crates/cambium-server/tests/config.rs`
- `docs/CONFIG.md`
- `docs/FILE-MAP.md`
- `surface/app/src/features/signin/SigninView.tsx`
- `surface/app/src/features/signin/signin-state.ts`
- `surface/app/src/features/signin/google-start.ts`

**cambium_only_if_s256_configuration_approved**

- `crates/cambium-door/src/http/oidc.rs`
- `crates/cambium-door/tests/identity.rs`
- `crates/cambium-store/src/traits/identity.rs`
- `crates/cambium-store/tests/oidc_link.rs`
- `crates/cambium-store/tests/mutation_faults/identity_fixtures.rs`

**lys**

- `deploy/identity/config.example.toml`
- `deploy/identity/README.md`
- `docs/design/identity/reports/IDENTITY-001-cambium.md`

### Work

- Preserve the existing authenticated, explicit account-binding path; map the Rauthy issuer-subject onto the existing Cambium participant. Never bulk match by email or recreate memberships, authorship or agent seats.
- Separate Google Drive/Calendar OAuth client credentials and provider endpoints from sign-in. A Cambium-to-Rauthy secret is never sent to Google. Existing API grants preserve their original client identity or require an explicit reconnect; a new client cannot refresh an old client's grant by assumption.
- Configure the selected confidential Rauthy client and exact redirect URI. If S256 is approved, persist a private verifier per transaction, send its challenge and exchange that verifier only after the existing one-use state/nonce/cookie checks. Old incomplete transactions refuse by name and require a new sign-in; verifier values never appear in logs or public types.
- Exercise both direct-provider standalone Cambium configuration and the platform issuer configuration; neither silently falls back to the other. Check sign-in wording and issuer-independent return behavior.

### Acceptance

- ID001_CAMBIUM_LINK: same person through both providers sees the exact pre-existing participant ID, projects, messages and attribution after sign-out/reopen; duplicate/collision binding refuses.
- ID001_GOOGLE_SEPARATE: Rauthy login plus a separate Google API consent/refresh works against isolated test providers; captured upstream requests prove credentials go only to their intended issuer. Revoked grants require reconnect by name.
- ID001_PKCE, conditional: S256 challenge matches the privately stored verifier across reopen; replay, wrong verifier, wrong state/nonce and old pending transaction fail as specified. Omission of this leg requires the reviewed client configuration to be recorded, not a silent skip.
- No test writes to the production Cambium store or real operator accounts. Live provider acceptance uses explicit test accounts and records where operator-controlled app registration is still needed.

### Acceptance greps (locators, not test verdicts)

- rg -n 'ID001_CAMBIUM_LINK|ID001_GOOGLE_SEPARATE|ID001_PKCE' crates/cambium-door/tests crates/cambium-store/tests

## Row 07 — Gate, install and demonstrate the exact release

Estimated focused hours: **5**. Depends on: 06, IDENTITY-002.

Provenance: Statement: each step installed and shown on a screen; Waffles 15:04:12 venue rules; Gypsy 15:02 runtime ownership.

### File wall

**lys**

- `gates.json`
- `scripts/identity-gates/`
- `deploy/identity/README.md`
- `docs/design/identity/reports/IDENTITY-001-release.md`
- `docs/design/identity/reports/IDENTITY-001-commands.jsonl`
- `docs/design/identity/briefs/IDENTITY-001.json`
- `docs/design/identity/briefs/IDENTITY-001.md`

**cambium**

- `docs/design/identity/IDENTITY-001-install.md`

### Work

- Register each required leg with the actual gate workflow before the row that first needs it, using this row's gate-registration wall as shared release metadata. Rows 03 and 05 require their own exact-ref gate/install/showing receipts; this final row verifies the combined release. A missing venue/workflow leg is a named blocker, never replaced by a local release build or pass claim.
- Gate the exact pushed Lys, Rauthy and Cambium refs and preserve the dependency pins; independent review checks authentication, linking, actor attribution and crash outcomes. No change to cryptographic formats is authorized by this brief.
- Verify the earlier row-03 and row-05 live install receipts, install any final matching standalone artifacts, and coordinate Cambium integration/install with Gypsy. Record backups and a rollback that respects migrated schemas; an old Rauthy binary is never launched against a forward-only database.
- Record every command and result, per-target counts, tested and installed refs/hashes, missing credentials or unrun live legs. Update brief status from actual evidence only.

### Acceptance

- All required venue legs green on exact refs; standalone acceptance recorded before Cambium cutover; operator can inspect who the two providers resolve to and the agent's recorded creation.
- Health alone is not completion: installed screen behavior and preserved Cambium identities are observed. Every deferred requirement is named, with no claim of step-2 permissions, step-3 broker or Archie's context work.

### Acceptance greps (locators, not test verdicts)

- rg -n 'ref|artifact|standalone|Cambium|not run|command' docs/design/identity/reports/IDENTITY-001-release.md

## Required venue checks

- Lys: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps.
- Cambium: exact gates.json legs, including cargo clippy --workspace --all-targets -- -D warnings, cargo fmt --check, scoped and full required test targets, frontend strictness, ast-grep, LOC gate, file-map regeneration and git diff --check.
- Rauthy: pinned upstream baseline legs plus counted link/migration/audit regressions; name pre-existing upstream failures and lint allowances rather than hiding them or importing them as local policy.
- New platform code: no lint bypasses, ignored tests, unwrap/expect/panic, underscore renames, invented defaults or secrets in debug/logs; named modules, documented first lines and 500-LOC boundary. Frontend strict types and generated API schema; meaningful negative and crash-boundary tests.
- Native installed browser journey, two real upstream provider test accounts, service stop/start and database restore. Missing client registrations block a live-proof claim even when isolated-provider tests pass.

## Current evidence and limits

- Row01 accepted by Waffles15:36:25. Fork ablative and Lys pin are pushed at dd61ac3/146796f. Row02 preparation is running for isolated development only. No application patches, compiler invocation, tests, gate, service install or production sign-in change has occurred.
- Cambium source already implements explicit OIDC linking; that is reused, not claimed as new work. Its current Google API code still uses sign-in client/provider settings, requiring separation before issuer cutover.
- Rauthy v0.36.2 clients.rs validate_code_challenge is configuration-driven; authorization-code client authentication and challenge validation are separate. The saved source extract is read-only evidence, not the fork checkout.
- The existing Lys log retains leaves in memory; this step uses small identity/audit records and makes no claim of unlimited history or production anchoring.
- Revision5 includes the approved Rust CLI deployment shape and exact row02 module manifest; no product source has been written. Total44.5 hours fits48 with3.5 contingency. The four-hour release rebase is future IDENTITY-002 outside this brief and blocks06/07. Row01 accepted; row02 source starts after Waffles re-checks this commit. Venue gate path remains with Heimdall.
