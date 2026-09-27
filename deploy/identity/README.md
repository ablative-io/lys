# Identity product dependencies: development install

The standalone identity product needs three dependency processes beside the
platform door: the maintained Rauthy (ADR-009), SpiceDB, and one PostgreSQL
service with one durable database (ADR-005). Cambium, Manifold, Argus and Aion
servers are not runtime dependencies (ADR-004). This directory packages the
three for a development install on a node the operator names, with disposable
test identities only. Release builds, checks and tests stay on the gate
workflow; a development install is never a substitute for it.

| File | What it is |
|---|---|
| `deploy/identity/compose.yaml` | The three services and SpiceDB's one-shot migration, images pinned by digest |
| `deploy/identity/versions.json` | Pinned releases, image digests, the Rauthy fork commit, and the release and advisory check |
| `deploy/identity/postgres-init.sql` | The two least-privilege roles and their schemas, run by PostgreSQL's own init directory |
| `deploy/identity/config.example.toml` | The deployment configuration, with the local and TLS origins |
| `deploy/identity/rauthy-themes.json` | The dark theme mapping for both clients (ADR-021) |
| `deploy/identity/theme-map.md` | Every source token, conversion, gap and contrast ratio of that mapping |

Every command below runs from the repository root.

## What SpiceDB does in step 1

SpiceDB's step-1 role: in step 1 SpiceDB is installed, migrated, backed up and health-checked against the shared PostgreSQL database, and it enforces nothing, because no component asks it for a permission decision or writes a grant to it, and live capability policy and its enforcement stay with road step 2.

It answers no check for any identity, it holds no grant or relationship of the
directory, the lifecycle state recorded by DIRECTORY-003 gates nothing through
it, and running it is not permission enforcement. `lys identity health` reads
its readiness (`/healthz`) and nothing else.

## Before every install: releases and advisories

Each development install checks the upstream releases and advisories for
Rauthy, SpiceDB and PostgreSQL first, and records the result and the accepted
Rauthy v0.36.2 exception in
`docs/design/identity/reports/IDENTITY-001-deployment.md`. The last check is
recorded under `advisory_check` in `deploy/identity/versions.json`. Rows 02 to
05 run on v0.36.2 under Waffles' ruling of 15:36:25; real sign-in waits on
IDENTITY-002.

## Configure

Copy `deploy/identity/config.example.toml` outside the repository and edit it.

