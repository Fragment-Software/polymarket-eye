use std::fmt::Display;

use serde::{de, Deserialize, Deserializer};

#[allow(unused)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub markets: Vec<Market>,
    pub neg_risk: Option<bool>,
}

#[allow(unused)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Market {
    pub id: String,
    pub question: String,
    pub active: bool,
    #[serde(rename = "questionID")]
    pub question_id: Option<String>,
    #[serde(deserialize_with = "deserialize_into_string_array")]
    pub outcomes: [String; 2],
    #[serde(deserialize_with = "deserialize_outcome_prices")]
    pub outcome_prices: Option<[f64; 2]>,
    pub rewards_max_spread: f64,
    #[serde(deserialize_with = "deserialize_into_string_array")]
    pub clob_token_ids: [String; 2],
    pub spread: f64,
    pub order_price_min_tick_size: f64,
}

fn deserialize_outcome_prices<'de, D>(deserializer: D) -> Result<Option<[f64; 2]>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt_s: Option<String> = Option::deserialize(deserializer)?;

    match opt_s {
        Some(s) => {
            let vec_str: Vec<String> =
                serde_json::from_str(&s).map_err(|err| de::Error::custom(err.to_string()))?;

            if vec_str.len() != 2 {
                return Err(de::Error::invalid_length(
                    vec_str.len(),
                    &"expected an array of length 2",
                ));
            }

            let mut vec_f64 = [0.0; 2];
            for (i, val_str) in vec_str.iter().enumerate() {
                vec_f64[i] = val_str
                    .parse::<f64>()
                    .map_err(|e| de::Error::custom(e.to_string()))?;
            }

            Ok(Some(vec_f64))
        }
        None => Ok(None),
    }
}

fn deserialize_into_string_array<'de, D>(deserializer: D) -> Result<[String; 2], D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    let vec: Vec<String> =
        serde_json::from_str(&s).map_err(|err| de::Error::custom(err.to_string()))?;

    if vec.len() != 2 {
        return Err(de::Error::invalid_length(
            vec.len(),
            &"expected an array of length 2",
        ));
    }

    Ok([vec[0].clone(), vec[1].clone()])
}

impl Event {
    pub fn get_url(&self) -> String {
        format!("https://polymarket.com/event/{}", self.slug)
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.title, self.get_url(),)
    }
}
