# IDENTITY-001 row 02: deployment report

DIRECTORY-002 (IDENTITY-001 revision 5, row 02). Dependency artifact digests, versions, the
resolved configuration without secrets, and the backup and restore result. **This row does not
claim the product directory exists yet**: that is DIRECTORY-003.

Two kinds of evidence are recorded here and kept apart:

- **Observed by hand** on 2026-09-27, during the development install of this row on the
  writing seat's node (macOS 26.6.2, arm64, Docker Desktop engine 29.5.2). This is what the row
  saw with its own eyes before its tests existed.
- **Measured by the gate**: the identity leg of `.land/gates.sh` running `identity_deploy`,
  `identity_refusals`, `identity_shared_db`, `identity_restart` and `identity_theme` at the
  venue. Those results belong to the gate's receipt and are appended below when it runs; nothing
  here claims them in advance. ID001_PIN_CLONE is due at that venue gate as well.

## Upstream release and advisory check (2026-09-27)

| Dependency | Pinned | Latest upstream release | Advisories |
| --- | --- | --- | --- |
| Rauthy | v0.36.2, fork `ablative` at `dd61ac3c84d6b238108dc8438b53043b5177a662` | v0.36.2 (2026-08-08) | three published, none naming v0.36.2 as affected |
| SpiceDB | v1.56.2 | v1.56.2 (2026-09-11) | all patched at or before v1.56.2 |
| PostgreSQL | 17.11 | 17.11 is the latest 17.x | supported major release |

**Accepted exception, recorded again for this install**: IDENTITY-001-UPSTREAM-AUTH-STATE.
v0.36.2 lacks upstream hardening #1696 and #1728; it is accepted for isolated development
installs with test identities in rows 02 to 05 only (Waffles, door time 15:36:25). It still
binds real sign-in and install in rows 06 and 07, through IDENTITY-002.

## Artifacts

Image references in `deploy/identity/compose.yaml` pin these index digests; the per-platform
digests are in `deploy/identity/versions.json`.

| Service | Image | Index digest |
| --- | --- | --- |
| Rauthy | `ghcr.io/sebadob/rauthy:0.36.2` | `sha256:f7d3c501402165e023edbd958b032b41c9cfdac5ea7f8ca7d62217327145577e` |
| SpiceDB (and `spicedb-migrate`) | `ghcr.io/authzed/spicedb:v1.56.2` | `sha256:aa96009a0477f8a759149823407d47ad16d1a74390bae3102b3b0b7143502764` |
| PostgreSQL | `public.ecr.aws/docker/library/postgres:17.11-bookworm` | `sha256:639ab7ceb90e13123085b741fb31ef493fba25463002f6da665352e7b534b652` |

The pinned `ablative` commit is the v0.36.2 tag commit itself, so upstream's published v0.36.2
image is built from the pinned source; a fork-built image replaces it when `ablative` diverges
(row 03). The PostgreSQL image is the Docker Official Image through its ECR Public mirror, same
index digest as `docker.io/library/postgres:17.11-bookworm`; Docker Hub pulls on the writing seat
were refused by a locked credential helper, which the mirror avoids.

## Resolved configuration, without secrets

As rendered into `identity.env` for the hand-observed install. Every `<generated>` value was a
fresh random test credential, owner-only, never printed and gone with the install.

