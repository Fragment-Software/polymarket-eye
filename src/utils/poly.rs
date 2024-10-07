use std::sync::Arc;

use alloy::{
    dyn_abi::SolType,
    primitives::{address, keccak256, Address, U256},
    signers::Signer,
    sol,
    sol_types::eip712_domain,
};

use cookie::Cookie;
use indexmap::IndexMap;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};

use crate::polymarket::api::typedefs::AmpCookie;

use super::constants::{INIT_CODE_HASH, PROXY_FACTORY_ADDRESS};

pub fn build_poly_headers(
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
) -> HeaderMap {
    amp_cookie.tick();
    let amp_value = amp_cookie.to_base64_url_encoded();

    let cookies = vec![
        ("polymarketnonce", polymarket_nonce),
        ("AMP_4572e28e5c", &amp_value),
        ("polymarketsession", polymarket_session),
        ("polymarketauthtype", "metamask"),
    ];

    let cookie_header_value = build_cookie_header(&cookies);

    vec![(COOKIE, &cookie_header_value)]
        .into_iter()
        .map(|(name, value)| (name, HeaderValue::from_str(value).unwrap()))
        .collect::<HeaderMap>()
}

pub fn get_proxy_wallet_address<S>(signer: Arc<S>) -> Address
where
    S: Signer,
{
    let encoded_address = <sol! { address }>::abi_encode(&signer.address());
    let salt = keccak256(encoded_address);
    PROXY_FACTORY_ADDRESS.create2(salt, INIT_CODE_HASH)
}

sol! {
    #[derive(Debug)]
    struct CreateProxy {
        address paymentToken;
        uint256 payment;
        address paymentReceiver;
    }
}

impl Default for CreateProxy {
    fn default() -> Self {
        Self {
            paymentToken: Address::ZERO,
            payment: U256::ZERO,
            paymentReceiver: Address::ZERO,
        }
    }
}

pub async fn sign_enable_trading_message<S>(signer: Arc<S>) -> String
where
    S: Signer + Send + Sync,
{
    let create_proxy = CreateProxy::default();

    let domain = eip712_domain! {
        name: "Polymarket Contract Proxy Factory",
        chain_id: 137,
        verifying_contract: address!("aacfeea03eb1561c4e67d661e40682bd20e3541b"),
    };

    let signed_message = signer
        .sign_typed_data(&create_proxy, &domain)
        .await
        .unwrap();

    const_hex::encode_prefixed(signed_message.as_bytes())
}

pub fn parse_cookies(header: &str) -> IndexMap<String, String> {
    header
        .split(';')
        .filter_map(|s| Cookie::parse(s.trim()).ok())
        .map(|cookie| (cookie.name().to_string(), cookie.value().to_string()))
        .collect()
}

pub fn build_cookie_header(cookies: &[(&str, &str)]) -> String {
    cookies
        .iter()
        .map(|(name, value)| {
            let cookie = Cookie::new(*name, *value);
            cookie.to_string()
        })
        .collect::<Vec<_>>()
        .join("; ")
}
