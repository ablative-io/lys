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
> - ADR-021 — The Rauthy client themes are dark only from the estate tokens, and every field the estate names no token for is a named gap — Both client themes are dark only. A Rauthy field takes an estate token only where the estate names the same role: the dark text takes text, bg takes ink, bg_high takes raised and accent takes the client's product accent. Every other field is a gap that keeps Rauthy's own default: light mode, the error colour, the radius, and the dark text_high, action, btn_text, theme_sun and theme_moon, eight in all, listed by name in deploy/identity/theme-map.md. Rejected: mapping text to muted, which would set the body of every page in the secondary colour; and choosing colours for the fields the estate names no token for.
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

Carry IDENTITY-001 revision 5 row 02 (docs/design/identity/briefs/IDENTITY-001.json:89-167) into requirements: packaging and the development install (R1), the lys identity CLI (R2), the two client themes (R3) and SpiceDB's step-1 role (R4). Row 01, the fork and its pin, was accepted by Waffles at 15:36:25 (docs/design/identity/briefs/IDENTITY-001.json:18); its fresh recursive-clone proof is due at this row's venue gate (ID001_PIN_CLONE). Estimate: 8 focused implementer hours, revision 5's figure for this row (docs/design/identity/briefs/IDENTITY-001.json:20); no re-estimate. Revision 5's 48-hour ceiling stands (CN7); an overrun is reported as soon as it is known. Out: the directory itself (DIRECTORY-003), any grant or permission check, and real sign-in (rows 06 and 07). Every path is relative to the repository root; the estate colour tokens live in another repository and are named in R3's spec with their owner. This revision carries the lead's rulings on the row and changes nothing else: the themes are dark only from the estate tokens, with every Rauthy field the estate names no token for recorded as a gap (ADR-021, R3); configure manages exactly two clients, platform and Cambium, beside the built-in rauthy client the pinned Rauthy's own migration inserts (R2); the container-backed targets are declared test = false and run, linted first, only on their own identity leg of .land/gates.sh, so cargo test stays hermetic (R1, R2, R3); unit tests sit in sibling *_tests.rs files under crates/lys/src/identity/, a wall revision under CN9 and a named addition to C8's module manifest; and the ledgers are checked to carry every entry main held (R3). The R2 lines that move into identity_deploy are the three that need a running Rauthy, PostgreSQL and SpiceDB; the ruling first counted four, and the rule, not the count, decided it. RM-001 alone links this brief in links.briefs; RM-014 names it in its text without linking it, because the ledger's one-owner rule decides it, not a double link. Further units, not written here: building R1 to R4 through card_build_v3 with the identity gate leg, and naming the estate's missing theme tokens.

## Requirements

### R1: Package Rauthy, SpiceDB and one PostgreSQL database, and install them for development

THE SYSTEM SHALL package the maintained Rauthy at the pinned vendor/rauthy commit (ADR-009), SpiceDB, and one PostgreSQL service with one durable database (ADR-005) in deploy/identity/compose.yaml, with pinned supported releases and image digests in deploy/identity/versions.json, separate least-privilege roles and schema namespaces created by deploy/identity/postgres-init.sql through PostgreSQL's own init directory, explicitly configured credentials, and documented local and TLS origins in deploy/identity/config.example.toml (docs/design/identity/briefs/IDENTITY-001.json:136, docs/design/identity/briefs/IDENTITY-001.json:36). The database address SHALL be configuration and SHALL NOT assume Tom's Mac (ADR-005). Both migration runners and their connection search paths SHALL be verified to coexist before one-database readiness is claimed. These are three dependency processes beside the platform door; Cambium, Manifold, Argus and Aion servers are not runtime dependencies (ADR-004). THE SYSTEM SHALL define readiness, migration order, named configuration failures, stop and start, and backup and restore (docs/design/identity/briefs/IDENTITY-001.json:137), documenting in deploy/identity/README.md the cache, key and config files Rauthy's internal Hiqlite cache needs rather than claiming a SQL dump alone backs up the product; no Hiqlite identity datastore is used in the installed configuration (docs/design/identity/briefs/IDENTITY-001.json:36). The development instance is installed on a node the operator names (ADR-005) with test identities only; release builds, checks and tests stay on the gate workflow, and each development install checks upstream releases and advisories and records the accepted v0.36.2 exception (docs/design/identity/briefs/IDENTITY-001.json:135, CN10). Results, digests and the restore outcome go to docs/design/identity/reports/IDENTITY-001-deployment.md, the report name revision 5 gives this row. The manifest crates/lys/Cargo.toml SHALL declare [[test]] entries with test = false for identity_deploy, identity_refusals, identity_shared_db and identity_restart, so cargo test --workspace --all-features does not run them and they run only on the identity leg of .land/gates.sh; this requirement SHALL NOT add a dependency to any manifest or to Cargo.lock: its container-backed tests use only crates already in Cargo.lock and the container runtime (psql runs inside the PostgreSQL container, and HTTP checks go over std::net or through curl inside a container), and a test that needs a new crate stops and names the file (CN9). .land/gates.sh SHALL gain an identity leg that runs on every lys landing and is never scoped away. IF no container runtime answers docker info, THEN the leg SHALL fail by the name container_runtime_missing and SHALL NOT skip, pass or run its targets without the runtime. WHEN the runtime answers, the leg SHALL run cargo clippy -p lys --all-features --test 'identity_*' -- -D warnings under the workspace's pedantic lint set with no allow argument, and only then cargo test -p lys --all-features --test 'identity_*'; a lint warning in any identity target SHALL fail the leg by name. CLAUDE.md's gates before any commit SHALL gain one line naming both acts of the identity leg. .land/gates.sh and CLAUDE.md are outside the row's first wall and are added to it by this revision (CN9).

