# The identity product's dependencies, installed for development

The standalone identity product stands on three dependency processes: the
maintained Rauthy at the commit `vendor/rauthy` pins (ADR-009), SpiceDB, and one
PostgreSQL service holding the one durable database both of them use (ADR-005).
Nothing else of the estate is a runtime dependency: Cambium, Manifold, Argus and
Aion are absent (ADR-004).

`compose.yaml` runs the three. `versions.json` pins each image by release and
digest; `compose.yaml` carries the same digests, and a change to one is a
change to the other in one gated row. `postgres-init.sql` is what PostgreSQL
runs itself, from its own init directory, the first time its data volume is
empty. `config.example.toml` is the configuration `lys identity` reads; it holds
addresses and names, never a secret.

## What SpiceDB does in step 1

in step 1 SpiceDB is installed, migrated, backed up and health-checked against
the shared PostgreSQL database, and it enforces nothing, because no component
asks it for a permission decision or writes a grant to it, and live capability
policy and its enforcement stay with road step 2

It answers no check for any identity, it holds no grant or relationship of the
directory, the lifecycle state the directory records gates nothing through it,
and running it is not permission enforcement. `lys identity health` reads its
readiness and nothing more.

## Install

1. Copy `config.example.toml` to a file of your own and set the addresses: the
   database host, Rauthy's public origin, the host ports. The file never holds a
   secret.
2. `lys identity prepare --config <your file>` generates every secret and writes
   the private files the services read into the venue directory, each with mode
   0600: `compose.env`, `postgres.env`, `spicedb.env`, `rauthy/config.toml` and
   `rauthy/secrets.toml`. The venue is never committed (`.gitignore`).
3. Start the services from this directory:

   ```sh
   docker compose --env-file <venue>/compose.env --profile local-db up -d --wait
   ```

   With the database on another host, set `[database].host` to it, leave the
   `local-db` profile off, and run the init script against that server once as
   its administrator with the two role passwords in the environment:

   ```sh
   RAUTHY_PG_PASSWORD=... SPICEDB_PG_PASSWORD=... psql -h <host> -U <admin> -d identity -f postgres-init.sql
   ```

4. `lys identity health --config <your file>` names each service and the
   database as ready or, by name, why not.
5. `lys identity configure --config <your file>` registers the platform and
   Cambium OIDC clients and their themes on Rauthy. Run it as often as you like:
   it leaves exactly two clients.

## Readiness and migration order

PostgreSQL is ready when `pg_isready` answers on the admin role and database.
The init script has then created the `rauthy` and `spicedb` roles, each owning
a schema of the same name with its search path set to it, with no privilege on
the other's schema and none on `public`. Both migration runners then create
their tables in their own namespace without being told about the other: Rauthy
migrates on start, and `spicedb-migrate` runs `datastore migrate head` to
completion before `spicedb` starts.

Rauthy is ready when `GET /auth/v1/health` answers `db_healthy` and
`cache_healthy` true. SpiceDB is ready when its gRPC health service answers
serving; with `--http-enabled`, a `POST /v1/schema/read` with the preshared key
reaches the datastore (an empty schema answers "no schema has been defined",
which is readiness, not a fault).

## Named configuration failures

A missing secret, an issuer or redirect that does not match the origin served,
or an unreachable database is a named failure and never a substitute:

- Rauthy refuses to start without its encryption key, its cluster secrets or its
  WebAuthn relying party, naming the missing key in its log, and exits.
- Rauthy with an unreachable database exits and is restarted by compose until
  the database answers; it never opens an embedded database in its place
  (`hiqlite = false`).
- SpiceDB with an unreachable or wrong-password database exits at migration
  and never serves.
- `lys identity health` names each unready service or database, one line per
  declared service, and prints no secret.

## Stop and start

`docker compose --env-file <venue>/compose.env --profile local-db stop` and
`start`, or `restart`. Restart keeps the issuer identity (the JWKS key ids are
unchanged) and every record, because both live in the database and the keys
that protect them live in `rauthy/secrets.toml`.

## Backup and restore

A SQL dump alone does not back up the product. The product is:

- the database: `pg_dump --clean --if-exists` of `identity` as `identity_admin`, both
  schemas, so the dump restores over a database the init script has already
  prepared;
- the encryption keys in `<venue>/rauthy/secrets.toml`: every client secret and
  every encrypted value in the database is unreadable without them;
- the configuration in `<venue>/rauthy/config.toml`, `postgres.env`,
  `spicedb.env` and `compose.env`, which name the issuer and the roles;
- Rauthy's internal Hiqlite cache and node state in the `rauthy-data` volume.
  In this configuration Hiqlite is a cache, not an identity datastore; the
  volume can be recreated empty, and a restore then starts with a cold cache.

Restore: recreate the venue from its backup, start PostgreSQL alone, restore the
dump as `identity_admin` into the empty `identity` database (the roles and
schemas come from the init script, the tables from the dump), then start the
rest and repeat the readiness checks above.

## Releases and advisories

Each development install checks the upstream Rauthy releases and advisories
against the pinned `v0.36.2` and records the outcome, with the digests and the
restore result, in `docs/design/identity/reports/IDENTITY-001-deployment.md`.
The accepted v0.36.2 exception IDENTITY-001 records (CN10) stands until the pin
moves in its own gated row.

## Host ports

The defaults in `config.example.toml` publish PostgreSQL on 15432, Rauthy on
18080, and SpiceDB on 15051 (gRPC) and 18443 (HTTP), because the estate's own
listeners hold 50051 and 8080 on the development Mac.
