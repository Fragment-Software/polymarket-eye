use std::{
    str::FromStr,
    sync::{Arc, RwLock},
};

use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use reqwest::Proxy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    polymarket::api::clob::schemas::ClobApiKeyResponseBody, utils::poly::get_proxy_wallet_address,
};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Account {
    private_key: String,
    proxy: Option<String>,
    address: String,
    is_registered: bool,
    funded: bool,
    pub proxy_address: String,
    pub polymarket_nonce: Option<String>,
    pub polymarket_session: Option<String>,
    #[serde(
        serialize_with = "serialize_arc_rwlock_option_string",
        deserialize_with = "deserialize_arc_rwlock_option_string"
    )]
    pub api_key: Arc<RwLock<Option<String>>>,
    #[serde(
        serialize_with = "serialize_arc_rwlock_option_string",
        deserialize_with = "deserialize_arc_rwlock_option_string"
    )]
    pub secret: Arc<RwLock<Option<String>>>,
    #[serde(
        serialize_with = "serialize_arc_rwlock_option_string",
        deserialize_with = "deserialize_arc_rwlock_option_string"
    )]
    pub passphrase: Arc<RwLock<Option<String>>>,
}

fn serialize_arc_rwlock_option_string<S>(
    data: &Arc<RwLock<Option<String>>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let data = data.read().unwrap();
    data.serialize(serializer)
}

fn deserialize_arc_rwlock_option_string<'de, D>(
    deserializer: D,
) -> Result<Arc<RwLock<Option<String>>>, D::Error>
where
    D: Deserializer<'de>,
{
    let data = Option::<String>::deserialize(deserializer)?;
    Ok(Arc::new(RwLock::new(data)))
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

    pub fn get_address(&self) -> Address {
        Address::from_str(&self.address).expect("Address to be valid")
    }

    pub fn get_proxy_address(&self) -> Address {
        Address::from_str(&self.proxy_address).unwrap()
    }

    pub fn update_credentials(&self, response: ClobApiKeyResponseBody) {
        *self.api_key.write().unwrap() = Some(response.api_key);
        *self.secret.write().unwrap() = Some(response.secret);
        *self.passphrase.write().unwrap() = Some(response.passphrase);
    }

    pub fn get_api_creds(&self) -> Option<ApiCreds> {
        let api_key_guard = self.api_key.read().unwrap();
        let passphrase_guard = self.passphrase.read().unwrap();
        let secret_guard = self.secret.read().unwrap();

        if let (Some(api_key), Some(api_passphrase), Some(api_secret)) =
            (&*api_key_guard, &*passphrase_guard, &*secret_guard)
        {
            Some(ApiCreds {
                api_key: api_key.clone(),
                api_passphrase: api_passphrase.clone(),
                api_secret: api_secret.clone(),
            })
        } else {
            None
        }
    }
}

pub struct ApiCreds {
    pub api_key: String,
    pub api_passphrase: String,
    pub api_secret: String,
}
