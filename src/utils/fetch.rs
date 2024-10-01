use std::time::Duration;

use reqwest::{header::HeaderMap, Client, Proxy};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;

use std::collections::HashMap;

use reqwest::Method;

use crate::errors::custom::CustomError;

#[derive(Clone)]
pub struct RequestParams<'a, S: Serialize> {
    pub url: &'a str,
    pub method: Method,
    pub body: Option<S>,
    pub query_args: Option<HashMap<String, String>>,
}

#[derive(Debug)]
pub struct HttpResponse<ResponseBody> {
    pub body: Option<ResponseBody>,
    pub headers: HeaderMap,
}

pub async fn send_http_request<R: DeserializeOwned>(
    request_params: &RequestParams<'_, impl Serialize>,
    headers: Option<&HeaderMap>,
    proxy: Option<&Proxy>,
) -> Result<HttpResponse<R>, CustomError> {
    let client = proxy.map_or_else(Client::new, |proxy| {
        Client::builder()
            .proxy(proxy.clone())
            .build()
            .unwrap_or_else(|err| {
                tracing::error!("Failed to build a client with proxy: {proxy:?}. Error: {err}");
                Client::new()
            })
    });

    let mut request = client.request(request_params.method.clone(), request_params.url);

    if let Some(params) = &request_params.query_args {
        request = request.query(&params);
    }

    if let Some(body) = &request_params.body {
        request = request.json(&body);
    }

    if let Some(headers) = headers {
        request = request.headers(headers.clone());
    }

    let response = request
        .send()
        .await
        .inspect_err(|e| tracing::error!("Request failed: {}", e))?
        .error_for_status()
        .inspect_err(|e| tracing::error!("Non-successful status code: {}", e))?;

    let response_headers = response.headers().clone();

    let content_type = response_headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    let text = response
        .text()
        .await
        .inspect_err(|e| tracing::error!("Failed to retrieve response text: {}", e))?;

    let response_body = if text.trim().is_empty() {
        None
    } else {
        let deserialized = if content_type.contains("application/json") {
            serde_json::from_str::<R>(&text)
        } else {
            let json_value = json!(text);
            serde_json::from_value::<R>(json_value)
        }
        .inspect_err(|e| tracing::error!("Failed to deserialize response: {}\n {:#?}", e, text))?;

        Some(deserialized)
    };

    Ok(HttpResponse {
        body: response_body,
        headers: response_headers,
    })
}

pub async fn send_http_request_with_retries<R: DeserializeOwned>(
    request_params: &RequestParams<'_, impl Serialize>,
    headers: Option<&HeaderMap>,
    proxy: Option<&Proxy>,
    max_retries: Option<usize>,
    retry_delay: Option<Duration>,
    should_retry: impl Fn(&CustomError) -> bool,
) -> Result<HttpResponse<R>, CustomError> {
    let max_retries = max_retries.unwrap_or(5);
    let retry_delay = retry_delay.unwrap_or(Duration::from_secs(3));

    for _ in 0..max_retries {
        match send_http_request(request_params, headers, proxy).await {
            Ok(response) => return Ok(response),
            Err(e) => {
                if !should_retry(&e) {
                    return Err(e);
                }
                tokio::time::sleep(retry_delay).await;
            }
        }
    }

    Err(CustomError::TriesExceeded)
}
