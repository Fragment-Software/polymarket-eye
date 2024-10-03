use std::sync::Arc;

use alloy::primitives::Address;
use alloy_signer::Signer;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::utils::{constants::PROXY_FACTORY_ADDRESS, poly::get_proxy_wallet_address};

#[allow(unused)]
#[derive(Deserialize, Debug)]
pub struct LoginReponseBody {
    #[serde(rename = "type")]
    type_: String,
    address: String,
}

#[derive(Deserialize, Debug)]
pub struct GetNonceResponseBody {
    pub nonce: String,
}

#[allow(unused)]
#[serde_with::skip_serializing_none]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserResponseBody {
    pub id: String,
    pub name: String,
    pub user: u64,
    pub referral: String,
    pub created_at: String,
    pub utm_source: String,
    pub utm_medium: String,
    pub utm_campaign: String,
    pub utm_content: String,
    pub utm_term: String,
    pub wallet_activated: bool,
    pub pseudonym: String,
    pub display_username_public: bool,
    #[serde(rename = "_sync")]
    pub sync: bool,
    pub proxy_wallet: String,
    pub users: Vec<User>,
    pub is_close_only: bool,
}

#[serde_with::skip_serializing_none]
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequestBody<'a> {
    pub display_username_public: bool,
    pub email_opt_in: bool,
    pub name: String,
    pub proxy_wallet: String,
    pub pseudonym: String,
    pub referral: &'a str,

    pub utm_campaign: &'a str,
    pub utm_content: &'a str,
    pub utm_medium: &'a str,
    pub utm_source: &'a str,
    pub utm_term: &'a str,

    pub wallet_activated: bool,
    pub users: Vec<User>,
}

#[serde_with::skip_serializing_none]
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub address: String,
    pub is_external_auth: bool,
    pub provider: String,
    pub proxy_wallet: String,
    pub username: String,

    pub preferences: Vec<Preferences>,
    pub wallet_preferences: Vec<WalletPreferences>,

    pub id: Option<String>,
    pub blocked: Option<bool>,
    pub created_at: Option<String>,
    #[serde(rename = "profileID")]
    pub profile_id: Option<u64>,
    pub creator: Option<bool>,
    #[serde(rename = "mod")]
    pub mod_: Option<bool>,
    #[serde(rename = "_sync")]
    pub sync: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub email_notification_preferences: String,
    pub app_notification_preferences: String,
    pub market_interests: String,
    pub preferences_status: String,
    pub subscription_status: bool,

    pub id: Option<String>,
    #[serde(rename = "userID")]
    pub user_id: Option<u64>,
    #[serde(rename = "_sync")]
    pub sync: Option<bool>,
}

#[serde_with::skip_serializing_none]
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WalletPreferences {
    pub advanced_mode: bool,
    pub custom_gas_price: String,
    pub gas_preference: String,
    pub wallet_preferences_status: String,

    pub id: Option<String>,
    #[serde(rename = "userID")]
    pub user_id: Option<u64>,
    #[serde(rename = "_sync")]
    pub sync: Option<bool>,
}

impl<'a> CreateUserRequestBody<'a> {
    pub fn new<S: Signer>(signer: Arc<S>) -> Self {
        let now = Utc::now().timestamp_millis().to_string();
        let proxy_wallet_address = get_proxy_wallet_address(signer.clone());
        let name = format!("{proxy_wallet_address}-{now}");
        let user = User::new(signer.address(), proxy_wallet_address, &name);

        Self {
            display_username_public: true,
            email_opt_in: false,
            name,
            proxy_wallet: proxy_wallet_address.to_string(),
            pseudonym: proxy_wallet_address.to_string(),
            referral: "",
            utm_campaign: "",
            utm_content: "",
            utm_medium: "",
            utm_source: "",
            utm_term: "",
            wallet_activated: false,
            users: vec![user],
        }
    }
}

impl User {
    fn new(wallet_address: Address, proxy_wallet_address: Address, username: &str) -> Self {
        Self {
            address: wallet_address.to_string(),
            is_external_auth: true,
            provider: "metamask".to_string(),
            proxy_wallet: proxy_wallet_address.to_string(),
            username: username.to_string(),
            preferences: vec![Preferences::new()],
            wallet_preferences: vec![WalletPreferences::new()],
            id: None,
            blocked: None,
            created_at: None,
            profile_id: None,
            creator: None,
            mod_: None,
            sync: None,
        }
    }
}

