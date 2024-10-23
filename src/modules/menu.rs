use super::{
    bets::opposing::opposing_bets, deposit::deposit_to_accounts, registration::register_accounts,
    stats_check::check_and_display_stats,
};
use crate::{
    config::Config, db::database::Database, modules::sell::sell_all::sell_all_open_positions,
};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Select};

const LOGO: &str = r#"
    ___                                                  __
  /'___\                                                /\ \__
 /\ \__/  _ __    __       __     ___ ___      __    ___\ \ ,_\
 \ \ ,__\/\`'__\/'__`\   /'_ `\ /' __` __`\  /'__`\/' _ `\ \ \/
  \ \ \_/\ \ \//\ \L\.\_/\ \L\ \/\ \/\ \/\ \/\  __//\ \/\ \ \ \_
   \ \_\  \ \_\\ \__/.\_\ \____ \ \_\ \_\ \_\ \____\ \_\ \_\ \__\
    \/_/   \/_/ \/__/\/_/\/___L\ \/_/\/_/\/_/\/____/\/_/\/_/\/__/
                  ___  __  /\____/
                /'___\/\ \_\_/__/
   ____    ___ /\ \__/\ \ ,_\ __  __  __     __    _ __    __
  /',__\  / __`\ \ ,__\\ \ \//\ \/\ \/\ \  /'__`\ /\`'__\/'__`\
 /\__, `\/\ \L\ \ \ \_/ \ \ \\ \ \_/ \_/ \/\ \L\.\\ \ \//\  __/
 \/\____/\ \____/\ \_\   \ \__\ \___x___/'\ \__/.\_\ \_\\ \____\
  \/___/  \/___/  \/_/    \/__/\/__//__/   \/__/\/_/\/_/ \/____/

                     t.me/fragment_software
"#;

pub async fn menu() -> eyre::Result<()> {
    let config = Config::read_default().await;
    let logo = LOGO.red();

    println!("{logo}");

    loop {
        let options = vec![
            "Accounts registration",
            "USDC deposit",
            "Opposing bets",
            "Proxy wallets stats check",
            "Sell all open positions",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choice:")
            .items(&options)
            .default(0)
            .interact()
            .unwrap();

        match selection {
            0 => {
                let db = Database::new().await?;
                register_accounts(db, &config).await?;
            }
            1 => {
                let mut db = Database::read().await;
                db.shuffle();
                deposit_to_accounts(db, &config).await?;
            }
            2 => {
                let mut db = Database::read().await;
                db.shuffle();
                opposing_bets(db, &config).await?;
            }
            3 => {
                let db = Database::read().await;
                check_and_display_stats(db, &config).await?;
            }
            4 => {
                let db = Database::read().await;
                sell_all_open_positions(db, &config).await?;
            }
            5 => {
                return Ok(());
            }
            _ => tracing::error!("Invalid selection"),
        }
    }
}
