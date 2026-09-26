# The identity product's dependencies: development install

DIRECTORY-002 (IDENTITY-001 revision 5, row 02). Three dependency processes, beside the
platform door and nothing else:

- **Rauthy** v0.36.2, the maintained fork's pinned commit (ADR-009): it authenticates people.
- **SpiceDB** v1.56.2: installed and checked, enforcing nothing in step 1 (see the last section).
- **One PostgreSQL service holding one durable database** (ADR-005): Rauthy and SpiceDB each
  in their own schema under their own least-privilege role.

Cambium, Manifold, Argus and Aion servers are not runtime dependencies of any of them (ADR-004).
This row does not claim the product directory exists: that is DIRECTORY-003.

Every path below is relative to the repository root, and every command runs from it.

| File | What it is |
| --- | --- |
| `deploy/identity/compose.yaml` | The three services, image references pinned by digest |
| `deploy/identity/versions.json` | Releases, digests, the advisory read and the accepted exception |
| `deploy/identity/config.example.toml` | The deployment config: database address, local and TLS origins, clients |
| `deploy/identity/postgres-init.sql` | Roles and schema namespaces, run once by PostgreSQL's own init directory |
| `deploy/identity/rauthy-themes.json` | The two client themes, dark only (ADR-021); `deploy/identity/theme-map.md` explains them |

## Install

The development instance runs on a node the operator names (`deployment.node`), with
disposable test identities only: no production token, real business sign-in or live Cambium
participant migration (CN2). Before every install, read upstream's current releases and
advisories and record the result and the accepted v0.36.2 exception in
`deploy/identity/versions.json` and `docs/design/identity/reports/IDENTITY-001-deployment.md`
(CN10). Release builds, checks and tests stay on the gate workflow.

```sh
cp deploy/identity/config.example.toml ~/identity-dev/identity.toml   # then edit it
lys identity prepare --config ~/identity-dev/identity.toml
docker compose -f deploy/identity/compose.yaml -p <deployment.project> \
  --env-file ~/identity-dev/private/identity.env --profile database up -d
lys identity health --config ~/identity-dev/identity.toml
lys identity configure --config ~/identity-dev/identity.toml \
  --themes deploy/identity/rauthy-themes.json
```

`prepare` generates every credential once, owner-only (`0600` files in a `0700` directory),
and reuses them on every later run; it prints file names, never values. `configure` registers
the platform and Cambium clients and their themes and is idempotent: a second run reports every
stable operation identifier `unchanged`. It never creates, changes or deletes the built-in
`rauthy` client.

## The database address is configuration

`database.host` names where the one database lives. It may be a network device rather than
this machine (ADR-005), and nothing defaults it: every compose interpolation is `${NAME:?...}`,
so an absent value refuses by name. Rauthy, SpiceDB and `lys identity health` all dial that
address, so give one that both the containers and this host reach; a loopback name is refused
because inside a container it names the container.

- **Database on this node**: add `--profile database`, set `database.publish` to the host port
  PostgreSQL is published on, and set `database.host` to this node's routable address.
- **Database on another device**: leave the profile off, set `database.sslmode = "require"`,
  and have its administrator run `deploy/identity/postgres-init.sql` with psql and the variables
  `POSTGRES_DB`, `LYS_IDENTITY_RAUTHY_DB_PASSWORD` and `LYS_IDENTITY_SPICEDB_DB_PASSWORD` from
  `identity.env`. No local stand-in starts.

## Origins

- **Local**: `rauthy.public_origin = "http://localhost:8080"`. Plain http is accepted only on a
  loopback host. The issuer is `http://localhost:8080/auth/v1/`.
- **TLS**: `rauthy.public_origin = "https://id.example.net"`, TLS terminated by a proxy in front
  of Rauthy, with `rauthy.trusted_proxy` set to the proxy's CIDR. Rauthy runs in proxy mode and
  the issuer is `https://id.example.net/auth/v1/`.

`rauthy.admin_origin` is always the node-local plain-http listener this CLI calls.

## Migration order

1. PostgreSQL becomes healthy. On first initialisation only, its init directory runs
   `deploy/identity/postgres-init.sql`: roles `rauthy` and `spicedb`, schemas of the same names
   owned by them, each role's `search_path` set to its own schema, and nothing granted to PUBLIC.
2. `spicedb-migrate` runs SpiceDB's migrations to head, into the `spicedb` schema (its history
   table is `spicedb.alembic_version`), and exits.
3. SpiceDB serves, and Rauthy starts and runs its own embedded migrations into the `rauthy`
   schema (history `rauthy.refinery_schema_history`).

The two runners never share a schema or a history table; ID001_SHARED_DB measures that.

## Readiness and named failures

