#!/bin/bash
# Runs once, the first time the Postgres volume is created.
# Three roles on purpose:
#   app_schema_sync: owns product tables and applies additive schema changes at API startup
#   app_runtime:  can read and write rows but cannot CREATE/ALTER/DROP anything (used by the running app)
#   app_queue:    used only by the Postgres job queue. Loco creates its queue tables at start-up, so this
#                 role may create objects, but only inside its own "queue" schema, never next to app tables.
set -euo pipefail

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname postgres <<SQL
CREATE ROLE app_schema_sync LOGIN PASSWORD '${SCHEMA_SYNC_PASSWORD}';
CREATE ROLE app_runtime  LOGIN PASSWORD '${RUNTIME_PASSWORD}';
CREATE ROLE app_queue    LOGIN PASSWORD '${QUEUE_PASSWORD}';
CREATE DATABASE brightcone_dev  OWNER app_schema_sync;
CREATE DATABASE brightcone_test OWNER app_schema_sync;
SQL

for db in brightcone_dev brightcone_test; do
  psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$db" <<SQL
REVOKE ALL ON DATABASE ${db} FROM PUBLIC;
GRANT CONNECT ON DATABASE ${db} TO app_runtime, app_queue;
GRANT USAGE ON SCHEMA public TO app_runtime;
-- Tables created by startup schema sync are readable/writable by the runtime role.
-- Tighter rules (for example no DELETE on audit_logs) are set by schema sync.
ALTER DEFAULT PRIVILEGES FOR ROLE app_schema_sync IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_runtime;
ALTER DEFAULT PRIVILEGES FOR ROLE app_schema_sync IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO app_runtime;
-- The job queue lives in its own schema.
CREATE SCHEMA queue AUTHORIZATION app_queue;
ALTER ROLE app_queue IN DATABASE ${db} SET search_path = queue;
SQL
done
