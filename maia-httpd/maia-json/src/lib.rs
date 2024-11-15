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
    /// DDC settings.
    pub ddc: DDCConfigSummary,
    /// Device geolocation.
    pub geolocation: DeviceGeolocation,
    /// IQ recorder settings.
    pub recorder: Recorder,
    /// Metadata for the current recording.
    pub recording_metadata: RecordingMetadata,
    /// Spectrometer settings.
    pub spectrometer: Spectrometer,
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
    /// Input source.
    pub input: SpectrometerInput,
    /// Input sampling frequency in samples per second (read-only).
    pub input_sampling_frequency: f64,
    /// Output sampling frequency in samples per second.
    pub output_sampling_frequency: f64,
    /// Number of non-coherent integrations.
    pub number_integrations: u32,
    /// FFT size (read-only).
    pub fft_size: u32,
    /// Spectrometer mode.
    pub mode: SpectrometerMode,
}

/// Spectrometer PATCH JSON schema.
///
/// This JSON schema corresponds to PATCH requests on `/api/spectrometer`. It is
/// used to change the spectrometer rate, by specifying the target output
/// sampling frequency, or the number of integrations. Since each parameter can
/// be computed in terms of the other, only one of them should be used in the PATCH request.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct PatchSpectrometer {
    /// Input source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<SpectrometerInput>,
    /// Output sampling frequency in samples per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_sampling_frequency: Option<f64>,
    /// Number of non-coherent integrations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_integrations: Option<u32>,
    /// Spectrometer mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<SpectrometerMode>,
}

/// Spectrometer input source.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SpectrometerInput {
    /// AD9361 IQ ADC output.
    AD9361,
    /// DDC output.
    DDC,
}

impl_str_conv!(SpectrometerInput,
               "AD9361" => AD9361,
               "DDC" => DDC);

/// Spectrometer mode.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SpectrometerMode {
    /// Power average mode.
    ///
    /// The average power over the integration period is computed.
    Average,
    /// Peak detect mode.
    ///
    /// The maximum (peak) power over the integration period is computed.
    PeakDetect,
}

impl_str_conv!(SpectrometerMode,
               "Average" => Average,
               "Peak detect" => PeakDetect);

/// DDC design PUT JSON schema.
///
/// This JSON schema corresponds to PUT requests on `/api/ddc/design`. It is
/// used to define design constraints for the DDC and have maia-httpd calculate
/// suitable FIR filters coefficients using pm-remez.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PutDDCDesign {
    /// Frequency for the mixer, in Hz.
    pub frequency: f64,
    /// Decimation factor for the DDC.
    pub decimation: u32,
    /// Transition bandwidth of the DDC output.
    ///
    /// This is the fraction (in [0, 1]) of the total output bandwidth that gets
    /// used as transition bands.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_bandwidth: Option<f64>,
    /// Passband ripple.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passband_ripple: Option<f64>,
    /// Stopband attenuation in dB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopband_attenuation_db: Option<f64>,
    /// Use 1/f response in the stopband.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopband_one_over_f: Option<bool>,
}

/// DDC configuration GET JSON schema.
///
/// This JSON schema corresponds to GET requests on `/api/ddc/config`. It lists
/// the configuration of each FIR filter in the DDC, as well as some values
/// calculated from this configuration.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DDCConfig {
    /// Indicates whether the DDC is currently enabled.
    pub enabled: bool,
    /// Frequency for the mixer, in Hz.
    pub frequency: f64,
    /// Total decimation of this DDC configuration.
    pub decimation: u32,
    /// Input sampling frequency in samples per second.
    pub input_sampling_frequency: f64,
    /// Output sampling frequency in samples per second.
    pub output_sampling_frequency: f64,
    /// Maximum input sampling frequency supported by this DDC configuration.
    pub max_input_sampling_frequency: f64,
    /// Configuration of the first FIR filter.
    pub fir1: DDCFIRConfig,
    /// Configuration of the second FIR filter.
    ///
    /// This has the value `None` if the second FIR filter is bypassed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fir2: Option<DDCFIRConfig>,
    /// Configuration of the third FIR filter.
    ///
    /// This has the value `None` if the third FIR filter is bypassed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fir3: Option<DDCFIRConfig>,
}

/// DDC configuration summary GET JSON schema.
///
/// This JSON schema is similar to [`DDCConfig`], but it does not include the
/// FIR coefficients. It is used for the DDC entry in `/api`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DDCConfigSummary {
    /// Indicates whether the DDC is currently enabled.
    pub enabled: bool,
    /// Frequency for the mixer, in Hz.
    pub frequency: f64,
    /// Total decimation of this DDC configuration.
    pub decimation: u32,
    /// Input sampling frequency in samples per second.
    pub input_sampling_frequency: f64,
    /// Output sampling frequency in samples per second.
    pub output_sampling_frequency: f64,
    /// Maximum input sampling frequency supported by this DDC configuration.
    pub max_input_sampling_frequency: f64,
}

macro_rules! ddcconfig_from {
    ($value:expr, $($field:ident),*) => {
        DDCConfigSummary {
            $($field: $value.$field),*
        }
    }
}

impl From<DDCConfig> for DDCConfigSummary {
    fn from(value: DDCConfig) -> DDCConfigSummary {
        ddcconfig_from!(
            value,
            enabled,
            frequency,
            decimation,
            input_sampling_frequency,
            output_sampling_frequency,
            max_input_sampling_frequency
        )
    }
}

