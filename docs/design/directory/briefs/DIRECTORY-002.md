---
type: brief
id: DIRECTORY-002
cluster: directory
title: Install the standalone service dependencies on one PostgreSQL database
---

# DIRECTORY-002: Install the standalone service dependencies on one PostgreSQL database

> **Cluster:** directory
> **Depends on:** DIRECTORY-001
> **Blocked by:** Waffles' review of this brief before it is dispatched (DIRECTORY-001 boundary: no row brief is dispatched until Waffles has reviewed it), Waffles' re-check of revision 5's row-02 module manifest, which IDENTITY-001 records as due before row 02 source starts (docs/design/identity/briefs/IDENTITY-001.json:20, docs/design/identity/briefs/IDENTITY-001.json:407), The venue gate path ID001_PIN_CLONE needs, which revision 5 records as resting with Heimdall (docs/design/identity/briefs/IDENTITY-001.json:17, docs/design/identity/briefs/IDENTITY-001.json:407); the row's gate is not submitted until it is confirmed
> **Design anchor:**
> - ADR-003 — Everything is pegged to a human authority — A person signs in first; an agent is provisioned under that person with its own identity; the person's permissions are the ceiling and the agent holds an explicit subset; every grant says who may exercise it and who may pass it on; withdrawing the authority stops every grant derived from it. The exact delegation schema is not settled by this decision.
> - ADR-005 — The identity database is PostgreSQL, possibly on a network device — PostgreSQL is used for the identity product's database. It may be set up on one of the network devices rather than on Tom's Mac.
> - ADR-009 — People sign in through a maintained Rauthy fork of our own — Rauthy authenticates people, and its one-provider-per-user limit is changed in a fork we maintain, ablative-io/rauthy, not contributed upstream as a prerequisite. The maintained branch is ablative, created from upstream v0.36.2 commit dd61ac3c84d6b238108dc8438b53043b5177a662; the fork's main stays an untouched upstream mirror; lys pins an exact commit of ablative as the submodule vendor/rauthy. Upgrades rebase ablative onto upstream release tags only, each in its own gated row; no cherry-picks and no reset of main.
> - ADR-010 — Every product shares one design and keeps its own accent; the identity product's is orange — The identity screens follow Aion's structure, typography, spacing and interaction, and Rauthy's client themes take the same colours, with no build dependency on Cambium or Aion. Each product keeps its own accent within the estate colour family: Cambium green, Aion blue and black, Argus light blue, Haematite mustard. The identity product's accent is orange (accent #D4975A, deep #A86B2E, wash #3D2A17 in the estate colour tokens), set apart from Manifold's copper. No product is silently made Aion-blue, and purple is not used.
> - ADR-021 — The identity install themes Rauthy in dark only, from the estate tokens as they stand — The install themes both Rauthy clients in dark only, from the estate tokens as they stand, and a Rauthy field takes an estate token only where the estate names the same role: bg takes ink, the page background; bg_high takes raised, the highest of the three backgrounds; text takes text, the primary text, since Rauthy's text is the body text; accent takes each client's own product accent (ADR-010). Every other field is a gap and keeps Rauthy's own default. Rejected: mapping text to muted, which would set the body of every page in the secondary colour; taking light mode, the error colour or the radius from a source outside the estate tokens, which would make that source a colour authority beside the design system.
> **Checklist:**
> - C7 — Rauthy, SpiceDB and one PostgreSQL service and database are packaged and installed for development, with separate roles and schema namespaces, the database address as configuration, and readiness, restart, named refusals, the shared database and restore proved (ID001_DEPLOY, ID001_DEPLOY_REFUSAL, ID001_SHARED_DB, ID001_PIN_CLONE).
> - C8 — lys identity prepare, configure and health exist in the CLI exactly as revision 5's module manifest names them, register the platform and Cambium clients idempotently, and keep every secret out of Git, logs, errors and health output.
> - C9 — Both Rauthy client themes carry Aion's neutral vocabulary with each product's own accent, Cambium green and the identity orange (ID001_THEME).
> - C10 — SpiceDB's step-1 role is written down and held: installed and checked, asked for no decision, holding no grant.
> **Stories:**
> - S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

## Purpose

Row 02 of IDENTITY-001 revision 5: the three dependency processes of the standalone identity product (the maintained Rauthy, ADR-009; SpiceDB; one PostgreSQL service and database, ADR-005) installed for development with durable storage, the lys CLI that prepares, configures and checks them, and both Rauthy client themes (ADR-010). Revised for Tom's PostgreSQL ruling, which lets the database live on a network device so its address is configuration, and for the grant ruling (ADR-003), which makes SpiceDB's step-1 role explicit: installed and checked, enforcing nothing yet.

## Task

Carry IDENTITY-001 revision 5 row 02 (docs/design/identity/briefs/IDENTITY-001.json:89-167) into requirements: packaging and the development install (R1), the lys identity CLI (R2), the two client themes (R3) and SpiceDB's step-1 role (R4). Row 01, the fork and its pin, was accepted by Waffles at 15:36:25 (docs/design/identity/briefs/IDENTITY-001.json:18); its fresh recursive-clone proof is due at this row's venue gate (ID001_PIN_CLONE). Estimate: 8 focused implementer hours, revision 5's figure for this row (docs/design/identity/briefs/IDENTITY-001.json:20); no re-estimate. Revision 5's 48-hour ceiling stands (CN7); an overrun is reported as soon as it is known. Out: the directory itself (DIRECTORY-003), any grant or permission check, and real sign-in (rows 06 and 07). Every path is relative to the repository root; the estate colour tokens live in another repository and are named in R3's spec with their owner. Wall revisions under CN9, approved with this brief: unit tests of the modules under crates/lys/src/identity/ sit beside them in sibling <module>_tests.rs files, as crates/lys/src/cli_tests.rs does, and are the only files added there beyond the manifest; and CLAUDE.md gains one line, the identity leg, in its Gates before any commit list and changes nowhere else (R1). The three blocked_by entries stand as written: each is discharged only by its blocker having landed, checked live when the build starts, and a carried-over sign-off discharges none of them. The container runtime R1 names as a prerequisite of the landing venue is checked live at the same moment.

## Requirements

### R1: Package Rauthy, SpiceDB and one PostgreSQL database, and install them for development

THE SYSTEM SHALL package the maintained Rauthy at the pinned vendor/rauthy commit (ADR-009), SpiceDB, and one PostgreSQL service with one durable database (ADR-005) in deploy/identity/compose.yaml, with pinned supported releases and image digests in deploy/identity/versions.json, separate least-privilege roles and schema namespaces created by deploy/identity/postgres-init.sql through PostgreSQL's own init directory, explicitly configured credentials, and documented local and TLS origins in deploy/identity/config.example.toml (docs/design/identity/briefs/IDENTITY-001.json:136, docs/design/identity/briefs/IDENTITY-001.json:36). The database address SHALL be configuration and SHALL NOT assume Tom's Mac (ADR-005). Both migration runners and their connection search paths SHALL be verified to coexist before one-database readiness is claimed. These are three dependency processes beside the platform door; Cambium, Manifold, Argus and Aion servers are not runtime dependencies (ADR-004). THE SYSTEM SHALL define readiness, migration order, named configuration failures, stop and start, and backup and restore (docs/design/identity/briefs/IDENTITY-001.json:137), documenting in deploy/identity/README.md the cache, key and config files Rauthy's internal Hiqlite cache needs rather than claiming a SQL dump alone backs up the product; no Hiqlite identity datastore is used in the installed configuration (docs/design/identity/briefs/IDENTITY-001.json:36). The development instance is installed on a node the operator names (ADR-005) with test identities only; release builds, checks and tests stay on the gate workflow, and each development install checks upstream releases and advisories and records the accepted v0.36.2 exception (docs/design/identity/briefs/IDENTITY-001.json:135, CN10). Results, digests and the restore outcome go to docs/design/identity/reports/IDENTITY-001-deployment.md, the report name revision 5 gives this row. The container-backed test targets identity_deploy, identity_refusals, identity_shared_db and identity_restart run in their own leg of .land/gates.sh, the identity leg, which runs cargo test -p lys --all-features --test 'identity_*' on every land with no path scoping; cargo test --workspace --all-features stays hermetic. crates/lys/Cargo.toml SHALL declare [[test]] entries with test = false for identity_deploy, identity_refusals, identity_shared_db and identity_restart, so cargo test --workspace --all-features does not run them and they run only on that container gate leg. Prerequisite of the landing venue: a container runtime, which a stranger checks on that host by running docker version, and podman version where docker is absent, one of them exiting 0; it is checked live when the build starts, alongside the blocked_by entries. IF neither docker version nor podman version exits 0 on the landing host, THEN the identity leg's first command SHALL fail the leg with the named refusal container_runtime_missing, naming the act: install or start Docker or Podman on the landing host. The identity leg SHALL NOT be skipped, SHALL NOT be reported green without running its tests, and no identity test SHALL carry #[ignore]. CLAUDE.md's Gates before any commit list SHALL gain the identity leg's command as one line, so .land/gates.sh still runs the gates exactly as that list names them; no other line of CLAUDE.md changes.

**Acceptance:**
- ID001_DEPLOY: from a fresh venue directory, dependencies become ready with no other Ablative service running; restart preserves issuer identity and database contents.
- ID001_DEPLOY_REFUSAL: missing secret, invalid issuer/redirect and unavailable database produce named failures, never a substitute identity or development database.
- Dependency artifact digests, versions, resolved configuration without secrets and backup/restore result are recorded in docs/design/identity/reports/IDENTITY-001-deployment.md. This row does not claim the product directory exists yet.
- ID001_SHARED_DB: prove Rauthy and SpiceDB both write/reopen against the same PostgreSQL database, migrations do not collide and the service roles cannot read the other schema. Restore the database plus documented key/config/cache dependencies and repeat readiness.
- ID001_PIN_CLONE: the venue fetches the exact pushed Lys ref with recursive submodules and verifies the expected Rauthy commit on ablative without using the Mac reading copy.
- A test sets the database address in the environment rendered from deploy/identity/compose.yaml and deploy/identity/config.example.toml to a host other than the local host: the value that environment passes to Rauthy's PG_HOST and the host in the datastore URI it passes to SpiceDB both name that address, and when that address is unreachable neither is replaced by a default local address (ADR-005).
- cargo test --workspace --all-features, run on a host with no container runtime, exits 0, starts no container and runs none of the test targets identity_deploy, identity_refusals, identity_shared_db and identity_restart.
- The identity leg of .land/gates.sh, run on a host where neither docker nor podman is on PATH, prints container_runtime_missing with the act install or start Docker or Podman on the landing host, reports the leg's status as non-zero, and .land/gates.sh exits non-zero.
- rg -n '#\[ignore' crates/lys/tests crates/lys/src/identity prints nothing and exits 1.
- git diff of CLAUDE.md against the brief's src commit shows exactly one added line and zero removed lines, the added line sits inside the Gates before any commit block, and it is the identity leg's command as .land/gates.sh runs it.

**Files:**
- create: deploy/identity/compose.yaml
- create: deploy/identity/versions.json
- create: deploy/identity/config.example.toml
- create: deploy/identity/postgres-init.sql
- create: deploy/identity/README.md
- create: docs/design/identity/reports/IDENTITY-001-deployment.md
- create: crates/lys/tests/identity_deploy.rs
- create: crates/lys/tests/identity_refusals.rs
- create: crates/lys/tests/identity_shared_db.rs
- create: crates/lys/tests/identity_restart.rs
- create: crates/lys/tests/identity_support/mod.rs
- create: crates/lys/tests/identity_support/fixtures.rs
- create: crates/lys/tests/identity_support/server.rs
- create: crates/lys/tests/identity_support/compose.rs
- modify: crates/lys/Cargo.toml
- modify: .land/gates.sh
- modify: CLAUDE.md

**Checklist:**
- C7 — Rauthy, SpiceDB and one PostgreSQL service and database are packaged and installed for development, with separate roles and schema namespaces, the database address as configuration, and readiness, restart, named refusals, the shared database and restore proved (ID001_DEPLOY, ID001_DEPLOY_REFUSAL, ID001_SHARED_DB, ID001_PIN_CLONE).

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

### R2: Prepare, configure and check the deployment from the lys CLI

THE SYSTEM SHALL implement lys identity prepare, configure and health in the existing lys CLI with exactly revision 5's module manifest (docs/design/identity/briefs/IDENTITY-001.json:154-166): mod.rs declarations and re-exports only; cli.rs the identity subcommand arguments; config.rs typed deployment configuration and validation with no secret value in diagnostics; credentials.rs Zeroizing, redacted credential material and its stable reuse; private_files.rs restricted-mode durable file creation and outcome reconciliation; prepare.rs validating inputs and materialising the declared private artifacts; configure.rs idempotent client and theme reconciliation with stable operation identifiers; rauthy.rs typed Rauthy API requests and responses, named status errors and read-back after an uncertain outcome; health.rs named readiness checks for the declared services; error.rs typed errors carrying operation, resource and path, never secret bytes; themes.rs is R3's. configure SHALL register separate platform and Cambium OIDC clients with exact redirect URIs and RS256 where the Cambium verifier requires it, and the platform's own confidential client with S256 as revision 5 proposes (docs/design/identity/briefs/IDENTITY-001.json:138, docs/design/identity/briefs/IDENTITY-001.json:39). Google and GitHub federation credentials belong to Rauthy; Google API consent remains a different client purpose. Generated credentials stay out of Git and logs, and health output excludes secrets. No Python or shell provisioning engine: health.sh and the external tests/identity_deployment are replaced by these subcommands and Rust integration tests (docs/design/identity/briefs/IDENTITY-001.json:19, docs/design/identity/briefs/IDENTITY-001.json:140). configure registers exactly two managed clients, platform and Cambium, beside the built-in rauthy client the pinned Rauthy's own migration inserts, and SHALL NOT change that built-in client. Unit tests sit in sibling <module>_tests.rs files beside the modules they test under crates/lys/src/identity/ (a wall revision under CN9); themes_tests.rs is R3's. The first four acceptance lines of this requirement need a running Rauthy, PostgreSQL and SpiceDB and run in R1's container-backed target identity_deploy, crates/lys/tests/identity_deploy.rs: they run only in the identity leg of .land/gates.sh and SHALL NOT run in cargo test --workspace --all-features, under the test = false manifest entry R1 declares.

**Acceptance:**
- A redaction test formats every credential and error type under crates/lys/src/identity/ with Debug and Display, and captures the health output of a configured deployment: none contains a byte of a generated secret.
- prepare creates every declared private file with a restricted mode; a test reads each file's mode and counts the files against the declared set.
- After two configure runs against the same Rauthy, the platform client and the Cambium client each exist exactly once with the configured values and unchanged operation identifiers, the built-in rauthy client is unchanged, and no other client exists; a transport failure after a request is resolved by read-back and never creates a second client.
- health names each unready service or database: a test makes each one unavailable in turn and counts one named failure per case, equal to the number of declared services.
- The ten modules of the manifest this requirement creates (mod.rs, cli.rs, config.rs, credentials.rs, private_files.rs, prepare.rs, configure.rs, rauthy.rs, health.rs, error.rs) exist under crates/lys/src/identity/; the only other files this requirement adds there are its sibling tests config_tests.rs, credentials_tests.rs, private_files_tests.rs, prepare_tests.rs, configure_tests.rs, rauthy_tests.rs, health_tests.rs and error_tests.rs; none exceeds 500 lines of code.

**Files:**
- create: crates/lys/src/identity/mod.rs
- create: crates/lys/src/identity/cli.rs
- create: crates/lys/src/identity/config.rs
- create: crates/lys/src/identity/credentials.rs
- create: crates/lys/src/identity/private_files.rs
- create: crates/lys/src/identity/prepare.rs
- create: crates/lys/src/identity/configure.rs
- create: crates/lys/src/identity/rauthy.rs
- create: crates/lys/src/identity/health.rs
- create: crates/lys/src/identity/error.rs
- create: crates/lys/src/identity/config_tests.rs
- create: crates/lys/src/identity/credentials_tests.rs
- create: crates/lys/src/identity/private_files_tests.rs
- create: crates/lys/src/identity/prepare_tests.rs
- create: crates/lys/src/identity/configure_tests.rs
- create: crates/lys/src/identity/rauthy_tests.rs
- create: crates/lys/src/identity/health_tests.rs
- create: crates/lys/src/identity/error_tests.rs
- modify: Cargo.toml
- modify: Cargo.lock
- modify: crates/lys/Cargo.toml
- modify: crates/lys/src/main.rs
- modify: crates/lys/src/cli.rs
- modify: crates/lys/src/commands/error.rs
- modify: crates/lys/tests/identity_deploy.rs

**Checklist:**
- C8 — lys identity prepare, configure and health exist in the CLI exactly as revision 5's module manifest names them, register the platform and Cambium clients idempotently, and keep every secret out of Git, logs, errors and health output.

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

### R3: Theme both Rauthy clients with Aion's vocabulary and each product's own accent

THE SYSTEM SHALL configure both Rauthy client themes in dark mode only, from the estate colour tokens as they stand and each client's own product accent (ADR-010, ADR-021): the Cambium client green, the identity client the identity orange. A Rauthy dark field SHALL take an estate token only where the estate names the same role: bg takes ink (page background), bg_high takes raised, the highest of the three backgrounds, text takes text (primary text, Rauthy's body text), and accent takes the client's product accent. These dark fields are written as HSL in deploy/identity/rauthy-themes.json, and deploy/identity/theme-map.md states the mapping as a table naming the estate token beside each Rauthy field with the hex value read from the tokens file, never typed by hand, and its colour conversion; the themes are read and validated by crates/lys/src/identity/themes.rs, whose unit tests sit beside it in crates/lys/src/identity/themes_tests.rs, and SHALL verify readable contrast (docs/design/identity/briefs/IDENTITY-001.json:139). THE SYSTEM SHALL NOT map text to muted, and SHALL NOT set the light fields, the dark error, the border radius, or the dark text_high, action and btn_text: each keeps Rauthy's own default, and theme-map.md names six gaps for a design-system card, light mode, the error colour, the border radius, and the dark text_high, action and btn_text, each keeping Rauthy's own default until the estate names a token for it; nothing reads a gap. Fonts and page layout remain Rauthy's; no font or layout patch and no cross-application build dependency enters the fork. The colour values are read from the estate colour tokens, docs/design-system-v2/palette/estate-colour-tokens.json in the ablative docs repository (owner: Waffles, who added the identity entry at ablative-docs 385916e; docs/design/identity/briefs/IDENTITY-001.json:44); that file is another repository's, is read and never changed, and is named here rather than listed in files (CN4). Its purple status token is not copied. crates/lys/Cargo.toml SHALL declare a [[test]] entry with test = false for identity_theme, so cargo test --workspace --all-features does not run it and it runs only on the container gate leg R1 gives the container-backed tests.

