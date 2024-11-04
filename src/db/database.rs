use std::fs::File;

use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng,
};
use serde::{Deserialize, Serialize};

use crate::utils::files::read_file_lines;

use super::{
    account::Account,
    constants::{DB_FILE_PATH, PRIVATE_KEYS_FILE_PATH, PROXIES_FILE_PATH, RECIPIENTS_FILE_PATH},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Database(pub Vec<Account>);

impl Database {
    async fn read_from_file(file_path: &str) -> eyre::Result<Self> {
        let contents = tokio::fs::read_to_string(file_path).await?;
        let db = serde_json::from_str::<Self>(&contents)?;
        Ok(db)
    }

    #[allow(unused)]
    pub async fn read() -> eyre::Result<Self> {
        Self::read_from_file(DB_FILE_PATH).await
    }

    pub async fn new() -> eyre::Result<Self> {
        let private_keys = read_file_lines(PRIVATE_KEYS_FILE_PATH).await.unwrap();
        let proxies = read_file_lines(PROXIES_FILE_PATH).await.unwrap();
        let recipients = read_file_lines(RECIPIENTS_FILE_PATH).await.unwrap();
        let mut data = Vec::with_capacity(private_keys.len());

        let max_len = private_keys.len().max(proxies.len()).max(recipients.len());

        for i in 0..max_len {
            let private_key = match private_keys.get(i) {
                Some(pk) => pk.clone(),
                None => eyre::bail!("Missing private key at position {}", i),
            };

            let proxy = proxies.get(i).cloned();
            let recipient = recipients.get(i).cloned();

            let account = Account::new(&private_key, proxy, recipient);
            data.push(account);
        }

        let db_file = File::create(DB_FILE_PATH)?;
        serde_json::to_writer_pretty(db_file, &data)?;

        Ok(Self(data))
    }

    pub fn get_random_account_with_filter<F>(&mut self, filter: F) -> Option<&mut Account>
    where
        F: Fn(&Account) -> bool,
    {
        let mut rng = thread_rng();

        self.0
            .iter_mut()
            .filter(|account| filter(account))
            .choose(&mut rng)
    }

    pub fn update(&self) {
        let file = File::create(DB_FILE_PATH).expect("Default database must be vaild");
        let _ = serde_json::to_writer_pretty(file, &self);
    }

    pub fn shuffle(&mut self) {
        self.0.shuffle(&mut thread_rng());
        self.update();
    }
}
