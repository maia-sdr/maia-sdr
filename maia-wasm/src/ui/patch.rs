use serde::Serialize;
use wasm_bindgen::JsValue;
use web_sys::{Request, RequestInit};

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
