use std::{str::FromStr, sync::Arc};

use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use reqwest::{Proxy, StatusCode};

use crate::{
    config::Config,
    db::{account::Account, database::Database},
    errors::custom::CustomError,
    onchain::constants::POLYGON_EXPLORER_TX_BASE_URL,
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

pub async fn register_accounts(mut db: Database, config: &Config) -> eyre::Result<()> {
    while let Some(account) =
        db.get_random_account_with_filter(|account: &Account| !account.get_is_registered())
    {
        register_account(account, config).await?;

        account.set_is_registered(true);
        db.update();

        pretty_sleep(config.registration_sleep_range).await;
    }

    Ok(())
}

async fn register_account(account: &mut Account, config: &Config) -> eyre::Result<()> {
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

        // TODO: check if the proxy wallet is activated
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

    // TODO: check if approvals are given
    if !user_exists {
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
        Err(CustomError::Request(e)) if e.status() == Some(StatusCode::BAD_REQUEST) => {
            tracing::warn!("Account has no existing API key, creating one");
            create_api_key(signer.clone(), proxy).await?
        }
        Err(e) => eyre::bail!("Failed to derive API key: {e}"),
    };

    Ok(response)
}
