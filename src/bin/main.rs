use brightcone_hr_backend::app::{App, Migrator};
use loco_rs::cli;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    let _ = dotenvy::dotenv();
    cli::main::<App, Migrator>().await
}