use serde::Deserialize;
use std::path::Path;

const CONFIG_FILE_PATH: &str = "data/config.toml";

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Config {
    pub registration_sleep_range: [u64; 2],
    pub mobile_proxies: bool,
    pub swap_ip_link: String,
    pub polygon_rpc_url: String,
    pub ignore_existing_balance: bool,
    pub usdc_amount_deposit_range: [f64; 2],
    pub deposit_sleep_range: [u64; 2],
    pub price_difference_threshold: f64,
    pub spread_threshold: f64,
    pub bet_balance_percentage: [u64; 2],
    pub sell_delay_range: [u64; 2],
    pub batch_delay_range: [u64; 2],
    pub cycle_count: u64,
}

impl Config {
    async fn read_from_file(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let cfg_str = tokio::fs::read_to_string(path).await?;
        Ok(toml::from_str(&cfg_str)?)
    }

    pub async fn read_default() -> Self {
        Self::read_from_file(CONFIG_FILE_PATH)
            .await
            .expect("Default config to be valid")
    }
}
