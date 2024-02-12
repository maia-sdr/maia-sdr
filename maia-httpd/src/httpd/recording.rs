use super::json_error::JsonError;
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
use tokio_util::{io::ReaderStream, sync::CancellationToken};

#[derive(Debug, Clone)]
pub struct Recorder {
    metadata: Arc<tokio::sync::Mutex<RecordingMeta>>,
    ad9361: Arc<tokio::sync::Mutex<Ad9361>>,
    ip_core: Arc<std::sync::Mutex<IpCore>>,
}

struct FinishWaiter {
    waiter: InterruptWaiter,
    metadata: Arc<tokio::sync::Mutex<RecordingMeta>>,
}

impl Recorder {
    pub async fn new(
        ad9361: Arc<tokio::sync::Mutex<Ad9361>>,
        ip_core: Arc<std::sync::Mutex<IpCore>>,
        interrupt_waiter: InterruptWaiter,
    ) -> Result<Recorder> {
        let metadata = Arc::new(tokio::sync::Mutex::new(
            RecordingMeta::new(&ad9361, &ip_core).await?,
        ));
        let finish_waiter = FinishWaiter::new(Arc::clone(&metadata), interrupt_waiter);
        tokio::spawn(finish_waiter.run());
        Ok(Recorder {
            metadata,
            ad9361,
            ip_core,
        })
    }
}

impl FinishWaiter {
    fn new(
        metadata: Arc<tokio::sync::Mutex<RecordingMeta>>,
        waiter: InterruptWaiter,
    ) -> FinishWaiter {
        FinishWaiter { metadata, waiter }
    }

