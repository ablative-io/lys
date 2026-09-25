-- One database, two least-privilege roles, two schema namespaces.
--
-- PostgreSQL runs this file itself from its init directory the first time the
-- data volume is empty. The role passwords come from the environment the venue
-- gives the container (RAUTHY_PG_PASSWORD, SPICEDB_PG_PASSWORD); psql reads
-- them with \getenv so no password is written here.
--
-- Each role owns its own schema and has its search path set to it, so both
-- migration runners create their tables in their own namespace without either
-- being told about the other. Neither role can read the other's schema, and
-- nothing lands in public.

\set ON_ERROR_STOP on
\getenv rauthy_password RAUTHY_PG_PASSWORD
\getenv spicedb_password SPICEDB_PG_PASSWORD

CREATE ROLE rauthy LOGIN PASSWORD :'rauthy_password';
CREATE ROLE spicedb LOGIN PASSWORD :'spicedb_password';

REVOKE ALL ON SCHEMA public FROM PUBLIC;
REVOKE ALL ON DATABASE :"DBNAME" FROM PUBLIC;
GRANT CONNECT ON DATABASE :"DBNAME" TO rauthy, spicedb;

CREATE SCHEMA rauthy AUTHORIZATION rauthy;
CREATE SCHEMA spicedb AUTHORIZATION spicedb;

ALTER ROLE rauthy SET search_path = rauthy;
ALTER ROLE spicedb SET search_path = spicedb;
