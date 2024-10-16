use std::{str::FromStr, sync::Arc};

use alloy::{
    primitives::{utils::format_units, Address},
    providers::ProviderBuilder,
};

use itertools::Itertools;
use reqwest::{Proxy, Url};
use tabled::{settings::Style, Table, Tabled};
use tokio::task::JoinSet;

use crate::{
    config::Config,
    db::database::Database,
    onchain::{multicall::multicall_balance_of, types::token::Token},
    polymarket::api::user::endpoints::get_user_positions,
};

#[derive(Tabled)]
struct UserStats {
    #[tabled(rename = "Address")]
    address: String,
    #[tabled(rename = "USDC.e Balance")]
    balance: String,
    #[tabled(rename = "Open positions")]
    open_positions: usize,
}

pub async fn check_and_display_stats(db: Database, config: &Config) -> eyre::Result<()> {
    let spawn_task = |address: String, proxy: Option<Proxy>, handles: &mut JoinSet<_>| {
        handles.spawn(async move {
            let positions = get_user_positions(&address, proxy.as_ref()).await;
            (positions, address, proxy)
        })
    };

    let address_to_proxy =
        db.0.iter()
            .map(|account| {
                (
                    Address::from_str(&account.proxy_address).unwrap(),
                    account.proxy(),
                )
            })
            .collect_vec();

    let provider = Arc::new(
        ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(Url::parse(&config.polygon_rpc_url)?),
    );

    let mut handles = JoinSet::new();

    for (address, proxy) in &address_to_proxy {
        let address = address.to_string();
        let proxy = proxy.clone();

        spawn_task(address, proxy, &mut handles);
    }

    let mut positions_result = vec![];

    while let Some(res) = handles.join_next().await {
        let (positions, address, proxy) = res.unwrap();

        match positions {
            Ok(positions) => {
                positions_result.push((positions, address));
            }
            Err(e) => {
                tracing::error!("Failed to get user positions: {e}");
                spawn_task(address, proxy, &mut handles);
            }
        }
    }

    let balances = multicall_balance_of(
        &address_to_proxy
            .iter()
            .map(|(address, _)| *address)
            .collect_vec(),
        Token::USDCE,
        provider,
    )
    .await?;

    let mut balance_entries = vec![];

    for (address_to_proxy, balance) in address_to_proxy.iter().zip(balances.iter()) {
        let balance_in_usdce = format_units(*balance, 6).unwrap_or_else(|_| "0".to_string());
        let open_positions = positions_result
            .iter()
            .find(|res| res.1 == address_to_proxy.0.to_string())
            .map(|positions| positions.0.len())
            .unwrap_or(0);

        let entry = UserStats {
            address: address_to_proxy.0.to_string(),
            balance: balance_in_usdce,
            open_positions,
        };

        balance_entries.push(entry);
    }

    let mut table = Table::new(&balance_entries);
    let table = table.with(Style::modern_rounded());

    println!("{table}");

    Ok(())
}
