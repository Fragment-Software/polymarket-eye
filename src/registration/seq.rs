use std::{str::FromStr, sync::Arc};

use alloy::primitives::Address;
use alloy_signer_local::PrivateKeySigner;
use itertools::{EitherOrBoth, Itertools};
use reqwest::Proxy;

use crate::{
    polymarket::{
        api::{
            create_profile, enable_trading, get_nonce, login, update_preferences, update_username,
            wait_for_transaction_confirmation,
        },
        typedefs::{AmpCookie, AuthHeaderPayload},
    },
    utils::{
        common::{
            append_line_to_file, generate_random_username, pretty_sleep,
            sign_enable_trading_message,
        },
        constants::OUT_FILE_PATH,
    },
};

pub async fn seq_register(
    signers: Vec<Arc<PrivateKeySigner>>,
    proxies: Vec<Proxy>,
) -> eyre::Result<()> {
    for pair in signers.into_iter().zip_longest(proxies.into_iter()) {
        match pair {
            EitherOrBoth::Both(signer, proxy) => register_account(signer, Some(proxy)).await?,
            EitherOrBoth::Left(signer) => register_account(signer, None).await?,
            EitherOrBoth::Right(_) => {
                eyre::bail!("Amount of proxies is greater then amount of private keys")
            }
        }
    }

    Ok(())
}

async fn register_account(signer: Arc<PrivateKeySigner>, proxy: Option<Proxy>) -> eyre::Result<()> {
    tracing::info!("Wallet: `{}`", signer.address());
    tracing::info!("Creating profile");

    let (msg_nonce, polymarket_nonce) = get_nonce(proxy.as_ref()).await?;
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
    let preferences_id = profile.users[0].preferences[0].id.clone().unwrap();
    let proxy_address = Address::from_str(&profile.proxy_wallet)?.to_checksum(None);

    tracing::info!("Saving results to a file");

    append_line_to_file(
        OUT_FILE_PATH,
        &format!(
            "{}|{}|{}|{}",
            profile.users[0].address, proxy_address, profile_id, preferences_id
        ),
    )
    .await?;

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

    let signature = sign_enable_trading_message(signer.clone()).await;

    tracing::info!("Enabling trading");
    let tx_id = enable_trading(
        signer.clone(),
        &signature,
        &mut amp_cookie,
        &polymarket_nonce,
        &polymarket_session,
        None,
    )
    .await?;

    wait_for_transaction_confirmation(
        &tx_id,
        &mut amp_cookie,
        &polymarket_nonce,
        &polymarket_session,
        None,
        None,
        None,
    )
    .await?;

    pretty_sleep([10, 20]).await;

    Ok(())
}
