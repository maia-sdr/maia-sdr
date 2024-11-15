use super::{
    ad9361::ad9361_json,
    ddc::ddc_json,
    geolocation::device_geolocation,
    json_error::JsonError,
    recording::{recorder_json, recording_metadata_json},
    spectrometer::spectrometer_json,
    time::time_json,
};
use crate::app::AppState;
use anyhow::Result;
use axum::{extract::State, Json};

async fn api_json(state: &AppState) -> Result<maia_json::Api> {
    let ad9361 = {
        let ad9361 = state.ad9361().lock().await;
        ad9361_json(&ad9361).await
    }?;
    let ddc = ddc_json(state).await?;
    let spectrometer = spectrometer_json(state).await?;
    let recorder = recorder_json(state).await?;
    let recording_metadata = recording_metadata_json(state).await;
    let geolocation = device_geolocation(state);
    let time = time_json()?;
    Ok(maia_json::Api {
        ad9361,
        ddc,
        geolocation,
        spectrometer,
        recorder,
        recording_metadata,
        time,
    })
}

pub async fn get_api(State(state): State<AppState>) -> Result<Json<maia_json::Api>, JsonError> {
    api_json(&state)
        .await
        .map_err(JsonError::server_error)
        .map(Json)
}
