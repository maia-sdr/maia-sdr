use super::ad9361::ad9361_json;
use super::json_error::JsonError;
use super::recording::{recorder_json, recording_metadata_json, Recorder};
use super::spectrometer::{self, spectrometer_json};
use super::time::time_json;
use crate::iio::Ad9361;
use anyhow::Result;
use axum::{extract::State, Json};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Api {
    ad9361: Arc<tokio::sync::Mutex<Ad9361>>,
    spectrometer: spectrometer::State,
    recorder: Recorder,
}

impl Api {
    pub fn new(
        ad9361: Arc<tokio::sync::Mutex<Ad9361>>,
        spectrometer: spectrometer::State,
        recorder: Recorder,
    ) -> Api {
        Api {
            ad9361,
            spectrometer,
            recorder,
        }
    }

    async fn json(&self) -> Result<maia_json::Api> {
        let ad9361 = {
            let ad9361 = self.ad9361.lock().await;
            ad9361_json(&ad9361).await
        }?;
        let spectrometer = spectrometer_json(&self.spectrometer).await?;
        let recorder = recorder_json(&self.recorder).await;
        let recording_metadata = recording_metadata_json(&self.recorder).await;
        let time = time_json()?;
        Ok(maia_json::Api {
            ad9361,
            spectrometer,
            recorder,
            recording_metadata,
            time,
        })
    }
}

pub async fn get_api(State(api): State<Api>) -> Result<Json<maia_json::Api>, JsonError> {
    api.json().await.map_err(JsonError::server_error).map(Json)
}
