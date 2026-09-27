# IDENTITY-001 row 02: deployment report

This report records only what was measured. Everything that needs a running Rauthy,
PostgreSQL and SpiceDB is measured on the venue by the identity leg of
`.land/gates.sh` and is marked **pending the venue run** below until that run's
results are written here. This row does not claim the product directory exists yet;
that is DIRECTORY-003.

## Dependency artifacts

Resolved on 2026-09-27 with `docker buildx imagetools inspect`, which reads registry
manifests and starts no container.

| Dependency | Version | Image | Index digest |
|---|---|---|---|
| Rauthy (maintained) | v0.36.2, `ablative` at `dd61ac3c84d6b238108dc8438b53043b5177a662`, no application patch | `ghcr.io/sebadob/rauthy:0.36.2` | `sha256:f7d3c501402165e023edbd958b032b41c9cfdac5ea7f8ca7d62217327145577e` |
| SpiceDB | v1.56.2 | `docker.io/authzed/spicedb:v1.56.2` | `sha256:aa96009a0477f8a759149823407d47ad16d1a74390bae3102b3b0b7143502764` |
| PostgreSQL | 17.11 | `docker.io/library/postgres:17.11` | `sha256:d74eeac9a635390a49bc21bd49fccd973de707e2a53a76ac49b552b8712ec46f` |

Rauthy platform digests: linux/amd64
`sha256:2f227cdc1cb8b5377c34deecf4472218e7b33d7c3d891344bf0b3911312744a6`, linux/arm64
`sha256:979c51e5fcc4215602618ebaff557bd0f9157b7e0c70636a8e657afd95f6f945`. Both carry
the labels `org.opencontainers.image.version=0.36.2` and
`org.opencontainers.image.created=2026-08-08 20:10:47+02:00`.

The pinned commit is upstream tag v0.36.2 (checked in the fork: `git tag
--points-at dd61ac3c` answers `v0.36.2`, and `origin/ablative` is at the same
commit), so the upstream release image is the maintained Rauthy at the pin. The
first row that patches `ablative` replaces it with an image built from the pin.

## Upstream releases and advisories at this install

Measured from registry tags on 2026-09-27:

- Rauthy: tags `0.36.3`, `0.36.4`, `0.37.0`, `0.37.1`, `0.37.2`, `0.38.0`, `0.39.0` and
  `0.40.0` do not resolve. `latest` resolves to
  `sha256:90524fa135aecb397103b5bdc56573d6b1e2f4df7b8d557e3a81c73a157edbf6`, a
  different digest from `0.36.2`, whose image labels also name version `0.36.2`. No
  newer release tag was found among those probed; the probe is not a full tag list.
- SpiceDB: `latest` resolves to the same digest as `v1.56.2`.
- PostgreSQL: `17` resolves to the same digest as `17.11`; `18.3` exists and was not
  chosen, since 17 is the line both services were written against.
- Security advisories for the three: **pending the venue run**; no advisory database
  was consulted from this Mac.

Accepted exception: Rauthy v0.36.2 for isolated development installs with test
identities only (Waffles 15:36:25). Real sign-in (row 06) and install (row 07) wait on
IDENTITY-002.

## Resolved configuration, without secrets

`lys identity prepare` over `deploy/identity/config.example.toml`, measured on this
Mac: ten private files, all mode 600 in a mode 700 directory, and the compose
environment below with its nine credential variables removed. The credentials were
generated for the measurement and discarded.

```
COMPOSE_PROJECT_NAME=lys-identity
COMPOSE_PROFILES=bundled-db
IDENTITY_NODE=dev-node
IDENTITY_DB_HOST=postgres
IDENTITY_DB_PORT=5432
IDENTITY_DB_NAME=identity
IDENTITY_DB_TLS=disable
IDENTITY_DB_PUBLISH_PORT=55432
RAUTHY_LISTEN_PORT=8480
RAUTHY_PUB_URL=localhost:8480
RAUTHY_RP_ID=localhost
RAUTHY_RP_ORIGIN=http://localhost:8480
RAUTHY_PROXY_MODE=false
RAUTHY_TRUSTED_PROXIES=""
RAUTHY_ENC_KEY_ACTIVE=lys01
RAUTHY_BOOTSTRAP_ADMIN_EMAIL=admin@identity.test
RAUTHY_BOOTSTRAP_API_KEY=eyJhY2Nlc3MiOlt7ImFjY2Vzc19yaWdodHMiOlsicmVhZCIsImNyZWF0ZSIsInVwZGF0ZSJdLCJncm91cCI6IkNsaWVudHMifSx7ImFjY2Vzc19yaWdodHMiOlsicmVhZCJdLCJncm91cCI6IlNlY3JldHMifV0sImV4cCI6bnVsbCwibmFtZSI6Imx5c19jb25maWd1cmUifQ==
SPICEDB_GRPC_PORT=50051
SPICEDB_HTTP_PORT=58443
```

