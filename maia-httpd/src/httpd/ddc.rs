use super::json_error::JsonError;
use crate::{app::AppState, ddc};
use anyhow::Result;
use axum::{Json, extract::State};
use maia_json::{DDCConfig, DDCConfigSummary, PatchDDCConfig, PutDDCConfig, PutDDCDesign};

async fn ddc_config(state: &AppState) -> Result<Json<DDCConfig>, JsonError> {
    let samp_rate = state
        .ad9361_samp_rate()
        .await
        .map_err(JsonError::server_error)?;
    Ok(Json(state.ip_core().lock().unwrap().ddc_config(samp_rate)))
}

pub async fn ddc_json(state: &AppState) -> Result<DDCConfigSummary> {
    let samp_rate = state.ad9361_samp_rate().await?;
    Ok(state
        .ip_core()
        .lock()
        .unwrap()
        .ddc_config_summary(samp_rate))
}

pub async fn get_ddc_config(State(state): State<AppState>) -> Result<Json<DDCConfig>, JsonError> {
    ddc_config(&state).await
}

async fn set_ddc_config(state: &AppState, config: PutDDCConfig) -> Result<(), JsonError> {
    let samp_rate = state
        .ad9361_samp_rate()
        .await
        .map_err(JsonError::server_error)?;
    state
        .ip_core()
        .lock()
        .unwrap()
        .set_ddc_config(&config, samp_rate)
        .map_err(JsonError::client_error_alert)
}

pub async fn put_ddc_config(
    State(state): State<AppState>,
    Json(put): Json<PutDDCConfig>,
) -> Result<Json<DDCConfig>, JsonError> {
    set_ddc_config(&state, put).await?;
    ddc_config(&state).await
}

pub async fn patch_ddc_config(
    State(state): State<AppState>,
    Json(patch): Json<PatchDDCConfig>,
) -> Result<Json<DDCConfig>, JsonError> {
    if let Some(frequency) = patch.frequency {
        let samp_rate = state
            .ad9361_samp_rate()
            .await
            .map_err(JsonError::server_error)?;
        state
            .ip_core()
            .lock()
            .unwrap()
            .set_ddc_frequency(frequency, samp_rate)
            .map_err(JsonError::client_error_alert)?;
    }
    ddc_config(&state).await
}

async fn set_ddc_design(state: &AppState, design: PutDDCDesign) -> Result<(), JsonError> {
    let samp_rate = state
        .ad9361_samp_rate()
        .await
        .map_err(JsonError::server_error)?;
    // The DDC design can take a couple seconds to calculate, so it is run in a
    // blocking thread.
    let config = tokio::task::spawn_blocking(move || ddc::make_design(&design, samp_rate))
        .await
        .map_err(JsonError::server_error)?
        .map_err(JsonError::client_error_alert)?;
    state
        .ip_core()
        .lock()
        .unwrap()
        .set_ddc_config(&config, samp_rate)
        // If the design was successful, this call should succeed. If it doesn't
        // it's a server error, not a client parameters error.
        .map_err(JsonError::server_error)
}

pub async fn put_ddc_design(
    State(state): State<AppState>,
    Json(put): Json<PutDDCDesign>,
) -> Result<Json<DDCConfig>, JsonError> {
    set_ddc_design(&state, put).await?;
    ddc_config(&state).await
}
