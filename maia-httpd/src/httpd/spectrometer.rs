use super::json_error::JsonError;
use crate::{fpga::IpCore, iio::Ad9361, spectrometer::SpectrometerConfig};
use anyhow::Result;
use axum::Json;
use maia_json::{PatchSpectrometer, Spectrometer};
use std::sync::Arc;

// TODO: do not hardcode FFT size
const FFT_SIZE: u32 = 4096;

#[derive(Debug, Clone)]
pub struct State {
    pub ip_core: Arc<std::sync::Mutex<IpCore>>,
    pub ad9361: Arc<tokio::sync::Mutex<Ad9361>>,
    pub spectrometer_config: SpectrometerConfig,
}

impl State {
    async fn samp_rate(&self) -> Result<f64> {
        Ok(self.ad9361.lock().await.get_sampling_frequency().await? as f64)
    }
}

pub async fn spectrometer_json(state: &State) -> Result<Spectrometer> {
    let samp_rate = state.samp_rate().await?;
    let ip_core = state.ip_core.lock().unwrap();
    let num_integrations = ip_core.spectrometer_number_integrations();
    let mode = ip_core.spectrometer_mode();
    drop(ip_core);
    state
        .spectrometer_config
        .set_samp_rate_mode(samp_rate as f32, mode);
    Ok(Spectrometer {
        input_sampling_frequency: samp_rate,
        output_sampling_frequency: samp_rate / (f64::from(FFT_SIZE) * f64::from(num_integrations)),
        number_integrations: num_integrations,
        fft_size: FFT_SIZE,
        mode,
    })
}

async fn get_spectrometer_json(state: &State) -> Result<Json<Spectrometer>, JsonError> {
    spectrometer_json(state)
        .await
        .map_err(JsonError::server_error)
        .map(Json)
}

pub async fn get_spectrometer(
    axum::extract::State(state): axum::extract::State<State>,
) -> Result<Json<Spectrometer>, JsonError> {
    get_spectrometer_json(&state).await
}

async fn update_spectrometer(state: &State, patch: &PatchSpectrometer) -> Result<()> {
    if let Some(mode) = &patch.mode {
        state.ip_core.lock().unwrap().set_spectrometer_mode(*mode);
    }
    match patch {
        PatchSpectrometer {
            number_integrations: Some(n),
            ..
        } => state
            .ip_core
            .lock()
            .unwrap()
            .set_spectrometer_number_integrations(*n)?,
        PatchSpectrometer {
            output_sampling_frequency: Some(out_freq),
            ..
        } => {
            let in_freq = state.samp_rate().await?;
            let num_integrations = (in_freq / (f64::from(FFT_SIZE) * *out_freq))
                .round()
                .clamp(1.0, f64::from(u32::MAX)) as u32;
            state
                .ip_core
                .lock()
                .unwrap()
                .set_spectrometer_number_integrations(num_integrations)?
        }
        _ => {
            // No parameters were specified. We don't do anything.
        }
    }
    Ok(())
}

pub async fn patch_spectrometer(
    axum::extract::State(state): axum::extract::State<State>,
    Json(patch): Json<PatchSpectrometer>,
) -> Result<Json<Spectrometer>, JsonError> {
    update_spectrometer(&state, &patch)
        .await
        .map_err(JsonError::server_error)?;
    get_spectrometer_json(&state).await
}
