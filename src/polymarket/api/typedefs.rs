use std::sync::Arc;

use alloy::{
    primitives::{Address, U256},
    signers::Signer,
    sol,
    sol_types::eip712_domain,
};
use base64::{engine::general_purpose::URL_SAFE, prelude::BASE64_STANDARD, Engine};
use chrono::Utc;
use hmac::{Hmac, Mac};
use rand::{thread_rng, Rng};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Serialize;
use serde_json::Value;
use sha2::Sha256;
use uuid::Uuid;

use crate::{db::account::ApiCreds, utils::misc::get_timestamp_with_offset};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthHeaderPayload<'a> {
    address: String,
    chain_id: u64,
    nonce: &'a str,
    domain: &'a str,
    issued_at: String,
    expiration_time: String,
    uri: &'a str,
    statement: &'a str,
    version: &'a str,
}

impl<'a> AuthHeaderPayload<'a> {
    pub fn new(address: Address, msg_nonce: &'a str) -> Self {
        let (issued_at, expiration_time) = get_timestamp_with_offset(7 * 24);

        Self {
            address: address.to_string(),
            chain_id: 137,
            nonce: msg_nonce,
            domain: "polymarket.com",
            issued_at,
            expiration_time,
            uri: "https://polymarket.com",
            statement: "Welcome to Polymarket! Sign to connect.",
            version: "1",
        }
    }

    pub async fn get_auth_header_value<S>(&self, signer: Arc<S>) -> String
    where
        S: Signer + Send + Sync,
    {
        let message = self.generate_message(signer.clone());
        let signed_message = signer.sign_message(message.as_bytes()).await.unwrap();
        let signature = const_hex::encode_prefixed(signed_message.as_bytes());

        self.to_base64(&signature)
    }

    pub fn to_base64(&self, signature: &str) -> String {
        let msg_json_string = serde_json::to_string(self).unwrap();
        BASE64_STANDARD.encode(format!("{msg_json_string}:::{signature}"))
    }

    fn generate_message<S>(&self, signer: Arc<S>) -> String
    where
        S: Signer + Send + Sync,
    {
        [
            "polymarket.com wants you to sign in with your Ethereum account:",
            &format!("{}", signer.address()),
            "",
            self.statement,
            "",
            &format!("URI: {}", self.uri),
            &format!("Version: {}", self.version),
            &format!("Chain ID: {}", self.chain_id),
            &format!("Nonce: {}", self.nonce),
            &format!("Issued At: {}", self.issued_at),
            &format!("Expiration Time: {}", self.expiration_time),
        ]
        .join("\n")
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmpCookie {
    device_id: String,
    user_id: Option<String>,
    session_id: i64,
    opt_out: bool,
    last_event_time: i64,
    last_event_id: u32,
}

impl AmpCookie {
    pub fn new() -> Self {
        let device_id = Uuid::new_v4().to_string();
        let session_id = Utc::now().timestamp_millis();
        let last_event_time = session_id + thread_rng().gen_range(50..=1000);
        let last_event_id = thread_rng().gen_range(5..=40);

        Self {
            device_id,
            user_id: None,
            session_id,
            opt_out: false,
            last_event_time,
            last_event_id: last_event_id as u32,
        }
    }

    pub fn set_user_id(&mut self, user_id: Option<String>) {
        self.user_id = user_id
    }

    fn tick_last_event_id(&mut self) {
        self.last_event_id = thread_rng().gen_range(5..=40);
    }

    fn tick_last_event_time(&mut self) {
        self.last_event_time += thread_rng().gen_range(500..=3000);
    }

    pub fn tick(&mut self) {
        self.tick_last_event_id();
        self.tick_last_event_time();
    }

    pub fn to_base64_url_encoded(&self) -> String {
        let header_json_str = serde_json::to_string(self).unwrap();
        let url_encoded = urlencoding::encode(&header_json_str).to_string();
        BASE64_STANDARD.encode(url_encoded)
    }
}

sol! {
    #[derive(Debug)]
    struct ClobAuth {
        address address;
        string timestamp;
        uint256 nonce;
        string message;
    }
}

pub trait HeaderMapSerializeable {
    fn to_headermap(&self) -> HeaderMap
    where
        Self: Serialize,
    {
        let mut headers = HeaderMap::new();
        let value = serde_json::to_value(self).unwrap();

        if let Value::Object(map) = value {
            for (k, v) in map {
                if let Value::String(s) = v {
                    let header_name = HeaderName::from_bytes(k.as_bytes()).unwrap();
                    let header_value = HeaderValue::from_str(&s).unwrap();
                    headers.insert(header_name, header_value);
                }
            }
        }

        headers
    }
}

#[derive(Serialize)]
pub struct LayerOneClobAuthHeaders {
    poly_address: String,
    poly_nonce: String,
    poly_signature: String,
    poly_timestamp: String,
}

impl HeaderMapSerializeable for LayerOneClobAuthHeaders {}

impl LayerOneClobAuthHeaders {
    pub async fn new<S: Signer + Send + Sync>(signer: Arc<S>) -> Self {
        let timestamp = Utc::now().timestamp().to_string();
        let signature = Self::sign_clob_auth_message(signer.clone(), &timestamp).await;

        Self {
            poly_address: signer.address().to_string(),
            poly_nonce: "0".to_string(),
            poly_signature: signature,
            poly_timestamp: timestamp,
        }
    }

    pub async fn sign_clob_auth_message<S: Signer + Send + Sync>(
        signer: Arc<S>,
        timestamp: &str,
    ) -> String {
        let message = "This message attests that I control the given wallet";

        let auth = ClobAuth {
            address: signer.address(),
            timestamp: timestamp.to_string(),
            nonce: U256::ZERO,
            message: message.to_string(),
        };

        let domain = eip712_domain! {
            name: "ClobAuthDomain",
            version: "1",
            chain_id: 137,
        };

        let signed_message = signer.sign_typed_data(&auth, &domain).await.unwrap();

        const_hex::encode_prefixed(signed_message.as_bytes())
    }
}

#[derive(Serialize, Debug)]
pub struct LayerTwoClobAuthHeaders {
    poly_address: String,
    poly_signature: String,
    poly_timestamp: String,
    poly_api_key: String,
    poly_passphrase: String,
}

impl HeaderMapSerializeable for LayerTwoClobAuthHeaders {}

impl LayerTwoClobAuthHeaders {
    pub fn new(
        address: &str,
        api_creds: ApiCreds,
        method: &str,
        path: &str,
        body: Option<&str>,
        timestamp: Option<String>,
    ) -> Self {
        let timestamp = timestamp.unwrap_or(Utc::now().timestamp().to_string());
        let signature =
            Self::build_hmac_signature(&timestamp, &api_creds.api_secret, method, path, body);

        Self {
            poly_address: address.to_string(),
            poly_signature: signature,
            poly_timestamp: timestamp,
            poly_api_key: api_creds.api_key.to_string(),
            poly_passphrase: api_creds.api_passphrase.to_string(),
        }
    }

    fn build_hmac_signature(
        timestamp: &str,
        secret: &str,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> String {
        let mut message = format!("{}{}{}", timestamp, method, path);
        if let Some(body) = body {
            message = format!("{}{}", message, body);
        }

        let bs64_secret = URL_SAFE.decode(secret).unwrap();

        let mut mac = Hmac::<Sha256>::new_from_slice(&bs64_secret).unwrap();
        mac.update(message.as_bytes());

        let result = mac.finalize();
        let hmac_bytes = result.into_bytes();

        URL_SAFE.encode(hmac_bytes)
    }
}