    async fn run(self) -> Result<()> {
        loop {
            self.waiter.wait().await;
            tracing::info!("recorder finished");
            let mut metadata = self.metadata.lock().await;
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
        let mode = ip_core.lock().unwrap().recorder_mode();
        let datatype = mode.into();
        let sample_rate = ad9361.lock().await.get_sampling_frequency().await? as f64;
        let frequency = ad9361.lock().await.get_rx_lo_frequency().await? as f64;
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

    async fn update_for_new_recording(
        &mut self,
        ad9361: &tokio::sync::Mutex<Ad9361>,
        ip_core: &Arc<std::sync::Mutex<IpCore>>,
    ) -> Result<()> {
        self.sigmf_meta.set_datetime_now();

        if let Some(duration) = self.maximum_duration {
            // set up timer task to automatically stop the recording
            let token = CancellationToken::new();
            // stop_timer_cancellation should always be None in the Stopped
            // state (which is when this function can be called).
            assert!(self.stop_timer_cancellation.is_none());
            self.stop_timer_cancellation = Some(token.clone());
            let ip_core_ = Arc::clone(ip_core);
            tokio::spawn(async move {
                tokio::select! {
                    _ = token.cancelled() => return,
                    // add 0.1 s duration to the time to sleep in case the ADC
                    // sample clock is slower than our clock
                    _ = tokio::time::sleep(duration + Duration::from_millis(100)) => {}
                };
                ip_core_.lock().unwrap().recorder_stop()
            });
        }

        if self.prepend_timestamp {
            self.prepend_timestamp_to_filename();
        }
        self.mode = ip_core.lock().unwrap().recorder_mode();
        self.sigmf_meta.set_datatype(self.mode.into());
        self.sigmf_meta
            .set_sample_rate(ad9361.lock().await.get_sampling_frequency().await? as f64);
        self.sigmf_meta
            .set_frequency(ad9361.lock().await.get_rx_lo_frequency().await? as f64);
        Ok(())
    }

    fn json(&self) -> maia_json::RecordingMetadata {
        maia_json::RecordingMetadata {
            filename: self.filename.clone(),
            description: self.sigmf_meta.description().to_string(),
            author: self.sigmf_meta.author().to_string(),
        }
    }

    fn recorder_json(&self, ip_core: &std::sync::Mutex<IpCore>) -> maia_json::Recorder {
        maia_json::Recorder {
            state: self.recorder_state,
            mode: ip_core.lock().unwrap().recorder_mode(),
            prepend_timestamp: self.prepend_timestamp,
            maximum_duration: self
                .maximum_duration
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
        }
    }

    fn patch_json(&mut self, patch: maia_json::PatchRecordingMetadata) {
        if let Some(filename) = patch.filename {
            self.filename = filename;
        }
        if let Some(description) = patch.description {
            self.sigmf_meta.set_description(&description);
        }
        if let Some(author) = patch.author {
            self.sigmf_meta.set_author(&author);
        }
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
}

pub async fn recorder_json(recorder: &Recorder) -> maia_json::Recorder {
    recorder
        .metadata
        .lock()
        .await
        .recorder_json(&recorder.ip_core)
}

pub async fn get_recorder(State(recorder): State<Recorder>) -> Json<maia_json::Recorder> {
    Json(recorder_json(&recorder).await)
}

pub async fn patch_recorder(
    State(recorder): State<Recorder>,
    Json(patch): Json<maia_json::PatchRecorder>,
) -> Result<Json<maia_json::Recorder>, JsonError> {
    Ok(Json(
        recorder_patch(recorder, patch)
            .await
            .map_err(JsonError::server_error)?,
    ))
}

async fn recorder_patch(
    recorder: Recorder,
    patch: maia_json::PatchRecorder,
) -> Result<maia_json::Recorder> {
    if let Some(mode) = patch.mode {
        recorder.ip_core.lock().unwrap().set_recorder_mode(mode);
    }
    let mut metadata = recorder.metadata.lock().await;
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
            metadata.recorder_state = maia_json::RecorderState::Running;
            recorder.ip_core.lock().unwrap().recorder_start();
            metadata
                .update_for_new_recording(&recorder.ad9361, &recorder.ip_core)
                .await?;
        }
        (Some(maia_json::RecorderStateChange::Stop), maia_json::RecorderState::Running) => {
            recorder.ip_core.lock().unwrap().recorder_stop()
        }
        (_, _) => (),
    }
    Ok(metadata.recorder_json(&recorder.ip_core))
}

pub async fn recording_metadata_json(recorder: &Recorder) -> maia_json::RecordingMetadata {
    recorder.metadata.lock().await.json()
}

pub async fn get_recording_metadata(
    State(recorder): State<Recorder>,
) -> Json<maia_json::RecordingMetadata> {
    Json(recording_metadata_json(&recorder).await)
}

pub async fn put_recording_metadata(
    State(recorder): State<Recorder>,
    Json(put): Json<maia_json::RecordingMetadata>,
) -> Json<maia_json::RecordingMetadata> {
    let mut metadata = recorder.metadata.lock().await;
    metadata.patch_json(put.into());
    Json(metadata.json())
}

pub async fn patch_recording_metadata(
    State(recorder): State<Recorder>,
    Json(patch): Json<maia_json::PatchRecordingMetadata>,
) -> Json<maia_json::RecordingMetadata> {
    let mut metadata = recorder.metadata.lock().await;
    metadata.patch_json(patch);
    Json(metadata.json())
}

pub type SigmfStream = ReaderStream<DuplexStream>;

pub async fn get_recording(
    State(recorder): State<Recorder>,
) -> Result<(HeaderMap, Body), JsonError> {
    let metadata = recorder.metadata.lock().await.clone();
    let max_samples = metadata.maximum_duration.map(|duration| {
        let samp_rate = metadata.sigmf_meta.sample_rate();
        (duration.as_secs_f64() * samp_rate).round() as usize
    });
    let recorder_next_address = recorder.ip_core.lock().unwrap().recorder_next_address();
    let (recording, size) = recording_stream(&metadata, recorder_next_address, max_samples)
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
    Ok((headers, Body::from_stream(recording)))
}

async fn recording_stream(
    metadata: &RecordingMeta,
    recorder_next_address: usize,
    max_samples: Option<usize>,
) -> Result<(SigmfStream, usize)> {
    const DUPLEX_SIZE: usize = 1 << 20;
    let buffer = RecordingBuffer::new(metadata.mode, recorder_next_address, max_samples).await?;
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
    data_header.set_size(buffer.output_size().try_into().unwrap());
    data_header.set_mode(0o0444);
    data_header.set_entry_type(tokio_tar::EntryType::Regular);
    data_header.set_mtime(timestamp);
    data_header.set_cksum();

    let tar_header_size = 512;
    let num_headers = 3;
    let tar_finish_size = 1024;
    let tar_size = tar_header_size * num_headers
        + round_up_multiple_512(sigmf_meta.len())
        + round_up_multiple_512(buffer.output_size())
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
    chunk: *const u8,
    mode: Mode,
    input_bytes_per_item: usize,
    chunk_bytes: usize,
}

unsafe impl Send for RecordingBuffer {}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct Mode(RecorderMode);

impl Mode {
    fn input_bytes_per_item(&self) -> usize {
        match self.0 {
            RecorderMode::IQ8bit => 2,
            RecorderMode::IQ12bit => 3,
        }
    }