- **Database address.** `database.host = "postgres"` uses this compose file's
  own PostgreSQL service. Any other host, for example a network device's
  address, is an external PostgreSQL: create the database there, then run
  `deploy/identity/postgres-init.sql` once against it as a superuser with the
  three `IDENTITY_*` values it reads (see the file's header). Nothing falls back
  to a local database when an external one does not answer.
- **Local origin.** `rauthy.public_origin = "http://localhost:8080"`: an http
  origin is accepted on a loopback host only.
- **TLS origin.** `rauthy.public_origin = "https://identity.example.test"`
  behind a reverse proxy that terminates TLS and forwards to
  `rauthy.publish`; list the proxy in `rauthy.trusted_proxies`. Rauthy then runs
  in proxy mode and its issuer is `https://identity.example.test/auth/v1/`.
- **Clients.** Exactly two: `platform` (RS256, PKCE S256) and `cambium`
  (RS256). The built-in `rauthy` client is never managed.

## Prepare

```
cargo run -p lys -- identity prepare --config <config.toml>
```

prepare validates the configuration and writes, under `state_dir`, the private
env file `identity.env` and one file per generated credential in
`credentials/`: every file mode 0600, every directory 0700, and the state
directory refused if it lies inside a Git work tree. A credential that exists is
reused, never regenerated. It prints paths and outcomes, never a value.

## Start, and the migration order

```
docker compose --env-file <state_dir>/identity.env -f deploy/identity/compose.yaml up -d
```

The env file selects the `local-database` profile only when the database is
the local service. Compose starts, in order:

1. PostgreSQL, healthy only after `postgres-init.sql` has created the roles
   `identity_rauthy` and `identity_spicedb`, their schemas `rauthy` and
   `spicedb`, and each role's search path.
2. `spicedb-migrate`, `spicedb datastore migrate head`, into the `spicedb`
   schema; it must exit 0.
3. Rauthy, whose own migrations run at start into the `rauthy` schema, and
   SpiceDB serve.

Both migration runners create unqualified tables, so each lands only in its
role's schema: Rauthy's history is `rauthy.refinery_schema_history` and
SpiceDB's is `spicedb.alembic_version`, and neither role can read the other's
schema. `crates/lys/tests/identity_shared_db.rs` proves both before one-database
readiness is claimed.

A missing value in the env file stops compose by the name
`identity_secret_missing` or `identity_config_missing` before any container
starts.

## Readiness

```
cargo run -p lys -- identity health --config <config.toml>
```

Checks the database at its configured address, Rauthy's `/auth/v1/health`
(database and cache healthy) and SpiceDB's `/healthz`, prints one line per
service, and exits 0 only when all three are ready. Each unready service is
named: `database_unreachable`, `rauthy_unreachable`, `rauthy_unhealthy`,
`spicedb_unreachable`, `spicedb_unready`, collected as `services_unready`.

## Configure the clients and themes

```
cargo run -p lys -- identity configure --config <config.toml> \
  --themes deploy/identity/rauthy-themes.json
```

Creates or updates the platform and Cambium clients, measures and writes their
dark themes, and keeps each client's Rauthy-issued secret once under
`credentials/`. A second run reports every operation `unchanged` (secrets
`reused`) under the same operation identifiers. It refuses to run while Rauthy
holds a client it does not manage (`unmanaged_clients`) and deletes nothing.

## Named configuration failures

| Name | Raised by | Cause |
|---|---|---|
| `config_unreadable` / `config_invalid` | every subcommand | the file is missing, or not the expected TOML |
| `invalid_issuer` | every subcommand | the public origin is not http(s), carries a path, or is http on a non-loopback host |
| `invalid_redirect_uri` | every subcommand | not absolute, carries a fragment or wildcard, or is http on a non-loopback host |
| `invalid_config_value` | every subcommand | any other value outside what the deployment accepts |
| `builtin_client_reserved` / `duplicate_client_id` | every subcommand | a client takes the id `rauthy`, or both take one id |
| `state_dir_inside_git` | prepare | the state directory is inside a Git work tree |
| `secret_missing` / `private_file_too_open` | configure | a credential is absent, or readable by group or other |
| `identity_secret_missing` / `identity_config_missing` | compose, `postgres-init.sql` | the env file lacks a value |
| `services_unready` | health | one or more services are not ready, each named |

## Stop and start

```
docker compose --env-file <state_dir>/identity.env -f deploy/identity/compose.yaml stop
docker compose --env-file <state_dir>/identity.env -f deploy/identity/compose.yaml start
```

`down` without `-v` removes the containers and keeps the volumes;
`up -d` then recreates them. Either way the issuer, its signing keys and the
database contents persist (`crates/lys/tests/identity_restart.rs`).

## Backup and restore

A SQL dump alone does not back up the product. A restore needs:

1. **The database**: `pg_dump -Fc` of the identity database, both schemas.
2. **The keys and config**: the whole `state_dir`, above all
   `credentials/rauthy_encryption_key`. Rauthy encrypts its signing keys and
   client secrets in the database with that key (`ENC_KEYS`); a dump restored
   without it cannot be decrypted. The role passwords in `credentials/` must
   match the roles `postgres-init.sql` recreates.
3. **The cache**: the `rauthy-data` volume (`/app/data`). With `HIQLITE=false`
   no identity data lives there: Rauthy's internal Hiqlite node keeps only its
   cache Raft logs and state machine (`logs_cache/`, `state_machine_cache/`),
   its own node logs and state (`logs/`, `state_machine/`) and any bootstrap
   secrets container. It is rebuilt empty if lost; back it up with Rauthy
   stopped so a restore returns the same node state.

Backup, with Rauthy stopped for the cache:

```
C="docker compose --env-file <state_dir>/identity.env -f deploy/identity/compose.yaml"
$C exec -T postgres pg_dump -U postgres -d identity -Fc > identity.dump
$C stop rauthy
docker run --rm -v <project>_rauthy-data:/data -v "$PWD":/backup --entrypoint sh \
  <postgres image from versions.json> -c 'tar -C /data -czf /backup/rauthy-data.tgz .'
cp -Rp <state_dir> state-backup
```

Restore into fresh volumes, then repeat readiness:

```
$C up -d --wait postgres
$C exec -T postgres pg_restore -U postgres -d identity --clean --if-exists --exit-on-error < identity.dump
$C create rauthy
docker run --rm -v <project>_rauthy-data:/data -v "$PWD":/backup --entrypoint sh \
  <postgres image from versions.json> -c 'tar -C /data -xzf /backup/rauthy-data.tgz'
$C up -d
cargo run -p lys -- identity health --config <config.toml>
```

For an external database, dump and restore with `pg_dump` and `pg_restore`
against its host; run `postgres-init.sql` on a fresh database before the
restore. `crates/lys/tests/identity_shared_db.rs` runs this whole cycle.

## Gate

The container-backed targets `identity_deploy`, `identity_refusals`,
`identity_shared_db`, `identity_restart` and `identity_theme` are declared
`test = false`, so `cargo test --workspace --all-features` stays hermetic. They
run on the identity leg of `.land/gates.sh`, which fails by the name
`container_runtime_missing` when `docker info` does not answer, lints them with
`cargo clippy -p lys --all-features --test 'identity_*' -- -D warnings`, and only
then runs `cargo test -p lys --all-features --test 'identity_*'`.