**Acceptance:**
- ID001_THEME: inspect both client login pages in dark mode; the dark bg, bg_high, text and accent match the declared mapping, persist after restart and retain readable contrast, and the light fields, the dark error, text_high, action and btn_text and the border radius equal the values Rauthy returns for them before configure. Record the source-token ref and Rauthy theme export without credentials.
- The Cambium client's accent is Cambium green and the identity client's is the identity orange of the estate tokens; neither is Aion blue, and no purple token appears in deploy/identity/rauthy-themes.json.
- deploy/identity/rauthy-themes.json declares, for each client, exactly the dark fields bg, bg_high, text and accent, and no light field, no error, no text_high, no action, no btn_text and no border radius.
- deploy/identity/theme-map.md holds a table with one row each for bg, bg_high, text and accent, naming the estate tokens ink, raised, text and the product accent beside them, and names exactly six gaps: light mode, the error colour, the border radius, the dark text_high, the dark action and the dark btn_text.
- cargo test --workspace --all-features, run on a host with no container runtime, does not run the test target identity_theme, and the identity leg of .land/gates.sh runs it.
- Comparing docs/design/decisions.json at the brief's src commit on main with docs/design/decisions.json on this row's branch, entry by entry by id, counts zero entries missing and zero entries whose JSON value differs, and the ledger holds ADR-021, the theme decision this requirement anchors to.
- Comparing docs/design/roadmap.json at the brief's src commit on main with docs/design/roadmap.json on this row's branch, row by row by id, counts zero rows missing and zero rows whose JSON value differs.

