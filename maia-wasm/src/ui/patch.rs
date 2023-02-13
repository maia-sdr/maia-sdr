use serde::Serialize;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

pub fn json_patch<T: Serialize>(url: &str, json: &T) -> Result<Request, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("PATCH");
    let json =
        serde_json::to_string(json).map_err(|_| "unable to format JSON for PATCH request")?;
    opts.body(Some(&json.into()));
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

pub enum PatchError {
    RequestFailed(String),
    OtherError(JsValue),
}

impl From<JsValue> for PatchError {
    fn from(value: JsValue) -> PatchError {
        PatchError::OtherError(value)
    }
}

pub fn ignore_request_failed<T>(x: Result<T, PatchError>) -> Result<(), JsValue> {
    match x {
        Err(PatchError::OtherError(err)) => Err(err),
        _ => Ok(()),
    }
}