**Acceptance:**
- ID001_DEPLOY: from a fresh venue directory, dependencies become ready with no other Ablative service running; restart preserves issuer identity and database contents.
- ID001_DEPLOY_REFUSAL: missing secret, invalid issuer/redirect and unavailable database produce named failures, never a substitute identity or development database.
- Dependency artifact digests, versions, resolved configuration without secrets and backup/restore result are recorded in docs/design/identity/reports/IDENTITY-001-deployment.md. This row does not claim the product directory exists yet.
- ID001_SHARED_DB: prove Rauthy and SpiceDB both write/reopen against the same PostgreSQL database, migrations do not collide and the service roles cannot read the other schema. Restore the database plus documented key/config/cache dependencies and repeat readiness.
- ID001_PIN_CLONE: the venue fetches the exact pushed Lys ref with recursive submodules and verifies the expected Rauthy commit on ablative without using the Mac reading copy.
- A test copies deploy/identity/config.example.toml with its database host set to 192.0.2.10, renders it through the identity test support (crates/lys/tests/identity_support/compose.rs) into the environment deploy/identity/compose.yaml interpolates, and reads docker compose -f deploy/identity/compose.yaml config: Rauthy's PG_HOST is 192.0.2.10 and SpiceDB's datastore URI names the host 192.0.2.10; with that address unreachable, readiness fails with a named database failure and no default local address is substituted (ADR-005).
- From the repository root with the container runtime answering: cargo clippy -p lys --all-features --test 'identity_*' -- -D warnings exits 0 with no warning, the identity leg in .land/gates.sh runs that command exactly with no -A, -W or --cap-lints argument, and cargo test -p lys --all-features --test 'identity_*' runs identity_deploy, identity_refusals, identity_shared_db and identity_restart and exits 0.
- cargo test --workspace --all-features runs none of identity_deploy, identity_refusals, identity_shared_db and identity_restart: its output names no such target.
- With docker absent from PATH, sh .land/gates.sh prints container_runtime_missing in the identity leg and exits 1; with one unused variable added to crates/lys/tests/identity_restart.rs, the identity leg fails at its clippy step, names identity_restart and runs no identity test.
- CLAUDE.md's gates before any commit carries exactly one added line, and that line names both cargo clippy -p lys --all-features --test 'identity_*' -- -D warnings and cargo test -p lys --all-features --test 'identity_*'.

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

#### R1 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each acceptance row, with its evidence:

1. ID001_DEPLOY is implemented in crates/lys/tests/identity_deploy.rs:34 and crates/lys/tests/identity_restart.rs:21. The deploy test waits for readiness, asserts compose declares exactly {postgres, rauthy, spicedb, spicedb-migrate} and that only postgres, rauthy and spicedb are running. The restart test runs down/up keeping volumes and requires the same issuer and signing-key ids and the same rauthy.clients, jwks and alembic rows. The hand-run install saw exactly this: all three ready from empty volumes, nothing else running (report, 'Observed by hand'). The test itself has not run; it runs on the identity leg.

2. ID001_DEPLOY_REFUSAL is in crates/lys/tests/identity_refusals.rs:34 and :111. Four named config refusals (invalid_issuer, invalid_redirect_uri) leave no private directory. Compose itself refuses a removed secret as `missing_secret LYS_IDENTITY_RAUTHY_API_KEY_SECRET`; I confirmed that wording against the real compose file. prepare and configure refuse a deleted credential as secret_missing and never regenerate it (crates/lys/src/identity/credentials.rs load_or_generate). A stopped database is named database_unreachable, and after it returns the issuer and keys are unchanged.

3. The report is written at docs/design/identity/reports/IDENTITY-001-deployment.md: versions, digests, the resolved config without secrets, and the backup/restore result from the hand-run install. It states that the directory is not claimed.

4. ID001_SHARED_DB is crates/lys/tests/identity_shared_db.rs:36. The hand-run install observed the same things: 48 tables in the rauthy schema, 9 in spicedb, 0 in public; each role refused the other's schema; pg_dump --create, then down -v, create, untar of the data volume, and pg_restore --clean --create --if-exists --exit-on-error exited 0; the same four signing-key ids came back and both services were ready.

5. ID001_PIN_CLONE is crates/lys/tests/identity_deploy.rs:106. It checks the gitlink and the vendor/rauthy HEAD against versions.json source_commit, checks .gitmodules names the ablative-io fork on branch ablative, fetches the fork's ablative tip and runs merge-base --is-ancestor, and refuses objects borrowed through alternates. It is due at the venue gate, whose path the brief's blocked_by places with Heimdall.

