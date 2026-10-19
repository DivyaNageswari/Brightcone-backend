#!/bin/bash
# Runs once to set up roles and databases in PostgreSQL.
# Roles:
#   app_schema_sync: owns product tables and applies additive schema changes at API startup
#   app_runtime:     can read and write rows but cannot CREATE/ALTER/DROP anything (used by the running app)
#   app_queue:       used only by the Postgres job queue (creates tables in its own "queue" schema)
#   app_migrator:    legacy migration role retained for compatibility
set -euo pipefail

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname postgres <<SQL
DO \$\$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_schema_sync') THEN
    CREATE ROLE app_schema_sync LOGIN PASSWORD '${SCHEMA_SYNC_PASSWORD:-schema_sync_dev_pw}';
  END IF;
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_runtime') THEN
    CREATE ROLE app_runtime LOGIN PASSWORD '${RUNTIME_PASSWORD:-runtime_dev_pw}';
  END IF;
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_queue') THEN
    CREATE ROLE app_queue LOGIN PASSWORD '${QUEUE_PASSWORD:-queue_dev_pw}';
  END IF;
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_migrator') THEN
    CREATE ROLE app_migrator LOGIN PASSWORD '${MIGRATOR_PASSWORD:-migrator_dev_pw}';
  END IF;
END
\$\$;
SELECT 'CREATE DATABASE brightcone_dev OWNER app_schema_sync' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'brightcone_dev')\gexec
SELECT 'CREATE DATABASE brightcone_test OWNER app_schema_sync' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'brightcone_test')\gexec
SQL

for db in brightcone_dev brightcone_test; do
  psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$db" <<SQL
REVOKE ALL ON DATABASE ${db} FROM PUBLIC;
GRANT CONNECT ON DATABASE ${db} TO app_runtime, app_queue, app_schema_sync, app_migrator;
GRANT USAGE, CREATE ON SCHEMA public TO app_schema_sync, app_migrator;
GRANT USAGE ON SCHEMA public TO app_runtime;

-- Tables created by startup schema sync are readable/writable by the runtime role.
ALTER DEFAULT PRIVILEGES FOR ROLE app_schema_sync IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_runtime;
ALTER DEFAULT PRIVILEGES FOR ROLE app_schema_sync IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO app_runtime;

ALTER DEFAULT PRIVILEGES FOR ROLE app_migrator IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_runtime;
ALTER DEFAULT PRIVILEGES FOR ROLE app_migrator IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO app_runtime;

-- The job queue lives in its own schema.
CREATE SCHEMA IF NOT EXISTS queue AUTHORIZATION app_queue;
ALTER ROLE app_queue IN DATABASE ${db} SET search_path = queue;
SQL
done
