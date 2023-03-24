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
                    ui.elements.$name.onchange().unwrap().call0(&JsValue::NULL)?;
                )*
                Ok(())
            }
        }
    }
}

impl_preference_data! {
    colormap_select: super::colormap::Colormap = super::colormap::Colormap::Turbo,
    waterfall_min: f32 = 35.0,
    waterfall_max: f32 = 85.0,
    ad9361_rx_lo_frequency: u64 = 2_400_000_000,
    ad9361_sampling_frequency: u32 = 61_440_000,
    ad9361_rx_rf_bandwidth: u32 = 56_000_000,
    ad9361_rx_gain_mode: maia_json::Ad9361GainMode = maia_json::Ad9361GainMode::SlowAttack,
    ad9361_rx_gain: f64 = 70.0,
    spectrometer_output_sampling_frequency: f64 = 20.0,
    recording_metadata_filename: String = "recording".to_string(),
    recorder_prepend_timestamp: bool = false,
    recording_metadata_description: String = "".to_string(),
    recording_metadata_author: String = "".to_string(),
    recorder_mode: maia_json::RecorderMode = maia_json::RecorderMode::IQ12bit,
    recorder_maximum_duration: f64 = 0.0,
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