| Variable | Value |
| --- | --- |
| `LYS_IDENTITY_PG_HOST` | the node's routable address (`192.168.0.123`) — configuration, not a default |
| `LYS_IDENTITY_PG_PORT`, `LYS_IDENTITY_PG_PUBLISH` | `15432` |
| `LYS_IDENTITY_PG_DATABASE` | `identity` |
| `LYS_IDENTITY_PG_ADMIN_USER` | `identity_admin` |
| `LYS_IDENTITY_PG_SSLMODE` | `disable` (database on the node's own compose network) |
| `LYS_IDENTITY_RAUTHY_DB_USER`, `LYS_IDENTITY_SPICEDB_DB_USER` | `rauthy`, `spicedb` |
| `LYS_IDENTITY_RAUTHY_PUB_URL` | `localhost:18080` (issuer `http://localhost:18080/auth/v1/`) |
| `LYS_IDENTITY_RAUTHY_PROXY_MODE`, `LYS_IDENTITY_RAUTHY_TRUSTED_PROXIES` | `false`, empty |
| `LYS_IDENTITY_RAUTHY_RP_ID`, `LYS_IDENTITY_RAUTHY_RP_ORIGIN` | `localhost`, `http://localhost:18080` |
| `LYS_IDENTITY_RAUTHY_ADMIN_EMAIL` | `admin@example.net` (test identity) |
| `LYS_IDENTITY_RAUTHY_API_KEY` | base64 of `{"name":"lys_identity","access":[{"group":"Clients","access_rights":["read","create","update"]}]}` |
| `LYS_IDENTITY_RAUTHY_PUBLISH` | `127.0.0.1:18080` |
| `LYS_IDENTITY_SPICEDB_HTTP_PUBLISH`, `LYS_IDENTITY_SPICEDB_GRPC_PUBLISH` | `127.0.0.1:18443`, `127.0.0.1:15051` |
| `LYS_IDENTITY_PG_ADMIN_PASSWORD`, `LYS_IDENTITY_RAUTHY_DB_PASSWORD`, `LYS_IDENTITY_SPICEDB_DB_PASSWORD`, `LYS_IDENTITY_RAUTHY_ENC_KEY` (and its id), `LYS_IDENTITY_RAUTHY_HQL_SECRET_RAFT`, `LYS_IDENTITY_RAUTHY_HQL_SECRET_API`, `LYS_IDENTITY_RAUTHY_ADMIN_PASSWORD`, `LYS_IDENTITY_RAUTHY_API_KEY_SECRET`, `LYS_IDENTITY_SPICEDB_PRESHARED_KEY` | `<generated>` |

That environment was rendered by hand, with the variable names `lys identity prepare` writes,
because the row's own CLI could not be built on the writing seat outside the gate. The gate's
identity leg renders it through `prepare` itself.

## Observed by hand

- **Readiness**: from empty volumes, PostgreSQL became healthy, `spicedb-migrate` ran every
  SpiceDB migration to head and exited 0, SpiceDB answered `{"status":"SERVING"}` on `/healthz`,
  and Rauthy migrated, initialised an empty production database and answered
  `{"db_healthy":true,"cache_healthy":true}` on `/auth/v1/health`. No other Ablative service ran.
- **One database, two schemas**: 48 tables in `rauthy`, 9 in `spicedb`, none in `public`;
  `rauthy.refinery_schema_history` and `spicedb.alembic_version` each in their own schema.
- **Isolation**: as `rauthy`, `select count(*) from spicedb.relation_tuple` failed with
  `permission denied for schema spicedb`; as `spicedb`, `select count(*) from rauthy.clients`
  failed with `permission denied for schema rauthy`.
- **Clients**: after first start the only client was the built-in `rauthy`. The bootstrap API
  key listed clients, read the default theme, and created a confidential client; a second create
  of the same id was refused `400 ID exists already`, and a wrong key `401`.
- **SpiceDB writes**: a fixture schema write and a relationship touch both answered with a
  `writtenAt` token and read back; test fixtures, not grants.
- **Backup and restore**, with Rauthy and SpiceDB stopped: `pg_dump --create -Fc` exited 0
  (110,868 bytes) and the `rauthy-data` volume archive was 8,194 bytes. `docker compose down -v`
  destroyed both volumes. `create`, the volume archive extracted, `up -d postgres`, then
  `pg_restore --dbname postgres --clean --create --if-exists --exit-on-error` exited 0. The
  restored database held clients `rauthy` and `platform`, one SpiceDB relationship, both roles'
  `search_path` settings and a database ACL granting CONNECT to `rauthy` and `spicedb` only.
  After `up -d`, Rauthy answered healthy with **the same four signing key ids** as before the
  loss, and SpiceDB answered `SERVING`: readiness repeated, issuer identity preserved.

## Measured by the gate

Nothing writes to this section automatically; each run is recorded here by hand.

**2026-09-27, DIRECTORY-002 review round 1**, `sh .land/gates.sh` from the repository root on
Dean's laptop (macOS, Docker answering `docker info`). This is one machine, one toolchain and
one image pull, so it is not independence of platform, and it is not the venue gate: the venue
gate's run of the identity leg, including ID001_PIN_CLONE against the venue's own recursive
clone, is still owed. Every leg exited 0, the identity leg included:

- `cargo clippy -p lys --all-features --test 'identity_*' -- -D warnings`: clean.
- `cargo test -p lys --all-features --test 'identity_*'`: seven tests in five targets, all
  passed: `id001_deploy_ready_configured_and_named` and `id001_pin_clone` (identity_deploy),
  `id001_deploy_refusal_config_and_database_address` and
  `id001_deploy_refusal_running_deployment` (identity_refusals),
  `id001_deploy_restart_preserves_issuer_and_contents` (identity_restart),
  `id001_shared_db_coexist_isolate_and_restore` (identity_shared_db) and
  `id001_theme_contrast` (identity_theme).
- ID001_THEME contrast, from the values the running Rauthy exported:

  | Pair | Platform | Cambium | Tier |
  | --- | --- | --- | --- |
  | text over bg (ink) | 16.25 | 16.25 | 4.5:1 |
  | text over bg_high (raised) | 13.48 | 13.48 | 4.5:1 |
  | text_high over ink | 15.61 | 15.61 | 4.5:1 |
  | btn_text over action | 9.87 | 9.87 | 4.5:1 |
  | theme_sun over ink | 5.24 | 5.24 | 3:1 |
  | theme_moon over ink | 5.83 | 3.97 | 3:1 |

  `contrast: 12 pairs measured, 12 at or above their tier`.