**Files:**
- create: deploy/identity/rauthy-themes.json
- create: deploy/identity/theme-map.md
- create: crates/lys/src/identity/themes.rs
- create: crates/lys/src/identity/themes_tests.rs
- create: crates/lys/tests/identity_theme.rs
- modify: crates/lys/Cargo.toml

**Checklist:**
- C9 — Both Rauthy client themes carry Aion's neutral vocabulary with each product's own accent, Cambium green and the identity orange (ID001_THEME).

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

### R4: State what SpiceDB enforces in step 1, and hold to it

SpiceDB's step-1 role, in one sentence: in step 1 SpiceDB is installed, migrated, backed up and health-checked against the shared PostgreSQL database, and it enforces nothing, because no component asks it for a permission decision or writes a grant to it, and live capability policy and its enforcement stay with road step 2 (docs/design/identity/briefs/IDENTITY-001.json:30; docs/design/identity/STATEMENT-2026-09-22.md:143). What it does not do in step 1: it answers no check for any identity, it holds no grant or relationship of the directory, the lifecycle state recorded by DIRECTORY-003 gates nothing through it, and running it is not permission enforcement. THE SYSTEM SHALL write that sentence into deploy/identity/README.md, and SHALL NOT add to crates/lys/src/identity/ any SpiceDB permission check, relationship write or schema write; health reads SpiceDB's readiness only. Relationships a test writes to prove ID001_SHARED_DB are test fixtures, not grants.

