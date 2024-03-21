use super::json_error::JsonError;
use crate::fpga::IpCore;
use anyhow::Result;
use axum::{extract::State, Json};
use maia_json::{DDCConfig, PutDDCConfig};
use std::sync::{Arc, Mutex};

pub async fn get_ddc_config(State(ip_core): State<Arc<Mutex<IpCore>>>) -> Json<DDCConfig> {
    let config = ip_core.lock().unwrap().ddc_config().clone();
    Json(config)
}

fn set_ddc_config(ip_core: &Mutex<IpCore>, config: PutDDCConfig) -> Result<()> {
    ip_core.lock().unwrap().set_ddc_config(&config)
}

pub async fn put_ddc_config(
    State(ip_core): State<Arc<Mutex<IpCore>>>,
    Json(put): Json<PutDDCConfig>,
) -> Result<Json<()>, JsonError> {
    set_ddc_config(&ip_core, put)
        .map_err(JsonError::server_error)
        .map(Json)
}
