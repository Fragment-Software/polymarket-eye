use std::{str::FromStr, sync::Arc};

use alloy::{
    dyn_abi::SolType,
    primitives::{address, keccak256, Address, U256},
    sol,
    sol_types::eip712_domain,
};

use alloy_signer::Signer;
use alloy_signer_local::PrivateKeySigner;
use chrono::{DateTime, Duration, Utc};
use cookie::Cookie;
use fake::{faker::internet::en::Username, Fake};
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressStyle};
use rand::{thread_rng, Rng};
use reqwest::{
    header::{HeaderMap, HeaderValue, COOKIE},
    Proxy,
};
use term_size::dimensions;
use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncWriteExt},
};

use crate::polymarket::typedefs::AmpCookie;

use super::constants::{
    INIT_CODE_HASH, PRIVATE_KEYS_FILE_PATH, PROXIES_FILE_PATH, PROXY_FACTORY_ADDRESS,
};

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
            paymentToken: address!("0000000000000000000000000000000000000000"),
            payment: U256::from(0),
            paymentReceiver: address!("0000000000000000000000000000000000000000"),
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

    format!("0x{}", hex::encode(signed_message.as_bytes()))
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

pub fn get_timestamp_with_offset(hours_to_add: i64) -> (String, String) {
    let current_time: DateTime<Utc> = Utc::now();
    let adjusted_time = current_time + Duration::hours(hours_to_add);
    (
        current_time.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        adjusted_time.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
    )
}

pub fn get_proxy_wallet_address<S>(signer: Arc<S>) -> Address
where
    S: Signer,
{
    let encoded_address = <sol! { address }>::abi_encode(&signer.address());
    let salt = keccak256(encoded_address);
    PROXY_FACTORY_ADDRESS.create2(salt, INIT_CODE_HASH)
}

pub fn generate_random_username() -> String {
    let mut username: String = Username().fake();
    username = username.replace("_", "-");

    let mut rng = thread_rng();

    if rng.gen_bool(0.3) {
        let random_number: u16 = rng.gen_range(1..=999);
        format!("{}{}", username, random_number)
    } else {
        username
    }
}

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

async fn read_file_lines(path: &str) -> eyre::Result<Vec<String>> {
    let file = tokio::fs::read(path).await?;
    let mut lines = file.lines();

    let mut lines_vec = vec![];
    while let Some(line) = lines.next_line().await? {
        lines_vec.push(line)
    }

    Ok(lines_vec)
}

pub async fn read_private_keys() -> Vec<Arc<PrivateKeySigner>> {
    let private_keys = read_file_lines(PRIVATE_KEYS_FILE_PATH)
        .await
        .expect("Private keys file to be valid");

    private_keys
        .into_iter()
        .map(|pk| Arc::new(PrivateKeySigner::from_str(&pk).expect("Private key to be valid")))
        .collect()
}

pub async fn read_proxies() -> Vec<Proxy> {
    let proxies = read_file_lines(PROXIES_FILE_PATH)
        .await
        .expect("Private keys file to be valid");

    proxies
        .into_iter()
        .map(|proxy| Proxy::all(proxy).expect("Proxy to be valid"))
        .collect()
}

pub async fn append_line_to_file(file_path: &str, line: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .await?;

    file.write_all(format!("{}\n", line).as_bytes()).await?;
    Ok(())
}

pub async fn pretty_sleep(sleep_range: [u64; 2]) {
    let random_sleep_duration_secs = random_in_range(sleep_range);

    let pb = ProgressBar::new(random_sleep_duration_secs);

    let term_width = dimensions().map(|(w, _)| w - 2).unwrap_or(40);
    let bar_width = if term_width > 20 { term_width - 20 } else { 20 };

    pb.set_style(
        ProgressStyle::default_bar()
            .template(&format!(
                "{{spinner:.green}} [{{elapsed_precise}}] [{{bar:{bar_width}.cyan/blue}}] {{pos}}/{{len}}s"
            ))
            .expect("Invalid progress bar template.")
            .progress_chars("#>-"),
    );

    let step = std::time::Duration::from_secs(1);

    for _ in 0..random_sleep_duration_secs {
        pb.inc(1);
        tokio::time::sleep(step).await;
    }

    pb.finish_with_message("Done!");
}

pub fn random_in_range<T>(range: [T; 2]) -> T
where
    T: rand::distributions::uniform::SampleUniform + PartialOrd + Copy,
{
    let start = range[0];
    let end = range[1];

    let inclusive_range = if start <= end {
        start..=end
    } else {
        end..=start
    };

    rand::thread_rng().gen_range(inclusive_range)
}
