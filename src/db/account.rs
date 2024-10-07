use std::{str::FromStr, sync::Arc};

use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use reqwest::Proxy;
use serde::{Deserialize, Serialize};

use crate::{
    polymarket::api::user::schemas::ClobApiKeyResponseBody, utils::poly::get_proxy_wallet_address,
};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Account {
    private_key: String,
    proxy: Option<String>,
    address: String,
    is_registered: bool,
    funded: bool,
    pub proxy_address: String,
    pub polymarket_nonce: Option<String>,
    pub polymarket_session: Option<String>,
    pub api_key: Option<String>,
    pub secret: Option<String>,
    pub passphrase: Option<String>,
}

impl Account {
    pub fn new(private_key: &str, proxy: Option<String>) -> Self {
        let signer =
            Arc::new(PrivateKeySigner::from_str(private_key).expect("Private key to be valid"));
        let address = signer.address();
        let proxy_address = get_proxy_wallet_address(signer);

        Self {
            private_key: private_key.to_string(),
            proxy,
            address: address.to_string(),
            proxy_address: proxy_address.to_string(),
            ..Default::default()
        }
    }

    pub fn get_is_registered(&self) -> bool {
        self.is_registered
    }

    pub fn set_is_registered(&mut self, is_registered: bool) {
        self.is_registered = is_registered
    }

    pub fn proxy(&self) -> Option<Proxy> {
        self.proxy
            .as_ref()
            .map(|proxy| Proxy::all(proxy).expect("Proxy to be valid"))
    }

    pub fn signer(&self) -> Arc<PrivateKeySigner> {
        Arc::new(PrivateKeySigner::from_str(&self.private_key).unwrap())
    }

    pub fn set_polymarket_session(&mut self, polymarket_session: &str) {
        self.polymarket_session = Some(polymarket_session.to_string())
    }

    pub fn set_polymarket_nonce(&mut self, polymarket_nonce: &str) {
        self.polymarket_nonce = Some(polymarket_nonce.to_string())
    }

    pub fn get_funded(&self) -> bool {
        self.funded
    }

    pub fn set_funded(&mut self, funded: bool) {
        self.funded = funded
    }

    pub fn get_private_key(&self) -> &str {
        &self.private_key
    }

    pub fn get_proxy_address(&self) -> Address {
        Address::from_str(&self.proxy_address).unwrap()
    }

    pub fn update_credentials(&mut self, response: ClobApiKeyResponseBody) {
        self.api_key = Some(response.api_key);
        self.secret = Some(response.secret);
        self.passphrase = Some(response.passphrase);
    }
}
