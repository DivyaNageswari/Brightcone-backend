mod schema;

use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use loco_rs::{app::AppContext, prelude::Error};

pub use schema::{run_autonomous_sync, Column, ColumnType, Table, TABLES};

#[derive(Clone)]
pub struct Database {
    pool: Pool<AsyncPgConnection>,
}

impl Database {
    pub async fn connect(test: bool) -> loco_rs::Result<Self> {
        let sync_url = if test {
            let url = std::env::var("TEST_DATABASE_URL")
                .map_err(|_| Error::string("TEST_DATABASE_URL is required in test environment"))?;
            if database_name(&url) != Some("brightcone_test") {
                return Err(Error::string(
                    "TEST_DATABASE_URL must target the isolated brightcone_test database",
                ));
            }
            url
        } else {
            std::env::var("SCHEMA_SYNC_DATABASE_URL")
                .map_err(|_| Error::string("SCHEMA_SYNC_DATABASE_URL is required"))?
        };
        run_autonomous_sync(&sync_url)
            .await
            .map_err(database_error)?;
        if test {
            schema::reset_test_data(&sync_url)
                .await
                .map_err(database_error)?;
        }

        let runtime_url = if test {
            std::env::var("TEST_DATABASE_URL")
                .map_err(|_| Error::string("TEST_DATABASE_URL is required in test environment"))?
        } else {
            std::env::var("DATABASE_URL").map_err(|_| Error::string("DATABASE_URL is required"))?
        };
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(runtime_url);
        let pool = Pool::builder(manager)
            .max_size(16)
            .build()
            .map_err(database_error)?;
        Ok(Self { pool })
    }

    pub async fn get(
        &self,
    ) -> loco_rs::Result<impl std::ops::DerefMut<Target = AsyncPgConnection>> {
        self.pool.get().await.map_err(database_error)
    }

    pub fn from_context(ctx: &AppContext) -> loco_rs::Result<Self> {
        ctx.shared_store
            .get::<Self>()
            .ok_or_else(|| Error::string("Diesel database pool is not initialized"))
    }

    pub fn map_error(error: impl std::fmt::Display) -> Error {
        database_error(error)
    }
}

fn database_name(url: &str) -> Option<&str> {
    url.rsplit('/')
        .next()?
        .split('?')
        .next()
        .filter(|name| !name.is_empty())
}

fn database_error(error: impl std::fmt::Display) -> Error {
    tracing::error!(error = %error, "Diesel database operation failed");
    Error::string(&format!("Database operation failed: {error}"))
}
