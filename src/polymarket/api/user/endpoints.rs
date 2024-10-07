use std::{collections::HashMap, sync::Arc};

use alloy::signers::Signer;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, SET_COOKIE},
    Method, Proxy, StatusCode,
};

use crate::{
    errors::custom::CustomError,
    polymarket::api::typedefs::{AmpCookie, ClobAuthHeaders},
    utils::{
        fetch::{send_http_request_with_retries, RequestParams},
        poly::{build_cookie_header, build_poly_headers, parse_cookies},
    },
};

use super::schemas::{
    ClobApiKeyResponseBody, CreateUserRequestBody, CreateUserResponseBody,
    GetAuthNonceResponseBody, LoginReponseBody, UpdatePreferencesRequestBody,
    UpdateUsernameRequestBody, User,
};

pub async fn get_auth_nonce(proxy: Option<&Proxy>) -> Result<(String, String), CustomError> {
    let request_params = RequestParams {
        url: "https://gamma-api.polymarket.com/nonce",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: None,
    };

    let response = send_http_request_with_retries::<GetAuthNonceResponseBody>(
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

pub async fn get_user(
    amp_cookie: &mut AmpCookie,
    polymarket_nonce: &str,
    polymarket_session: &str,
    proxy: Option<&Proxy>,
) -> Result<Option<User>, CustomError> {
    let headers = build_poly_headers(amp_cookie, polymarket_nonce, polymarket_session);

    let request_params = RequestParams {
        url: "https://gamma-api.polymarket.com/users",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: None,
    };

    let response = send_http_request_with_retries::<Option<Vec<User>>>(
        &request_params,
        Some(&headers),
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    let user = response
        .body
        .flatten()
        .and_then(|users| users.first().cloned());

    Ok(user)
}

pub async fn derive_api_key<S>(
    signer: Arc<S>,
    proxy: Option<&Proxy>,
) -> Result<ClobApiKeyResponseBody, CustomError>
where
    S: Signer + Send + Sync,
{
    let headers = ClobAuthHeaders::new(signer.clone()).await.to_headers();
    let mut query_args = HashMap::new();
    query_args.insert("geo_block_token", "");

    let request_params = RequestParams {
        url: "https://clob.polymarket.com/auth/derive-api-key",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: Some(query_args),
    };

    let response = send_http_request_with_retries::<ClobApiKeyResponseBody>(
        &request_params,
        Some(&headers),
        proxy,
        None,
        None,
        |err| match err {
            CustomError::Request(error) => {
                if let Some(status) = error.status() {
                    status != StatusCode::BAD_REQUEST
                } else {
                    true
                }
            }
            _ => true,
        },
    )
    .await?;

    Ok(response.body.unwrap())
}

pub async fn create_api_key<S>(
    signer: Arc<S>,
    proxy: Option<&Proxy>,
) -> Result<ClobApiKeyResponseBody, CustomError>
where
    S: Signer + Send + Sync,
{
    let headers = ClobAuthHeaders::new(signer.clone()).await.to_headers();
    let mut query_args = HashMap::new();
    query_args.insert("geo_block_token", "");

    let request_params = RequestParams {
        url: "https://clob.polymarket.com/auth/api-key",
        method: Method::POST,
        body: None::<serde_json::Value>,
        query_args: Some(query_args),
    };

    let response = send_http_request_with_retries::<ClobApiKeyResponseBody>(
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
