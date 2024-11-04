use core::f64;
use std::{cmp::min, sync::Arc, time::Duration};

use alloy::{
    primitives::{utils::format_units, Address, U256},
    providers::ProviderBuilder,
};
use itertools::Itertools;
use rand::{seq::SliceRandom, thread_rng};
use reqwest::{Proxy, Url};
use tokio::task::JoinSet;

use crate::{
    config::Config,
    db::{account::Account, database::Database},
    modules::registration::create_or_derive_api_key,
    onchain::{multicall::multicall_balance_of, types::token::Token},
    polymarket::api::{
        clob::{
            endpoints::{get_neg_risk, get_order_book, place_order},
            math::calculate_market_price,
            order_builder::OrderBuilder,
            schemas::{OrderRequest, OrderType, PlaceOrderResponseBody},
            typedefs::{
                CreateOrderOptions, Side, SignedOrder, TickSize, UserMarketOrder, UserOrder,
            },
        },
        events::schemas::Event,
        user::{endpoints::get_user_positions, schemas::UserPosition},
    },
    utils::misc::random_in_range,
};

use super::events_monitor::get_filtered_events;

pub async fn opposing_bets(db: Database, config: &Config) -> eyre::Result<()> {
    let proxy = db.0.first().and_then(|account| account.proxy());

    tracing::info!("Scanning events");
    let filtered_events = get_filtered_events(proxy.as_ref(), config).await?;

    if filtered_events.is_empty() {
        tracing::warn!("Events with selected filters not found");
        return Ok(());
    }

    let addresses =
        db.0.iter()
            .map(|account| account.get_proxy_address())
            .collect_vec();

    let provider = Arc::new(
        ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(Url::parse(&config.polygon_rpc_url)?),
    );

    let bet_amounts = multicall_balance_of(&addresses, Token::USDCE, provider)
        .await?
        .chunks(2)
        .map(|pair| min(pair[0], pair[1]))
        .map(|max_bet| {
            let multiplier = random_in_range(config.bet_balance_percentage);
            max_bet * U256::from(multiplier) / U256::from(100)
        })
        .collect_vec();

    let mut handles = JoinSet::new();

    for _ in 0..config.cycle_count {
        for (accounts_pair, bet_amount) in db.0.chunks(2).zip(bet_amounts.iter()) {
            let event = filtered_events.choose(&mut thread_rng()).unwrap().clone();
            let spawn_delay = random_in_range(config.batch_delay_range);
            let sell_delay_range = config.sell_delay_range;

            let first_account = accounts_pair[0].clone();
            let second_account = accounts_pair[1].clone();
            let amount = *bet_amount;

            handles.spawn(async move {
                tokio::time::sleep(Duration::from_secs(spawn_delay)).await;

                place_opposing_bets_with_timeout(
                    first_account,
                    second_account,
                    event,
                    amount,
                    sell_delay_range,
                )
                .await
            });
        }

        while let Some(res) = handles.join_next().await {
            let result = res.unwrap();

            match result {
                Ok(pair) => tracing::info!("Pair {pair} is finished"),
                Err(e) => tracing::error!("Unexpected error during placing opposing bets: {e}"),
            }
        }

        db.update();
    }

    Ok(())
}

async fn place_opposing_bets_with_timeout(
    first_account: Account,
    second_account: Account,
    event: Event,
    amount: U256,
    sell_delay_range: [u64; 2],
) -> eyre::Result<String> {
    tracing::info!(
        "{} - {} | Event chosen: {event}",
        first_account.proxy_address,
        second_account.proxy_address
    );

    let market = event.markets.first().unwrap();

    let token_ids = market
        .clob_token_ids
        .iter()
        .map(|token_id| token_id.as_str())
        .collect_vec();

    let tick_size = TickSize::from_str(&market.order_price_min_tick_size.to_string()).unwrap();

    let float_amount = format_units(amount, "mwei")?.parse::<f64>()?;

    let futures_results = tokio::join!(
        create_and_place_buy_market_order(
            &first_account,
            token_ids[0],
            &event,
            float_amount,
            tick_size
        ),
        create_and_place_buy_market_order(
            &second_account,
            token_ids[1],
            &event,
            float_amount,
            tick_size
        ),
    );

    match futures_results {
        (Ok(_), Err(e)) => {
            tracing::info!(
                "{} - {} | Failed to place an order: {e}. Selling position on {}", // first succeeded, second failed -> sell the position on the first account
                first_account.proxy_address,
                second_account.proxy_address,
                first_account.proxy_address,
            );

            create_and_place_sell_market_order(&first_account, token_ids[0], tick_size).await?;
        }
        (Err(e), Ok(_)) => {
            tracing::info!(
                "{} - {} | Failed to place an order: {e}. Selling position on {}", // second succeeded, first failed -> sell the position on the second account
                first_account.proxy_address,
                second_account.proxy_address,
                second_account.proxy_address,
            );

            create_and_place_sell_market_order(&second_account, token_ids[1], tick_size).await?;
        }
        (Ok(_), Ok(_)) => {
            let delay = Duration::from_secs(random_in_range(sell_delay_range)); // both landed, then sleep for random delay and sell

            tracing::info!(
                "{} - {} | Both orders placed, sleeping for {} seconds",
                first_account.proxy_address,
                second_account.proxy_address,
                delay.as_secs()
            );
            tokio::time::sleep(delay).await;

            let _ = tokio::join!(
                create_and_place_sell_market_order(&first_account, token_ids[0], tick_size),
                create_and_place_sell_market_order(&second_account, token_ids[1], tick_size),
            );
        }
        _ => {
            tracing::error!(
                "{} - {} | Failed to place both orders",
                first_account.proxy_address,
                second_account.proxy_address
            )
        } // Case of both fails (tbh don't really care about them)
    }

    Ok(format!(
        "{} - {}",
        first_account.proxy_address, second_account.proxy_address
    ))
}

