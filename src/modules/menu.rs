use super::{deposit::deposit_to_accounts, registration::register_accounts};
use crate::{config::Config, db::database::Database};
use dialoguer::{theme::ColorfulTheme, Select};

pub async fn menu() -> eyre::Result<()> {
    let config = Config::read_default().await;

    loop {
        let options = vec!["Accounts registration", "USDC deposit", "Exit"];

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
                let db = Database::read().await;
                deposit_to_accounts(db, &config).await?;
            }
            2 => {
                return Ok(());
            }
            _ => tracing::error!("Invalid selection"),
        }
    }
}
