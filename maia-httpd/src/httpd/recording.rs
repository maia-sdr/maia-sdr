use super::json_error::JsonError;
use crate::app::AppState;
use crate::fpga::{InterruptWaiter, IpCore};
use crate::iio::Ad9361;
use crate::sigmf;
use anyhow::Result;
use axum::{body::Body, extract::State, Json};
use bytes::{Bytes, BytesMut};
use futures::Stream;
use http::header::{HeaderMap, CONTENT_DISPOSITION, CONTENT_LENGTH};
use maia_json::RecorderMode;
use std::os::unix::io::AsRawFd;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::fs;
use tokio::io::DuplexStream;
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use tokio_util::{io::ReaderStream, sync::CancellationToken};

pub mod iqengine;

type InProgress = tokio::sync::Mutex<Option<OwnedRwLockWriteGuard<RecordingBuffer>>>;

/// Recorder state.
///
/// This struct contains the state of the recorder. It is used by the REST API.
#[derive(Debug)]
pub struct RecorderState {
    metadata: tokio::sync::Mutex<RecordingMeta>,
    buffer: Arc<RwLock<RecordingBuffer>>,
    recording_in_progress: InProgress,
}

/// Recorder finish waiter.
///
/// This struct implements a [`run`](RecorderFinishWaiter::run) async method
/// that should be run concurrently with the rest of the application. The method
/// waits until an interrupt notifying of the end of a recording is received,
/// and then updates the recording state accordingly.
#[derive(Debug)]
pub struct RecorderFinishWaiter {
    state: AppState,
    waiter: InterruptWaiter,
}

impl RecorderState {
    /// Creates a new recorder state.
    pub async fn new(
        ad9361: &tokio::sync::Mutex<Ad9361>,
        ip_core: &std::sync::Mutex<IpCore>,
    ) -> Result<RecorderState> {
        let metadata = tokio::sync::Mutex::new(RecordingMeta::new(ad9361, ip_core).await?);
        let buffer = Arc::new(RwLock::new(RecordingBuffer::new().await?));
        let recording_in_progress = tokio::sync::Mutex::new(None);
        Ok(RecorderState {
            metadata,
            buffer,
            recording_in_progress,
        })
    }
}

impl RecorderFinishWaiter {
    /// Creates a new recorder finish waiter.
    ///
    /// The `waiter` is the [`InterruptWaiter`] corresponding to the recorder
    /// interrupt. This function only creates the object. The
    /// [`run`](RecorderFinishWaiter) method needs to be called afterwards.
    pub fn new(state: AppState, waiter: InterruptWaiter) -> RecorderFinishWaiter {
        RecorderFinishWaiter { state, waiter }
    }

    /// Runs the recorder finish waiter.
    ///
    /// This function loops forever, waiting for interrupts and updating the
    /// state of the recorder. The function only returns if there is an error.
    pub async fn run(self) -> Result<()> {
        loop {
            self.waiter.wait().await;
            tracing::info!("recorder finished");
            {
                let mut in_progress = self.state.recorder().recording_in_progress.lock().await;
                if let Some(buffer) = in_progress.as_mut() {
                    // mmap() the buffer again to invalidate the cache
                    **buffer = RecordingBuffer::new().await?;
                }
                *in_progress = None;
            }
            let mut metadata = self.state.recorder().metadata.lock().await;
            // Cancel the stop timer (perhaps it has already expired, but this
            // doesn't matter).
            if let Some(token) = metadata.stop_timer_cancellation.take() {
                token.cancel()
            }
            metadata.recorder_state = maia_json::RecorderState::Stopped;
        }
    }
}

#[derive(Debug, Clone)]
struct RecordingMeta {
    sigmf_meta: sigmf::Metadata,
    mode: RecorderMode,
    filename: String,
    prepend_timestamp: bool,
    maximum_duration: Option<Duration>,
    stop_timer_cancellation: Option<CancellationToken>,
    recorder_state: maia_json::RecorderState,
}

