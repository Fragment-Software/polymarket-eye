use std::sync::Arc;

use alloy_signer::Signer;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, SET_COOKIE},
    Method, Proxy,
};

use tokio::time::{sleep, timeout, Duration};

use crate::{
    errors::custom::CustomError,
    utils::{
        common::{
            build_cookie_header, build_poly_headers, get_proxy_wallet_address, parse_cookies,
        },
        fetch::{send_http_request_with_retries, RequestParams},
    },
};

use super::{
    schemas::{
        CreateUserRequestBody, CreateUserResponseBody, EnableTradingRequestBody,
        EnableTradingResponseBody, GetNonceResponseBody, GetTransactionStatusResponse,
        LoginReponseBody, TransactionState, UpdatePreferencesRequestBody,
        UpdateUsernameRequestBody,
    },
    typedefs::AmpCookie,
};

pub async fn get_nonce(proxy: Option<&Proxy>) -> Result<(String, String), CustomError> {
    let request_params = RequestParams {
        url: "https://gamma-api.polymarket.com/nonce",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: None,
    };

    let response = send_http_request_with_retries::<GetNonceResponseBody>(
        &request_params,
        None,
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    let polymarket_nonce = response
        .headers
        .get(SET_COOKIE)
        .and_then(|hdr| hdr.to_str().ok())
        .and_then(|cookie_str| parse_cookies(cookie_str).get("polymarketnonce").cloned());

    polymarket_nonce
        .map(|nonce| (response.body.unwrap().nonce, nonce))
        .ok_or_else(|| CustomError::PolymarketApi("Failed to get polymarket nonce".to_string()))
}

pub async fn login(
    amp_value: &str,
    polymarket_nonce: &str,
    auth_header_value: &str,
) -> Result<String, CustomError> {
    let cookies = vec![
        ("polymarketnonce", polymarket_nonce),
        ("AMP_4572e28e5c", amp_value),
    ];

    let cookie_header_value = build_cookie_header(&cookies);

    let headers = vec![
        (AUTHORIZATION, auth_header_value),
        (COOKIE, &cookie_header_value),
    ]
    .into_iter()
    .map(|(name, value)| (name, HeaderValue::from_str(value).unwrap()))
    .collect::<HeaderMap>();

    let request_params = RequestParams {
        url: "https://gamma-api.polymarket.com/login",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: None,
    };

    let response = send_http_request_with_retries::<LoginReponseBody>(
        &request_params,
        Some(&headers),
        None,
        None,
        None,
        |_| true,
    )
    .await?;

    let polymarket_session = response
        .headers
        .get(SET_COOKIE)
        .and_then(|hdr| hdr.to_str().ok())
        .and_then(|cookie_str| parse_cookies(cookie_str).get("polymarketsession").cloned());

    polymarket_session
        .ok_or_else(|| CustomError::PolymarketApi("Failed to get polymarket session".to_string()))
}

pub async fn create_profile<S: Signer>(
    signer: Arc<S>,
    proxy: Option<&Proxy>,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
) -> Result<CreateUserResponseBody, CustomError> {
    let headers = build_poly_headers(amp_cookie, polymarket_nonce, polymarket_session);

    let body = CreateUserRequestBody::new(signer);

    let request_params = RequestParams {
        url: "https://gamma-api.polymarket.com/profiles",
        method: Method::POST,
        query_args: None,
        body: Some(body),
    };

    let response = send_http_request_with_retries::<CreateUserResponseBody>(
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

pub async fn update_username(
    username: &str,
    profile_id: &str,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
    proxy: Option<&Proxy>,
) -> Result<(), CustomError> {
    let headers = build_poly_headers(amp_cookie, polymarket_nonce, polymarket_session);

    let url = format!("https://gamma-api.polymarket.com/profiles/{}", profile_id);

    let body = UpdateUsernameRequestBody::new(username);

    let request_params = RequestParams {
        url: &url,
        method: Method::PUT,
        body: Some(body),
        query_args: None,
    };

    let _ = send_http_request_with_retries::<serde_json::Value>(
        &request_params,
        Some(&headers),
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    Ok(())
}

pub async fn update_preferences(
    preferences_id: &str,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
    proxy: Option<&Proxy>,
) -> Result<(), CustomError> {
    let headers = build_poly_headers(amp_cookie, polymarket_nonce, polymarket_session);

    let url = format!(
        "https://gamma-api.polymarket.com/preferences/{}",
        preferences_id
    );

    let body = UpdatePreferencesRequestBody::new();

    let request_params = RequestParams {
        url: &url,
        method: Method::PUT,
        body: Some(body),
        query_args: None,
    };

    let _ = send_http_request_with_retries::<serde_json::Value>(
        &request_params,
        Some(&headers),
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    Ok(())
}

pub async fn enable_trading<S: Signer>(
    signer: Arc<S>,
    signature: &str,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
    proxy: Option<&Proxy>,
) -> Result<String, CustomError> {
    let headers = build_poly_headers(amp_cookie, polymarket_nonce, polymarket_session);

    let body =
        EnableTradingRequestBody::new(signer.clone(), get_proxy_wallet_address(signer), signature);

    let request_params = RequestParams {
        url: "https://relayer-v2.polymarket.com/submit",
        method: Method::POST,
        body: Some(body),
        query_args: None,
    };

    let response = send_http_request_with_retries::<EnableTradingResponseBody>(
        &request_params,
        Some(&headers),
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    Ok(response.body.unwrap().transaction_id)
}

pub async fn get_transaction_status(
    transaction_id: &str,
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
    proxy: Option<&Proxy>,
) -> Result<bool, CustomError> {
    let headers = build_poly_headers(amp_cookie, polymarket_nonce, polymarket_session);

    let query_args = [("id".to_string(), transaction_id.to_string())]
        .iter()
        .map(|(arg, value)| (arg.to_owned(), value.to_owned()))
        .collect();

    let request_params = RequestParams {
        url: "https://relayer-v2.polymarket.com/transaction",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: Some(query_args),
    };

    let response = send_http_request_with_retries::<Vec<GetTransactionStatusResponse>>(
        &request_params,
        Some(&headers),
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    match response.body.unwrap()[0].state {
        TransactionState::Mined => Ok(true),
        _ => Ok(false),
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
) -> Result<(), CustomError> {
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
                Ok(true) => return Ok(()),
                Ok(false) => {
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
