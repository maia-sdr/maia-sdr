use super::json_error::JsonError;
use anyhow::Result;
use axum::{extract::Path, http::header, response::IntoResponse};
use std::io::Read;

pub async fn decompress_asset(filename: &str) -> Result<impl IntoResponse + use<>> {
    let path = std::path::Path::new("iqengine").join(format!("{}.lz4", filename));
    let compressed = tokio::fs::read(path).await?;
    let decompressed = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let mut decoder = lz4_flex::frame::FrameDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    })
    .await??;
    let mime = mime_guess::from_path(filename).first_or_octet_stream();
    Ok((
        [(header::CONTENT_TYPE, mime.as_ref().to_string())],
        decompressed,
    ))
}

pub async fn serve_assets(Path(filename): Path<String>) -> Result<impl IntoResponse, JsonError> {
    decompress_asset(&filename)
        .await
        .map_err(JsonError::server_error)
}