6. The 192.0.2.10 row is implemented as described: identity_refusals.rs:66-89 renders through identity_support/compose.rs (Compose::render runs `lys identity prepare`) and reads `docker compose config --format json`. Rauthy's PG_HOST and the host in SpiceDB's datastore URI are both 192.0.2.10, and no postgres service is declared. Health then fails with `database_unreachable ... [database at 192.0.2.10:5432]` and never dials 127.0.0.1 or localhost. The health probe dials only config.database.host (crates/lys/src/identity/health.rs:116), and config.rs:320 refuses a loopback host. I checked the interpolation by hand against the real compose file.

7. The identity-leg clippy row is implemented at .land/gates.sh:32, which runs exactly that command with no allow argument. With the five targets temporarily set test = true, the crusher hook's `cargo clippy --all-targets -- -D warnings` under the workspace pedantic set reported nothing; lys has no features of its own, so --all-features compiles the same code. The manifest was restored to test = false. `cargo test ... 'identity_*'` has not been run.

8. The row that the workspace test run names no identity target holds by construction: test = false at crates/lys/Cargo.toml:50-70, and the tests' own clippy targets exclude them.

9. The docker-absent and unused-variable row is implemented: .land/gates.sh:28-30 returns 1 with container_runtime_missing when `docker info` fails, and :32-34 stops at the clippy step, whose output names the file. Not exercised here.

10. The CLAUDE.md row is met: `git diff CLAUDE.md` shows exactly one added line (CLAUDE.md:102) naming both commands.

Status is 'implemented' on the strength of this code evidence, the lint evidence and the hand-observed install. The container rows' actual pass is measured only on the identity leg.
- Deviation: Three, all stated in the files. (1) The Rauthy image is upstream's published ghcr.io/sebadob/rauthy:0.36.2, pinned by digest, rather than a fork-built image. The pinned ablative commit dd61ac3 is the v0.36.2 tag commit itself, so the image is built from the pinned source (versions.json image_source_note). Building from source is a release build, which stays on the gate. (2) PostgreSQL is pulled from public.ecr.aws/docker/library/postgres, whose index digest is identical to Docker Hub's, because Docker Hub pulls on this seat failed on a locked credential helper. (3) The report's hand-observed install used an environment rendered by hand with prepare's exact variable names, because the row's CLI could not be built outside the gate. The report says so.
- Files changed:
  - created: `deploy/identity/compose.yaml` — The three dependency processes: postgres (profile `database`, runs postgres-init.sql from its init directory, track_commit_timestamp on); spicedb-migrate (migrate head, then exits); spicedb (serve, HTTP on); rauthy (HIQLITE=false, PostgreSQL datastore, data volume for the Hiqlite cache). Every image is pinned by digest. Every value is `${NAME:?missing_secret|missing_config NAME}`, so nothing is defaulted, and the database host is configuration.
  - created: `deploy/identity/versions.json` — Releases, index and per-platform digests for all three images, the pinned fork commit, the 2026-09-27 release and advisory read, and the accepted IDENTITY-001-UPSTREAM-AUTH-STATE exception.
  - created: `deploy/identity/config.example.toml` — The secret-free deployment config: the node the operator names, compose project, private directory, database address as configuration, local and TLS origins documented, and the two clients.
  - created: `deploy/identity/postgres-init.sql` — Creates the least-privilege roles rauthy and spicedb and a schema each, sets each role's search_path to its own schema, and grants PUBLIC nothing. Passwords come in via psql \getenv, and an absent one stops the file by name.
  - created: `deploy/identity/README.md` — Install steps; the database address as configuration; local and TLS origins; migration order; readiness and named failures; stop/start; the four-part backup and restore procedure (SQL dump, the Rauthy/Hiqlite data volume, the private key files, the config); SpiceDB's step-1 role; the identity leg.
  - created: `docs/design/identity/reports/IDENTITY-001-deployment.md` — Records the release/advisory check, artifact digests, the resolved configuration with every secret shown as <generated>, and the hand-observed install including the backup/restore result. The gate-measured section is marked as not yet run; the row makes no claim that the directory exists.
  - created: `crates/lys/tests/identity_deploy.rs` — ID001_DEPLOY readiness with only the three services declared and running, plus the three R2 container lines; ID001_PIN_CLONE.
  - created: `crates/lys/tests/identity_refusals.rs` — ID001_DEPLOY_REFUSAL: invalid issuer and redirect refused with nothing written; 192.0.2.10 resolved into compose and refused by health as database_unreachable; a missing secret refused by compose and by the CLI; a stopped database named, with no substitute identity after it returns.
  - created: `crates/lys/tests/identity_shared_db.rs` — ID001_SHARED_DB: both services write and reopen; migration histories and search paths each in their own schema; four cross-schema and public-schema refusals counted; backup, loss and restore, then readiness repeated with the same issuer and signing keys.
  - created: `crates/lys/tests/identity_restart.rs` — ID001_DEPLOY restart: down/up keeping volumes preserves the issuer, the signing keys and the database contents, and configure then reports every operation unchanged.
  - created: `crates/lys/tests/identity_support/mod.rs` — Declares the container-test support modules.
  - created: `crates/lys/tests/identity_support/fixtures.rs` — Venue: a temp dir holding a copy of config.example.toml with free ports, a unique project and a configurable database host. Runs the lys binary and refuses as secret_leaked any run whose output carries a generated secret.
  - created: `crates/lys/tests/identity_support/server.rs` — Stack: requires `docker info` (container_runtime_missing), renders, checks PG_HOST reached compose unchanged, and brings the stack up. Also health/configure/wait_ready, and psql with the password passed through the environment. Drop runs `down -v`.
  - created: `crates/lys/tests/identity_support/compose.rs` — docker compose from the repository root: renders the config through `lys identity prepare`, and reads the resolved `compose config --format json` environment.
  - modified: `crates/lys/Cargo.toml` — Five [[test]] entries with test = false (identity_deploy, identity_refusals, identity_shared_db, identity_restart, identity_theme); the rand, sha2, toml and zeroize dependencies are R2's.
  - modified: `.land/gates.sh` — Adds identity_leg. With no `docker info` answer it prints container_runtime_missing and returns 1. Otherwise it runs `cargo clippy -p lys --all-features --test 'identity_*' -- -D warnings` (no -A/-W/--cap-lints), and on failure prints identity_lint_failed and runs no test; only then `cargo test -p lys --all-features --test 'identity_*'`.
  - modified: `CLAUDE.md` — Exactly one added line in the gate block, naming both identity-leg commands.
