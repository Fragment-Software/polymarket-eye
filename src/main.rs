use registration::seq::seq_register;
use utils::{
    common::{read_private_keys, read_proxies},
    logger::init_default_logger,
};

mod errors;
mod polymarket;
mod registration;
mod utils;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let _guard = init_default_logger();

    let signers = read_private_keys().await;
    let proxies = read_proxies().await;

    seq_register(signers, proxies).await?;

    Ok(())
}