impl RecordingMeta {
    async fn new(
        ad9361: &tokio::sync::Mutex<Ad9361>,
        ip_core: &std::sync::Mutex<IpCore>,
    ) -> Result<RecordingMeta> {
        let mode;
        let decimation;
        {
            let ip_core = ip_core.lock().unwrap();
            mode = ip_core.recorder_mode()?;
            decimation = ip_core.recorder_input_decimation();
        }
        let datatype = mode.into();
        let sample_rate;
        let frequency;
        {
            let ad9361 = ad9361.lock().await;
            sample_rate = ad9361.get_sampling_frequency().await? as f64 / decimation as f64;
            frequency = ad9361.get_rx_lo_frequency().await? as f64;
        }
        let sigmf_meta = sigmf::Metadata::new(datatype, sample_rate, frequency);
        let filename = "recording".to_string();
        let recorder_state = maia_json::RecorderState::Stopped;
        Ok(RecordingMeta {
            sigmf_meta,
            mode,
            filename,
            prepend_timestamp: false,
            maximum_duration: None,
            stop_timer_cancellation: None,
            recorder_state,
        })
    }

    async fn update_for_new_recording(&mut self, state: &AppState) -> Result<()> {
        if let Some(geolocation) = state.geolocation().lock().unwrap().as_ref() {
            // It is assumed that the geolocation has been validated, so it
            // should not error when converting to a GeoJSON point.
            self.sigmf_meta
                .set_geolocation(geolocation.clone().try_into().unwrap())
        } else {
            self.sigmf_meta.remove_geolocation();
        }
        self.sigmf_meta.set_datetime_now();

        if let Some(duration) = self.maximum_duration {
            // set up timer task to automatically stop the recording
            let token = CancellationToken::new();
            // stop_timer_cancellation should always be None in the Stopped
            // state (which is when this function can be called).
            assert!(self.stop_timer_cancellation.is_none());
            self.stop_timer_cancellation = Some(token.clone());
            {
                let state = state.clone();
                tokio::spawn(async move {
                    tokio::select! {
                        _ = token.cancelled() => return,
                        // add 0.1 s duration to the time to sleep in case the ADC
                        // sample clock is slower than our clock
                        _ = tokio::time::sleep(duration + Duration::from_millis(100)) => {}
                    };
                    state.ip_core().lock().unwrap().recorder_stop()
                });
            }
        }

        if self.prepend_timestamp {
            self.prepend_timestamp_to_filename();
        }
        let (offset, decimation) = {
            let ip_core = state.ip_core().lock().unwrap();
            self.mode = ip_core.recorder_mode()?;
            (
                ip_core.recorder_input_frequency_offset(),
                ip_core.recorder_input_decimation(),
            )
        };
        self.sigmf_meta.set_datatype(self.mode.into());
        {
            let ad9361 = state.ad9361().lock().await;
            self.sigmf_meta
                .set_sample_rate(ad9361.get_sampling_frequency().await? as f64 / decimation as f64);
            self.sigmf_meta
                .set_frequency(ad9361.get_rx_lo_frequency().await? as f64 + offset);
        }
        Ok(())
    }

    fn json(&self) -> maia_json::RecordingMetadata {
        maia_json::RecordingMetadata {
            filename: self.filename.clone(),
            description: self.sigmf_meta.description().to_string(),
            author: self.sigmf_meta.author().to_string(),
            geolocation: maia_json::DeviceGeolocation {
                point: self.sigmf_meta.geolocation().map(|g| g.into()),
            },
        }
    }

    fn recorder_json(&self, ip_core: &std::sync::Mutex<IpCore>) -> Result<maia_json::Recorder> {
        Ok(maia_json::Recorder {
            state: self.recorder_state,
            mode: ip_core.lock().unwrap().recorder_mode()?,
            prepend_timestamp: self.prepend_timestamp,
            maximum_duration: self
                .maximum_duration
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
        })
    }

