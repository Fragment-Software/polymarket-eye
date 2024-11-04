use crate::{
    errors::custom::CustomError,
    polymarket::api::typedefs::AmpCookie,
    utils::{
        fetch::{send_http_request_with_retries, RequestParams},
        poly::build_poly_headers,
    },
};

use alloy::primitives::Address;
use reqwest::{Method, Proxy};
use tokio::time::{sleep, timeout, Duration};

use super::schemas::{
    GetRelayerNonceResponseBody, GetTransactionStatusResponseBody, RelayerRequestBody,
    RelayerResponseBody, TransactionState,
};

pub async fn get_transaction_status(
    transaction_id: &str,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
    proxy: Option<&Proxy>,
) -> Result<Option<String>, CustomError> {
    let headers = build_poly_headers(amp_cookie, polymarket_nonce, polymarket_session);

    let query_args = [("id", transaction_id)]
        .iter()
        .map(|(arg, value)| (*arg, *value))
        .collect();

    let request_params = RequestParams {
        url: "https://relayer-v2.polymarket.com/transaction",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: Some(query_args),
    };

    let response = send_http_request_with_retries::<Vec<GetTransactionStatusResponseBody>>(
        &request_params,
        Some(&headers),
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    let tx_status = &response.body.as_ref().unwrap()[0];

    match tx_status.state {
        TransactionState::Mined => Ok(Some(tx_status.transaction_hash.clone())),
        _ => Ok(None),
    }
}

pub async fn wait_for_transaction_confirmation(
    transaction_id: &str,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
    proxy: Option<&Proxy>,
    timeout_duration: Option<Duration>,
    poll_interval: Option<Duration>,
) -> Result<String, CustomError> {
    let timeout_duration = timeout_duration.unwrap_or(Duration::from_secs(100));
    let poll_interval = poll_interval.unwrap_or(Duration::from_secs(5));

    let polling_future = async {
        loop {
            match get_transaction_status(
                transaction_id,
                amp_cookie,
                polymarket_nonce,
                polymarket_session,
                proxy,
            )
            .await
            {
                Ok(Some(transaction_hash)) => return Ok(transaction_hash),
                Ok(None) => {
                    sleep(poll_interval).await;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    };

    match timeout(timeout_duration, polling_future).await {
        Ok(result) => result,
        Err(_) => Err(CustomError::Timeout(
            "Transaction not mined within timeout".to_string(),
        )),
    }
}

pub async fn get_nonce(
    address: Address,
    proxy: Option<&Proxy>,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
) -> Result<u64, CustomError> {
    let headers = build_poly_headers(amp_cookie, polymarket_nonce, polymarket_session);
    let address = address.to_string();

    let query_args = [("address", address.as_str()), ("type", "SAFE")]
        .iter()
        .map(|(arg, value)| (*arg, *value))
        .collect();

    let request_params = RequestParams {
        url: "https://relayer-v2.polymarket.com/nonce",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: Some(query_args),
    };

    let response = send_http_request_with_retries::<GetRelayerNonceResponseBody>(
        &request_params,
        Some(&headers),
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    let nonce = response.body.unwrap().nonce.parse().unwrap();

    Ok(nonce)
}

pub async fn send_relayer_transaction<'a>(
    proxy: Option<&Proxy>,
    body: RelayerRequestBody<'a>,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
) -> Result<RelayerResponseBody, CustomError> {
    let headers = build_poly_headers(amp_cookie, polymarket_nonce, polymarket_session);

    let request_params = RequestParams {
        url: "https://relayer-v2.polymarket.com/submit",
        method: Method::POST,
        body: Some(body),
        query_args: None,
    };

    let response = send_http_request_with_retries::<RelayerResponseBody>(
        &request_params,
        Some(&headers),
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    Ok(response.body.unwrap())
}
