use super::json_error::JsonError;
use anyhow::Result;
use axum::Json;
use maia_json::{PatchTime, Time};
use nix::{
    sys::time::{TimeSpec, time_t},
    time::ClockId,
};
use std::{ffi::c_long, time::UNIX_EPOCH};

pub fn time_json() -> Result<Time> {
    let milliseconds = UNIX_EPOCH.elapsed()?.as_secs_f64() * 1e3;
    Ok(Time { time: milliseconds })
}

fn set_time(patch: &PatchTime) -> Result<()> {
    if let Some(milliseconds) = patch.time {
        let seconds = (milliseconds * 1e-3) as i64;
        let nanoseconds = milliseconds * 1e6 - seconds as f64 * 1e9;
        let timespec = TimeSpec::new(seconds as time_t, nanoseconds as c_long);
        ClockId::CLOCK_REALTIME.set_time(timespec)?;
    }
    Ok(())
}

fn set_and_get_time(patch: &PatchTime) -> Result<Time> {
    set_time(patch)?;
    time_json()
}

pub async fn get_time() -> Result<Json<Time>, JsonError> {
    time_json().map_err(JsonError::server_error).map(Json)
}

pub async fn put_time(Json(patch): Json<Time>) -> Result<Json<Time>, JsonError> {
    set_and_get_time(&PatchTime::from(patch))
        .map_err(JsonError::server_error)
        .map(Json)
}

pub async fn patch_time(Json(patch): Json<PatchTime>) -> Result<Json<Time>, JsonError> {
    set_and_get_time(&patch)
        .map_err(JsonError::server_error)
        .map(Json)
}
