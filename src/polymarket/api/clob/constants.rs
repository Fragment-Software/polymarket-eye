use std::{collections::HashMap, sync::LazyLock};

pub struct RoundingConfig {
    pub price: u32,
    pub size: u32,
    pub amount: u32,
}

pub static ROUNDING_CONFIG: LazyLock<HashMap<&str, RoundingConfig>> = LazyLock::new(|| {
    [
        (
            "0.1",
            RoundingConfig {
                price: 1,
                size: 2,
                amount: 3,
            },
        ),
        (
            "0.01",
            RoundingConfig {
                price: 2,
                size: 2,
                amount: 4,
            },
        ),
        (
            "0.001",
            RoundingConfig {
                price: 3,
                size: 2,
                amount: 5,
            },
        ),
        (
            "0.0001",
            RoundingConfig {
                price: 4,
                size: 2,
                amount: 6,
            },
        ),
    ]
    .into_iter()
    .collect()
});

#[allow(unused)]
pub struct ContractConfig {
    pub exchange: &'static str,
    pub neg_risk_adapter: &'static str,
    pub neg_risk_exchange: &'static str,
    pub collateral: &'static str,
    pub conditional_tokens: &'static str,
}

pub const MATIC_CONTRACTS: ContractConfig = ContractConfig {
    exchange: "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E",
    neg_risk_adapter: "0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296",
    neg_risk_exchange: "0xC5d563A36AE78145C45a50134d48A1215220f80a",
    collateral: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
    conditional_tokens: "0x4D97DCd97eC945f40cF65F87097ACe5EA0476045",
};

pub const AMOY_CONTRACTS: ContractConfig = ContractConfig {
    exchange: "0xdFE02Eb6733538f8Ea35D585af8DE5958AD99E40",
    neg_risk_adapter: "0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296",
    neg_risk_exchange: "0xC5d563A36AE78145C45a50134d48A1215220f80a",
    collateral: "0x9c4e1703476e875070ee25b56a58b008cfb8fa78",
    conditional_tokens: "0x69308FB512518e39F9b16112fA8d994F4e2Bf8bB",
};

pub fn get_contract_config(chain_id: u64) -> eyre::Result<&'static ContractConfig> {
    match chain_id {
        137 => Ok(&MATIC_CONTRACTS),
        80002 => Ok(&AMOY_CONTRACTS),
        _ => eyre::bail!("Invalid network"),
    }
}

pub const PROTOCOL_NAME: &str = "Polymarket CTF Exchange";
pub const PROTOCOL_VERSION: &str = "1";