    fn patch_json(&mut self, patch: maia_json::PatchRecordingMetadata) -> Result<()> {
        if let Some(filename) = patch.filename {
            self.filename = filename;
        }
        if let Some(description) = patch.description {
            self.sigmf_meta.set_description(&description);
        }
        if let Some(author) = patch.author {
            self.sigmf_meta.set_author(&author);
        }
        if let Some(geolocation) = patch.geolocation {
            self.sigmf_meta
                .set_geolocation_optional(geolocation.point.map(|g| g.try_into()).transpose()?);
        }
        Ok(())
    }

    fn prepend_timestamp_to_filename(&mut self) {
        // The sigmf metadata has already been set with the timestamp
        // corresponding to the recording start.
        let datetime = self.sigmf_meta.datetime();
        // Remove previous timestamp if there is already one
        let filename = if Self::begins_with_timestamp(&self.filename) {
            &self.filename[Self::TIMESTAMP_LEN..]
        } else {
            &self.filename
        };
        self.filename = format!("{}_{}", datetime.format("%Y-%m-%d-%H-%M-%S"), filename);
    }

    // Timestamp format XXXX-XX-XX-XX-XX-XX_
    const TIMESTAMP_LEN: usize = 20;

    fn begins_with_timestamp(s: &str) -> bool {
        if s.len() < Self::TIMESTAMP_LEN {
            return false;
        }
        for (j, c) in s[..Self::TIMESTAMP_LEN].chars().enumerate() {
            if j == 19 {
                if c != '_' {
                    return false;
                }
            } else if j == 4 || j == 7 || j == 10 || j == 13 || j == 16 {
                if c != '-' {
                    return false;
                }
            } else if !c.is_ascii_digit() {
                return false;
            }
        }
        true
    }

    fn max_samples(&self) -> Option<usize> {
        self.maximum_duration.map(|duration| {
            let samp_rate = self.sigmf_meta.sample_rate();
            (duration.as_secs_f64() * samp_rate).round() as usize
        })
    }
}

pub async fn recorder_json(state: &AppState) -> Result<maia_json::Recorder> {
    state
        .recorder()
        .metadata
        .lock()
        .await
        .recorder_json(state.ip_core())
}

pub async fn get_recorder(
    State(state): State<AppState>,
) -> Result<Json<maia_json::Recorder>, JsonError> {
    recorder_json(&state)
        .await
        .map_err(JsonError::server_error)
        .map(Json)
}

pub async fn patch_recorder(
    State(state): State<AppState>,
    Json(patch): Json<maia_json::PatchRecorder>,
) -> Result<Json<maia_json::Recorder>, JsonError> {
    if let Some(mode) = patch.mode {
        state.ip_core().lock().unwrap().set_recorder_mode(mode);
    }
    let mut metadata = state.recorder().metadata.lock().await;
    if let Some(prepend) = patch.prepend_timestamp {
        metadata.prepend_timestamp = prepend;
    }
    if let Some(duration) = patch.maximum_duration {
        if duration <= 0.0 {
            // Unlimited duration
            metadata.maximum_duration = None;
        } else {
            // Use try_from_secs_f64 to avoid panics when duration overflows
            // Duration or is infinite.
            metadata.maximum_duration = Duration::try_from_secs_f64(duration).ok();
        }
    }
    match (patch.state_change, metadata.recorder_state) {
        (Some(maia_json::RecorderStateChange::Start), maia_json::RecorderState::Stopped) => {
            let lock = state
                .recorder()
                .buffer
                .clone()
                .try_write_owned()
                .map_err(|_| {
                    JsonError::client_error_alert(anyhow::anyhow!(
                        "cannot start new recording: current recording is begin accessed"
                    ))
                })?;
            state
                .recorder()
                .recording_in_progress
                .lock()
                .await
                .replace(lock);
            metadata.recorder_state = maia_json::RecorderState::Running;
            state.ip_core().lock().unwrap().recorder_start();
            metadata
                .update_for_new_recording(&state)
                .await
                .map_err(JsonError::server_error)?;
        }
        (Some(maia_json::RecorderStateChange::Stop), maia_json::RecorderState::Running) => {
            state.ip_core().lock().unwrap().recorder_stop();
            metadata.recorder_state = maia_json::RecorderState::Stopping;
        }
        (_, _) => (),
    }
    metadata
        .recorder_json(state.ip_core())
        .map(Json)
        .map_err(JsonError::server_error)
}

