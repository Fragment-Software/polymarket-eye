use std::sync::Arc;

use alloy::primitives::Address;
use alloy_signer::Signer;
use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::Utc;
use rand::{thread_rng, Rng};
use serde::Serialize;
use uuid::Uuid;

use crate::utils::common::get_timestamp_with_offset;

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
        let signature = format!("0x{}", hex::encode(signed_message.as_bytes()));

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
        self.last_event_time = self.session_id + thread_rng().gen_range(500..=3000);
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
