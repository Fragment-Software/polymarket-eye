use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

use super::signature_params::{RelayerRequestType, SignatureParams};

#[serde_with::skip_serializing_none]
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RelayerRequestBody<'a> {
    from: String,
    to: String,
    proxy_wallet: String,
    data: &'a str,
    nonce: Option<&'a str>,
    signature: &'a str,
    signature_params: SignatureParams<'a>,
    #[serde(rename = "type")]
    type_: RelayerRequestType,
}

impl<'a> RelayerRequestBody<'a> {
    pub fn set_from(&mut self, from: Address) {
        self.from = from.to_string()
    }

    pub fn with_from(mut self, from: Address) -> Self {
        self.set_from(from);
        self
    }

    pub fn set_to(&mut self, to: Address) {
        self.to = to.to_string();
    }

    pub fn with_to(mut self, to: Address) -> Self {
        self.set_to(to);
        self
    }

    pub fn set_proxy_wallet(&mut self, proxy_wallet_address: Address) {
        self.proxy_wallet = proxy_wallet_address.to_string();
    }

    pub fn with_proxy_wallet(mut self, proxy_wallet_address: Address) -> Self {
        self.set_proxy_wallet(proxy_wallet_address);
        self
    }

    pub fn set_data(&mut self, data: &'a str) {
        self.data = data;
    }

    pub fn with_data(mut self, data: &'a str) -> Self {
        self.set_data(data);
        self
    }

    pub fn set_nonce(&mut self, nonce: &'a str) {
        self.nonce = Some(nonce);
    }

    pub fn with_nonce(mut self, nonce: &'a str) -> Self {
        self.set_nonce(nonce);
        self
    }

    pub fn set_signature(&mut self, signature: &'a str) {
        self.signature = signature;
    }

    pub fn with_signature(mut self, signature: &'a str) -> Self {
        self.set_signature(signature);
        self
    }

    pub fn set_signature_params(&mut self, signature_params: SignatureParams<'a>) {
        self.signature_params = signature_params;
    }

    pub fn with_signature_params(mut self, signature_params: SignatureParams<'a>) -> Self {
        self.set_signature_params(signature_params);
        self
    }

    pub fn set_type(&mut self, req_type: RelayerRequestType) {
        self.type_ = req_type;
    }

    pub fn with_type(mut self, req_type: RelayerRequestType) -> Self {
        self.set_type(req_type);
        self
    }
}

#[allow(unused)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RelayerResponseBody {
    #[serde(rename = "transactionID")]
    pub transaction_id: String,
    pub transaction_hash: String,
    pub state: String,
}

#[derive(Deserialize, Debug)]
pub enum TransactionState {
    #[serde(rename = "STATE_NEW")]
    New,
    #[serde(rename = "STATE_EXECUTED")]
    Executed,
    #[serde(rename = "STATE_MINED")]
    Mined,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetTransactionStatusResponseBody {
    pub state: TransactionState,
    pub transaction_hash: String,
}

#[derive(Deserialize, Debug)]
pub struct GetRelayerNonceResponseBody {
    pub nonce: String,
}
