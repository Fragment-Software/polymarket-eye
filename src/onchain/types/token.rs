use std::fmt::Display;

use alloy::primitives::{Address, U256};

use crate::onchain::constants::USDCE_CONTRACT_ADDRESS;

pub struct Token {
    pub contract_address: Address,
    pub decimals: u8,
    pub symbol: &'static str,
    pub is_erc20: bool,
}

impl Token {
    pub const USDCE: Token = Token {
        contract_address: USDCE_CONTRACT_ADDRESS,
        decimals: 6,
        symbol: "USDC",
        is_erc20: true,
    };

    pub fn to_wei(&self, amount: f64) -> U256 {
        U256::from(amount * 10f64.powi(self.decimals as i32))
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${}", self.symbol)
    }
}
