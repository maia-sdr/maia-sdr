use super::json_error::JsonError;
use crate::app::AppState;
use anyhow::Result;
use axum::{extract::State, Json};
use maia_json::{DeviceGeolocation, Geolocation};

pub fn device_geolocation(state: &AppState) -> DeviceGeolocation {
    DeviceGeolocation {
        point: state.geolocation().lock().unwrap().clone(),
    }
}

pub async fn get_geolocation(State(state): State<AppState>) -> Json<DeviceGeolocation> {
    Json(device_geolocation(&state))
}

fn validate_geolocation(geolocation: Geolocation) -> Result<Geolocation> {
    anyhow::ensure!(
        (-90.0..=90.0).contains(&geolocation.latitude),
        "latitude is not between -90 and +90 degrees"
    );
    anyhow::ensure!(
        (-180.0..=180.0).contains(&geolocation.longitude),
        "longitude is not between -180 and +180 degrees"
    );
    Ok(geolocation)
}

pub async fn put_geolocation(
    State(state): State<AppState>,
    Json(put): Json<DeviceGeolocation>,
) -> Result<Json<DeviceGeolocation>, JsonError> {
    let geolocation = put
        .point
        .map(validate_geolocation)
        .transpose()
        .map_err(JsonError::client_error_alert)?;
    state.geolocation().lock().unwrap().clone_from(&geolocation);
    Ok(Json(DeviceGeolocation { point: geolocation }))
}
