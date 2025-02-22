use super::json_error::JsonError;
use crate::{app::AppState, iio};
use anyhow::Result;
use axum::{Json, extract::State};
use maia_json::{Ad9361, PatchAd9361};

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
                    $iio.[<set_ $attribute>](value.into()).await.map_err(JsonError::server_error)?;
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

async fn ad9361_update(
    state: &AppState,
    iio: &iio::Ad9361,
    json: &PatchAd9361,
) -> Result<(), JsonError> {
    if let Some(freq) = json.sampling_frequency {
        // here the input sample rate to the DDC does not matter, because we only
        // need its config to check the maximum input sampling frequency and the enable
        let ddc_config = state.ip_core().lock().unwrap().ddc_config_summary(0.0);
        // check that DDC can support this input frequency if it is enabled
        if ddc_config.enabled && f64::from(freq) > ddc_config.max_input_sampling_frequency {
            return Err(JsonError::client_error_alert(anyhow::anyhow!(
                "tried to set AD9361 sampling rate to {freq}, \
                           but DDC is enabled and its maximum input sampling frequency is {}",
                ddc_config.max_input_sampling_frequency
            )));
        }
        iio.set_sampling_frequency(freq)
            .await
            .map_err(JsonError::server_error)?;

        // maintain the DDC frequency after the sample rate change
        state
            .ip_core()
            .lock()
            .unwrap()
            .set_ddc_frequency(ddc_config.frequency, f64::from(freq))
            .map_err(JsonError::client_error_alert)?;
    }
    try_set_attributes!(
        iio,
        json,
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

pub async fn get_ad9361(State(state): State<AppState>) -> Result<Json<Ad9361>, JsonError> {
    let iio = state.ad9361().lock().await;
    get_ad9361_json(&iio).await
}

async fn patch_ad9361_json(
    State(state): State<AppState>,
    patch: &PatchAd9361,
) -> Result<Json<Ad9361>, JsonError> {
    let iio = state.ad9361().lock().await;
    ad9361_update(&state, &iio, patch).await?;
    get_ad9361_json(&iio).await
}

pub async fn put_ad9361(
    state: State<AppState>,
    Json(put): Json<Ad9361>,
) -> Result<Json<Ad9361>, JsonError> {
    let patch = PatchAd9361::from(put);
    patch_ad9361_json(state, &patch).await
}

pub async fn patch_ad9361(
    state: State<AppState>,
    Json(patch): Json<PatchAd9361>,
) -> Result<Json<Ad9361>, JsonError> {
    patch_ad9361_json(state, &patch).await
}
