use super::input::InputElement;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::{Storage, Window};

const PREFERENCES_KEY: &str = "preferences";

pub struct Preferences {
    storage: Option<Storage>,
    data: PreferenceData,
}

macro_rules! impl_preference_data {
    {$($name:ident : $ty:ty = $default:expr,)*} => {
        #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
        struct PreferenceData {
            $(
                $name: $ty,
            )*
        }

        impl Default for PreferenceData {
            fn default() -> Self {
                Self {
                    $(
                        $name: $default,
                    )*
                }
            }
        }

        impl Preferences {
            $(
                paste::paste! {
                    pub fn [<update_ $name>](&mut self, value: &$ty) -> Result<(), JsValue> {
                        if (*value != self.data.$name) {
                            self.data.$name.clone_from(value);
                            self.store()
                        } else {
                            Ok(())
                        }
                    }
                }
            )*
        }

        impl Preferences {
            // pub(super) is here to avoid complaints about leaking the private type
            // Elements
            pub(super) fn apply(&self, ui: &super::Ui) -> Result<(), JsValue> {
                $(
                    ui.elements.$name.set(&self.data.$name);
                    if let Some(onchange) = ui.elements.$name.onchange() {
                        onchange.call0(&JsValue::NULL)?;
                    }
                )*
                Ok(())
            }
        }
    }
}

impl_preference_data! {
    colormap_select: super::colormap::Colormap = super::colormap::Colormap::Turbo,
    waterfall_show_waterfall: bool = true,
    waterfall_show_spectrum: bool = false,
    waterfall_show_ddc: bool = true,
    waterfall_min: f32 = 35.0,
    waterfall_max: f32 = 85.0,
    ad9361_rx_lo_frequency: u64 = 2_400_000_000,
    ad9361_sampling_frequency: u32 = 61_440_000,
    ad9361_rx_rf_bandwidth: u32 = 56_000_000,
    ad9361_rx_gain_mode: maia_json::Ad9361GainMode = maia_json::Ad9361GainMode::SlowAttack,
    ad9361_rx_gain: f64 = 70.0,
    ddc_frequency: f64 = 0.0,
    ddc_decimation: u32 = 20,
    ddc_transition_bandwidth: f64 = 0.05,
    ddc_passband_ripple: f64 = 0.01,
    ddc_stopband_attenuation_db: f64 = 60.0,
    ddc_stopband_one_over_f: bool = true,
    spectrometer_input: maia_json::SpectrometerInput = maia_json::SpectrometerInput::AD9361,
    spectrometer_output_sampling_frequency: f64 = 20.0,
    spectrometer_mode: maia_json::SpectrometerMode = maia_json::SpectrometerMode::Average,
    recording_metadata_filename: String = "recording".to_string(),
    recorder_prepend_timestamp: bool = false,
    recording_metadata_description: String = "".to_string(),
    recording_metadata_author: String = "".to_string(),
    recorder_mode: maia_json::RecorderMode = maia_json::RecorderMode::IQ12bit,
    recorder_maximum_duration: f64 = 0.0,
    geolocation_watch: bool = false,
}

impl Preferences {
    pub fn new(window: &Window) -> Result<Preferences, JsValue> {
        let storage = window.local_storage()?;
        let data = match &storage {
            Some(storage) => match storage.get_item(PREFERENCES_KEY)? {
                Some(data) => match serde_json::from_str(&data) {
                    Ok(x) => x,
                    Err(_) => {
                        web_sys::console::error_1(&"preferences corrupted; removing".into());
                        storage.remove_item(PREFERENCES_KEY)?;
                        PreferenceData::default()
                    }
                },
                None => PreferenceData::default(),
            },
            None => PreferenceData::default(),
        };
        Ok(Preferences { storage, data })
    }

    fn store(&self) -> Result<(), JsValue> {
        if let Some(storage) = self.storage.as_ref() {
            let data = serde_json::to_string(&self.data).unwrap();
            storage.set_item(PREFERENCES_KEY, &data)
        } else {
            Ok(())
        }
    }
}

/// UI preferences macro: implements dummy `update_` methods for `Preferences`.
///
/// This macro is used to generate dummy `update_` methods that do nothing for
/// values that aren't stored in the preferences. This is needed because the
/// `set_values_if_inactive` macro (which is is used by
/// [`impl_update_elements`](crate::impl_update_elements)) always calls the
/// `update_` method of the preferences.
///
/// # Example
///
/// ```
/// use maia_wasm::impl_dummy_preferences;
///
/// struct Preferences {}
///
/// impl_dummy_preferences!(
///     my_section_my_float: f64,
///     my_section_my_string: String,
///  );
///
///  // Now it is possible to call update_ methods
///  let mut preferences = Preferences {};
///  preferences.update_my_section_my_float(&0.5);
///  preferences.update_my_section_my_string(&"hello".to_string());
///  ```
#[macro_export]
macro_rules! impl_dummy_preferences {
    {$($name:ident : $ty:ty,)*} => {
        impl Preferences {
            $(
                paste::paste! {
                    pub fn [<update_ $name>](&mut self, _value: &$ty) -> Result<(), wasm_bindgen::JsValue> {
                        Ok(())
                    }
                }
            )*
        }
    }
}

impl_dummy_preferences!(
    ddc_output_sampling_frequency: f64,
    ddc_max_input_sampling_frequency: f64,
);