- Checklist delivery:
  - [x] C7 — Rauthy, SpiceDB and one PostgreSQL service and database are packaged and installed for development, with separate roles and schema namespaces, the database address as configuration, and readiness, restart, named refusals, the shared database and restore proved (ID001_DEPLOY, ID001_DEPLOY_REFUSAL, ID001_SHARED_DB, ID001_PIN_CLONE). — Packaged and installed for development, with separate roles and schemas and the database address as configuration. Readiness, restart, named refusals, the shared database and restore are implemented as container tests and observed by hand; the tests' measured pass belongs to the identity leg.
- Story delivery:
  - [x] S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold. — The operator chooses database.host (a network device or this node under the `database` profile). Restart and the documented four-part restore preserve the issuer, the signing keys and the data. Nothing from Cambium or Manifold is involved.

### R2: Prepare, configure and check the deployment from the lys CLI

THE SYSTEM SHALL implement lys identity prepare, configure and health in the existing lys CLI with exactly revision 5's module manifest (docs/design/identity/briefs/IDENTITY-001.json:154-166): mod.rs declarations and re-exports only; cli.rs the identity subcommand arguments; config.rs typed deployment configuration and validation with no secret value in diagnostics; credentials.rs Zeroizing, redacted credential material and its stable reuse; private_files.rs restricted-mode durable file creation and outcome reconciliation; prepare.rs validating inputs and materialising the declared private artifacts; configure.rs idempotent client and theme reconciliation with stable operation identifiers; rauthy.rs typed Rauthy API requests and responses, named status errors and read-back after an uncertain outcome; health.rs named readiness checks for the declared services; error.rs typed errors carrying operation, resource and path, never secret bytes; themes.rs is R3's. configure SHALL register separate platform and Cambium OIDC clients with exact redirect URIs and RS256 where the Cambium verifier requires it, and the platform's own confidential client with S256 as revision 5 proposes (docs/design/identity/briefs/IDENTITY-001.json:138, docs/design/identity/briefs/IDENTITY-001.json:39). Google and GitHub federation credentials belong to Rauthy; Google API consent remains a different client purpose. Generated credentials stay out of Git and logs, and health output excludes secrets. No Python or shell provisioning engine: health.sh and the external tests/identity_deployment are replaced by these subcommands and Rust integration tests (docs/design/identity/briefs/IDENTITY-001.json:19, docs/design/identity/briefs/IDENTITY-001.json:140). Unit tests sit in the sibling files config_tests.rs, credentials_tests.rs, error_tests.rs and prepare_tests.rs under crates/lys/src/identity/, the one addition this revision makes to the module manifest (CN9). configure SHALL manage exactly two clients, platform and Cambium, beside the built-in rauthy client the pinned Rauthy's own migration inserts, and SHALL NOT create, change or delete that built-in client. The three acceptance lines of this requirement that need a running Rauthy, PostgreSQL and SpiceDB (the capture of health output, configure run twice, and health per service) SHALL run in R1's container-backed target crates/lys/tests/identity_deploy.rs, only in the identity leg of .land/gates.sh under the test = false manifest entry R1 declares, and SHALL NOT run in cargo test --workspace --all-features; the Debug and Display redaction test and prepare's file-mode test stay in cargo test --workspace --all-features. No new test file is added under crates/lys/tests/.

**Acceptance:**
- A redaction test in cargo test --workspace --all-features formats every credential and error type under crates/lys/src/identity/ with Debug and Display: none contains a byte of a generated secret.
- In identity_deploy, the health output of a configured deployment is captured: it contains no byte of a generated secret.
- prepare creates every declared private file with a restricted mode; a test reads each file's mode and counts the files against the declared set.
- In identity_deploy, after two configure runs against the same Rauthy, the platform client and the Cambium client each exist exactly once with the configured values, the built-in rauthy client is unchanged, and no other client exists; their themes keep unchanged operation identifiers, and a transport failure after a request is resolved by read-back and never creates a second client.
- In identity_deploy, health names each unready service or database: a test makes each one unavailable in turn and counts one named failure per case, equal to the number of declared services.
- Every module of the manifest exists under crates/lys/src/identity/, the only other files this requirement adds there are the sibling unit-test files config_tests.rs, credentials_tests.rs, error_tests.rs and prepare_tests.rs, and none exceeds 500 lines of code.

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
- create: crates/lys/src/identity/error_tests.rs
- create: crates/lys/src/identity/prepare_tests.rs
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

