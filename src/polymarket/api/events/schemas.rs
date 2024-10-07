use serde::{de, Deserialize, Deserializer};

#[allow(unused)]
#[derive(Deserialize, Debug, Clone)]
pub struct Event {
    id: String,
    markets: Vec<Market>,
}

#[allow(unused)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Market {
    id: String,
    question: String,
    active: bool,
    #[serde(rename = "questionID")]
    question_id: String,
    #[serde(deserialize_with = "deserialize_into_string_array")]
    outcomes: [String; 2],
    #[serde(deserialize_with = "deserialize_outcome_prices")]
    outcome_prices: [f64; 2],
    rewards_max_spread: f64,
    #[serde(deserialize_with = "deserialize_into_string_array")]
    clob_token_ids: [String; 2],
    spread: f64,
}

fn deserialize_outcome_prices<'de, D>(deserializer: D) -> Result<[f64; 2], D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
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

    Ok(vec_f64)
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
