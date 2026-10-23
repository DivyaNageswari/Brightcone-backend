pub mod schema;

use loco_rs::app::AppContext;
pub use schema::{run_autonomous_sync, Table, TABLES};
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct Database(pub DatabaseConnection);

impl Database {
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }

    #[must_use]
    pub fn conn(&self) -> &DatabaseConnection {
        &self.0
    }

    pub fn from_context(ctx: &AppContext) -> loco_rs::Result<DatabaseConnection> {
        Ok(ctx.db.clone())
    }

    pub fn get(&self) -> loco_rs::Result<&DatabaseConnection> {
        Ok(&self.0)
    }
}

impl std::ops::Deref for Database {
    type Target = DatabaseConnection;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}