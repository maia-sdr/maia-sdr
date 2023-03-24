//! maia-json contains the JSON schemas used by maia-httpd and maia-wasm.

#![warn(missing_docs)]

use serde::{Deserialize, Serialize};

/// API JSON schema.
///
/// This JSON schema corresponds to GET requests on `/api`. It contains the
/// settings of the full Maia SDR system.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Api {
    /// AD9361 settings.
    pub ad9361: Ad9361,
    /// Spectrometer settings.
    pub spectrometer: Spectrometer,
    /// IQ recorder settings.
    pub recorder: Recorder,
    /// Metadata for the current recording.
    pub recording_metadata: RecordingMetadata,
    /// System time.
    pub time: Time,
}

/// AD9361 JSON schema.
///
/// This JSON schema corresponds to GET and PUT requests on `/api/ad9361`. It
/// contains the settings of the AD9361.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Ad9361 {
    /// Sampling frequency in samples per second.
    pub sampling_frequency: u32,
    /// Receive RF bandwidth in Hz.
    pub rx_rf_bandwidth: u32,
    /// Transmit RF bandwidth in Hz.
    pub tx_rf_bandwidth: u32,
    /// Receive LO frequency in Hz.
    pub rx_lo_frequency: u64,
    /// Transmit LO frequency in Hz.
    pub tx_lo_frequency: u64,
    /// Receive gain in dB.
    pub rx_gain: f64,
    /// Receive AGC mode.
    pub rx_gain_mode: Ad9361GainMode,
    /// Transmit gain in dB.
    pub tx_gain: f64,
}

/// AD9361 PATCH JSON schema.
///
/// This JSON schema corresponds to PATCH requests on `/api/ad9361`. It contains
/// a subset of the settings of the AD9361.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct PatchAd9361 {
    /// Sampling frequency in samples per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling_frequency: Option<u32>,
    /// Receive RF bandwidth in Hz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx_rf_bandwidth: Option<u32>,
    /// Transmit RF bandwidth in Hz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_rf_bandwidth: Option<u32>,
    /// Receive LO frequency in Hz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx_lo_frequency: Option<u64>,
    /// Transmit LO frequency in Hz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_lo_frequency: Option<u64>,
    /// Receive gain in dB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx_gain: Option<f64>,
    /// Receive AGC mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx_gain_mode: Option<Ad9361GainMode>,
    /// Transmit gain in dB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_gain: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
/// AD9361 gain control modes.
///
/// This enum lists the automatic gain control modes supported by the AD9361.
pub enum Ad9361GainMode {
    /// Manual AGC.
    Manual,
    /// Fast attack AGC.
    FastAttack,
    /// Slow attack AGC.
    SlowAttack,
    /// Hybrid AGC.
    Hybrid,
}

macro_rules! impl_str_conv {
    ($ty:ty, $($s:expr => $v:ident),*) => {
        impl std::str::FromStr for $ty {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, ()> {
                Ok(match s {
                    $(
                        $s => <$ty>::$v,
                    )*
                        _ => return Err(()),
                })
            }
        }

        impl std::fmt::Display for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                write!(f, "{}", match self {
                    $(
                        <$ty>::$v => $s,
                    )*
                })
            }
        }
    }
}

impl_str_conv!(Ad9361GainMode,
               "Manual" => Manual,
               "Fast attack" => FastAttack,
               "Slow attack" => SlowAttack,
               "Hybrid" => Hybrid);

macro_rules! get_fields {
    ($struct:ident, $x:expr, $($field:ident),*) => {
        $struct {
            $(
                $field: Some($x.$field),
            )*
        }
    }
}

impl From<Ad9361> for PatchAd9361 {
    fn from(val: Ad9361) -> PatchAd9361 {
        get_fields!(
            PatchAd9361,
            val,
            sampling_frequency,
            rx_rf_bandwidth,
            tx_rf_bandwidth,
            rx_lo_frequency,
            tx_lo_frequency,
            rx_gain,
            rx_gain_mode,
            tx_gain
        )
    }
}

