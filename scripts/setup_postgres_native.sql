-- ============================================================================
-- Brightcone HR Portal - Native PostgreSQL Setup Script
-- Run this script as the 'postgres' superuser (via pgAdmin Query Tool or psql)
-- ============================================================================

-- 1. Create Application Roles
DO $$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'app_schema_sync') THEN
    CREATE ROLE app_schema_sync LOGIN PASSWORD 'schema_sync_dev_pw';
  END IF;
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'app_runtime') THEN
    CREATE ROLE app_runtime LOGIN PASSWORD 'runtime_dev_pw';
  END IF;
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'app_queue') THEN
    CREATE ROLE app_queue LOGIN PASSWORD 'queue_dev_pw';
  END IF;
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'app_migrator') THEN
    CREATE ROLE app_migrator LOGIN PASSWORD 'migrator_dev_pw';
  END IF;
END
$$;

-- 2. Create Databases owned by app_schema_sync
SELECT 'CREATE DATABASE brightcone_dev OWNER app_schema_sync'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'brightcone_dev')\gexec

SELECT 'CREATE DATABASE brightcone_test OWNER app_schema_sync'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'brightcone_test')\gexec

-- 3. Connect to brightcone_dev to configure schemas & privileges
\c brightcone_dev

REVOKE ALL ON DATABASE brightcone_dev FROM PUBLIC;
GRANT CONNECT ON DATABASE brightcone_dev TO app_runtime, app_queue, app_schema_sync, app_migrator;
GRANT USAGE, CREATE ON SCHEMA public TO app_schema_sync, app_migrator;
GRANT USAGE ON SCHEMA public TO app_runtime;

ALTER DEFAULT PRIVILEGES FOR ROLE app_schema_sync IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_runtime;
ALTER DEFAULT PRIVILEGES FOR ROLE app_schema_sync IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO app_runtime;

ALTER DEFAULT PRIVILEGES FOR ROLE app_migrator IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_runtime;
ALTER DEFAULT PRIVILEGES FOR ROLE app_migrator IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO app_runtime;

CREATE SCHEMA IF NOT EXISTS queue AUTHORIZATION app_queue;
ALTER ROLE app_queue IN DATABASE brightcone_dev SET search_path = queue;

-- 4. Connect to brightcone_test to configure test schemas & privileges
\c brightcone_test

REVOKE ALL ON DATABASE brightcone_test FROM PUBLIC;
GRANT CONNECT ON DATABASE brightcone_test TO app_runtime, app_queue, app_schema_sync, app_migrator;
GRANT USAGE, CREATE ON SCHEMA public TO app_schema_sync, app_migrator;
GRANT USAGE ON SCHEMA public TO app_runtime;

ALTER DEFAULT PRIVILEGES FOR ROLE app_schema_sync IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_runtime;
ALTER DEFAULT PRIVILEGES FOR ROLE app_schema_sync IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO app_runtime;

ALTER DEFAULT PRIVILEGES FOR ROLE app_migrator IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_runtime;
ALTER DEFAULT PRIVILEGES FOR ROLE app_migrator IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO app_runtime;

CREATE SCHEMA IF NOT EXISTS queue AUTHORIZATION app_queue;
ALTER ROLE app_queue IN DATABASE brightcone_test SET search_path = queue;
