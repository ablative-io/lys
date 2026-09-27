# IDENTITY-001 row 02: deployment report

The report revision 5 names for row 02 (DIRECTORY-002). It records the
dependency artifacts, their versions and digests, the resolved configuration
without secrets, the release and advisory check, and the backup and restore
result. It does not claim the product directory exists: that is DIRECTORY-003.

## Where the results come from

- **Development trial, 2026-09-27 (Australia/Melbourne), Dean's laptop.** The
  compose file, init SQL and configuration of this row were brought up by hand
  once, with disposable generated credentials that were deleted afterwards, to
  confirm the packaging before the Rust around it was written. Its outcomes are
  recorded below as *trial*.
- **The identity leg of `.land/gates.sh`.** The container-backed targets
  (`identity_deploy`, `identity_refusals`, `identity_shared_db`,
  `identity_restart`, `identity_theme`) are the measured proof of ID001_DEPLOY,
  ID001_DEPLOY_REFUSAL, ID001_SHARED_DB, ID001_PIN_CLONE and ID001_THEME. They
  run at the venue on the pushed ref; their output, not this file, is the gate
  result. The live demonstration to Tom is a separate step after the row lands.

## Artifacts

| Component | Release | Image | Index digest |
|---|---|---|---|
| Rauthy (maintained fork, ADR-009) | v0.36.2 | `ghcr.io/sebadob/rauthy:0.36.2` | `sha256:f7d3c501402165e023edbd958b032b41c9cfdac5ea7f8ca7d62217327145577e` |
| SpiceDB | v1.56.2 | `ghcr.io/authzed/spicedb:v1.56.2` | `sha256:aa96009a0477f8a759149823407d47ad16d1a74390bae3102b3b0b7143502764` |
| PostgreSQL | 17.11 | `public.ecr.aws/docker/library/postgres:17.11-bookworm` | `sha256:639ab7ceb90e13123085b741fb31ef493fba25463002f6da665352e7b534b652` |

- Rauthy platform digests: linux/amd64
  `sha256:2f227cdc1cb8b5377c34deecf4472218e7b33d7c3d891344bf0b3911312744a6`,
  linux/arm64
  `sha256:979c51e5fcc4215602618ebaff557bd0f9157b7e0c70636a8e657afd95f6f945`.
- Rauthy source: `vendor/rauthy` pins
  `dd61ac3c84d6b238108dc8438b53043b5177a662` on `ablative-io/rauthy` branch
  `ablative`, which is the commit of upstream tag v0.36.2; the release image is
  the build of that commit. The fork carries no change of its own yet; row 03's
  first change replaces the image with a build of the pin.
- PostgreSQL is pulled from the public.ecr.aws mirror of the Docker Official
  Image: the same index digest as `docker.io/library/postgres:17.11-bookworm`,
  and it pulls anonymously where Docker Hub's credential helper is unavailable.
- Every digest is also in `deploy/identity/versions.json`, which the compose
  file's image references match.

## Release and advisory check (2026-09-27T02:12:41Z)

- **Rauthy.** Latest release v0.36.2 (2026-08-08). Published advisories:
  GHSA-wx92-7mmw-5x82 (high, patched in 0.36.2), GHSA-7qh2-3hc5-2vqp (medium,
  patched in 0.36.1), GHSA-x8jp-v2j6-6vjf (medium, patched in 0.36.0); none open
  against v0.36.2. **Accepted exception:** upstream's later hardening, #1696
  (merged 2026-08-26) and #1728 (merged 2026-09-18), is in no release. Waffles'
  ruling of 15:36:25 accepts v0.36.2 for isolated development installs with test
  identities only; IDENTITY-001-UPSTREAM-AUTH-STATE still binds real sign-in
  (row 06) and install (row 07), and the rebase is IDENTITY-002.
- **SpiceDB.** Latest release v1.56.2. GHSA-jf7w-wx43-32gc (low) is patched in
  v1.56.2; GHSA-5784-6qcr-48fq (low) in v1.56.0.
- **PostgreSQL.** Latest 17.x is 17.11.

## Resolved configuration, without secrets

From `deploy/identity/config.example.toml` through `lys identity prepare`:

| Setting | Value |
|---|---|
| Database | `identity` on host `postgres` (the local service, profile `local-database`), port 5432, TLS disabled |
| Roles and schemas | `identity_rauthy` owns `rauthy`; `identity_spicedb` owns `spicedb`; `public` grants nothing |
| Rauthy datastore | PostgreSQL (`HIQLITE=false`); Hiqlite runs only as the internal cache |
| Issuer | `http://localhost:8080/auth/v1/` (local origin); a TLS origin runs Rauthy in proxy mode behind a listed reverse proxy |
| Published | Rauthy `127.0.0.1:8080`, SpiceDB HTTP `127.0.0.1:8443` and gRPC `127.0.0.1:50051`, PostgreSQL `127.0.0.1:55432` |
| Clients | `platform` (confidential, RS256, PKCE S256) and `cambium` (confidential, RS256), beside the built-in `rauthy` |
| Credentials | nine generated files in `<state_dir>/credentials/`, plus each client's issued secret; mode 0600, outside Git |

With `database.host = "192.0.2.10"` the same render resolves Rauthy's `PG_HOST`
and SpiceDB's datastore host to 192.0.2.10 and omits the local service.

## Trial outcomes

- **Readiness.** From empty volumes: PostgreSQL healthy after the init SQL,
  `spicedb-migrate` exited 0, Rauthy migrated and bootstrapped its production
  database on PostgreSQL, `/auth/v1/health` answered
  `{"db_healthy":true,"cache_healthy":true}` and SpiceDB `/healthz` answered
  `SERVING`. No other service ran. Rauthy's log carried no generated password.
- **Migrations.** 48 tables in `rauthy` (history `rauthy.refinery_schema_history`),
  9 in `spicedb` (history `spicedb.alembic_version`), none in `public`.
- **Isolation.** As `identity_rauthy`, reading `spicedb.relation_tuple` failed
  with `permission denied for schema spicedb`.
- **API.** The bootstrap API key listed the clients (`rauthy` only), created
  and updated a client, read its secret, and read and wrote its theme; a wrong
  key was refused 401; a duplicate create was refused 400 (`ID exists already`),
  so a retried create cannot produce a second client.
- **Backup and restore.** `pg_dump -Fc` (113469 bytes), `down -v` (every volume
  lost, including the Rauthy cache), a fresh PostgreSQL with the init SQL,
  `pg_restore --clean --if-exists --exit-on-error` (exit 0), `up -d`: Rauthy and
  SpiceDB ready again, the clients present, and the JWKS byte-identical to
  before, so the issuer's signing keys survived with the same encryption key
  from the state directory.
- **Unavailable database.** With the host set to 192.0.2.10 and no local
  service, `spicedb-migrate` failed, compose refused to start Rauthy, and no
  local database was started in its place.

## Theme

The declared mapping, its source-token ref (ablative-docs `385916e`,
`docs/design-system-v2/palette/estate-colour-tokens.json`, owner Waffles), the
conversions, the eight gaps and the twelve contrast ratios are recorded in
`deploy/identity/theme-map.md`. The Rauthy theme export of each client, without
credentials, is printed by `identity_theme` at the gate.
