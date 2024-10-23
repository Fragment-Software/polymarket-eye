use rand::{rngs::ThreadRng, seq::SliceRandom, thread_rng, Rng};

use crate::{
    config::Config,
    db::{account::Account, database::Database},
    modules::bets::opposing::create_and_place_sell_market_order,
    polymarket::api::{
        clob::{
            endpoints::get_tick_size,
            typedefs::{Side, TickSize},
        },
        user::endpoints::get_user_positions,
    },
    utils::misc::pretty_sleep,
};

pub async fn sell_all_open_positions(db: Database, config: &Config) -> eyre::Result<()> {
    let mut accounts = db.0.clone();
    let mut rng = thread_rng();

    while !accounts.is_empty() {
        let index = rng.gen_range(0..accounts.len());
        let account = &accounts[index];

        match sell_random_open_positions(account, &mut rng).await {
            Ok(res) => {
                if !res {
                    accounts.remove(index);
                }

                pretty_sleep(config.sell_delay_range).await;
            }
            Err(e) => {
                tracing::error!("Failed to sell a random position: {e}")
            }
        }
    }

    tracing::info!("No more open positions left");

    Ok(())
}

async fn sell_random_open_positions(account: &Account, rng: &mut ThreadRng) -> eyre::Result<bool> {
    let proxy = account.proxy();
    let positions = get_user_positions(&account.proxy_address, proxy.as_ref()).await?;

    tracing::info!(
        "{} has {} open positions",
        account.proxy_address,
        positions.len()
    );

    if positions.is_empty() {
        return Ok(false);
    }

    let position = positions.choose(rng).unwrap();
    let tick_size = TickSize::from_str(
        &get_tick_size(proxy.as_ref(), &position.asset)
            .await?
            .to_string(),
    )
    .unwrap();

    let response = create_and_place_sell_market_order(account, &position.asset, tick_size).await?;

    response.log_successful_placement(Side::Sell, &account.proxy_address);

    Ok(true)
}
