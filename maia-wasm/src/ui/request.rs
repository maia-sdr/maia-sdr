//! HTTP requests.
//!
//! This module implements some convenience functions that use the [Fetch
//! API](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API) to make and
//! process asynchronous HTTP requests.

use serde::Serialize;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

/// Constructs a JSON HTTP request.
///
/// Given a serializable value in the `json` parameter, this function serializes
/// it to JSON with [`serde_json`] and creates a [`Request`] with that JSON as
/// body. The URL and HTTP method of the request are given in the `url` and
/// `method` arguments.
pub fn json_request<T: Serialize>(url: &str, json: &T, method: &str) -> Result<Request, JsValue> {
    let opts = RequestInit::new();
    opts.set_method(method);
    let json = serde_json::to_string(json)
        .map_err(|_| format!("unable to format JSON for {method} request"))?;
    opts.set_body(&json.into());
    let request = Request::new_with_str_and_init(url, &opts)?;
    request.headers().set("Content-Type", "application/json")?;
    Ok(request)
}

/// Converts the text of a [`Response`] to a Rust [`String`].
///
/// This function awaits for the text of a `Response` and tries to convert it to
/// a Rust string, which is returned.
pub async fn response_to_string(response: &Response) -> Result<String, JsValue> {
    Ok(JsFuture::from(response.text()?)
        .await?
        .as_string()
        .ok_or("unable to convert fetch text to string")?)
}

/// Converts the text of a JSON [`Response`] to a Rust value.
///
/// For a deserializable Rust type `T`, this function awaits for the text of a
/// `Response` and tries to convert it to a Rust value of type `T` by using
/// [`serde_json`] to deserialize the JSON in the text of the response. If the
/// deserialization is successful, the Rust value is returned.
pub async fn response_to_json<T>(response: &Response) -> Result<T, JsValue>
where
    for<'a> T: serde::Deserialize<'a>,
{
    let json = serde_json::from_str(&response_to_string(response).await?)
        .map_err(|_| format!("unable to parse {} JSON", std::any::type_name::<T>()))?;
    Ok(json)
}

/// Request error.
///
/// This enum represents the errors that can happen with a request.
pub enum RequestError {
    /// The request failed on the server.
    ///
    /// The server has returned an HTTP error response that states that the
    /// request failed server-side. This variant contains the error returned by
    /// the server.
    RequestFailed(maia_json::Error),
    /// Other error.
    ///
    /// Other error has happened. The error is contained as a [`JsValue`].
    OtherError(JsValue),
}

impl From<JsValue> for RequestError {
    fn from(value: JsValue) -> RequestError {
        RequestError::OtherError(value)
    }
}

/// Ignore [`RequestError::RequestFailed`] errors.
///
/// This function transforms a `Result<T, RequestError>` by ignoring
/// `RequestError::RequestFailed` in the following way. If the input `Result` is
/// `Ok(x)`, then `Okay(Some(x))` is returned. If the input `Result` is
/// `Err(RequestError::RequestFailed(_))`, then `Ok(None)` is returned, thus
/// discarding the Error. If the input is `Result` is
/// `Err(RequestError::OtherError(jsvalue))`, then `Err(jsvalue)` is returned,
/// thus propagating the error.
///
/// This utility function is useful because request errors are logged by the
/// function that does the request, so often we want to do nothing more to
/// handle this error, but we want to handle other errors by failing a promise.
pub fn ignore_request_failed<T>(x: Result<T, RequestError>) -> Result<Option<T>, JsValue> {
    match x {
        Ok(y) => Ok(Some(y)),
        Err(RequestError::RequestFailed(_)) => Ok(None),
        Err(RequestError::OtherError(err)) => Err(err),
    }
}
