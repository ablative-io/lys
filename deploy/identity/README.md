# The standalone identity product's dependencies

Three dependency processes, installed for development beside the platform door:

- the maintained Rauthy, `ablative-io/rauthy` branch `ablative` at the commit
  `vendor/rauthy` pins (ADR-009);
- SpiceDB;
- one PostgreSQL service with one durable database that both share, under separate
  roles and schemas (ADR-005).

Cambium, Manifold, Argus and Aion servers are not runtime dependencies. Development
installs use test identities and test provider registrations only.

| File | What it is |
|---|---|
| `compose.yaml` | The three services, pinned by image digest |
| `versions.json` | Pinned releases, digests and the accepted v0.36.2 exception |
| `config.example.toml` | The deployment configuration; local and TLS origins documented inline |
| `postgres-init.sql` | Roles and schema namespaces, run by PostgreSQL's own init directory |
| `rauthy-themes.json`, `theme-map.md` | The two client themes and their token map (ADR-021) |

## SpiceDB in step 1

SpiceDB's step-1 role, in one sentence: in step 1 SpiceDB is installed, migrated, backed up and health-checked against the shared PostgreSQL database, and it enforces nothing, because no component asks it for a permission decision or writes a grant to it, and live capability policy and its enforcement stay with road step 2.

It answers no check for any identity, holds no grant or relationship of the
directory, gates nothing through the lifecycle state DIRECTORY-003 records, and
running it is not permission enforcement. `lys identity health` reads its readiness
and nothing else.

## Before every install

Check upstream Rauthy, SpiceDB and PostgreSQL releases and advisories, and record
the result in `docs/design/identity/reports/IDENTITY-001-deployment.md`. Rauthy
v0.36.2 is an accepted exception for isolated development installs only; real
sign-in and install wait on the rebase brief IDENTITY-002.

## Install

The operator names the node in the configuration. From the repository root:

```sh
cp deploy/identity/config.example.toml ~/identity/identity.toml   # then edit it
lys identity prepare --config ~/identity/identity.toml
docker compose -f deploy/identity/compose.yaml --env-file ~/identity/state/compose.env up -d
lys identity health --config ~/identity/identity.toml
lys identity configure --config ~/identity/identity.toml
```

`prepare` validates the configuration and writes every credential and the compose
environment owner-only (mode 600, directory 700) into `state_dir`. A credential that
exists is reused, never rotated. With `credentials.source = "provided"` a missing
credential is refused as `secret_missing` instead of generated. No credential is
ever in Git, in the configuration, in a log line lys prints, or in `health` output.

`configure` registers exactly two clients beside Rauthy's built-in `rauthy` client,
which it never creates, changes or deletes: the platform's confidential client with
PKCE S256, and Cambium's confidential client with RS256 tokens. It applies both
themes, stores each client secret owner-only in `state_dir` as
`<client id>-client-secret`, and prints a stable operation identifier per resource; a
second run reports the same identifiers and changes nothing. A request whose
response is lost is resolved by reading the client back, never by creating a second
one. Google and GitHub federation credentials belong to Rauthy and are not managed
here; Google API consent is a different client purpose.

## The database address is configuration

`[database]` names the host Rauthy and SpiceDB connect to. `bundled = true` runs the
compose file's own PostgreSQL service (host `postgres`, published on loopback). A
database on a network device is named by its own address with `bundled = false`, for
example `host = "192.0.2.10"`; compose then starts no PostgreSQL service, and the
operator runs `postgres-init.sql` against that database once (the command is at the
top of the file) and enables `track_commit_timestamp`, which SpiceDB needs. Nothing
assumes the operator's machine: an unset address is refused as
`database_address_missing` and no local address is substituted.

## Readiness and migration order

1. PostgreSQL answers `pg_isready` over TCP, never only on its local socket, which its
   first-start init server alone listens on; on first start its init directory runs
   `postgres-init.sql`: roles `rauthy` and `spicedb`, each owning one schema of the
   same name with only that schema on its `search_path`, neither with usage on the
   other's, and nothing for `PUBLIC`.