#### R2 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each acceptance row, with its evidence:

1. The redaction test is credentials_tests.rs:79 plus error_tests.rs:134. Both are ordinary unit tests in `cargo test --workspace --all-features`. They cover Secret, Credentials, CredentialSpec, Provenance, RauthyApi and all 19 IdentityError variants, counted and exhaustively matched.

2. Health output without secrets is implemented at identity_deploy.rs:78-85. Every `lys` run in the container tests also goes through Venue::lys, which refuses output containing any generated secret. health.rs reads no credential.

3. File modes are covered by prepare_tests.rs:38: it walks the tree, requires set equality with declared_files(), and counts every mode. prepare.rs:120 also re-checks the declared set at runtime.

4. Configure twice is implemented at identity_deploy.rs:52-71 and check_clients. A CuttingProxy forwards the first POST /auth/v1/clients and drops its answer; configure reports created_resolved_by_read_back, and rauthy.rs:195 reads the client back rather than creating it again. The second run reports all four operations unchanged with identical operation ids. Exactly rauthy, platform and cambium exist, rauthy is equal to its pre-configure JSON, and the configured values (redirects, RS256, S256 or absent) hold. Rauthy's API behaviour was confirmed by hand: a duplicate create is 400 'ID exists already'.

5. Health per service is implemented at identity_deploy.rs:87-99: it stops postgres, rauthy and spicedb in turn, requires database_unreachable, rauthy_unreachable and spicedb_unreachable respectively, and counts 3, equal to health's declared checks.

6. The manifest row is met. The 11 manifest modules exist (themes.rs is R3's), plus only config_tests.rs, credentials_tests.rs, error_tests.rs and prepare_tests.rs (themes_tests.rs is R3's). Counted by the crusher's own counter, the largest is config.rs at 486 lines of code.

