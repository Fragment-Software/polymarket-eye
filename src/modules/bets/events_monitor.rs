use itertools::Itertools;
use reqwest::Proxy;

use crate::{
    config::Config,
    errors::custom::CustomError,
    polymarket::api::events::{
        endpoints::get_events,
        schemas::{Event, Market},
    },
};

pub async fn get_filtered_events(
    proxy: Option<&Proxy>,
    config: &Config,
) -> Result<Vec<Event>, CustomError> {
    let mut offset = 0;
    let mut filtered_events = vec![];

    loop {
        let events = get_events(None, offset, proxy)
            .await?
            .into_iter()
            .filter(|event| event.volume >= config.min_event_volume)
            .collect_vec();

        if events.is_empty() {
            break;
        }

        filtered_events.extend(events.into_iter().filter_map(
            |event| match event.markets.as_slice() {
                [market]
                    if market_fits_filters(
                        market,
                        config.price_difference_threshold,
                        config.spread_threshold,
                    ) =>
                {
                    Some(event)
                }
                _ => None,
            },
        ));

        offset += 20;
    }

    Ok(filtered_events)
}

fn market_fits_filters(market: &Market, max_price_diff: f64, min_spread: f64) -> bool {
    let outcome_prices = market.outcome_prices.unwrap_or([0.0, 1000.0]);

    let price_diff = (outcome_prices[0] - outcome_prices[1]).abs();

    let price_diff_suitable = price_diff <= max_price_diff;
    let min_spread_suitable = market.spread <= min_spread;

    min_spread_suitable && price_diff_suitable
}
