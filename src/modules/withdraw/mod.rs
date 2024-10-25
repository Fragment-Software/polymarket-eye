use std::sync::Arc;

use alloy::{
    network::Ethereum,
    primitives::utils::format_units,
    providers::{Provider, ProviderBuilder},
    transports::Transport,
};
use alloy_chains::NamedChain;
use rand::{thread_rng, Rng};
use reqwest::Url;

use crate::{
    config::Config,
    db::{account::Account, database::Database},
    onchain::{client::EvmClient, constants::POLYGON_EXPLORER_TX_BASE_URL, types::token::Token},
    polymarket::api::{relayer::common::withdraw_usdc, typedefs::AmpCookie},
    utils::misc::pretty_sleep,
};

pub async fn withdraw_for_all(db: &mut Database, config: &Config) -> eyre::Result<()> {
    let mut rng = thread_rng();

    let provider = Arc::new(
        ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(Url::parse(&config.polygon_rpc_url)?),
    );

    while !db.0.is_empty() {
        let index = rng.gen_range(0..db.0.len());
        let account = &db.0[index];

        match withdraw_full_balance(account, provider.clone()).await {
            Ok(_) => {
                db.0.remove(index);
                pretty_sleep(config.withdraw_delay_range).await;
            }
            Err(e) => {
                tracing::error!("Withdrawal failed: {e}")
            }
        }
    }

    Ok(())
}

pub async fn withdraw_full_balance<P, T>(account: &Account, provider: Arc<P>) -> eyre::Result<()>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    let proxy_wallet_address = account.get_proxy_address();
    let evm_client = EvmClient::new(provider, account.get_private_key(), NamedChain::Polygon);

    let balance = evm_client
        .get_token_balance(&Token::USDCE, Some(proxy_wallet_address))
        .await?;

    let mut amp_cookie = AmpCookie::new();
    let polymarket_nonce = account.polymarket_nonce.as_ref().unwrap();
    let polymarket_session = account.polymarket_session.as_ref().unwrap();
    let proxy = account.proxy();
    let signer = account.signer();

    let to = account.get_address();

    let ui_amount = format_units(balance, "mwei")?;
    tracing::info!("Proxy wallet `{proxy_wallet_address}` withdrawing {ui_amount} USDC.e to {to}");

    let tx_hash = withdraw_usdc(
        signer,
        &mut amp_cookie,
        polymarket_nonce,
        polymarket_session,
        proxy.as_ref(),
        to,
        balance,
    )
    .await?;

    tracing::info!("USDC.e withdrawn: {POLYGON_EXPLORER_TX_BASE_URL}{tx_hash}");

    Ok(())
}