**Acceptance:**
- deploy/identity/README.md carries the step-1 sentence of this requirement word for word.
- A search of crates/lys/src/identity/ finds SpiceDB named only by the health readiness check and its configuration: no permission check, relationship write or schema write call.

**Files:**
- modify: deploy/identity/README.md

**Checklist:**
- C10 — SpiceDB's step-1 role is written down and held: installed and checked, asked for no decision, holding no grant.

**Stories:**
- S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold.

## Boundaries

- Only the files listed in this brief's requirements change; a row that needs a file outside its wall stops and names it, and Waffles approves a brief revision before that file is edited (CN9).
- Development isolation: disposable test identities and test provider registrations only; no production tokens, real business sign-in or live Cambium participant migration (CN2).
- No credential, token or key value is written in code, tests, fixtures, logs or documents; tests use generated values that are never real credentials.
- Nothing open for Tom is decided: every decision the design's non-goals record as open stays open.
- A live demonstration to Tom is a verification step a person performs after the row lands, never an acceptance criterion and never claimed by a loop completion (CN5, CN6).
- No row is dispatched until Waffles has reviewed it.
- lys-core, its cryptographic primitives and its published wire formats are unchanged.
- SpiceDB is asked for no permission decision and holds no grant in this row (R4).
- No Python or shell provisioning engine.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0.
- From the repository root: rg -n 'ID001_DEPLOY|ID001_DEPLOY_REFUSAL|ID001_THEME|ID001_SHARED_DB|ID001_PIN_CLONE' crates/lys/tests finds each identifier in a test.
- A search of every file this row touches finds no credential, token or key value.
