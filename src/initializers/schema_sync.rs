use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Initializer},
    environment::Environment,
    Result,
};
use sea_orm::Database;
use crate::database::run_autonomous_sync;

/// Lifecycle initializer executing entity-first autonomous schema synchronization
/// on boot before the HTTP server accepts any connections.
pub struct SchemaSyncInitializer;

#[async_trait]
impl Initializer for SchemaSyncInitializer {
    fn name(&self) -> String {
        "schema_sync".to_string()
    }

    async fn before_run(&self, ctx: &AppContext) -> Result<()> {
        let test = ctx.environment == Environment::Test;
        let sync_url = if test {
            std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://app_schema_sync:schema_sync_dev_pw@localhost:5432/brightcone_test".to_string())
        } else {
            std::env::var("SCHEMA_SYNC_DATABASE_URL")
                .or_else(|_| std::env::var("DATABASE_URL"))
                .unwrap_or_else(|_| "postgres://app_schema_sync:schema_sync_dev_pw@localhost:5432/brightcone_dev".to_string())
        };
        let db = Database::connect(&sync_url)
            .await
            .map_err(|e| loco_rs::Error::string(&format!("Schema sync connection failed: {e}")))?;
        run_autonomous_sync(&db)
            .await
            .map_err(|e| loco_rs::Error::string(&format!("Schema sync failed: {e}")))?;
        tracing::info!("SchemaSyncInitializer: autonomous schema synchronization completed successfully before HTTP listening");
        Ok(())
    }
}