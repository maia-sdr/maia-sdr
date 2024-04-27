use super::json_error::JsonError;
use crate::{app::AppState, fpga::IpCore};
use anyhow::Result;
use axum::{extract::State, response::Html};
use std::sync::Mutex;

async fn fw_version() -> Result<String> {
    let iio_info = tokio::fs::read_to_string("/etc/libiio.ini").await?;
    for line in iio_info.lines() {
        if let Some(version) = line.strip_prefix("fw_version=") {
            return Ok(version.to_string());
        }
    }
    Err(anyhow::anyhow!(
        "/etc/libiio.ini does not contain fw_version"
    ))
}

async fn version(ip_core: &Mutex<IpCore>) -> Result<String> {
    Ok(format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8">
    <title>Maia SDR</title>
    <script type="module">
      import init, {{ maia_wasm_git_version, maia_wasm_version }} from "./pkg/maia_wasm.js";

      async function run() {{
          await init();
          document.getElementById('maia-wasm-git-version').innerHTML = maia_wasm_git_version();
          document.getElementById('maia-wasm-version').innerHTML = maia_wasm_version();
      }};

      run();
    </script>
  </head>
  <body>

    <p>Firmware version: {}</p>
    <p>maia-sdr git version for maia-httpd: {}</p>
    <p>maia-httpd version: {}</p>
    <p>maia-hdl version: {}</p>
    <p>maia-sdr git version for maia-wasm: <span id="maia-wasm-git-version"></span></p>
    <p>maia-wasm version: <span id="maia-wasm-version"></span></p>

  </body>
</html>
"#,
        fw_version().await?,
        git_version::git_version!(fallback = "unknown"),
        env!("CARGO_PKG_VERSION"),
        ip_core.lock().unwrap().version(),
    ))
}

pub async fn get_version(State(state): State<AppState>) -> Result<Html<String>, JsonError> {
    version(state.ip_core())
        .await
        .map_err(JsonError::server_error)
        .map(Html)
}
