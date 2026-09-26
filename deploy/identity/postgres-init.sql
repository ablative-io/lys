-- deploy/identity/postgres-init.sql
--
-- Creates the two service roles and their schema namespaces in the one identity
-- database (ADR-005). PostgreSQL's own entrypoint runs this file from
-- /docker-entrypoint-initdb.d exactly once, when the data directory is first
-- initialised, as the bootstrap superuser named by POSTGRES_USER, connected to
-- the database named by POSTGRES_DB. It is never run against a database that
-- already holds data.
--
-- On a database that does not run in this compose file (a network device the
-- operator names), an administrator runs the same file with psql and the same
-- three environment variables set; nothing here assumes the local container.
--
-- Invariants this file establishes, and which ID001_SHARED_DB measures:
--   * rauthy and spicedb are ordinary login roles: no superuser, no createdb,
--     no createrole, no replication, no bypassrls.
--   * each role owns exactly one schema, named after it, and its search_path
--     in this database is that schema alone, so both migration runners
--     (Rauthy's refinery history and SpiceDB's migration table) land in their
--     own namespace and cannot collide.
--   * neither role has USAGE on the other's schema, and PUBLIC holds nothing
--     on the database or on the public schema, so a service role cannot read
--     the other service's tables.
--
-- No password is written here. Each is read from the environment by psql's
-- \getenv, and an absent one stops the file by name before any role exists.

\set ON_ERROR_STOP on

\getenv identity_database POSTGRES_DB
\getenv rauthy_password LYS_IDENTITY_RAUTHY_DB_PASSWORD
\getenv spicedb_password LYS_IDENTITY_SPICEDB_DB_PASSWORD

\if :{?identity_database}
\else
DO $$ BEGIN RAISE EXCEPTION 'missing_secret: POSTGRES_DB is not set'; END $$;
\endif
\if :{?rauthy_password}
\else
DO $$ BEGIN RAISE EXCEPTION 'missing_secret: LYS_IDENTITY_RAUTHY_DB_PASSWORD is not set'; END $$;
\endif
\if :{?spicedb_password}
\else
DO $$ BEGIN RAISE EXCEPTION 'missing_secret: LYS_IDENTITY_SPICEDB_DB_PASSWORD is not set'; END $$;
\endif

-- Nothing is granted to PUBLIC: connection and schema use are per role.
REVOKE ALL ON DATABASE :"identity_database" FROM PUBLIC;
REVOKE ALL ON SCHEMA public FROM PUBLIC;

CREATE ROLE rauthy LOGIN PASSWORD :'rauthy_password'
    NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS NOINHERIT;
CREATE ROLE spicedb LOGIN PASSWORD :'spicedb_password'
    NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS NOINHERIT;

GRANT CONNECT ON DATABASE :"identity_database" TO rauthy;
GRANT CONNECT ON DATABASE :"identity_database" TO spicedb;

CREATE SCHEMA rauthy AUTHORIZATION rauthy;
CREATE SCHEMA spicedb AUTHORIZATION spicedb;

REVOKE ALL ON SCHEMA rauthy FROM PUBLIC;
REVOKE ALL ON SCHEMA spicedb FROM PUBLIC;

ALTER ROLE rauthy IN DATABASE :"identity_database" SET search_path = rauthy;
ALTER ROLE spicedb IN DATABASE :"identity_database" SET search_path = spicedb;