impl Preferences {
    fn new() -> Self {
        Self {
            email_notification_preferences: "{\"generalEmail\":{\"sendEmails\":false},\"marketEmails\":{\"sendEmails\":false},\"newsletterEmails\":{\"sendEmails\":false},\"promotionalEmails\":{\"sendEmails\":false},\"eventEmails\":{\"sendEmails\":false,\"tagIds\":[\"2\",\"21\",\"1\",\"107\",\"596\",\"74\"]},\"orderFillEmails\":{\"sendEmails\":false,\"hideSmallFills\":true},\"resolutionEmails\":{\"sendEmails\":false}}".to_string(),
            app_notification_preferences: "{\"eventApp\":{\"sendApp\":true,\"tagIds\":[\"2\",\"21\",\"1\",\"107\",\"596\",\"74\"]},\"marketPriceChangeApp\":{\"sendApp\":true},\"orderFillApp\":{\"sendApp\":true,\"hideSmallFills\":true},\"resolutionApp\":{\"sendApp\":true}}".to_string(),
            market_interests: "[]".to_string(),
            preferences_status: "New/Existing - Created Prefs".to_string(),
            subscription_status: false,
            id: None,
            user_id: None,
            sync: None,
        }
    }
}

impl WalletPreferences {
    fn new() -> Self {
        Self {
            advanced_mode: false,
            custom_gas_price: "30".to_string(),
            gas_preference: "fast".to_string(),
            wallet_preferences_status: "New/Existing - Created Wallet Prefs".to_string(),
            id: None,
            user_id: None,
            sync: None,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUsernameRequestBody<'a> {
    display_username_public: bool,
    name: &'a str,
    referral: &'a str,
}

impl<'a> UpdateUsernameRequestBody<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            display_username_public: true,
            name,
            referral: "",
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreferencesRequestBody<'a> {
    email_notification_preferences: &'a str,
    market_interests: &'a str,
}

impl<'a> UpdatePreferencesRequestBody<'a> {
    pub fn new() -> Self {
        Self {
            email_notification_preferences: "{\"generalEmail\":{\"sendEmails\":true},\"marketEmails\":{\"sendEmails\":true},\"newsletterEmails\":{\"sendEmails\":true},\"promotionalEmails\":{\"sendEmails\":true},\"eventEmails\":{\"sendEmails\":true,\"tagIds\":[\"2\",\"21\",\"1\",\"107\",\"596\",\"74\"]},\"orderFillEmails\":{\"sendEmails\":true,\"hideSmallFills\":true},\"resolutionEmails\":{\"sendEmails\":true}}",
            market_interests: "[]",
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableTradingRequestBody<'a> {
    from: String,
    to: String,
    proxy_wallet: String,
    data: &'a str,
    signature: &'a str,
    signature_params: SignatureParams<'a>,
    #[serde(rename = "type")]
    type_: &'a str,
}

impl<'a> EnableTradingRequestBody<'a> {
    pub fn new<S: Signer>(
        signer: Arc<S>,
        proxy_wallet_address: Address,
        signature: &'a str,
    ) -> Self {
        Self {
            from: signer.address().to_string(),
            to: PROXY_FACTORY_ADDRESS.to_string(),
            proxy_wallet: proxy_wallet_address.to_string(),
            data: "0x",
            signature,
            signature_params: SignatureParams::default(),
            type_: "SAFE-CREATE",
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignatureParams<'a> {
    payment_token: &'a str,
    payment: &'a str,
    payment_receiver: &'a str,
}

impl<'a> Default for SignatureParams<'a> {
    fn default() -> Self {
        Self {
            payment_token: "0x0000000000000000000000000000000000000000",
            payment: "0",
            payment_receiver: "0x0000000000000000000000000000000000000000",
        }
    }
}

#[allow(unused)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EnableTradingResponseBody {
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
pub struct GetTransactionStatusResponse {
    pub state: TransactionState,
    pub transaction_hash: String,
}