/// Spectrometer JSON schema.
///
/// This JSON schema corresponds to GET requests on `/api/spectrometer`. It
/// contains the settings of the spectrometer (waterfall).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Spectrometer {
    /// Input sampling frequency in samples per second (read-only).
    pub input_sampling_frequency: f64,
    /// Output sampling frequency in samples per second.
    pub output_sampling_frequency: f64,
    /// Number of non-coherent integrations.
    pub number_integrations: u32,
    /// FFT size (read-only).
    pub fft_size: u32,
}

/// Spectrometer PATCH JSON schema.
///
/// This JSON schema corresponds to PATCH requests on `/api/spectrometer`. It is
/// used to change the spectrometer rate, by specifying the target output
/// sampling frequency, or the number of integrations. Since each parameter can
/// be computed in terms of the other, only one of them should be used in the PATCH request.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct PatchSpectrometer {
    /// Output sampling frequency in samples per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_sampling_frequency: Option<f64>,
    /// Number of non-coherent integrations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_integrations: Option<u32>,
}

/// IQ recorder JSON schema.
///
/// This JSON schema corresponds to GET requests on `/api/recorder`. It contains
/// the settings of the IQ recorder.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash)]
pub struct Recorder {
    /// Current recorder state.
    pub state: RecorderState,
    /// Recoder sampling mode.
    pub mode: RecorderMode,
    /// Automatically prepend timestamp to file name.
    pub prepend_timestamp: bool,
}

/// IQ recorder PATCH JSON schema.
///
/// This JSON schema corresponds to PATCH requests on `/api/recorder`. It is
/// used to modify the settings of the IQ recorder.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, Hash)]
pub struct PatchRecorder {
    /// Command to change the recorder state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_change: Option<RecorderStateChange>,
    /// Recorder sampling mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<RecorderMode>,
    /// Automatically prepend timestamp to file name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepend_timestamp: Option<bool>,
}

/// Command to change the IQ recorder state.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Hash)]
pub enum RecorderStateChange {
    /// Command the IQ recoder to start recording.
    Start,
    /// Command the IQ recorder to stop recording.
    Stop,
}

/// IQ recorder sampling mode.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum RecorderMode {
    /// 8-bit sampling mode.
    ///
    /// Only the 8 MSBs of the ADC data are recorded.
    IQ8bit,
    /// 12-bit sampling mode.
    ///
    /// All the 12 bits of the ADC data are recorded.
    IQ12bit,
}

impl_str_conv!(RecorderMode,
               "8 bit IQ" => IQ8bit,
               "12 bit IQ" => IQ12bit);

/// IQ recorder state.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Hash)]
pub enum RecorderState {
    /// The IQ recorder is stopped.
    Stopped,
    /// The IQ recorder is running.
    Running,
}

/// Recording metadata JSON schema.
///
/// This JSON schema corresponds to GET and PUT requests on
/// `/api/recording/metadata`. It contains the metadata for the current
/// recording.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash)]
pub struct RecordingMetadata {
    /// Recording file name.
    pub filename: String,
    /// Recording description.
    pub description: String,
    /// Recording author.
    pub author: String,
}

/// Recording metadata PATCH JSON schema.
///
/// This JSON schema corresponds to PATCH requests on
/// `/api/recording/metadata`. It is used to modify the metadata for the current
/// recording.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, Hash)]
pub struct PatchRecordingMetadata {
    /// Recording file name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Recording description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Recording author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

impl From<RecordingMetadata> for PatchRecordingMetadata {
    fn from(val: RecordingMetadata) -> PatchRecordingMetadata {
        get_fields!(PatchRecordingMetadata, val, filename, description, author)
    }
}

/// System time JSON schema.
///
/// This JSON schema corresponds to GET requests on `/api/time`. It contains the
/// current system time.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Time {
    /// Number of milliseconds since UNIX timestamp.
    ///
    /// This uses the same format as JavaScript `Date.now()`.
    pub time: f64,
}

/// System time PATCH JSON schema.
///
/// This JSON schema corresponds to GET requests on `/api/time`. It contains the
/// current system time.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct PatchTime {
    /// Number of milliseconds since UNIX timestamp.
    ///
    /// This uses the same format as JavaScript `Date.now()`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<f64>,
}

impl From<Time> for PatchTime {
    fn from(val: Time) -> PatchTime {
        get_fields!(PatchTime, val, time)
    }
}
