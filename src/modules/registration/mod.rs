use std::{str::FromStr, sync::Arc};

use alloy::{
    network::Ethereum,
    primitives::{bytes, Address, Bytes},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    transports::Transport,
};
use reqwest::{Proxy, StatusCode, Url};

use crate::{
    config::Config,
    db::{account::Account, database::Database},
    errors::custom::CustomError,
    onchain::{constants::POLYGON_EXPLORER_TX_BASE_URL, multicall::check_token_approvals},
    polymarket::api::{
        clob::{
            endpoints::{create_api_key, derive_api_key},
            schemas::ClobApiKeyResponseBody,
        },
        relayer::{
            common::{approve_tokens, enable_trading},
            endpoints::wait_for_transaction_confirmation,
        },
        typedefs::{AmpCookie, AuthHeaderPayload},
        user::endpoints::{
            create_profile, get_auth_nonce, get_user, login, update_preferences, update_username,
        },
    },
    utils::{
        misc::{generate_random_username, pretty_sleep, swap_ip_address},
        poly::sign_enable_trading_message,
    },
};

const ZERO_BYTES: Bytes = bytes!("");

pub async fn register_accounts(mut db: Database, config: &Config) -> eyre::Result<()> {
    let provider = Arc::new(
        ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(Url::parse(&config.polygon_rpc_url)?),
    );

    while let Some(account) =
        db.get_random_account_with_filter(|account: &Account| !account.get_is_registered())
    {
        register_account(account, config, provider.clone()).await?;

        account.set_is_registered(true);
        db.update();

        pretty_sleep(config.registration_sleep_range).await;
    }

    Ok(())
}

async fn register_account<P, T>(
    account: &mut Account,
    config: &Config,
    provider: Arc<P>,
) -> eyre::Result<()>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    let signer = account.signer();
    let proxy = account.proxy();

    tracing::info!("Wallet address: `{}`", signer.address());

    if config.mobile_proxies {
        tracing::info!("Changing IP address");
        swap_ip_address(&config.swap_ip_link).await?;
    }

    tracing::info!("Creating a profile");

    let (msg_nonce, polymarket_nonce) = get_auth_nonce(proxy.as_ref()).await?;
    account.set_polymarket_nonce(&polymarket_nonce);

    let auth_header_value = AuthHeaderPayload::new(signer.address(), &msg_nonce)
        .get_auth_header_value(signer.clone())
        .await;

    let mut amp_cookie = AmpCookie::new();

    let polymarket_session = login(
        &amp_cookie.to_base64_url_encoded(),
        &polymarket_nonce,
        &auth_header_value,
    )
    .await?;

    account.set_polymarket_session(&polymarket_session);

    let user_exists = get_user(
        &mut amp_cookie,
        &polymarket_nonce,
        &polymarket_session,
        proxy.as_ref(),
    )
    .await?
    .is_some();

    if !user_exists {
        let profile = create_profile(
            signer.clone(),
            proxy.as_ref(),
            &mut amp_cookie,
            &polymarket_nonce,
            &polymarket_session,
        )
        .await?;

        let username = generate_random_username();
        let profile_id = profile.id;
        let preferences_id = profile.users[0].preferences.as_ref().unwrap()[0]
            .id
            .clone()
            .unwrap();
        let proxy_address = Address::from_str(&profile.proxy_wallet)?.to_checksum(None);

        amp_cookie.set_user_id(Some(proxy_address));

        tracing::info!("Updating prefernces");
        update_preferences(
            &preferences_id,
            &mut amp_cookie,
            &polymarket_nonce,
            &polymarket_session,
            proxy.as_ref(),
        )
        .await?;

        tracing::info!("Updating username");
        update_username(
            &username,
            &profile_id,
            &mut amp_cookie,
            &polymarket_nonce,
            &polymarket_session,
            None,
        )
        .await?;
    }

    let proxy_wallet_activated =
        check_if_proxy_wallet_activated(provider.clone(), account.get_proxy_address()).await?;

    if !proxy_wallet_activated {
        let signature = sign_enable_trading_message(signer.clone()).await;

        tracing::info!("Activating a proxy wallet");
        let tx_id = enable_trading(
            signer.clone(),
            &signature,
            &mut amp_cookie,
            &polymarket_nonce,
            &polymarket_session,
            None,
        )
        .await?;

        let tx_hash = wait_for_transaction_confirmation(
            &tx_id,
            &mut amp_cookie,
            &polymarket_nonce,
            &polymarket_session,
            None,
            None,
            None,
        )
        .await?;

        tracing::info!("Proxy wallet acitvated: {POLYGON_EXPLORER_TX_BASE_URL}{tx_hash}");
    }

    let api_creds_response = create_or_derive_api_key(signer.clone(), proxy.as_ref()).await?;
    account.update_credentials(api_creds_response);

    let approved = check_token_approvals(provider.clone(), account.get_proxy_address()).await?;

    if !approved {
        tracing::info!("Giving token approvals");
        let tx_id = approve_tokens(
            signer,
            &mut amp_cookie,
            &polymarket_nonce,
            &polymarket_session,
            proxy.as_ref(),
        )
        .await?;

        let tx_hash = wait_for_transaction_confirmation(
            &tx_id,
            &mut amp_cookie,
            &polymarket_nonce,
            &polymarket_session,
            None,
            None,
            None,
        )
        .await?;

        tracing::info!("Approval succeded: {POLYGON_EXPLORER_TX_BASE_URL}{tx_hash}");
    }

    Ok(())
}

async fn create_or_derive_api_key(
    signer: Arc<PrivateKeySigner>,
    proxy: Option<&Proxy>,
) -> eyre::Result<ClobApiKeyResponseBody> {
    tracing::info!("Deriving API key");

    let response = match derive_api_key(signer.clone(), proxy).await {
        Ok(response) => response,
        Err(CustomError::HttpStatusError { status, .. }) if status == StatusCode::BAD_REQUEST => {
            tracing::warn!("Account has no existing API key, creating one");
            create_api_key(signer.clone(), proxy).await?
        }
        Err(e) => eyre::bail!("Failed to derive API key: {e}"),
    };

    Ok(response)
}

async fn check_if_proxy_wallet_activated<P, T>(
    provider: Arc<P>,
    proxy_address: Address,
) -> eyre::Result<bool>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    let code = provider.get_code_at(proxy_address).await?;

    let exists = match code == ZERO_BYTES {
        true => false,
        false => true,
    };

    Ok(exists)
}
