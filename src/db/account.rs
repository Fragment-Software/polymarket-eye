use std::{str::FromStr, sync::Arc};

use alloy_signer_local::PrivateKeySigner;
use reqwest::Proxy;
use serde::{Deserialize, Serialize};

use crate::utils::poly::get_proxy_wallet_address;

#[derive(Serialize, Deserialize, Debug)]
pub struct Account {
    private_key: String,
    proxy: Option<String>,
    address: String,
    is_registered: bool,
    proxy_address: String,
    polymarket_nonce: Option<String>,
    polymarket_session: Option<String>,
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
            polymarket_session: None,
            polymarket_nonce: None,
            is_registered: false,
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
}
