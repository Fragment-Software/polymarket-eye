use modules::menu::menu;
use utils::logger::init_default_logger;

mod config;
mod db;
mod errors;
mod modules;
mod onchain;
mod polymarket;
mod utils;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let _guard = init_default_logger();

    if let Err(e) = menu().await {
        tracing::error!("Execution stopped with error: {e}");
    }

    Ok(())
}