pub async fn recording_metadata_json(state: &AppState) -> maia_json::RecordingMetadata {
    state.recorder().metadata.lock().await.json()
}

pub async fn get_recording_metadata(
    State(state): State<AppState>,
) -> Json<maia_json::RecordingMetadata> {
    Json(recording_metadata_json(&state).await)
}

async fn set_recording_metadata(
    state: &AppState,
    patch: maia_json::PatchRecordingMetadata,
) -> Result<Json<maia_json::RecordingMetadata>, JsonError> {
    let mut metadata = state.recorder().metadata.lock().await;
    metadata
        .patch_json(patch)
        .map_err(JsonError::client_error_alert)?;
    Ok(Json(metadata.json()))
}

pub async fn put_recording_metadata(
    State(state): State<AppState>,
    Json(put): Json<maia_json::RecordingMetadata>,
) -> Result<Json<maia_json::RecordingMetadata>, JsonError> {
    set_recording_metadata(&state, put.into()).await
}

pub async fn patch_recording_metadata(
    State(state): State<AppState>,
    Json(patch): Json<maia_json::PatchRecordingMetadata>,
) -> Result<Json<maia_json::RecordingMetadata>, JsonError> {
    set_recording_metadata(&state, patch).await
}

pub type SigmfStream = ReaderStream<DuplexStream>;

pub async fn get_recording(State(state): State<AppState>) -> Result<(HeaderMap, Body), JsonError> {
    let buffer = state
        .recorder()
        .buffer
        .clone()
        .try_read_owned()
        .map_err(|_| JsonError::client_error_alert(anyhow::anyhow!("recording in progress")))?;
    let metadata = state.recorder().metadata.lock().await.clone();
    let (recording, size) = recording_stream(buffer, &metadata, state.ip_core())
        .await
        .map_err(JsonError::server_error)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}.sigmf\"", metadata.filename)
            .parse()
            .unwrap(),
    );
    headers.insert(CONTENT_LENGTH, size.to_string().parse().unwrap());
    Ok::<_, JsonError>((headers, Body::from_stream(recording)))
}

