use serde::{de, Deserialize, Deserializer, Serialize};

use crate::onchain::constants::POLYGON_EXPLORER_TX_BASE_URL;

use super::typedefs::{Side, SignedOrder};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClobApiKeyResponseBody {
    pub api_key: String,
    pub secret: String,
    pub passphrase: String,
}

#[derive(Deserialize, Debug)]
pub struct GetTickSizeResponseBody {
    pub minimum_tick_size: f64,
}

#[derive(Serialize, Debug)]
pub struct TokenId<'a> {
    pub token_id: &'a str,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct OrderBookData {
    market: String,
    asset_id: String,
    timestamp: String,
    hash: String,
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Order {
    #[serde(deserialize_with = "string_to_f64")]
    pub price: f64,
    #[serde(deserialize_with = "string_to_f64")]
    pub size: f64,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct AccumulatedOrder {
    pub price: f64,
    pub size: f64,
    pub value: f64,
    pub net_value: f64,
    pub net_size: f64,
}

#[derive(Deserialize)]
pub struct NegRiskResponseBody {
    pub neg_risk: bool,
}

fn string_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<f64>().map_err(de::Error::custom)
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderRequest {
    pub order: SignedOrder,
    owner: String,
    order_type: OrderType,
}

#[allow(unused)]
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    Fok,
    Gtc,
    Gtd,
}

impl OrderRequest {
    pub fn new(signed_order: SignedOrder, owner: &str, order_type: Option<OrderType>) -> Self {
        let order_type = order_type.unwrap_or(OrderType::Fok);

        Self {
            order: signed_order,
            owner: owner.to_string(),
            order_type,
        }
    }
}

#[allow(unused)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlaceOrderResponseBody {
    pub error_msg: String,
    #[serde(rename = "orderID")]
    pub order_id: Option<String>,
    pub taking_amount: Option<String>,
    pub making_amount: Option<String>,
    pub status: Option<OrderStatus>,
    transactions_hashes: Option<Vec<String>>,
    pub success: Option<bool>,
}

impl PlaceOrderResponseBody {
    pub fn get_tx_hash(&self) -> String {
        format!(
            "{}{}",
            POLYGON_EXPLORER_TX_BASE_URL,
            self.transactions_hashes
                .as_ref()
                .unwrap()
                .first()
                .unwrap_or(&"MISSING".to_string())
        )
    }

    pub fn log_successful_placement(&self, side: Side, proxy_wallet_address: &str) {
        tracing::info!(
            "[{}] {proxy_wallet_address} | Placed an order with id {}. Making amount {}. Taking amount: {}. Tx hash: {}",
            side,
            self.order_id.as_ref().unwrap(),
            self.making_amount.as_ref().unwrap(),
            self.taking_amount.as_ref().unwrap(),
            self.get_tx_hash()
        );
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Live,
    Matched,
    Delayed,
    Unmatched,
}