    fn output_bytes_per_item(&self) -> usize {
        match self.0 {
            RecorderMode::IQ8bit => 2,
            RecorderMode::IQ12bit => 4,
        }
    }
}

impl RecordingBuffer {
    async fn new(
        mode: RecorderMode,
        next_address: usize,
        max_items: Option<usize>,
    ) -> Result<RecordingBuffer> {
        let base_address = usize::from_str_radix(
            fs::read_to_string(
                "/sys/class/maia-sdr/maia-sdr-recording/device/recording_base_address",
            )
            .await?
            .trim_end()
            .trim_start_matches("0x"),
            16,
        )?;

        let mode = Mode(mode);
        let input_bytes_per_item = mode.input_bytes_per_item();
        let max_size = max_items.map(|items| items * input_bytes_per_item);
        let size = next_address - base_address;
        // Constrain size <= max_size if max_size.is_some()
        let size = max_size.map(|x| x.min(size)).unwrap_or(size);

        let mem = fs::OpenOptions::new()
            .read(true)
            .open("/dev/maia-sdr-recording")
            .await?;
        let map = unsafe {
            match libc::mmap(
                std::ptr::null_mut::<libc::c_void>(),
                size,
                libc::PROT_READ,
                libc::MAP_SHARED,
                mem.as_raw_fd(),
                0,
            ) {
                libc::MAP_FAILED => anyhow::bail!("mmap /dev/maia-sdr-recording failed"),
                x => x,
            }
        };

        Ok(RecordingBuffer {
            base: map as *const u8,
            size,
            chunk: map as *const u8,
            mode,
            input_bytes_per_item,
            chunk_bytes: input_bytes_per_item * Self::CHUNK_ITEMS,
        })
    }

    fn output_size(&self) -> usize {
        self.size / self.input_bytes_per_item * self.mode.output_bytes_per_item()
    }

    const CHUNK_ITEMS: usize = 1 << 16;
}

impl Stream for RecordingBuffer {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let offset = unsafe { self.chunk.offset_from(self.base) as usize };
        let remaining = self.size - offset;
        if remaining < self.input_bytes_per_item {
            return Poll::Ready(None);
        }
        let (chunk_bytes, chunk_items) = match remaining {
            x if x >= self.chunk_bytes => (self.chunk_bytes, Self::CHUNK_ITEMS),
            x => {
                let chunk_items = x / self.input_bytes_per_item;
                (chunk_items * self.input_bytes_per_item, chunk_items)
            }
        };
        let data = unsafe { std::slice::from_raw_parts(self.chunk, chunk_bytes) };
        let bytes = match self.mode.0 {
            RecorderMode::IQ8bit => Bytes::copy_from_slice(data),
            RecorderMode::IQ12bit => {
                let mut bytes = BytesMut::zeroed(self.mode.output_bytes_per_item() * chunk_items);
                for (j, x) in data.chunks(3).enumerate() {
                    bytes[4 * j] = (x[0] << 4) | (x[1] >> 4);
                    bytes[4 * j + 1] = ((x[0] & 0xf0) as i8 >> 4) as u8;
                    bytes[4 * j + 2] = x[2];
                    bytes[4 * j + 3] = ((x[1] << 4) as i8 >> 4) as u8;
                }
                Bytes::from(bytes)
            }
        };
        self.chunk = unsafe { self.chunk.add(chunk_bytes) };
        Poll::Ready(Some(Ok(bytes)))
    }
}

impl Drop for RecordingBuffer {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base as *mut libc::c_void, self.size);
        }
    }
}
