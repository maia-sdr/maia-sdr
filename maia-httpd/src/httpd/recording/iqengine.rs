use super::super::json_error::JsonError;
use super::{unpack_12bit_to_16bit, Recorder, RecorderMode, RecordingBuffer, RecordingBufferInfo};
use anyhow::Result;
use axum::extract::{Query, State};
use bytes::{Bytes, BytesMut};
use serde_json::json;
use std::collections::HashMap;

async fn get_meta(recorder: &Recorder) -> Result<serde_json::Value> {
    let metadata = recorder.metadata.lock().await.clone();
    let mut meta = metadata.sigmf_meta.to_json_value();

    // compute recording length
    let recorder_next_address = recorder.ip_core.lock().unwrap().recorder_next_address();
    let buffer_info =
        RecordingBufferInfo::new(metadata.mode, recorder_next_address, metadata.max_samples())
            .await?;
    let sample_length = buffer_info.num_items();

    // add traceability, which is required by IQEngine
    let global = meta.get_mut("global").unwrap().as_object_mut().unwrap();
    global.insert("traceability:revision".to_string(), json!(0));
    global.insert(
        "traceability:origin".to_string(),
        json!({
            "type": "api",
            "account": "maiasdr",
            "container": "maiasdr",
            "file_path": "recording",
        }),
    );
    global.insert(
        "traceability:sample_length".to_string(),
        json!(sample_length),
    );
    Ok(meta)
}

pub async fn meta(State(recorder): State<Recorder>) -> Result<String, JsonError> {
    get_meta(&recorder)
        .await
        .map_err(JsonError::server_error)
        .map(|r| serde_json::to_string(&r).unwrap())
}

async fn get_iq_data(
    recorder: &Recorder,
    block_indexes: &[usize],
    block_size: usize,
) -> Result<Bytes> {
    let metadata = recorder.metadata.lock().await.clone();
    let recorder_next_address = recorder.ip_core.lock().unwrap().recorder_next_address();
    let buffer =
        RecordingBuffer::new(metadata.mode, recorder_next_address, metadata.max_samples()).await?;

    let bytes_per_input = buffer.info.input_bytes_per_item;
    let bytes_per_output = buffer.info.mode.output_bytes_per_item();
    let mut bytes = BytesMut::with_capacity(block_indexes.len() * block_size * bytes_per_output);
    for &idx in block_indexes {
        let start = idx * block_size * bytes_per_input;
        let len = block_size * bytes_per_input;
        if start + len >= buffer.info.size {
            anyhow::bail!("requested data is out of bounds");
        }
        let data = unsafe { std::slice::from_raw_parts(buffer.base.add(start), len) };
        match buffer.info.mode.0 {
            RecorderMode::IQ8bit => bytes.extend_from_slice(data),
            RecorderMode::IQ12bit => {
                let len0 = bytes.len();
                bytes.resize(len0 + block_size * bytes_per_output, 0);
                unpack_12bit_to_16bit(&mut bytes[len0..], data);
            }
        }
    }

    Ok(bytes.into())
}

async fn get_iq_data_params(
    recorder: &Recorder,
    params: &HashMap<String, String>,
) -> Result<Bytes> {
    let block_indexes_str = params
        .get("block_indexes_str")
        .ok_or_else(|| anyhow::anyhow!("block_indexes_str missing"))?;
    let mut block_indexes = Vec::new();
    for w in block_indexes_str.split(',') {
        block_indexes.push(w.parse()?);
    }
    let block_size = params
        .get("block_size")
        .ok_or_else(|| anyhow::anyhow!("block_size missing"))?
        .parse()?;
    get_iq_data(recorder, &block_indexes, block_size).await
}

pub async fn iq_data(
    State(recorder): State<Recorder>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Bytes, JsonError> {
    get_iq_data_params(&recorder, &params)
        .await
        .map_err(JsonError::server_error)
}

// See
// https://github.com/IQEngine/IQEngine/blob/main/api/app/iq_router.py#L213
// for details
async fn get_minimap_data(recorder: &Recorder) -> Result<Bytes> {
    const FFT_SIZE: usize = 64;
    const NUM_FFTS: usize = 1000;

    let metadata = recorder.metadata.lock().await.clone();
    let recorder_next_address = recorder.ip_core.lock().unwrap().recorder_next_address();
    let buffer =
        RecordingBuffer::new(metadata.mode, recorder_next_address, metadata.max_samples()).await?;

    let bytes_per_input = buffer.info.input_bytes_per_item;
    let bytes_per_output = buffer.info.mode.output_bytes_per_item();

    let total_ffts = buffer.info.num_items() / FFT_SIZE;

    let mut bytes = BytesMut::with_capacity(NUM_FFTS * FFT_SIZE * bytes_per_output);
    for j in 0..NUM_FFTS {
        let idx = j * total_ffts / NUM_FFTS;
        let start = idx * FFT_SIZE * bytes_per_input;
        let len = FFT_SIZE * bytes_per_input;
        if start + len >= buffer.info.size {
            anyhow::bail!("requested data is out of bounds");
        }
        let data = unsafe { std::slice::from_raw_parts(buffer.base.add(start), len) };
        match buffer.info.mode.0 {
            RecorderMode::IQ8bit => bytes.extend_from_slice(data),
            RecorderMode::IQ12bit => {
                let len0 = bytes.len();
                bytes.resize(len0 + FFT_SIZE * bytes_per_output, 0);
                unpack_12bit_to_16bit(&mut bytes[len0..], data);
            }
        }
    }

    Ok(bytes.into())
}

pub async fn minimap_data(State(recorder): State<Recorder>) -> Result<Bytes, JsonError> {
    get_minimap_data(&recorder)
        .await
        .map_err(JsonError::server_error)
}
