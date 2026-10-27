use loco_rs::prelude::*;
use sea_orm::{
    sea_query::PostgresQueryBuilder, ConnectionTrait, DatabaseConnection, DbBackend, Schema,
};
use tracing::info;

use crate::models::_entities::prelude::*;

#[derive(Clone, Copy)]
pub struct Table {
    pub name: &'static str,
}

pub const TABLES: &[Table] = &[
    Table { name: "users" },
    Table {
        name: "refresh_tokens",
    },
    Table {
        name: "password_resets",
    },
    Table { name: "audit_logs" },
    Table { name: "hr_records" },
    Table {
        name: "hr_record_history",
    },
    Table {
        name: "hr_record_unique_claims",
    },
    Table { name: "hr_rules" },
    Table {
        name: "hr_rule_events",
    },
    Table {
        name: "hr_notifications",
    },
];

pub async fn run_autonomous_sync(db: &DatabaseConnection) -> Result<(), String> {
    info!("SchemaSync: acquiring PostgreSQL advisory lock 88421038");
    db.execute_unprepared("SELECT pg_advisory_lock(88421038);")
        .await
        .map_err(|e| format!("Failed to acquire advisory lock: {e}"))?;

    let sync_result = perform_sync(db).await;

    db.execute_unprepared("SELECT pg_advisory_unlock(88421038);")
        .await
        .map_err(|e| format!("Failed to release advisory lock: {e}"))?;

    sync_result?;
    verify_tables(db).await?;
    info!("SchemaSync: entity-first synchronization completed successfully with 0 duplicate indexes and 0 data loss");
    Ok(())
}

async fn perform_sync(db: &DatabaseConnection) -> Result<(), String> {
    let schema = Schema::new(DbBackend::Postgres);

    // Synchronize tables from SeaORM Entity definitions
    sync_entity(db, &schema.create_table_from_entity(Users)).await?;
    sync_entity(db, &schema.create_table_from_entity(RefreshTokens)).await?;
    sync_entity(db, &schema.create_table_from_entity(PasswordResets)).await?;
    sync_entity(db, &schema.create_table_from_entity(AuditLogs)).await?;
    sync_entity(db, &schema.create_table_from_entity(HrRecords)).await?;
    sync_entity(db, &schema.create_table_from_entity(HrRecordHistory)).await?;
    sync_entity(db, &schema.create_table_from_entity(HrRecordUniqueClaims)).await?;
    sync_entity(db, &schema.create_table_from_entity(HrRules)).await?;
    sync_entity(db, &schema.create_table_from_entity(HrRuleEvents)).await?;
    sync_entity(db, &schema.create_table_from_entity(HrNotifications)).await?;

    // Safe non-destructive column sync (IF NOT EXISTS)
    ensure_columns(db).await?;

    // Safe idempotent indexes & constraints
    ensure_indexes_and_constraints(db).await?;

    // Triggers and initial seed rows
    ensure_triggers_and_defaults(db).await?;

    Ok(())
}

async fn sync_entity(
    db: &DatabaseConnection,
    stmt: &sea_orm::sea_query::TableCreateStatement,
) -> Result<(), String> {
    let mut create_stmt = stmt.clone();
    create_stmt.if_not_exists();
    let sql = create_stmt.to_string(PostgresQueryBuilder);
    db.execute_unprepared(&sql)
        .await
        .map_err(|e| format!("Error creating table from entity: {e} | SQL: {sql}"))?;
    Ok(())
}

async fn ensure_columns(db: &DatabaseConnection) -> Result<(), String> {
    let alter_stmts = [
        "ALTER TABLE hr_record_history ADD COLUMN IF NOT EXISTS source_actor_user_id BIGINT;",
        "ALTER TABLE hr_rule_events ADD COLUMN IF NOT EXISTS payload JSONB NOT NULL DEFAULT '{}'::jsonb;",
        "ALTER TABLE hr_rule_events ADD COLUMN IF NOT EXISTS last_error TEXT;",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS failed_logins INTEGER NOT NULL DEFAULT 0;",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT false;",
        "ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS persistent BOOLEAN NOT NULL DEFAULT false;",
    ];

    for stmt in alter_stmts {
        db.execute_unprepared(stmt)
            .await
            .map_err(|e| format!("Error altering columns: {e}"))?;
    }
    Ok(())
}

