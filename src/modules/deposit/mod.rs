use std::sync::Arc;

use alloy::{
    network::Ethereum,
    primitives::{utils::format_units, U256},
    providers::{Provider, ProviderBuilder},
    transports::Transport,
};
use alloy_chains::NamedChain;
use reqwest::Url;

use crate::{
    config::Config,
    db::{account::Account, database::Database},
    onchain::{client::EvmClient, types::token::Token},
    utils::misc::{pretty_sleep, random_in_range},
};

pub async fn deposit_to_accounts(mut db: Database, config: &Config) -> eyre::Result<()> {
    let provider = Arc::new(
        ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(Url::parse(&config.polygon_rpc_url)?),
    );

    while let Some(account) = db.get_random_account_with_filter(|a| !a.get_funded()) {
        process_account(provider.clone(), account, config).await?;
        db.update();

        pretty_sleep(config.deposit_sleep_range).await;
    }

    Ok(())
}

async fn process_account<P, T>(
    provider: Arc<P>,
    account: &mut Account,
    config: &Config,
) -> eyre::Result<()>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    let proxy_wallet_address = account.get_proxy_address();
    let amount = random_in_range(config.usdc_amount_deposit_range);
    let token = Token::USDCE;

    let client = EvmClient::new(
        provider.clone(),
        account.get_private_key(),
        NamedChain::Polygon,
    );

    tracing::info!(
        "Wallet address: `{}`. Proxy wallet address: `{}`",
        client.address(),
        proxy_wallet_address
    );

    let (proxy_wallet_balance, wallet_balance) = tokio::try_join!(
        client.get_token_balance(&token, Some(proxy_wallet_address)),
        client.get_token_balance(&token, None)
    )?;

    let mut value = token.to_wei(amount);

    if value > wallet_balance {
        value = wallet_balance;
    }

    if config.ignore_existing_balance {
        client.transfer(proxy_wallet_address, value, &token).await?;
    } else if proxy_wallet_balance > U256::ZERO {
        let ui_amount = format_units(proxy_wallet_balance, "mwei")?;
        tracing::warn!("Proxy wallet already holds {} {}", ui_amount, Token::USDCE);
    } else {
        client.transfer(proxy_wallet_address, value, &token).await?;
    }

    account.set_funded(true);

    Ok(())
}