`RAUTHY_BOOTSTRAP_API_KEY` is the key's description, not its secret: base64 of
`{"access":[{"access_rights":["read","create","update"],"group":"Clients"},{"access_rights":["read"],"group":"Secrets"}],"exp":null,"name":"lys_configure"}`.

`docker compose -f deploy/identity/compose.yaml --env-file <state>/compose.env config`
over that environment, measured on this Mac without starting a container, resolves
the services `postgres`, `rauthy`, `spicedb` and `spicedb-migrate`, with Rauthy's
`PG_HOST` `postgres`.

## Refusals measured without containers

Measured on this Mac with `docker compose config`, which starts no container:

- The example configuration with `bundled = false`, `host = "192.0.2.10"` and
  `check_address = "192.0.2.10:5432"`, rendered by `lys identity prepare`, resolves
  the services `rauthy`, `spicedb` and `spicedb-migrate` with no `postgres` service;
  Rauthy's `PG_HOST` is `192.0.2.10` and SpiceDB's datastore URI names
  `192.0.2.10:5432/identity`.
- With `RAUTHY_DB_PASSWORD` removed from the rendered environment, compose refuses:
  `required variable RAUTHY_DB_PASSWORD is missing a value: secret_missing
  RAUTHY_DB_PASSWORD`.
- `issuer_invalid`, `redirect_uri_invalid`, `secret_missing` for provided credentials
  and `private_file_mode_open` are refused by `lys identity prepare` in the unit tests
  of `crates/lys/src/identity/`, which pass.

That the unreachable address makes readiness fail by name is **pending the venue
run** (`identity_refusals::id001_deploy_refusal_unavailable_database_is_named`).

## Themes

Source tokens: ablative-docs `docs/design-system-v2/palette/estate-colour-tokens.json`
at `385916eb437cae62c4269e6db1db2e542af5749e`. Contrast of the declared mapping with
every gap at the pinned Rauthy's default, computed by the unit tests of
`crates/lys/src/identity/themes.rs` on this Mac: all twelve pairs at or above their
tier; the lowest is Cambium's theme_moon over ink at 3.97 against 3 (the full table is
in `deploy/identity/theme-map.md`).

The Rauthy theme export of both clients, their login-page CSS and the contrast leg
over the exported values: **pending the venue run** (`identity_theme`).

## Acceptance measurements

| Line | Test | Result |
|---|---|---|
| ID001_DEPLOY: fresh install ready with no other Ablative service | `identity_deploy::id001_deploy_fresh_install_is_ready_and_configure_is_idempotent` | pending the venue run |
| ID001_DEPLOY: restart keeps issuer identity and database contents | `identity_restart::id001_deploy_restart_preserves_issuer_identity_and_database_contents` | pending the venue run |
| ID001_DEPLOY_REFUSAL: missing secret, invalid issuer and redirect, unavailable database | `identity_refusals` (four tests) | pending the venue run |
| Database address 192.0.2.10 carried into both services, no local address substituted | `identity_refusals::a_network_database_address_is_carried_into_both_services` | pending the venue run |
| ID001_SHARED_DB, with backup and restore | `identity_shared_db::id001_shared_db_one_database_two_schemas_restore_and_ready_again` | pending the venue run |
| Backup and restore outcome | the same test | pending the venue run |
| ID001_PIN_CLONE | `identity_deploy::id001_pin_clone_vendor_rauthy_is_the_pinned_ablative_commit` | pending the venue run |
| ID001_THEME and the counted contrast leg | `identity_theme::contrast_and_id001_theme_persist_after_restart` | pending the venue run |