2. `spicedb-migrate` runs `spicedb datastore migrate head` as `spicedb`, into the
   `spicedb` schema, and exits.
3. `spicedb` serves once the migration completed successfully.
4. `rauthy` runs its own migrations as `rauthy`, into the `rauthy` schema, then
   serves. `HIQLITE=false`: PostgreSQL is its only identity datastore.

Ready means `lys identity health` exits 0: PostgreSQL answers at `check_address`,
Rauthy's `/auth/v1/health` reports its database and cache healthy, and SpiceDB's
`/healthz` answers 200.

## Named failures

| Name | Raised by | When |
|---|---|---|
| `secret_missing` | prepare, configure, compose, init SQL | A credential is absent |
| `secret_malformed` | prepare, configure | A credential file is not of its shape |
| `issuer_invalid` | prepare | The public origin or admin URL is not accepted |
| `redirect_uri_invalid` | prepare | A client redirect URI is not exact and absolute |
| `database_address_invalid` / `database_address_missing` | prepare / compose | The database address is malformed or unset |
| `client_invalid` | prepare | A client section is malformed or duplicated |
| `private_file_mode_open` | prepare, configure | A private file or directory is readable by others |
| `database_unreachable` | health | PostgreSQL does not answer |
| `rauthy_unreachable`, `rauthy_database_unhealthy`, `rauthy_cache_unhealthy` | health | Rauthy is down or reports a part unhealthy |
| `spicedb_unreachable`, `spicedb_unready` | health | SpiceDB is down or not ready |
| `rauthy_unauthorized`, `rauthy_forbidden`, `rauthy_bad_request`, `rauthy_server_error` | configure | Rauthy refused a call |
| `rauthy_outcome_uncertain` | configure | A write went unanswered and read-back could not settle it |
| `read_back_mismatch` | configure | A client or theme read back differs from what was written |

No failure substitutes an identity or a development database.

## Stop and start

```sh
docker compose -f deploy/identity/compose.yaml --env-file <state>/compose.env stop
docker compose -f deploy/identity/compose.yaml --env-file <state>/compose.env start
```

Data lives in the named volumes `postgres-data` and `rauthy-data`; stop, start and
`down` without `-v` keep them, and the issuer, its signing keys and every client
survive.

## Backup and restore

A SQL dump alone does not back up the product. A complete backup is four things:

1. **The database**: `pg_dump -Fc` of the one database as the superuser, which holds
   both schemas. Roles are not in the dump; `postgres-init.sql` recreates them from
   the same credentials.
2. **The credentials and compose environment**: the whole `state_dir`. Rauthy's
   encryption key (`rauthy-encryption-key`) decrypts client secrets and signing keys
   stored in the database; a database restored without it is unreadable.
3. **Rauthy's Hiqlite cache node**: the `rauthy-data` volume, mounted at `/app/data`
   (`HQL_DATA_DIR`). In the installed configuration Hiqlite is only Rauthy's cache
   and raft state, not an identity datastore, but restoring it with the database
   keeps the node's identity and avoids a cold rejoin.
4. **The configuration file** passed to `--config`.

Restore: recreate `state_dir` and the configuration, start PostgreSQL alone so its
init directory recreates the roles and schemas, `pg_restore --clean --if-exists` the
dump as the superuser, restore the `rauthy-data` volume, start the rest, and repeat
readiness with `lys identity health`. The container-backed test `identity_shared_db`
performs exactly this sequence.

## Tests

The container-backed targets `identity_deploy`, `identity_refusals`,
`identity_shared_db`, `identity_restart` and `identity_theme` are declared
`test = false`: `cargo test --workspace --all-features` never runs them, and they
run only on the identity leg of `.land/gates.sh`, which fails as
`container_runtime_missing` when no container runtime answers `docker info`.
