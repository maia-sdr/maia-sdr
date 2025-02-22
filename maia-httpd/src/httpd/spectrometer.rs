use super::json_error::JsonError;
use crate::app::AppState;
use anyhow::Result;
use axum::{Json, extract::State};
use maia_json::{PatchSpectrometer, Spectrometer};

// TODO: do not hardcode FFT size
const FFT_SIZE: u32 = 4096;

pub async fn spectrometer_json(state: &AppState) -> Result<Spectrometer> {
    let ad9361_samp_rate = state.ad9361_samp_rate().await?;
    let ip_core = state.ip_core().lock().unwrap();
    let samp_rate = ad9361_samp_rate / ip_core.spectrometer_input_decimation() as f64;
    let input = ip_core.spectrometer_input();
    let num_integrations = ip_core.spectrometer_number_integrations();
    let mode = ip_core.spectrometer_mode();
    drop(ip_core);
    state
        .spectrometer_config()
        .set_samp_rate_mode(samp_rate as f32, mode);
    Ok(Spectrometer {
        input,
        input_sampling_frequency: samp_rate,
        output_sampling_frequency: samp_rate / (f64::from(FFT_SIZE) * f64::from(num_integrations)),
        number_integrations: num_integrations,
        fft_size: FFT_SIZE,
        mode,
    })
}

async fn get_spectrometer_json(state: &AppState) -> Result<Json<Spectrometer>, JsonError> {
    spectrometer_json(state)
        .await
        .map_err(JsonError::server_error)
        .map(Json)
}

pub async fn get_spectrometer(
    State(state): State<AppState>,
) -> Result<Json<Spectrometer>, JsonError> {
    get_spectrometer_json(&state).await
}

async fn update_spectrometer(state: &AppState, patch: &PatchSpectrometer) -> Result<(), JsonError> {
    let ad9361_samp_rate = state
        .ad9361_samp_rate()
        .await
        .map_err(JsonError::server_error)?;
    if let Some(input) = &patch.input {
        state
            .ip_core()
            .lock()
            .unwrap()
            .set_spectrometer_input(*input, ad9361_samp_rate)
            .map_err(JsonError::client_error_alert)?;
    }
    if let Some(mode) = &patch.mode {
        state.ip_core().lock().unwrap().set_spectrometer_mode(*mode);
    }
    match patch {
        PatchSpectrometer {
            number_integrations: Some(n),
            ..
        } => state
            .ip_core()
            .lock()
            .unwrap()
            .set_spectrometer_number_integrations(*n)
            .map_err(JsonError::client_error)?,
        PatchSpectrometer {
            output_sampling_frequency: Some(out_freq),
            ..
        } => {
            let mut ip_core = state.ip_core().lock().unwrap();
            let in_freq = ad9361_samp_rate / ip_core.spectrometer_input_decimation() as f64;
            let num_integrations = (in_freq / (f64::from(FFT_SIZE) * *out_freq))
                .round()
                .clamp(1.0, f64::from(u32::MAX)) as u32;
            ip_core
                .set_spectrometer_number_integrations(num_integrations)
                .map_err(JsonError::client_error)?;
        }
        _ => {
            // No parameters were specified. We don't do anything.
        }
    }
    Ok(())
}

pub async fn patch_spectrometer(
    State(state): State<AppState>,
    Json(patch): Json<PatchSpectrometer>,
) -> Result<Json<Spectrometer>, JsonError> {
    update_spectrometer(&state, &patch).await?;
    get_spectrometer_json(&state).await
}