async fn recording_stream(
    buffer: OwnedRwLockReadGuard<RecordingBuffer>,
    metadata: &RecordingMeta,
    ip_core: &std::sync::Mutex<IpCore>,
) -> Result<(SigmfStream, usize)> {
    const DUPLEX_SIZE: usize = 1 << 20;
    let buffer = RecordingStream::new(buffer, metadata, ip_core).await?;
    let (duplex_write, duplex_read) = tokio::io::duplex(DUPLEX_SIZE);
    let stream = tokio_util::io::ReaderStream::new(duplex_read);

    let mut tar = tokio_tar::Builder::new(duplex_write);
    let filename = &metadata.filename;
    let sigmf_meta = metadata.sigmf_meta.to_json();
    let timestamp = u64::try_from(metadata.sigmf_meta.datetime().timestamp())?;

    // Set up tar headers
    let mut dir_header = tokio_tar::Header::new_ustar();
    dir_header.set_path(format!("{filename}/"))?;
    dir_header.set_size(0);
    dir_header.set_mode(0o0755);
    dir_header.set_entry_type(tokio_tar::EntryType::Directory);
    dir_header.set_mtime(timestamp);
    dir_header.set_cksum();

    let mut meta_header = tokio_tar::Header::new_ustar();
    meta_header.set_path(format!("{filename}/{filename}.sigmf-meta"))?;
    meta_header.set_size(sigmf_meta.len().try_into().unwrap());
    meta_header.set_mode(0o0444);
    meta_header.set_entry_type(tokio_tar::EntryType::Regular);
    meta_header.set_mtime(timestamp);
    meta_header.set_cksum();

    let mut data_header = tokio_tar::Header::new_ustar();
    data_header.set_path(format!("{filename}/{filename}.sigmf-data"))?;
    data_header.set_size(buffer.info.output_size().try_into().unwrap());
    data_header.set_mode(0o0444);
    data_header.set_entry_type(tokio_tar::EntryType::Regular);
    data_header.set_mtime(timestamp);
    data_header.set_cksum();

    let tar_header_size = 512;
    let num_headers = 3;
    let tar_finish_size = 1024;
    let tar_size = tar_header_size * num_headers
        + round_up_multiple_512(sigmf_meta.len())
        + round_up_multiple_512(buffer.info.output_size())
        + tar_finish_size;

    // Write tar into the duplex concurrently
    tokio::spawn(async move {
        let dir_data: &[u8] = &[];
        tar.append(&dir_header, dir_data).await?;
        tar.append(&meta_header, sigmf_meta.as_bytes()).await?;
        tar.append(&data_header, tokio_util::io::StreamReader::new(buffer))
            .await?;
        tar.into_inner().await?;
        Ok::<(), anyhow::Error>(())
    });

    Ok((stream, tar_size))
}

fn round_up_multiple_512(n: usize) -> usize {
    if n & 0x1ff != 0 {
        ((n >> 9) + 1) << 9
    } else {
        n
    }
}

#[derive(Debug)]
struct RecordingBuffer {
    base: *const u8,
    size: usize,
}

unsafe impl Send for RecordingBuffer {}
unsafe impl Sync for RecordingBuffer {}

impl RecordingBuffer {
    async fn new() -> Result<RecordingBuffer> {
        let size = usize::from_str_radix(
            fs::read_to_string("/sys/class/maia-sdr/maia-sdr-recording/device/recording_size")
                .await?
                .trim_end()
                .trim_start_matches("0x"),
            16,
        )?;
        let mem = fs::OpenOptions::new()
            .read(true)
            .open("/dev/maia-sdr-recording")
            .await?;
        // mmap()'ing the buffer can be quite expensive, because the cache is invalidated.
        // We run it with spawn_blocking.
        tokio::task::spawn_blocking(move || unsafe {
            match libc::mmap(
                std::ptr::null_mut::<libc::c_void>(),
                size,
                libc::PROT_READ,
                libc::MAP_SHARED,
                mem.as_raw_fd(),
                0,
            ) {
                libc::MAP_FAILED => Err(anyhow::anyhow!("mmap /dev/maia-sdr-recording failed")),
                x => Ok(RecordingBuffer {
                    base: x as *const u8,
                    size,
                }),
            }
        })
        .await?
    }
}

impl Drop for RecordingBuffer {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base as *mut libc::c_void, self.size);
        }
    }
}

#[derive(Debug)]
struct RecordingStream {
    buffer: OwnedRwLockReadGuard<RecordingBuffer>,
    chunk: *const u8,
    info: RecordingBufferInfo,
}

unsafe impl Send for RecordingStream {}

impl RecordingStream {
    async fn new(
        buffer: OwnedRwLockReadGuard<RecordingBuffer>,
        metadata: &RecordingMeta,
        ip_core: &std::sync::Mutex<IpCore>,
    ) -> Result<RecordingStream> {
        let info = RecordingBufferInfo::new(metadata, ip_core).await?;
        // chunk is a *const u8, which is not Send, so it must not be held
        // accross an await point.
        let chunk = buffer.base;
        Ok(RecordingStream {
            buffer,
            chunk,
            info,
        })
    }
}

