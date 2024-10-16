use std::{ops::Div, str::FromStr, sync::Arc};

use alloy::{
    dyn_abi::Eip712Domain,
    primitives::{utils::parse_units, Address},
    signers::Signer,
    sol_types::eip712_domain,
};
use chrono::Utc;
use rand::Rng;

use crate::polymarket::api::clob::constants::{PROTOCOL_NAME, PROTOCOL_VERSION};

use super::{
    constants::{get_contract_config, RoundingConfig, MATIC_CONTRACTS, ROUNDING_CONFIG},
    math::{adjust_amount, ClobPrecision},
    typedefs::{
        BuyOrderRawAmounts, CreateOrderOptions, Order, OrderData, OrderRawAmounts, Side,
        SignatureType, SignedOrder, UserMarketOrder, UserOrder,
    },
};

pub struct OrderBuilder<'a, S>
where
    S: Signer + Send + Sync,
{
    pub signer: Arc<S>,
    chain_id: u64,
    signature_type: Option<SignatureType>,
    funder_address: Option<&'a str>,
}

impl<'a, S> OrderBuilder<'a, S>
where
    S: Signer + Send + Sync,
{
    pub fn new(
        signer: Arc<S>,
        chain_id: u64,
        signature_type: Option<SignatureType>,
        funder_address: Option<&'a str>,
    ) -> Self {
        Self {
            signer: signer.clone(),
            chain_id,
            signature_type,
            funder_address,
        }
    }

    pub async fn build_signed_order(
        &self,
        user_order: UserOrder,
        options: CreateOrderOptions,
    ) -> eyre::Result<SignedOrder> {
        let signer_address = self.signer.address().to_string();

        let maker = match self.funder_address {
            Some(address) => address,
            None => &signer_address,
        };

        let contract_config = get_contract_config(self.chain_id).unwrap_or(&MATIC_CONTRACTS);

        let order_data = self.build_order_creation_args(
            &signer_address,
            maker,
            self.signature_type,
            &user_order,
            &ROUNDING_CONFIG[options.tick_size.as_str()],
        );

        let exchange_contract = match options.neg_risk.unwrap_or(false) {
            true => contract_config.neg_risk_exchange,
            false => contract_config.exchange,
        };

        self.create_signed_order(order_data, Address::from_str(exchange_contract)?)
            .await
    }

    pub async fn build_signed_market_buy_order(
        &self,
        user_market_order: UserMarketOrder,
        options: CreateOrderOptions,
    ) -> eyre::Result<SignedOrder> {
        let signer_address = self.signer.address().to_string();

        let maker = match self.funder_address {
            Some(address) => address,
            None => &signer_address,
        };

        let contract_config = get_contract_config(self.chain_id).unwrap_or(&MATIC_CONTRACTS);

        let order_data = self.build_market_buy_order_creation_args(
            &signer_address,
            maker,
            self.signature_type,
            user_market_order,
            &ROUNDING_CONFIG[options.tick_size.as_str()],
        );

        let exchange_contract = match options.neg_risk.unwrap_or(false) {
            true => contract_config.neg_risk_exchange,
            false => contract_config.exchange,
        };

        self.create_signed_order(order_data, Address::from_str(exchange_contract)?)
            .await
    }

    async fn create_signed_order(
        &self,
        order_data: OrderData,
        verifying_contract: Address,
    ) -> eyre::Result<SignedOrder> {
        let order = self.build_order(order_data)?;
        let order_domain = self.get_order_domain(verifying_contract);

        let order_signature = self.signer.sign_typed_data(&order, &order_domain).await?;

        SignedOrder::new(order, order_signature)
    }

    fn build_order(&self, mut order_data: OrderData) -> eyre::Result<Order> {
        if order_data.signer.is_none() || order_data.signer.as_ref().unwrap().is_empty() {
            order_data.signer = Some(order_data.maker.clone());
        }

        let signer_address = self.signer.address().to_string();

        if order_data.signer.as_ref().unwrap() != &signer_address {
            eyre::bail!("Signer does not match");
        }

        if order_data.expiration.is_none() || order_data.expiration.as_ref().unwrap().is_empty() {
            order_data.expiration = Some("0".to_string());
        }

        if order_data.signature_type.is_none() {
            order_data.signature_type = Some(SignatureType::PolyGnosisSafe);
        }

        let order = Order::new(&self.generate_salt(), order_data)?;

        Ok(order)
    }

    fn generate_salt(&self) -> String {
        let now = Utc::now().timestamp_millis() as u128;

        let random_value: u128 = rand::thread_rng().gen_range(0..now);

        random_value.to_string()
    }

    fn get_order_domain(&self, verifying_contract: Address) -> Eip712Domain {
        eip712_domain! {
            name: PROTOCOL_NAME,
            version: PROTOCOL_VERSION,
            chain_id: self.chain_id,
            verifying_contract: verifying_contract,
        }
    }

    fn build_market_buy_order_creation_args(
        &self,
        signer: &str,
        maker: &str,
        signature_type: Option<SignatureType>,
        user_market_order: UserMarketOrder,
        round_config: &RoundingConfig,
    ) -> OrderData {
        let price = user_market_order.price.unwrap_or(1f64);

        let BuyOrderRawAmounts {
            raw_maker_amount,
            raw_taker_amount,
        } = self.get_market_buy_order_raw_amounts(user_market_order.amount, price, round_config);

        let maker_amount = parse_units(&raw_maker_amount.to_string(), "MWEI")
            .unwrap()
            .to_string();
        let taker_amount = parse_units(&raw_taker_amount.to_string(), "MWEI")
            .unwrap()
            .to_string();

        let taker = match user_market_order.taker.clone() {
            Some(taker) => taker,
            None => Address::ZERO.to_string(),
        };

        let fee_rate_bps = match user_market_order.fee_rate_bps {
            Some(fee_rate) => fee_rate.to_string(),
            None => "0".to_string(),
        };

        let nonce = match user_market_order.nonce {
            Some(nonce) => nonce.to_string(),
            None => "0".to_string(),
        };

        OrderData {
            maker: maker.to_string(),
            taker,
            token_id: user_market_order.token_id,
            maker_amount,
            taker_amount,
            side: Side::Buy,
            fee_rate_bps,
            nonce,
            signer: Some(signer.to_string()),
            expiration: Some("0".to_string()),
            signature_type,
        }
    }

    fn get_market_buy_order_raw_amounts(
        &self,
        amount: f64,
        price: f64,
        round_config: &RoundingConfig,
    ) -> BuyOrderRawAmounts {
        let raw_maker_amount = amount.round_down(round_config.size);
        let raw_price = price.round_down(round_config.price);

        let raw_taker_amount = adjust_amount(raw_maker_amount.div(raw_price), round_config.amount);

        BuyOrderRawAmounts::new(raw_maker_amount, raw_taker_amount)
    }

    fn build_order_creation_args(
        &self,
        signer: &str,
        maker: &str,
        signature_type: Option<SignatureType>,
        user_order: &UserOrder,
        round_config: &RoundingConfig,
    ) -> OrderData {
        let OrderRawAmounts {
            side,
            raw_maker_amount,
            raw_taker_amount,
        } = self.get_order_raw_amounts(
            &user_order.side,
            user_order.size,
            user_order.price,
            round_config,
        );

        let maker_amount = parse_units(&raw_maker_amount.to_string(), "MWEI")
            .unwrap()
            .to_string();
        let taker_amount = parse_units(&raw_taker_amount.to_string(), "MWEI")
            .unwrap()
            .to_string();

        let taker = match user_order.taker.clone() {
            Some(taker) => taker,
            None => Address::ZERO.to_string(),
        };

        let fee_rate_bps = match user_order.fee_rate_bps {
            Some(fee_rate) => fee_rate.to_string(),
            None => "0".to_string(),
        };

        let nonce = match user_order.nonce {
            Some(nonce) => nonce.to_string(),
            None => "0".to_string(),
        };

        let expiration = match user_order.expiration {
            Some(exp) => exp.to_string(),
            None => "0".to_string(),
        };

        OrderData {
            maker: maker.to_string(),
            taker,
            token_id: user_order.token_id.clone(),
            maker_amount,
            taker_amount,
            side,
            fee_rate_bps,
            nonce,
            signer: Some(signer.to_string()),
            expiration: Some(expiration),
            signature_type,
        }
    }

    fn get_order_raw_amounts(
        &self,
        side: &Side,
        size: f64,
        price: f64,
        round_config: &RoundingConfig,
    ) -> OrderRawAmounts {
        let raw_price = price.round_normal(round_config.price);
        let (raw_maker_amount, raw_taker_amount) = match side {
            Side::Buy => {
                let raw_taker_amount = size.round_down(round_config.size);
                let raw_maker_amount =
                    adjust_amount(raw_taker_amount * raw_price, round_config.amount);
                (raw_maker_amount, raw_taker_amount)
            }
            Side::Sell => {
                let raw_maker_amount = size.round_down(round_config.size);
                let raw_taker_amount =
                    adjust_amount(raw_maker_amount * raw_price, round_config.amount);
                (raw_maker_amount, raw_taker_amount)
            }
        };

        OrderRawAmounts::new(side, raw_maker_amount, raw_taker_amount)
    }
}