/// DDC configuration PUT JSON schema.
///
/// This JSON schema corresponds to PUT requests on `/api/ddc/config`. It is
/// used to set the coefficients for each FIR filter manually, as opposed to
/// having maia-httpd design a filter satisfying some requirements.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PutDDCConfig {
    /// Frequency for the mixer, in Hz.
    pub frequency: f64,
    /// Configuration of the first FIR filter.
    pub fir1: DDCFIRConfig,
    /// Configuration of the second FIR filter.
    ///
    /// This has the value `None` if the second FIR filter is bypassed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fir2: Option<DDCFIRConfig>,
    /// Configuration of the third FIR filter.
    ///
    /// This has the value `None` if the third FIR filter is bypassed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fir3: Option<DDCFIRConfig>,
}

/// DDC configuration PUT JSON schema.
///
/// This JSON schema corresponds to PATCH requests on `/api/ddc/config`. It is
/// used to change the frequency without changing the FIR filter configuration
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct PatchDDCConfig {
    /// Frequency for the mixer, in Hz.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<f64>,
}

/// Configuration of a FIR filter in the DDC.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DDCFIRConfig {
    /// FIR filter coefficients.
    pub coefficients: Vec<i32>,
    /// Decimation factor.
    pub decimation: u32,
}

/// IQ recorder JSON schema.
///
/// This JSON schema corresponds to GET requests on `/api/recorder`. It contains
/// the settings of the IQ recorder.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Recorder {
    /// Current recorder state.
    pub state: RecorderState,
    /// Recoder sampling mode.
    pub mode: RecorderMode,
    /// Automatically prepend timestamp to file name.
    pub prepend_timestamp: bool,
    /// Maximum recording duration (in seconds).
    pub maximum_duration: f64,
}

/// IQ recorder PATCH JSON schema.
///
/// This JSON schema corresponds to PATCH requests on `/api/recorder`. It is
/// used to modify the settings of the IQ recorder.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
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
    /// Maximum recording duration (in seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_duration: Option<f64>,
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
    /// Only the 8 MSBs of the ADC data or the DDC output are recorded.
    IQ8bit,
    /// 12-bit sampling mode.
    ///
    /// All the 12 bits of the ADC data, or the 12 MSBs of the
    /// 16-bit DDC output are recorded.
    IQ12bit,
    /// 16-bit sampling mode.
    ///
    /// All the 16 bits of the DDC output are recorded. 12-bit ADC data is
    /// placed on the 12 MSBs.
    IQ16bit,
}

impl_str_conv!(RecorderMode,
               "8 bit IQ" => IQ8bit,
               "12 bit IQ" => IQ12bit,
               "16 bit IQ" => IQ16bit);

/// IQ recorder state.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Hash)]
pub enum RecorderState {
    /// The IQ recorder is stopped.
    Stopped,
    /// The IQ recorder is running.
    Running,
    /// The IQ recoder is stopping.
    Stopping,
}

/// Geolocation.
///
/// This is based on a GeoJSON point, but it is encoded differently in JSON.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Geolocation {
    /// Latitude in degrees.
    pub latitude: f64,
    /// Longitude in degrees.
    pub longitude: f64,
    /// Altitude in meters.
    ///
    /// The altitude is optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub altitude: Option<f64>,
}

/// Recording metadata JSON schema.
///
/// This JSON schema corresponds to GET and PUT requests on
/// `/api/recording/metadata`. It contains the metadata for the current
/// recording.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RecordingMetadata {
    /// Recording file name.
    pub filename: String,
    /// Recording description.
    pub description: String,
    /// Recording author.
    pub author: String,
    /// Recording geolocation.
    ///
    /// This corresponds to the SigMF "core:geolocation" key. It contains `None`
    /// if the geolocation is unknown.
    pub geolocation: DeviceGeolocation,
}

/// Recording metadata PATCH JSON schema.
///
/// This JSON schema corresponds to PATCH and PUT requests on
/// `/api/recording/metadata`. It is used to modify the metadata for the current
/// recording.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
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
    /// Recording geolocation.
    ///
    /// This corresponds to the SigMF "core:geolocation" key. It contains `None`
    /// inside the `DeviceGeolocation` to remove the geolocation from the
    /// metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<DeviceGeolocation>,
}

impl From<RecordingMetadata> for PatchRecordingMetadata {
    fn from(val: RecordingMetadata) -> PatchRecordingMetadata {
        get_fields!(
            PatchRecordingMetadata,
            val,
            filename,
            description,
            author,
            geolocation
        )
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

/// Device geolocation JSON schema.
///
/// This JSON schema corresponds to GET and PUT requests on
/// `/api/geolocation`. The GET request contains the current device geolocation,
/// or `None` if it has never been set or if it has been cleared. The PUT
/// request sets the current device geolocation, or clears it the request
/// contains `None`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct DeviceGeolocation {
    /// Current device geolocation.
    pub point: Option<Geolocation>,
}

/// Error.
///
/// This JSON schema is used to report errors to the client. It is used whenever
/// the API returns an HTTP error code such as 500.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// HTTP status code.
    pub http_status_code: u16,
    /// String describing the error in a human readable form.
    pub error_description: String,
    /// Sugested action to perform by the client.
    pub suggested_action: ErrorAction,
}

/// Actions for an error.
///
/// This enum lists the actions that a client may take to handle an error.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ErrorAction {
    /// Show a message using the JavaScript `alert()` function.
    Alert,
    /// Log the error.
    Log,
    /// Ignore the error.
    Ignore,
}