impl Stream for RecordingStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let offset = unsafe { self.chunk.offset_from(self.buffer.base) as usize };
        let remaining = self.info.size - offset;
        if remaining < self.info.input_bytes_per_item {
            return Poll::Ready(None);
        }
        let (chunk_bytes, chunk_items) = match remaining {
            x if x >= self.info.chunk_bytes => {
                (self.info.chunk_bytes, RecordingBufferInfo::CHUNK_ITEMS)
            }
            x => {
                let chunk_items = x / self.info.input_bytes_per_item;
                (chunk_items * self.info.input_bytes_per_item, chunk_items)
            }
        };
        let data = unsafe { std::slice::from_raw_parts(self.chunk, chunk_bytes) };
        let bytes = match self.info.mode.0 {
            RecorderMode::IQ8bit | RecorderMode::IQ16bit => Bytes::copy_from_slice(data),
            RecorderMode::IQ12bit => {
                let mut bytes =
                    BytesMut::zeroed(self.info.mode.output_bytes_per_item() * chunk_items);
                unpack_12bit_to_16bit(&mut bytes[..], data);
                Bytes::from(bytes)
            }
        };
        self.chunk = unsafe { self.chunk.add(chunk_bytes) };
        Poll::Ready(Some(Ok(bytes)))
    }
}

#[derive(Debug)]
struct RecordingBufferInfo {
    size: usize,
    mode: Mode,
    input_bytes_per_item: usize,
    chunk_bytes: usize,
}

impl RecordingBufferInfo {
    async fn new(
        metadata: &RecordingMeta,
        ip_core: &std::sync::Mutex<IpCore>,
    ) -> Result<RecordingBufferInfo> {
        let base_address = usize::from_str_radix(
            fs::read_to_string(
                "/sys/class/maia-sdr/maia-sdr-recording/device/recording_base_address",
            )
            .await?
            .trim_end()
            .trim_start_matches("0x"),
            16,
        )?;
        let next_address = ip_core.lock().unwrap().recorder_next_address();

        let mode = Mode(metadata.mode);
        let input_bytes_per_item = mode.input_bytes_per_item();
        let max_size = metadata
            .max_samples()
            .map(|items| items * input_bytes_per_item);
        let size = next_address - base_address;
        // Constrain size <= max_size if max_size.is_some()
        let size = max_size.map(|x| x.min(size)).unwrap_or(size);

        Ok(RecordingBufferInfo {
            size,
            mode,
            input_bytes_per_item,
            chunk_bytes: input_bytes_per_item * Self::CHUNK_ITEMS,
        })
    }

    fn output_size(&self) -> usize {
        self.size / self.input_bytes_per_item * self.mode.output_bytes_per_item()
    }

    fn num_items(&self) -> usize {
        self.size / self.input_bytes_per_item
    }

    const CHUNK_ITEMS: usize = 1 << 16;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct Mode(RecorderMode);

impl Mode {
    fn input_bytes_per_item(&self) -> usize {
        match self.0 {
            RecorderMode::IQ8bit => 2,
            RecorderMode::IQ12bit => 3,
            RecorderMode::IQ16bit => 4,
        }
    }

    fn output_bytes_per_item(&self) -> usize {
        match self.0 {
            RecorderMode::IQ8bit => 2,
            RecorderMode::IQ12bit | RecorderMode::IQ16bit => 4,
        }
    }
}

fn unpack_12bit_to_16bit(output: &mut [u8], input: &[u8]) {
    for (j, x) in input.chunks_exact(3).enumerate() {
        output[4 * j] = (x[0] << 4) | (x[1] >> 4);
        output[4 * j + 1] = ((x[0] & 0xf0) as i8 >> 4) as u8;
        output[4 * j + 2] = x[2];
        output[4 * j + 3] = ((x[1] << 4) as i8 >> 4) as u8;
    }
}