`lys identity health` checks the declared services and names each one that is not ready:

| Service | Check | Failure names |
| --- | --- | --- |
| database | an unauthenticated PostgreSQL `SSLRequest`, then a startup message with no password, to `database.host:database.port` | `database_unreachable`, `database_not_postgresql`, `database_not_accepting` (starting, recovering or shutting down) |
| rauthy | `GET /auth/v1/health` at `rauthy.admin_origin` | `rauthy_unreachable`, `rauthy_unready`, `rauthy_database_unhealthy`, `rauthy_cache_unhealthy` |
| spicedb | `GET /healthz` at `spicedb.http_origin` | `spicedb_unreachable`, `spicedb_unready`, `spicedb_not_serving` |

When all are ready it prints Rauthy's issuer and signing key ids, the public identity a restart
or restore must preserve. It reads no credential, so its output cannot carry one.

Configuration failures are named before anything is written: `config_invalid`,
`invalid_issuer`, `invalid_redirect_uri`, `invalid_database_host`, `invalid_sslmode`,
`invalid_trusted_proxy`, `invalid_local_origin`, `invalid_client_id`, `invalid_signing_alg`,
`invalid_challenge`, `invalid_publish`, `invalid_admin_email`, `invalid_identifier`. A prepared
deployment missing a credential is `secret_missing`, and nothing is generated in its place:
a new encryption key or password would substitute an identity or strand the data. Compose
itself refuses an absent value as `missing_secret NAME` or `missing_config NAME`.

## Stop and start

```sh
docker compose -f deploy/identity/compose.yaml -p <project> --env-file <private>/identity.env --profile database stop
docker compose -f deploy/identity/compose.yaml -p <project> --env-file <private>/identity.env --profile database start
```

`down` without `-v` also keeps both volumes. Restart preserves the issuer, its signing keys and
the database's contents (ID001_DEPLOY).

## Backup and restore

A SQL dump alone does not back up the product. The backup is four things:

1. **The database**: `pg_dump --create -Fc` of the identity database, taken as the bootstrap
   superuser with Rauthy and SpiceDB stopped. `--create` carries the database's ACL and each
   role's per-database `search_path`.
2. **Rauthy's data volume** (`<project>_rauthy-data`, mounted at `/app/data`). Rauthy's internal
   Hiqlite still starts with PostgreSQL as the datastore, as its cache node: this volume holds
   its Raft log and cache state machine and the bootstrap generated-secrets container
   (`bootstrap.secrets.enc`). It is cache, not a second identity datastore (`HIQLITE=false`),
   but restoring it keeps sessions and cached state coherent with the dump.
3. **The private directory**: `identity.env` and `secrets/`. `rauthy_enc_key` encrypts client
   secrets and other values inside the database: without it a restored dump cannot be read.
   `rauthy_hql_secret_raft` and `rauthy_hql_secret_api` authenticate the cache node; the
   database passwords and the bootstrap API key secret let the restored roles and configure in.
4. **The deployment config** (`identity.toml`).

Restore, in order: put the config and private directory back (owner-only); `docker compose ...
create` to recreate the volumes; extract the data volume archive into `<project>_rauthy-data`;
`up -d postgres` so its init directory recreates the roles; then
`pg_restore --dbname postgres --clean --create --if-exists --exit-on-error` from the dump; then
`up -d` and `lys identity health`. ID001_SHARED_DB runs exactly this and requires the issuer, the
signing keys, both services' data and the role isolation to come back unchanged.

Credentials reach the containers as environment variables, which `docker inspect` shows to
anyone with access to the container runtime: acceptable for this development install, not
for production (rows 06 and 07).

## SpiceDB in step 1

SpiceDB's step-1 role, in one sentence: in step 1 SpiceDB is installed, migrated, backed up and health-checked against the shared PostgreSQL database, and it enforces nothing, because no component asks it for a permission decision or writes a grant to it, and live capability policy and its enforcement stay with road step 2.

What it does not do in step 1: it answers no check for any identity, it holds no grant or
relationship of the directory, the lifecycle state recorded by DIRECTORY-003 gates nothing
through it, and running it is not permission enforcement. `lys identity` configures it and reads
its readiness, and nothing more. The schema and relationship ID001_SHARED_DB writes to prove the
shared database are test fixtures, not grants.

## The identity leg

The container-backed tests (`crates/lys/tests/identity_*.rs`) are declared `test = false`, so
`cargo test --workspace --all-features` stays hermetic. They run only on the identity leg of
`.land/gates.sh`, which fails as `container_runtime_missing` when `docker info` does not answer,
and otherwise lints them first:

```sh
cargo clippy -p lys --all-features --test 'identity_*' -- -D warnings
cargo test -p lys --all-features --test 'identity_*'
```
