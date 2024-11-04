use std::{collections::HashMap, sync::Arc};

use alloy::signers::Signer;
use itertools::Itertools;
use reqwest::{Method, Proxy, StatusCode};

use crate::{
    db::account::Account,
    errors::custom::CustomError,
    polymarket::api::{
        clob::schemas::OrderBookData,
        typedefs::{HeaderMapSerializeable, LayerOneClobAuthHeaders, LayerTwoClobAuthHeaders},
    },
    utils::fetch::{send_http_request_with_retries, RequestParams},
};

use super::schemas::{
    ClobApiKeyResponseBody, GetTickSizeResponseBody, NegRiskResponseBody, OrderRequest,
    PlaceOrderResponseBody, TokenId,
};

pub async fn derive_api_key<S>(
    signer: Arc<S>,
    proxy: Option<&Proxy>,
) -> Result<ClobApiKeyResponseBody, CustomError>
where
    S: Signer + Send + Sync,
{
    let headers = LayerOneClobAuthHeaders::new(signer.clone())
        .await
        .to_headermap();
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
            CustomError::HttpStatusError { status, .. } => status != &StatusCode::BAD_REQUEST,
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
    let headers = LayerOneClobAuthHeaders::new(signer.clone())
        .await
        .to_headermap();
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

#[allow(unused)]
pub async fn get_tick_size(proxy: Option<&Proxy>, token_id: &str) -> Result<f64, CustomError> {
    let query_args = [("token_id", token_id), ("geo_block_token", "")]
        .iter()
        .map(|(arg, value)| (*arg, *value))
        .collect();

    let request_params = RequestParams {
        url: "https://clob.polymarket.com/tick-size",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: Some(query_args),
    };

    let response = send_http_request_with_retries::<GetTickSizeResponseBody>(
        &request_params,
        None,
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    Ok(response.body.unwrap().minimum_tick_size)
}

#[allow(unused)]
pub async fn get_order_books(
    token_ids: &[&str],
    proxy: Option<&Proxy>,
) -> Result<Vec<OrderBookData>, CustomError> {
    let ids = token_ids
        .iter()
        .map(|id| TokenId { token_id: id })
        .collect_vec();

    let request_params = RequestParams {
        url: "https://clob.polymarket.com/books",
        method: Method::POST,
        body: Some(ids),
        query_args: None,
    };

    let response = send_http_request_with_retries::<Vec<OrderBookData>>(
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

pub async fn get_order_book(
    token_id: &str,
    proxy: Option<&Proxy>,
) -> Result<OrderBookData, CustomError> {
    let mut query_args = HashMap::new();
    query_args.insert("token_id", token_id);

    let request_params = RequestParams {
        url: "https://clob.polymarket.com/book",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: Some(query_args),
    };

    let response = send_http_request_with_retries::<OrderBookData>(
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

pub async fn get_neg_risk(token_id: &str, proxy: Option<&Proxy>) -> Result<bool, CustomError> {
    let mut query_args = HashMap::new();
    query_args.insert("token_id", token_id);

    let request_params = RequestParams {
        url: "https://clob.polymarket.com/neg-risk",
        method: Method::GET,
        body: None::<serde_json::Value>,
        query_args: Some(query_args),
    };

    let response = send_http_request_with_retries::<NegRiskResponseBody>(
        &request_params,
        None,
        proxy,
        None,
        None,
        |_| true,
    )
    .await?;

    Ok(response.body.unwrap().neg_risk)
}

pub async fn place_order(
    account: &Account,
    order: OrderRequest,
) -> Result<PlaceOrderResponseBody, CustomError> {
    let mut query_args = HashMap::new();
    query_args.insert("geo_block_token", "");

    let method = Method::POST;
    let path = "/order";
    let headers = LayerTwoClobAuthHeaders::new(
        &account.signer().address().to_string(),
        account.get_api_creds().unwrap(),
        method.as_str(),
        path,
        Some(&serde_json::to_string(&order).unwrap()),
        None,
    )
    .to_headermap();

    let request_params = RequestParams {
        url: &format!("https://clob.polymarket.com{path}"),
        method,
        body: Some(order),
        query_args: Some(query_args),
    };

    let response = send_http_request_with_retries::<PlaceOrderResponseBody>(
        &request_params,
        Some(&headers),
        account.proxy().as_ref(),
        None,
        None,
        |_| true,
    )
    .await?;

    let body = response.body.unwrap();

    match body.error_msg.is_empty() {
        true => Ok(body),
        false => Err(CustomError::ClobApiError(body.error_msg)),
    }
}
