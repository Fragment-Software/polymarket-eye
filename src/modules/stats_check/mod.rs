use std::sync::Arc;

use alloy::{
    primitives::{utils::format_units, Address},
    providers::ProviderBuilder,
};

use itertools::Itertools;
use reqwest::{Proxy, Url};
use scraping::{
    scrape_open_positions, scrape_users_open_pos_value, scrape_users_pnl, scrape_users_trade_count,
    scrape_users_volume,
};
use tabled::{settings::Style, Table, Tabled};

use crate::{
    config::Config,
    db::database::Database,
    onchain::{multicall::multicall_balance_of, types::token::Token},
};

mod scraping;

#[derive(Tabled)]
struct UserStats {
    #[tabled(rename = "Address")]
    address: String,
    #[tabled(rename = "USDC.e Balance")]
    balance: String,
    #[tabled(rename = "Open positions count")]
    open_positions_count: usize,
    #[tabled(rename = "Open positions value")]
    open_positions_value: f64,
    #[tabled(rename = "Volume")]
    volume: f64,
    #[tabled(rename = "P&L")]
    pnl: f64,
    #[tabled(rename = "Trade count")]
    trade_count: u64,
}

pub async fn check_and_display_stats(db: Database, config: &Config) -> eyre::Result<()> {
    let provider = Arc::new(
        ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(Url::parse(&config.polygon_rpc_url)?),
    );

    let (addresses, proxies): (Vec<Address>, Vec<Option<Proxy>>) =
        db.0.iter()
            .map(|account| (account.get_proxy_address(), account.proxy()))
            .unzip();

    let balances = multicall_balance_of(&addresses, Token::USDCE, provider).await?;

    let addresses = addresses
        .into_iter()
        .map(|addr| addr.to_string())
        .collect_vec();

    let (
        open_positions_stats,
        users_volume_stats,
        users_pnl_stats,
        users_trade_count_stats,
        users_open_pos_value_stats,
    ) = tokio::join!(
        scrape_open_positions(addresses.clone(), proxies.clone()),
        scrape_users_volume(addresses.clone(), proxies.clone()),
        scrape_users_pnl(addresses.clone(), proxies.clone()),
        scrape_users_trade_count(addresses.clone(), proxies.clone()),
        scrape_users_open_pos_value(addresses.clone(), proxies.clone())
    );

    let mut stats_entries = vec![];

    for (address, balance) in addresses.iter().zip(balances.iter()) {
        let balance_in_usdce = format_units(*balance, 6).unwrap_or_else(|_| "0".to_string());

        let open_positions_count = open_positions_stats
            .iter()
            .find(|res| &res.0 == address)
            .map(|positions| positions.1.len())
            .unwrap_or(0);

        let open_positions_value = users_open_pos_value_stats
            .iter()
            .find(|res| &res.0 == address)
            .map(|pos_values| pos_values.1.first().unwrap().value)
            .unwrap_or(0f64);

        let user_volume = users_volume_stats
            .iter()
            .find(|res| &res.0 == address)
            .map(|volume| volume.1.first().map_or(0f64, |v| v.amount))
            .unwrap_or(0f64);

        let user_pnl = users_pnl_stats
            .iter()
            .find(|res| &res.0 == address)
            .map(|volume| volume.1.first().map_or(0f64, |v| v.amount))
            .unwrap_or(0f64);

        let trade_count = users_trade_count_stats
            .iter()
            .find(|res| &res.0 == address)
            .map(|volume| volume.1.traded)
            .unwrap_or(0);

        let entry = UserStats {
            address: address.to_string(),
            balance: balance_in_usdce,
            open_positions_count,
            open_positions_value,
            volume: user_volume,
            pnl: user_pnl,
            trade_count,
        };

        stats_entries.push(entry);
    }

    let mut table = Table::new(&stats_entries);
    let table = table.with(Style::modern_rounded());

    println!("{table}");

    Ok(())
}
