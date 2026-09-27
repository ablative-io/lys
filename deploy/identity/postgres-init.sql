-- Roles and schema namespaces for the identity product's one PostgreSQL database.
--
-- The bundled PostgreSQL service runs this file once, through its own init directory
-- (/docker-entrypoint-initdb.d), as the bootstrap superuser, connected to the new
-- database. A database on a network device takes the same file from its operator:
--
--   RAUTHY_DB_PASSWORD=... SPICEDB_DB_PASSWORD=... \
--     psql -h <host> -U <superuser> -d identity -f deploy/identity/postgres-init.sql
--
-- Passwords are read from the environment with psql's \getenv, never written here;
-- the database is the one psql is connected to (its DBNAME variable).
-- Each service role owns one schema, has that schema alone on its search_path, and
-- has no usage on the other's, so neither migration runner can see or collide with
-- the other and neither role can read the other's tables.

\set ON_ERROR_STOP on

\getenv rauthy_password RAUTHY_DB_PASSWORD
\getenv spicedb_password SPICEDB_DB_PASSWORD

\if :{?rauthy_password}
\else
  DO $$ BEGIN RAISE EXCEPTION 'secret_missing: RAUTHY_DB_PASSWORD'; END $$;
\endif
\if :{?spicedb_password}
\else
  DO $$ BEGIN RAISE EXCEPTION 'secret_missing: SPICEDB_DB_PASSWORD'; END $$;
\endif

-- Nobody but the named roles may connect or create anything in public.
REVOKE ALL ON DATABASE :"DBNAME" FROM PUBLIC;
REVOKE ALL ON SCHEMA public FROM PUBLIC;

CREATE ROLE rauthy LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS
  PASSWORD :'rauthy_password';
CREATE ROLE spicedb LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS
  PASSWORD :'spicedb_password';

GRANT CONNECT, TEMPORARY ON DATABASE :"DBNAME" TO rauthy, spicedb;

CREATE SCHEMA rauthy AUTHORIZATION rauthy;
CREATE SCHEMA spicedb AUTHORIZATION spicedb;
REVOKE ALL ON SCHEMA rauthy FROM PUBLIC;
REVOKE ALL ON SCHEMA spicedb FROM PUBLIC;

ALTER ROLE rauthy IN DATABASE :"DBNAME" SET search_path = rauthy;
ALTER ROLE spicedb IN DATABASE :"DBNAME" SET search_path = spicedb;