pub async fn create_and_place_sell_market_order(
    account: &Account,
    token_id: &str,
    tick_size: TickSize,
) -> eyre::Result<PlaceOrderResponseBody> {
    let signed_order =
        build_market_sell_signed_order_for_account(account, token_id, tick_size).await?;

    let api_key = {
        let maybe_key = account.api_key.read().unwrap().clone();
        if let Some(key) = maybe_key {
            key
        } else {
            let response =
                create_or_derive_api_key(account.signer(), account.proxy().as_ref()).await?;
            account.update_credentials(response);
            account.api_key.read().unwrap().as_ref().unwrap().clone()
        }
    };

    let order_request = OrderRequest::new(signed_order, &api_key, Some(OrderType::Gtc));

    let place_order_result = place_order(account, order_request).await?;

    place_order_result.log_successful_placement(Side::Sell, &account.proxy_address);

    Ok(place_order_result)
}

async fn create_and_place_buy_market_order(
    account: &Account,
    token_id: &str,
    event: &Event,
    amount_in: f64,
    tick_size: TickSize,
) -> eyre::Result<PlaceOrderResponseBody> {
    let signed_order =
        build_market_buy_signed_order_for_account(account, token_id, event, amount_in, tick_size)
            .await?;

    let api_key = {
        let maybe_key = account.api_key.read().unwrap().clone();
        if let Some(key) = maybe_key {
            key
        } else {
            let response =
                create_or_derive_api_key(account.signer(), account.proxy().as_ref()).await?;
            account.update_credentials(response);
            account.api_key.read().unwrap().as_ref().unwrap().clone()
        }
    };

    let order_request = OrderRequest::new(signed_order, &api_key, None);

    let place_order_result = place_order(account, order_request).await?;

    place_order_result.log_successful_placement(Side::Buy, &account.proxy_address);

    Ok(place_order_result)
}

async fn build_market_buy_signed_order_for_account(
    account: &Account,
    token_id: &str,
    event: &Event,
    amount_in: f64,
    tick_size: TickSize,
) -> eyre::Result<SignedOrder> {
    let build_order_args = |price: f64,
                            token_id: String,
                            amount: f64,
                            tick_size: TickSize,
                            neg_risk: bool|
     -> (CreateOrderOptions, UserMarketOrder) {
        let order = UserMarketOrder::new(token_id, amount, Some(price), None, None, None);
        let options = CreateOrderOptions::new(tick_size, Some(neg_risk));

        (options, order)
    };

    let proxy_wallet_address = account.get_proxy_address().to_string();
    let proxy = account.proxy();

    let order_book = get_order_book(token_id, proxy.as_ref()).await?;
    let market_price = calculate_market_price(Side::Buy, order_book, amount_in, None);

    let order_builder = OrderBuilder::new(account.signer(), 137, None, Some(&proxy_wallet_address));

    let neg_risk = event
        .neg_risk
        .unwrap_or(get_neg_risk(token_id, proxy.as_ref()).await?);

    let (order_options, order) = build_order_args(
        market_price,
        token_id.to_string(),
        amount_in,
        tick_size,
        neg_risk,
    );

    let signed_order = order_builder
        .build_signed_market_buy_order(order, order_options)
        .await?;

    Ok(signed_order)
}

async fn build_market_sell_signed_order_for_account(
    account: &Account,
    token_id: &str,
    tick_size: TickSize,
) -> eyre::Result<SignedOrder> {
    let proxy = account.proxy();
    let proxy_wallet_address = account.get_proxy_address().to_string();
    let order_builder = OrderBuilder::new(account.signer(), 137, None, Some(&proxy_wallet_address));

    let position =
        wait_for_matching_user_position(&proxy_wallet_address, proxy.clone(), token_id, None)
            .await?;

    let order_book = get_order_book(token_id, proxy.as_ref()).await?;
    let market_price = calculate_market_price(Side::Sell, order_book, position.size, None);

    let order = UserOrder::default()
        .with_token_id(token_id)
        .with_price(market_price)
        .with_side(Side::Sell)
        .with_size(position.size)
        .with_taker(Address::ZERO.to_string());

    let order_options = CreateOrderOptions::new(tick_size, Some(position.negative_risk));

    let signed_order = order_builder
        .build_signed_order(order, order_options)
        .await?;

    Ok(signed_order)
}

async fn wait_for_matching_user_position(
    proxy_wallet_address: &str,
    proxy: Option<Proxy>,
    token_id: &str,
    timeout_duration: Option<Duration>,
) -> eyre::Result<UserPosition> {
    let timeout_duration = timeout_duration.unwrap_or(Duration::from_secs(20));
    let sleep_duration = Duration::from_secs(2);

    let result = tokio::time::timeout(timeout_duration, async {
        loop {
            let user_positions = get_user_positions(proxy_wallet_address, proxy.as_ref()).await?;

            if let Some(position) = user_positions
                .into_iter()
                .find(|position| position.asset == token_id)
            {
                return Ok(position);
            } else {
                tracing::warn!(
                    "{} | Positions are not synced yet, sleeping",
                    proxy_wallet_address
                );
                tokio::time::sleep(sleep_duration).await;
            }
        }
    })
    .await;

    match result {
        Ok(Ok(position)) => Ok(position),
        Ok(Err(e)) => Err(e), // Propagate the error from get_user_positions
        Err(_) => Err(eyre::eyre!(
            "Timeout while waiting for matching user position"
        )),
    }
}
