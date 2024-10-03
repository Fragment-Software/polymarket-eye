use config::Config;
use db::database::Database;
use modules::registration::register_accounts;
use utils::logger::init_default_logger;

mod config;
mod db;
mod errors;
mod modules;
mod polymarket;
mod utils;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let _guard = init_default_logger();

    let config = Config::read_default().await;
    let db = Database::new().await?;

    register_accounts(db, config).await?;

    Ok(())
}
