use super::json_error::JsonError;
use crate::iio;
use anyhow::Result;
use axum::extract::{Json, State};
use maia_json::{Ad9361, PatchAd9361};
use std::sync::Arc;
use tokio::sync::Mutex;

macro_rules! get_attributes {
    ($iio:expr, $($attribute:ident),*) => {
        paste::paste! {
            Ad9361 {
                $(
                    $attribute: $iio.[<get_ $attribute>]().await?.into(),
                )*
            }
        }
    }
}

macro_rules! try_set_attributes {
    ($iio:expr, $json:expr, $($attribute:ident),*) => {
        paste::paste! {
            $(
                if let Some(value) = $json.$attribute {
                    $iio.[<set_ $attribute>](value.into()).await?;
                }
            )*
        }
    }
}

pub async fn ad9361_json(iio: &iio::Ad9361) -> Result<Ad9361> {
    Ok(get_attributes!(
        iio,
        sampling_frequency,
        rx_rf_bandwidth,
        tx_rf_bandwidth,
        rx_lo_frequency,
        tx_lo_frequency,
        rx_gain,
        rx_gain_mode,
        tx_gain
    ))
}

async fn ad9361_update(iio: &iio::Ad9361, json: &PatchAd9361) -> Result<()> {
    try_set_attributes!(
        iio,
        json,
        sampling_frequency,
        rx_rf_bandwidth,
        tx_rf_bandwidth,
        rx_lo_frequency,
        tx_lo_frequency,
        // it is important to set the gain mode before the gain
        rx_gain_mode,
        rx_gain,
        tx_gain
    );
    Ok(())
}

async fn get_ad9361_json(iio: &iio::Ad9361) -> Result<Json<Ad9361>, JsonError> {
    ad9361_json(iio)
        .await
        .map_err(JsonError::server_error)
        .map(Json)
}

pub async fn get_ad9361(
    State(iio): State<Arc<Mutex<iio::Ad9361>>>,
) -> Result<Json<Ad9361>, JsonError> {
    let iio = iio.lock().await;
    get_ad9361_json(&iio).await
}

async fn patch_ad9361_json(
    iio: Arc<Mutex<iio::Ad9361>>,
    patch: &PatchAd9361,
) -> Result<Json<Ad9361>, JsonError> {
    let iio = iio.lock().await;
    ad9361_update(&iio, patch)
        .await
        .map_err(JsonError::server_error)?;
    get_ad9361_json(&iio).await
}

pub async fn put_ad9361(
    State(iio): State<Arc<Mutex<iio::Ad9361>>>,
    Json(put): Json<Ad9361>,
) -> Result<Json<Ad9361>, JsonError> {
    let patch = PatchAd9361::from(put);
    patch_ad9361_json(iio, &patch).await
}

pub async fn patch_ad9361(
    State(iio): State<Arc<Mutex<iio::Ad9361>>>,
    Json(patch): Json<PatchAd9361>,
) -> Result<Json<Ad9361>, JsonError> {
    patch_ad9361_json(iio, &patch).await
}
