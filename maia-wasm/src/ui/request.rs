use serde::Serialize;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

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

pub async fn response_to_string(response: &Response) -> Result<String, JsValue> {
    Ok(JsFuture::from(response.text()?)
        .await?
        .as_string()
        .ok_or("unable to convert fetch text to string")?)
}

pub async fn response_to_json<T>(response: &Response) -> Result<T, JsValue>
where
    for<'a> T: serde::Deserialize<'a>,
{
    let json = serde_json::from_str(&response_to_string(response).await?)
        .map_err(|_| format!("unable to parse {} JSON", std::any::type_name::<T>()))?;
    Ok(json)
}

pub enum RequestError {
    // allow(dead_code) included here beacuse the maia_json::Error field is
    // never read
    #[allow(dead_code)]
    RequestFailed(maia_json::Error),
    OtherError(JsValue),
}

impl From<JsValue> for RequestError {
    fn from(value: JsValue) -> RequestError {
        RequestError::OtherError(value)
    }
}

// This utility function is useful because request errors are logged by the
// function that does the request, so often times we want to do nothing more to
// handle this error, but we want to handle other errors by failing a promise.
pub fn ignore_request_failed<T>(x: Result<T, RequestError>) -> Result<Option<T>, JsValue> {
    match x {
        Ok(y) => Ok(Some(y)),
        Err(RequestError::RequestFailed(_)) => Ok(None),
        Err(RequestError::OtherError(err)) => Err(err),
    }
}
