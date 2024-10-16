use reqwest::{Method, Proxy};

use crate::{
    errors::custom::CustomError,
    polymarket::api::events::schemas::Event,
    utils::fetch::{send_http_request_with_retries, RequestParams},
};

pub async fn get_events(
    limit: Option<u64>,
    offset: u64,
    proxy: Option<&Proxy>,
) -> Result<Vec<Event>, CustomError> {
    let offset = offset.to_string();
    let limit = limit.unwrap_or(20).to_string();

    let query_args = [
        ("limit", limit.as_str()),
        ("active", "true"),
        ("archived", "false"),
        ("closed", "false"),
        ("order", "volume24hr"),
        ("ascending", "false"),
        ("offset", offset.as_str()),
    ]
    .iter()
    .map(|(arg, value)| (*arg, *value))
    .collect();

    let request_params = RequestParams {
        url: "https://gamma-api.polymarket.com/events",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: Some(query_args),
    };

    let response = send_http_request_with_retries::<Vec<Event>>(
        &request_params,
        None,
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    Ok(response.body.unwrap())
}
