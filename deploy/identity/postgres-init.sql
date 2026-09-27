-- deploy/identity/postgres-init.sql
--
-- One PostgreSQL database for the identity product (ADR-005): Rauthy and
-- SpiceDB each get a least-privilege login role that owns exactly one schema,
-- and neither role can read the other's schema.
--
-- The local development service runs this file through PostgreSQL's own init
-- directory (/docker-entrypoint-initdb.d), connected as the superuser to the
-- database named by POSTGRES_DB, on the first start of an empty data volume
-- only. For a PostgreSQL on a network device the operator creates the database
-- and runs this same file once with psql as a superuser connected to it:
--
--   psql -v ON_ERROR_STOP=1 -h <host> -U postgres -d <database> \
--        -f deploy/identity/postgres-init.sql
--
-- with IDENTITY_DB_NAME, IDENTITY_RAUTHY_DB_PASSWORD and
-- IDENTITY_SPICEDB_DB_PASSWORD in its environment, read from the private env
-- file `lys identity prepare` writes. No password is written in this file; psql
-- reads each one from the environment with \getenv (PostgreSQL 15 or later).
--
-- A missing value stops the script by the name identity_secret_missing, so the
-- database is never initialised with an empty or default password.

\set ON_ERROR_STOP on

\getenv identity_db IDENTITY_DB_NAME
\getenv rauthy_password IDENTITY_RAUTHY_DB_PASSWORD
\getenv spicedb_password IDENTITY_SPICEDB_DB_PASSWORD

\if :{?identity_db}
\else
DO $$ BEGIN RAISE EXCEPTION 'identity_secret_missing: IDENTITY_DB_NAME is not set'; END $$;
\endif

\if :{?rauthy_password}
\else
DO $$ BEGIN RAISE EXCEPTION 'identity_secret_missing: IDENTITY_RAUTHY_DB_PASSWORD is not set'; END $$;
\endif

\if :{?spicedb_password}
\else
DO $$ BEGIN RAISE EXCEPTION 'identity_secret_missing: IDENTITY_SPICEDB_DB_PASSWORD is not set'; END $$;
\endif

-- The script must run inside the identity database itself, so that the schemas
-- below are created where the services will look for them.
SELECT current_database() = :'identity_db' AS in_identity_db \gset
\if :in_identity_db
\else
DO $$ BEGIN RAISE EXCEPTION 'identity_database_mismatch: connect to the database named by IDENTITY_DB_NAME'; END $$;
\endif

-- Login roles: no superuser, no database or role creation, no inherited grants.
CREATE ROLE identity_rauthy LOGIN PASSWORD :'rauthy_password'
    NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION;
CREATE ROLE identity_spicedb LOGIN PASSWORD :'spicedb_password'
    NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION;

-- Nobody but the two service roles (and the superuser) may connect, and the
-- shared public schema grants nothing: every table lives in a service schema.
REVOKE ALL ON DATABASE :"identity_db" FROM PUBLIC;
GRANT CONNECT ON DATABASE :"identity_db" TO identity_rauthy, identity_spicedb;
REVOKE ALL ON SCHEMA public FROM PUBLIC;

-- One schema per service, owned by its role. Ownership is the whole grant: a
-- role that owns no object in the other schema and holds no USAGE on it cannot
-- read it.
CREATE SCHEMA rauthy AUTHORIZATION identity_rauthy;
CREATE SCHEMA spicedb AUTHORIZATION identity_spicedb;
REVOKE ALL ON SCHEMA rauthy FROM PUBLIC;
REVOKE ALL ON SCHEMA spicedb FROM PUBLIC;

-- Both migration runners create their tables unqualified (Rauthy's refinery
-- history and tables, SpiceDB's migration version and tables), so each role's
-- search path names only its own schema. SpiceDB's connection string also
-- carries search_path=spicedb; the role default is what Rauthy relies on.
ALTER ROLE identity_rauthy IN DATABASE :"identity_db" SET search_path = rauthy;
ALTER ROLE identity_spicedb IN DATABASE :"identity_db" SET search_path = spicedb;