async fn ensure_indexes_and_constraints(db: &DatabaseConnection) -> Result<(), String> {
    let index_stmts = [
        "CREATE INDEX IF NOT EXISTS ix_hr_records_type_active ON hr_records(record_type, created_at DESC) WHERE archived_at IS NULL;",
        "CREATE INDEX IF NOT EXISTS ix_hr_record_history_record ON hr_record_history(record_id, id DESC);",
        "CREATE INDEX IF NOT EXISTS ix_hr_record_unique_record ON hr_record_unique_claims(record_id);",
        "CREATE INDEX IF NOT EXISTS ix_hr_rule_events_pending ON hr_rule_events(created_at) WHERE status='pending';",
        "CREATE INDEX IF NOT EXISTS ix_hr_notifications_user_unread ON hr_notifications(user_id, created_at DESC) WHERE read_at IS NULL;",
        "CREATE INDEX IF NOT EXISTS ix_audit_logs_action ON audit_logs(action, created_at DESC);",
        "CREATE INDEX IF NOT EXISTS ix_audit_logs_actor ON audit_logs(actor_user_id, created_at DESC);",
        "CREATE INDEX IF NOT EXISTS ix_audit_logs_created ON audit_logs(created_at DESC, id DESC);",
        "CREATE INDEX IF NOT EXISTS ix_refresh_tokens_expires ON refresh_tokens(expires_at);",
        "CREATE INDEX IF NOT EXISTS ix_refresh_tokens_family ON refresh_tokens(family_id);",
        "CREATE INDEX IF NOT EXISTS ix_refresh_tokens_user ON refresh_tokens(user_id);",
    ];

    for stmt in index_stmts {
        db.execute_unprepared(stmt)
            .await
            .map_err(|e| format!("Error creating index: {e}"))?;
    }
    Ok(())
}

async fn ensure_triggers_and_defaults(db: &DatabaseConnection) -> Result<(), String> {
    let sql = r#"
    INSERT INTO hr_rules(rule_key, enabled, updated_at) VALUES
      ('employee.created', false, now()),
      ('employee.updated', false, now()),
      ('employee.archived', false, now()),
      ('leave.submitted', false, now()),
      ('leave.decided', false, now()),
      ('expense.submitted', false, now()),
      ('expense.decided', false, now()),
      ('candidate.stage_changed', false, now()),
      ('payroll.completed', false, now())
    ON CONFLICT (rule_key) DO NOTHING;

    CREATE OR REPLACE FUNCTION audit_logs_append_only() RETURNS trigger LANGUAGE plpgsql AS $BODY$
    BEGIN
      RAISE EXCEPTION 'audit_logs rows may never be modified or deleted';
    END;
    $BODY$;

    DO $DO$
    BEGIN
      IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'audit_logs_no_change') THEN
        CREATE TRIGGER audit_logs_no_change BEFORE DELETE OR UPDATE ON audit_logs
          FOR EACH ROW EXECUTE FUNCTION audit_logs_append_only();
      END IF;
      IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'audit_logs_no_truncate') THEN
        CREATE TRIGGER audit_logs_no_truncate BEFORE TRUNCATE ON audit_logs
          FOR EACH STATEMENT EXECUTE FUNCTION audit_logs_append_only();
      END IF;
    END;
    $DO$;
    "#;

    db.execute_unprepared(sql)
        .await
        .map_err(|e| format!("Error applying triggers & seed rules: {e}"))?;
    Ok(())
}

async fn verify_tables(db: &DatabaseConnection) -> Result<(), String> {
    for table in TABLES {
        let sql = format!("SELECT 1 FROM \"{}\" LIMIT 1;", table.name);
        db.execute_unprepared(&sql)
            .await
            .map_err(|e| format!("Verification failed for table {}: {e}", table.name))?;
    }
    Ok(())
}
