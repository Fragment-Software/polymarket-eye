use std::sync::Arc;

use alloy::{
    primitives::{bytes, Address, U256},
    signers::Signer,
    sol_types::SolCall,
};
use reqwest::Proxy;

use crate::{
    errors::custom::CustomError,
    onchain::client::IERC20::transferCall,
    polymarket::api::typedefs::AmpCookie,
    utils::{constants::PROXY_FACTORY_ADDRESS, poly::get_proxy_wallet_address},
};

use super::{
    constants::{
        CONDITIONAL_TOKENS_CONTRACT_ADDRESS, MULTISEND_CONTRACT_ADDRESS,
        UCHILD_ERC20_PROXY_CONTRACT_ADDRESS,
    },
    endpoints::{get_nonce, send_relayer_transaction},
    schemas::RelayerRequestBody,
    signature_params::{RelayerRequestType, SignatureParams},
    tx_builder::{get_multisend_calldata, get_packed_signature, RelayerTransaction},
};

pub fn get_approve_bundle() -> Vec<RelayerTransaction> {
    vec![
        (UCHILD_ERC20_PROXY_CONTRACT_ADDRESS, bytes!("095ea7b30000000000000000000000004d97dcd97ec945f40cf65f87097ace5ea0476045ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")),
        (UCHILD_ERC20_PROXY_CONTRACT_ADDRESS, bytes!("095ea7b30000000000000000000000004bfb41d5b3570defd03c39a9a4d8de6bd8b8982effffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")),
        (CONDITIONAL_TOKENS_CONTRACT_ADDRESS, bytes!("a22cb4650000000000000000000000004bfb41d5b3570defd03c39a9a4d8de6bd8b8982e0000000000000000000000000000000000000000000000000000000000000001")),
        (UCHILD_ERC20_PROXY_CONTRACT_ADDRESS, bytes!("095ea7b3000000000000000000000000c5d563a36ae78145c45a50134d48a1215220f80affffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")),
        (UCHILD_ERC20_PROXY_CONTRACT_ADDRESS, bytes!("095ea7b3000000000000000000000000d91e80cf2e7be2e162c6513ced06f1dd0da35296ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")),
        (CONDITIONAL_TOKENS_CONTRACT_ADDRESS, bytes!("a22cb465000000000000000000000000c5d563a36ae78145c45a50134d48a1215220f80a0000000000000000000000000000000000000000000000000000000000000001")),
        (CONDITIONAL_TOKENS_CONTRACT_ADDRESS, bytes!("a22cb465000000000000000000000000d91e80cf2e7be2e162c6513ced06f1dd0da352960000000000000000000000000000000000000000000000000000000000000001"))
    ].into_iter().map(|(address, data)| RelayerTransaction::new(0, address, U256::ZERO, data)).collect::<Vec<_>>()
}

pub async fn enable_trading<S: Signer>(
    signer: Arc<S>,
    signature: &str,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
    proxy: Option<&Proxy>,
) -> Result<String, CustomError> {
    let signature_params = SignatureParams::default()
        .with_payment_token()
        .with_payment()
        .with_payment_receiver();

    let body = RelayerRequestBody::default()
        .with_from(signer.address())
        .with_to(PROXY_FACTORY_ADDRESS)
        .with_proxy_wallet(get_proxy_wallet_address(signer))
        .with_data("0x")
        .with_signature(signature)
        .with_signature_params(signature_params)
        .with_type(RelayerRequestType::SafeCreate);

    let transaction_response = send_relayer_transaction(
        proxy,
        body,
        amp_cookie,
        polymarket_nonce,
        polymarket_session,
    )
    .await?;

    Ok(transaction_response.transaction_id)
}

pub async fn withdraw_usdc<S: Signer + Send + Sync>(
    signer: Arc<S>,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
    proxy: Option<&Proxy>,
    to: Address,
    amount: U256,
) -> Result<String, CustomError> {
    let nonce = get_nonce(
        signer.address(),
        proxy,
        amp_cookie,
        polymarket_nonce,
        polymarket_session,
    )
    .await?;

    let data = transferCall { to, amount }.abi_encode();
    let packed_signature = get_packed_signature(
        signer.clone(),
        0,
        U256::from(nonce),
        data.clone(),
        UCHILD_ERC20_PROXY_CONTRACT_ADDRESS,
    )
    .await?;

    let data_hex = const_hex::encode_prefixed(data);
    let nonce_str = nonce.to_string();

    let signature_params = SignatureParams::default()
        .with_gas_price()
        .with_operation("0")
        .with_safe_txn_gas()
        .with_base_gas()
        .with_gas_token()
        .with_refund_receiver();

    let body = RelayerRequestBody::default()
        .with_from(signer.address())
        .with_to(UCHILD_ERC20_PROXY_CONTRACT_ADDRESS)
        .with_proxy_wallet(get_proxy_wallet_address(signer))
        .with_data(&data_hex)
        .with_nonce(&nonce_str)
        .with_signature(&packed_signature)
        .with_signature_params(signature_params)
        .with_type(RelayerRequestType::Safe);

    let transaction_response = send_relayer_transaction(
        proxy,
        body,
        amp_cookie,
        polymarket_nonce,
        polymarket_session,
    )
    .await?;

    Ok(transaction_response.transaction_hash)
}

pub async fn approve_tokens<S: Signer + Send + Sync>(
    signer: Arc<S>,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
    proxy: Option<&Proxy>,
) -> Result<String, CustomError> {
    let nonce = get_nonce(
        signer.address(),
        proxy,
        amp_cookie,
        polymarket_nonce,
        polymarket_session,
    )
    .await?;

    let transactions = get_approve_bundle();

    let data = get_multisend_calldata(transactions);
    let packed_signature = get_packed_signature(
        signer.clone(),
        1,
        U256::from(nonce),
        data.clone(),
        MULTISEND_CONTRACT_ADDRESS,
    )
    .await?;
    let data_hex = const_hex::encode_prefixed(data);
    let nonce_str = nonce.to_string();

    let signature_params = SignatureParams::default()
        .with_gas_price()
        .with_operation("1")
        .with_safe_txn_gas()
        .with_base_gas()
        .with_gas_token()
        .with_refund_receiver();

    let body = RelayerRequestBody::default()
        .with_from(signer.address())
        .with_to(MULTISEND_CONTRACT_ADDRESS)
        .with_proxy_wallet(get_proxy_wallet_address(signer))
        .with_data(&data_hex)
        .with_nonce(&nonce_str)
        .with_signature(&packed_signature)
        .with_signature_params(signature_params)
        .with_type(RelayerRequestType::Safe);

    let transaction_response = send_relayer_transaction(
        proxy,
        body,
        amp_cookie,
        polymarket_nonce,
        polymarket_session,
    )
    .await?;

    Ok(transaction_response.transaction_id)
}