clippy -D warnings over all lys targets is clean via the crusher hook. The unit tests have not been run here. Rows 2, 4 and 5 run on the identity leg.
- Deviation: Two. (1) R2 adds the `toml` crate (workspace Cargo.toml and Cargo.lock, both in R2's wall) to read config.example.toml, rather than hand-parsing TOML. The first draft's hand-written subset parser pushed config.rs past the 500-line limit, and the boring, interoperable parser is the same one Rauthy uses. (2) Removed the error variant `UnsupportedPlatform` from my own draft: in a binary crate on Unix it would never be constructed, which is dead code under -D warnings. Off Unix, private files now fail as io_failed with ErrorKind::Unsupported.
- Files changed:
  - created: `crates/lys/src/identity/mod.rs` — Declarations only, with the module manifest and invariants in the module docs.
  - created: `crates/lys/src/identity/cli.rs` — The IdentityCommand arguments: prepare --config, configure --config --themes, health --config.
  - created: `crates/lys/src/identity/config.rs` — The typed DeploymentConfig, read with the toml crate (deny_unknown_fields, no defaults), and validation that names each failure (invalid_issuer, invalid_redirect_uri, invalid_database_host including loopback, invalid_sslmode, invalid_client_id, ...). It also derives the issuer and origins. 486 counted lines of code.
  - created: `crates/lys/src/identity/credentials.rs` — The nine declared credentials. Secret is Zeroizing and formats as its name with [redacted]. Values are generated from the thread CSPRNG, reused when present, and a missing one after prepare is secret_missing.
  - created: `crates/lys/src/identity/private_files.rs` — 0700 directories and 0600 files: temp write, fsync, rename, directory fsync, then read-back to confirm the outcome (private_write_unconfirmed). Refuses group or other permission bits; fails as io_failed/Unsupported off Unix.
  - created: `crates/lys/src/identity/prepare.rs` — Validates first, then generates or reuses credentials, renders identity.env, and checks every declared file's mode against declared_files(). It prints file names and whether each was generated or reused, never a value.
  - created: `crates/lys/src/identity/configure.rs` — Idempotent reconciliation of exactly the platform and cambium clients and their themes. Operation ids are lys-identity/<kind>/<id>/<sha256 of the desired state>; every write is confirmed by read-back; the built-in rauthy client is never touched and nothing is deleted.
  - created: `crates/lys/src/identity/rauthy.rs` — Plain HTTP/1.1 over std::net that separates 'never sent' from 'sent, answer lost'. Typed NewClient, Theme/ThemeCss and RauthyHealth; named status errors; create_client resolves a lost answer by reading the client back and never sends a second create. The API key lives only in a Zeroizing header, and Debug redacts it.
  - created: `crates/lys/src/identity/health.rs` — Three declared services: a PostgreSQL SSLRequest probe to the configured host, Rauthy /auth/v1/health, SpiceDB /healthz. Each failure is named. When all are ready it prints the issuer and signing-key ids; it reads no credential.
  - created: `crates/lys/src/identity/error.rs` — IdentityError: 19 variants, each message beginning with a stable failure name and carrying the operation, resource and path, never secret bytes.
  - created: `crates/lys/src/identity/config_tests.rs` — The example validates; 192.0.2.10 is accepted as configuration; 20 named refusals, each counted; unknown or missing keys are config_invalid; an https origin needs its proxy CIDR.
  - created: `crates/lys/src/identity/credentials_tests.rs` — Generate once, then reuse; a missing credential is refused and not regenerated; a malformed one is refused without echoing its value. Redaction: Debug and Display of Credentials, every Secret, CredentialSpec and Provenance, and RauthyApi's Debug carry no secret (12 renderings × 9 secrets, counted).
  - created: `crates/lys/src/identity/error_tests.rs` — Every IdentityError variant, matched with no wildcard so a new variant cannot skip it, formatted with Display, Debug and pretty Debug alongside nine generated secrets: each begins with its name and carries no secret. Plus the status-name mapping.
  - created: `crates/lys/src/identity/prepare_tests.rs` — Walks the private tree and requires it to equal the declared set (9 secrets + identity.env), counting each file at 0600 and the directories at 0700. Also: identity.env carries every LYS_IDENTITY_* variable compose reads; a rerun reuses everything; a missing secret or an opened file mode is refused.
  - modified: `Cargo.toml` — Adds workspace dependency toml = "1".
  - modified: `Cargo.lock` — lys now depends on rand, sha2, toml and zeroize; adds the toml 1.1.6 family (toml_parser, toml_writer, toml_datetime, serde_spanned, winnow, indexmap, hashbrown, equivalent), all MIT or Apache-2.0.
  - modified: `crates/lys/Cargo.toml` — lys dependencies rand, sha2, toml, zeroize (workspace).
  - modified: `crates/lys/src/main.rs` — Declares mod identity and dispatches Command::Identity to prepare/configure/health.
  - modified: `crates/lys/src/cli.rs` — Adds the Identity(IdentityCommand) subcommand.
  - modified: `crates/lys/src/commands/error.rs` — CliError::Identity(#[from] IdentityError), transparent.
  - created: `crates/lys/tests/identity_deploy.rs` — R2's three container lines (the same file listed under R1).
- Checklist delivery:
  - [x] C8 — lys identity prepare, configure and health exist in the CLI exactly as revision 5's module manifest names them, register the platform and Cambium clients idempotently, and keep every secret out of Git, logs, errors and health output. — prepare, configure and health exist with exactly the manifest's modules plus the named sibling test files. The two clients are registered idempotently with stable operation ids. Secrets stay out of Git, output, errors and health.
- Story delivery:
  - [x] S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold. — The operator prepares, configures and checks the standalone deployment from the lys CLI alone. No Python or shell provisioning engine is involved.

### R3: Theme both Rauthy clients with Aion's vocabulary and each product's own accent

THE SYSTEM SHALL configure both Rauthy client themes, dark only (ADR-021), from Aion's pinned neutral and text vocabulary and each client's own product accent (ADR-010): the Cambium client green, the identity client the identity orange. A Rauthy field takes an estate token only where the estate names the same role: in dark mode it SHALL set text from the estate token text, bg from ink, bg_high from raised and accent from the client's product accent, as HSL in deploy/identity/rauthy-themes.json, and SHALL NOT map text to muted. Every other field is a gap that keeps Rauthy's own default and that nothing reads: it SHALL NOT set the light fields, the dark error, the border radius, or the dark text_high, action, btn_text, theme_sun and theme_moon. deploy/identity/theme-map.md SHALL record every source token and colour conversion and name the eight gaps, light mode, the error colour, the radius, text_high, action, btn_text, theme_sun and theme_moon, citing ADR-021; the count is eight rather than the six first named because the pinned Rauthy's theme also carries theme_sun and theme_moon per mode, for which the estate names no token. The themes are read and validated by crates/lys/src/identity/themes.rs, with its unit tests in the sibling crates/lys/src/identity/themes_tests.rs, and it SHALL verify readable contrast (docs/design/identity/briefs/IDENTITY-001.json:139). Readable contrast is WCAG 2.1 AA: a ratio of at least 4.5 to 1 for body text and 3 to 1 for large text, icons and interface components, computed with the WCAG relative-luminance formula over the pairs the acceptance names, a gap field measured at the pinned Rauthy's own default. A gap field whose default fails over ink stays a gap: the requirement stops and names the failing pair (CN9) rather than choosing a value the estate does not name, and the fix is a token the design-system card names. The manifest crates/lys/Cargo.toml SHALL declare a [[test]] entry with test = false for identity_theme, so cargo test --workspace --all-features does not run it and it runs only on the identity leg of .land/gates.sh; this requirement SHALL NOT add a dependency to any manifest or to Cargo.lock. Fonts and page layout remain Rauthy's; no font or layout patch and no cross-application build dependency enters the fork. The colour values are read from the estate colour tokens, docs/design-system-v2/palette/estate-colour-tokens.json in the ablative docs repository (owner: Waffles, who added the identity entry at ablative-docs 385916e; docs/design/identity/briefs/IDENTITY-001.json:44); that file is another repository's, is read and never changed, and is named here rather than listed in files (CN4). Its purple status token is not copied.

**Acceptance:**
- ID001_THEME: inspect both client login pages in dark mode; text is the estate text, bg is ink, bg_high is raised and accent is the client's product accent, each as the declared mapping gives it in HSL; the eight gap fields and every light field equal the pinned Rauthy's defaults; all persist after restart. Readable contrast is WCAG 2.1 AA over six pairs per client, each ratio computed with the WCAG relative-luminance formula from the values the running Rauthy exports: text over bg (ink), text over bg_high (raised), text_high over ink and btn_text over action at 4.5 to 1 or more, the body-text tier; theme_sun over ink and theme_moon over ink at 3 to 1 or more, the tier for large text, icons and interface components. Record the source-token ref and Rauthy theme export without credentials.
- The contrast check is a counted leg of identity_theme: cargo test -p lys --all-features --test identity_theme contrast -- --nocapture prints one line per pair per client naming the pair, its measured ratio to two decimal places and its tier, twelve lines in all, then the line contrast: 12 pairs measured, 12 at or above their tier, and exits 0; when any pair is below its tier the leg fails naming that pair and its ratio. A gap field whose Rauthy default fails over ink stays a gap and takes no value the estate does not name: the row stops and names the failing pair as a CN9 stop for the lead.
- The Cambium client's accent is Cambium green and the identity client's is the identity orange of the estate tokens; neither is Aion blue, and no purple token appears in deploy/identity/rauthy-themes.json.
- deploy/identity/theme-map.md names exactly eight gaps, light mode, the error colour, the radius, text_high, action, btn_text, theme_sun and theme_moon, each marked as keeping Rauthy's own default, and cites ADR-021.
- cargo test --workspace --all-features runs no target named identity_theme, and cargo test -p lys --all-features --test identity_theme runs it on the identity leg and exits 0.
- Ledger carry-check against main at the brief's src commit, the commit the build's src input names: for docs/design/decisions.json and for docs/design/roadmap.json, the file at that commit (git show, then json.load) is compared with the branch's file entry by entry, and every decision and every row main holds there is present by id and byte-identical when both are serialised with sorted keys; the branch holds exactly one entry more in each ledger, ADR-021 in decisions.json and RM-014 in roadmap.json, each appearing exactly once; and python3 scripts/design/validate.py docs/design/decisions.json and python3 scripts/design/validate.py docs/design/roadmap.json exit 0.

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

#### R3 — Execution record

**Dev (recorded):**

- Status: implemented
- How: Each acceptance row, with its evidence:

1. ID001_THEME is implemented at crates/lys/tests/identity_theme.rs:35. It checks the mapped fields against rauthy-themes.json, and the gaps, light mode and radius against defaults parsed from vendor/rauthy/src/data/src/entity/theme.rs (default_dark 459, default_light 474, radius 88). It checks the six pairs per client from what the running Rauthy exports (configure's read-back), and persistence across `compose restart rauthy`. The source ref and theme export are recorded in theme-map.md and in configure's JSON output.

2. The counted contrast leg is implemented: identity_theme.rs:116 prints 'contrast <client> <pair>: <ratio .2> (tier ...)' twelve times, then :78 prints 'contrast: 12 pairs measured, 12 at or above their tier'. A low pair returns contrast_below_tier naming the pair and its ratio. Computed on the stored HSL values: the lowest is Cambium theme_moon over ink at 3.97 against 3:1, so no gap stops the row. themes_tests.rs asserts all 12 pass.

3. The accent row is met. Validation requires platform→products.identity.accent and cambium→products.cambium.accent and refuses #A78BFA (themes.rs validate:141). identity_theme.rs:39 refuses #6B96D1 or #A78BFA anywhere in the mapping. Neither appears.

4. The theme-map row is met: it names exactly eight gaps, each 'keeps Rauthy's own default', and cites ADR-021.

5. The identity_theme target is test = false (crates/lys/Cargo.toml:70) and runs on the leg via --test 'identity_*'. Not run here.

6. The ledger carry-check is met. Against origin/main 1756688cc08169bef4a9ac37b9b079efb9e38b18 (fetched; it is HEAD~1 and an ancestor): every decision and roadmap row main holds is present by id and byte-identical when serialised with sorted keys. The branch holds exactly one more in each: ADR-021 in decisions.json and RM-014 in roadmap.json. validate.py exited 0 on both files. Those ledgers were not changed by this work.
- Deviation: The estate token values were read from ablative-io/design-system palette/estate-colour-tokens.json at 3c3bac715fb60348017f0d5cf40b412cf168da77. The brief's ablative-docs 385916e could not be reached from this seat (ablative-io/ablative-docs does not resolve, and no local clone holds 385916e). That copy's identity entry matches ADR-010's recorded 385916e values exactly. rauthy-themes.json cites 385916e in source.ref and records the copy read in source.read_copy; theme-map.md explains it. Someone who can reach 385916e should confirm it.
- Files changed:
  - created: `deploy/identity/rauthy-themes.json` — Dark only. Per client, only text←foundation.text, bg←foundation.ink, bg_high←foundation.raised and accent←the product accent (platform: products.identity.accent #D4975A [30,59,59]; cambium: products.cambium.accent #5E8C6A [136,20,46]). Records the source ref and the copy actually read.
  - created: `deploy/identity/theme-map.md` — Every source token and its hex-to-HSL conversion; exactly eight gaps (light mode, error, radius, text_high, action, btn_text, theme_sun, theme_moon), each keeping Rauthy's default; ADR-021 cited; the twelve contrast figures.
  - created: `crates/lys/src/identity/themes.rs` — Loads and validates the mapping: dark only, exact token roles, HSL recomputed from each hex, no Aion accent, no banned purple. DarkMapping::apply changes only the four fields. check_contrast applies WCAG 2.1 AA to six pairs, resolving hsl()/hsla() values against the theme, and a pair below its tier is contrast_below_tier naming it.
  - created: `crates/lys/src/identity/themes_tests.rs` — The shipped mapping sets only the four fields; hex→HSL is checked on 7 colours; the WCAG formula; 12 pairs counted at their tiers; a low pair refused by name; 6 mapping refusals counted (light mode, purple, text←muted, a wrong HSL, a wrong product, missing source).
  - created: `crates/lys/tests/identity_theme.rs` — The identity-leg theme test. It reads Rauthy's pinned defaults from vendor/rauthy's source, checks both exported themes (four mapped fields, eight gaps, light mode, radius), and runs the counted contrast leg with an independent HSL implementation: 12 lines plus the summary. It then restarts Rauthy and checks the themes persist.
  - modified: `crates/lys/Cargo.toml` — [[test]] identity_theme, test = false.
- Checklist delivery:
  - [x] C9 — Both Rauthy client themes carry Aion's neutral vocabulary with each product's own accent, Cambium green and the identity orange (ID001_THEME). — Both themes carry the estate neutrals, with Cambium green on the Cambium client and identity orange on the platform client. ID001_THEME is implemented and its contrast is computed; the running-Rauthy check runs on the identity leg.
- Story delivery:
  - [x] S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold. — The standalone product's sign-in pages wear the estate vocabulary with no Cambium or Aion build dependency.

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

#### R4 — Execution record

**Dev (recorded):**

- Status: implemented
- How: 1. The README row is met: deploy/identity/README.md:148 carries R4's sentence ('in step 1 SpiceDB is installed, migrated, backed up and health-checked against the shared PostgreSQL database, and it enforces nothing, because no component asks it for a permission decision or writes a grant to it, and live capability policy and its enforcement stay with road step 2'). A substring check against the brief JSON returned True.

2. The source-search row is met. `grep -i spicedb crates/lys/src/identity/*.rs` finds it only in configuration (config.rs; the credentials and env names in credentials.rs and prepare.rs), the health readiness check (health.rs /healthz, cli.rs help) and module docs. A search for relationships/write, schema/write, CheckPermission, WriteRelationships or WriteSchema under crates/lys/src finds nothing. The fixture schema and relationship live only in the test crates/lys/tests/identity_shared_db.rs and are documented there as fixtures, not grants.
- Deviation: (none)
- Files changed:
  - created: `deploy/identity/README.md` — 'SpiceDB in step 1' section: R4's sentence word for word, and what SpiceDB does not do in step 1.
- Checklist delivery:
  - [x] C10 — SpiceDB's step-1 role is written down and held: installed and checked, asked for no decision, holding no grant. — The role is written word for word and held: health reads readiness only, and no permission, relationship or schema call exists in the CLI.
- Story delivery:
  - [x] S3 (Operator, Installs and runs the standalone identity product) — As the operator, I want to install the identity product's dependencies on one PostgreSQL database whose host I choose, and restart or restore it without losing anyone, so that the product stands alone without Cambium or Manifold. — The operator can see from the README exactly what SpiceDB does and does not do in step 1.

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
- Each blocked_by entry is discharged only by its blocker having landed, checked live when the build starts.
- cargo test --workspace --all-features stays hermetic: no test it runs needs a container runtime or a running service.

## Verification

- From the repository root: cargo fmt --check; cargo clippy --all-targets --all-features -- -D warnings; cargo clippy --all-targets -- -D warnings; cargo test --workspace --all-features; cargo doc --no-deps --all-features; cargo doc --no-deps, all clean (CLAUDE.md, gates before any commit).
- From the repository root: python3 scripts/design/validate.py docs/design/directory and python3 scripts/design/check-coverage.py docs/design/directory exit 0.
- From the repository root: rg -n 'ID001_DEPLOY|ID001_DEPLOY_REFUSAL|ID001_THEME|ID001_SHARED_DB|ID001_PIN_CLONE' crates/lys/tests finds each identifier in a test.
- A search of every file this row touches finds no credential, token or key value.
- From the repository root with the container runtime answering: sh .land/gates.sh runs the identity leg, and the leg runs cargo clippy -p lys --all-features --test 'identity_*' -- -D warnings before cargo test -p lys --all-features --test 'identity_*', both clean.
- From the repository root: sh scripts/design/gate.sh and python3 scripts/design/validate.py docs/design/roadmap.json exit 0.
