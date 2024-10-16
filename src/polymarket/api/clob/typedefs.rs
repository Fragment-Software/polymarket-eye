use std::{fmt::Display, str::FromStr};

use alloy::{
    primitives::{Address, Signature, U256},
    sol,
};
use eyre::bail;
use serde::Serialize;
use serde_repr::Serialize_repr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[repr(u8)]
#[serde(rename_all = "UPPERCASE")]
pub enum Side {
    Buy = 0,
    Sell = 1,
}

impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buy => write!(f, "BUY"),
            Self::Sell => write!(f, "SELL"),
        }
    }
}

impl Default for Side {
    fn default() -> Self {
        Self::Buy
    }
}

impl TryFrom<u8> for Side {
    type Error = eyre::Report;

    fn try_from(value: u8) -> eyre::Result<Self> {
        match value {
            0 => Ok(Side::Buy),
            1 => Ok(Side::Sell),
            _ => bail!("expected: 0 => Side::Buy\n1 => Side::Sell"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr)]
#[repr(u8)]
pub enum SignatureType {
    Eoa = 0,
    PolyProxy = 1,
    PolyGnosisSafe = 2,
}

impl TryFrom<u8> for SignatureType {
    type Error = eyre::Report;

    fn try_from(value: u8) -> eyre::Result<Self> {
        match value {
            0 => Ok(SignatureType::Eoa),
            1 => Ok(SignatureType::PolyProxy),
            2 => Ok(SignatureType::PolyGnosisSafe),
            _ => bail!("expected: 0 => SignatureType::Eoa\n1 => SignatureType::PolyProxy\n2 => SignatureType::PolyGnosisSafe, but got {}", value),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrderData {
    pub maker: String,
    pub taker: String,
    pub token_id: String,
    pub maker_amount: String,
    pub taker_amount: String,
    pub side: Side,
    pub fee_rate_bps: String,
    pub nonce: String,
    pub signer: Option<String>,
    pub expiration: Option<String>,
    pub signature_type: Option<SignatureType>,
}

sol! {
    #[derive(Debug)]
    struct Order {
        uint256 salt;
        address maker;
        address signer;
        address taker;
        uint256 tokenId;
        uint256 makerAmount;
        uint256 takerAmount;
        uint256 expiration;
        uint256 nonce;
        uint256 feeRateBps;
        uint8 side;
        uint8 signatureType;
    }
}

impl Order {
    pub fn new(salt: &str, order_data: OrderData) -> eyre::Result<Self> {
        Ok(Self {
            salt: U256::from_str_radix(salt, 10)?,
            maker: Address::from_str(&order_data.maker)?,
            signer: Address::from_str(&order_data.signer.unwrap())?,
            taker: Address::from_str(&order_data.taker)?,
            tokenId: U256::from_str_radix(&order_data.token_id, 10)?,
            makerAmount: U256::from_str_radix(&order_data.maker_amount, 10)?,
            takerAmount: U256::from_str_radix(&order_data.taker_amount, 10)?,
            expiration: U256::from_str_radix(&order_data.expiration.unwrap(), 10)?,
            nonce: U256::from_str_radix(&order_data.nonce, 10)?,
            feeRateBps: U256::from_str_radix(&order_data.fee_rate_bps, 10)?,
            side: order_data.side as u8,
            signatureType: order_data.signature_type.unwrap() as u8,
        })
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SignedOrder {
    pub salt: usize,
    pub maker: String,
    pub signer: String,
    pub taker: String,
    pub token_id: String,
    pub maker_amount: String,
    pub taker_amount: String,
    pub side: Side,
    pub expiration: String,
    pub nonce: String,
    pub fee_rate_bps: String,
    pub signature_type: SignatureType,
    pub signature: String,
}

impl SignedOrder {
    pub fn new(order: Order, signature: Signature) -> eyre::Result<Self> {
        Ok(Self {
            salt: order.salt.try_into().unwrap(),
            maker: order.maker.to_string(),
            signer: order.signer.to_string(),
            taker: order.taker.to_string(),
            token_id: order.tokenId.to_string(),
            maker_amount: order.makerAmount.to_string(),
            taker_amount: order.takerAmount.to_string(),
            expiration: order.expiration.to_string(),
            nonce: order.nonce.to_string(),
            fee_rate_bps: order.feeRateBps.to_string(),
            side: Side::try_from(order.side)?,
            signature_type: SignatureType::try_from(order.signatureType)?,
            signature: const_hex::encode_prefixed(signature.as_bytes()),
        })
    }
}

pub struct OrderRawAmounts {
    pub side: Side,
    pub raw_maker_amount: f64,
    pub raw_taker_amount: f64,
}

impl OrderRawAmounts {
    pub fn new(side: &Side, raw_maker_amount: f64, raw_taker_amount: f64) -> Self {
        Self {
            side: side.clone(),
            raw_maker_amount,
            raw_taker_amount,
        }
    }
}

pub struct BuyOrderRawAmounts {
    pub raw_maker_amount: f64,
    pub raw_taker_amount: f64,
}

impl BuyOrderRawAmounts {
    pub fn new(raw_maker_amount: f64, raw_taker_amount: f64) -> Self {
        Self {
            raw_maker_amount,
            raw_taker_amount,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TickSize {
    OneTenth,
    OneHundredth,
    OneThousandth,
    TenThousandth,
}

impl TickSize {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "0.1" => Some(TickSize::OneTenth),
            "0.01" => Some(TickSize::OneHundredth),
            "0.001" => Some(TickSize::OneThousandth),
            "0.0001" => Some(TickSize::TenThousandth),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            TickSize::OneTenth => "0.1",
            TickSize::OneHundredth => "0.01",
            TickSize::OneThousandth => "0.001",
            TickSize::TenThousandth => "0.0001",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateOrderOptions {
    pub tick_size: TickSize,
    pub neg_risk: Option<bool>,
}

impl CreateOrderOptions {
    pub fn new(tick_size: TickSize, neg_risk: Option<bool>) -> Self {
        Self {
            tick_size,
            neg_risk,
        }
    }
}

#[derive(Default)]
pub struct UserOrder {
    pub token_id: String,
    pub price: f64,
    pub size: f64,
    pub side: Side,
    pub fee_rate_bps: Option<f64>,
    pub nonce: Option<u64>,
    pub expiration: Option<u64>,
    pub taker: Option<String>,
}

#[allow(unused)]
impl UserOrder {
    pub fn set_token_id(&mut self, token_id: &str) {
        self.token_id = token_id.to_string();
    }

    pub fn with_token_id(mut self, token_id: &str) -> Self {
        self.set_token_id(token_id);
        self
    }

    pub fn set_price(&mut self, price: f64) {
        self.price = price;
    }

    pub fn with_price(mut self, price: f64) -> Self {
        self.set_price(price);
        self
    }

    pub fn set_size(&mut self, size: f64) {
        self.size = size;
    }

    pub fn with_size(mut self, size: f64) -> Self {
        self.set_size(size);
        self
    }

    pub fn set_side(&mut self, side: Side) {
        self.side = side;
    }

    pub fn with_side(mut self, side: Side) -> Self {
        self.set_side(side);
        self
    }

    pub fn set_fee_rate_bps(&mut self, fee_rate_bps: f64) {
        self.fee_rate_bps = Some(fee_rate_bps);
    }

    pub fn with_fee_rate_bps(mut self, fee_rate_bps: f64) -> Self {
        self.set_fee_rate_bps(fee_rate_bps);
        self
    }

    pub fn set_nonce(&mut self, nonce: u64) {
        self.nonce = Some(nonce);
    }

    pub fn with_nonce(mut self, nonce: u64) -> Self {
        self.set_nonce(nonce);
        self
    }

    pub fn set_expiration(&mut self, expiration: u64) {
        self.expiration = Some(expiration);
    }

    pub fn with_expiration(mut self, expiration: u64) -> Self {
        self.set_expiration(expiration);
        self
    }

    pub fn set_taker(&mut self, taker: String) {
        self.taker = Some(taker);
    }

    pub fn with_taker(mut self, taker: String) -> Self {
        self.set_taker(taker);
        self
    }
}

pub struct UserMarketOrder {
    pub token_id: String,
    pub price: Option<f64>,
    pub amount: f64,
    pub fee_rate_bps: Option<f64>,
    pub nonce: Option<u64>,
    pub taker: Option<String>,
}

impl UserMarketOrder {
    pub fn new(
        token_id: String,
        amount: f64,
        price: Option<f64>,
        fee_rate_bps: Option<f64>,
        nonce: Option<u64>,
        taker: Option<String>,
    ) -> Self {
        Self {
            token_id,
            price,
            amount,
            fee_rate_bps,
            nonce,
            taker,
        }
    }
}
