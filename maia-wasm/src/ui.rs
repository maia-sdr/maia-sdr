//! User interface.
//!
//! This module implements the user interface by linking HTML form elements
//! (buttons, input elements, etc.) with the RESTful API of maia-httpd and with
//! other operations that are performed client-side (such as changing the
//! waterfall levels or colormap).

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{future_to_promise, JsFuture};
use web_sys::{
    Document, HtmlButtonElement, HtmlDialogElement, HtmlElement, HtmlInputElement,
    HtmlParagraphElement, HtmlSelectElement, HtmlSpanElement, Response, Window,
};

use crate::render::RenderEngine;
use crate::waterfall::Waterfall;

use active::IsElementActive;
use input::{CheckboxInput, EnumInput, InputElement, NumberInput, NumberSpan, TextInput};

pub mod active;
pub mod colormap;
pub mod input;
#[macro_use]
pub mod macros;
mod preferences;
pub mod request;

const API_URL: &str = "/api";
const AD9361_URL: &str = "/api/ad9361";
const DDC_CONFIG_URL: &str = "/api/ddc/config";
const DDC_DESIGN_URL: &str = "/api/ddc/design";
const SPECTROMETER_URL: &str = "/api/spectrometer";
const RECORDER_URL: &str = "/api/recorder";
const RECORDING_METADATA_URL: &str = "/api/recording/metadata";
const TIME_URL: &str = "/api/time";

/// User interface.
///
/// This structure is used to create and set up the appropriate callbacks that
/// implement all the UI interactions.
#[derive(Clone)]
pub struct Ui {
    window: Rc<Window>,
    document: Rc<Document>,
    elements: Elements,
    api_state: Rc<RefCell<Option<maia_json::Api>>>,
    local_settings: Rc<RefCell<LocalSettings>>,
    preferences: Rc<RefCell<preferences::Preferences>>,
    render_engine: Rc<RefCell<RenderEngine>>,
    waterfall: Rc<RefCell<Waterfall>>,
}

// Defines the 'struct Elements' and its constructor
ui_elements! {
    colormap_select: HtmlSelectElement => EnumInput<colormap::Colormap>,
    waterfall_show_waterfall: HtmlInputElement => CheckboxInput,
    waterfall_show_spectrum: HtmlInputElement => CheckboxInput,
    waterfall_show_ddc: HtmlInputElement => CheckboxInput,
    recorder_button: HtmlButtonElement => Rc<HtmlButtonElement>,
    recorder_button_replica: HtmlButtonElement => Rc<HtmlButtonElement>,
    settings_button: HtmlButtonElement => Rc<HtmlButtonElement>,
    alert_dialog: HtmlDialogElement => Rc<HtmlDialogElement>,
    alert_message: HtmlParagraphElement => Rc<HtmlParagraphElement>,
    close_alert: HtmlButtonElement => Rc<HtmlButtonElement>,
    settings: HtmlDialogElement => Rc<HtmlDialogElement>,
    close_settings: HtmlButtonElement => Rc<HtmlButtonElement>,
    recording_tab: HtmlButtonElement => Rc<HtmlButtonElement>,
    ddc_tab: HtmlButtonElement => Rc<HtmlButtonElement>,
    waterfall_tab: HtmlButtonElement => Rc<HtmlButtonElement>,
    recording_panel: HtmlElement => Rc<HtmlElement>,
    ddc_panel: HtmlElement => Rc<HtmlElement>,
    waterfall_panel: HtmlElement => Rc<HtmlElement>,
    waterfall_min: HtmlInputElement => NumberInput<f32>,
    waterfall_max: HtmlInputElement => NumberInput<f32>,
    ad9361_rx_lo_frequency: HtmlInputElement
        => NumberInput<u64, input::MHzPresentation>,
    ad9361_sampling_frequency: HtmlInputElement
        => NumberInput<u32, input::MHzPresentation>,
    ad9361_rx_rf_bandwidth: HtmlInputElement
        => NumberInput<u32, input::MHzPresentation>,
    ad9361_rx_gain_mode: HtmlSelectElement => EnumInput<maia_json::Ad9361GainMode>,
    ad9361_rx_gain: HtmlInputElement => NumberInput<f64>,
    ddc_frequency: HtmlInputElement => NumberInput<f64, input::KHzPresentation>,
    ddc_decimation: HtmlInputElement => NumberInput<u32>,
    ddc_transition_bandwidth: HtmlInputElement => NumberInput<f64>,
    ddc_passband_ripple: HtmlInputElement => NumberInput<f64>,
    ddc_stopband_attenuation_db: HtmlInputElement => NumberInput<f64>,
    ddc_stopband_one_over_f: HtmlInputElement => CheckboxInput,
    ddc_output_sampling_frequency: HtmlSpanElement => NumberSpan<f64, input::MHzPresentation>,
    ddc_max_input_sampling_frequency: HtmlSpanElement => NumberSpan<f64, input::MHzPresentation>,
    spectrometer_input: HtmlSelectElement => EnumInput<maia_json::SpectrometerInput>,
    spectrometer_output_sampling_frequency: HtmlInputElement
        => NumberInput<f64, input::IntegerPresentation>,
    spectrometer_mode: HtmlSelectElement => EnumInput<maia_json::SpectrometerMode>,
    recording_metadata_filename: HtmlInputElement => TextInput,
    recorder_prepend_timestamp: HtmlInputElement => CheckboxInput,
    recording_metadata_description: HtmlInputElement => TextInput,
    recording_metadata_author: HtmlInputElement => TextInput,
    recorder_mode: HtmlSelectElement => EnumInput<maia_json::RecorderMode>,
    recorder_maximum_duration: HtmlInputElement => NumberInput<f64>,
}

#[derive(Default)]
struct LocalSettings {
    waterfall_show_ddc: bool,
}

impl Ui {
    /// Creates a new user interface.
    pub fn new(
        window: Rc<Window>,
        document: Rc<Document>,
        render_engine: Rc<RefCell<RenderEngine>>,
        waterfall: Rc<RefCell<Waterfall>>,
    ) -> Result<Ui, JsValue> {
        let elements = Elements::new(&document)?;
        let preferences = Rc::new(RefCell::new(preferences::Preferences::new(&window)?));
        let ui = Ui {
            window,
            document,
            elements,
            api_state: Rc::new(RefCell::new(None)),
            local_settings: Rc::new(RefCell::new(LocalSettings::default())),
            preferences,
            render_engine,
            waterfall,
        };
        ui.set_callbacks()?;
        ui.preferences.borrow().apply(&ui)?;
        ui.set_callbacks_post_apply()?;
        Ok(ui)
    }

    fn set_callbacks(&self) -> Result<(), JsValue> {
        self.set_api_get_periodic(1000)?;

        set_on!(
            change,
            self,
            colormap_select,
            waterfall_show_waterfall,
            waterfall_show_spectrum,
            waterfall_show_ddc,
            waterfall_min,
            waterfall_max,
            ad9361_rx_lo_frequency,
            ad9361_sampling_frequency,
            ad9361_rx_rf_bandwidth,
            ad9361_rx_gain_mode,
            ddc_frequency,
            spectrometer_input,
            spectrometer_output_sampling_frequency,
            spectrometer_mode,
            recording_metadata_filename,
            recorder_prepend_timestamp,
            recording_metadata_description,
            recording_metadata_author,
            recorder_mode,
            recorder_maximum_duration
        );

        // This uses a custom onchange function that calls the macro-generated one.
        self.elements.ad9361_rx_gain.set_onchange(Some(
            self.ad9361_rx_gain_onchange_manual()
                .into_js_value()
                .unchecked_ref(),
        ));

        set_on!(
            click,
            self,
            recorder_button,
            settings_button,
            close_alert,
            close_settings,
            recording_tab,
            ddc_tab,
            waterfall_tab
        );
        self.elements
            .recorder_button_replica
            .set_onclick(self.elements.recorder_button.onclick().as_ref());

        Ok(())
    }

    fn set_callbacks_post_apply(&self) -> Result<(), JsValue> {
        // onchange closure for DDC settings; they all use the same closure
        // this closure is here to prevent preferences.apply from calling
        // it multiple times, since the PUT request can be expensive to
        // execute by maia-httpd.
        let put_ddc_design = self.ddc_put_design_closure().into_js_value();
        let ddc_onchange = put_ddc_design.unchecked_ref();
        self.elements
            .ddc_decimation
            .set_onchange(Some(ddc_onchange));
        self.elements
            .ddc_transition_bandwidth
            .set_onchange(Some(ddc_onchange));
        self.elements
            .ddc_passband_ripple
            .set_onchange(Some(ddc_onchange));
        self.elements
            .ddc_stopband_attenuation_db
            .set_onchange(Some(ddc_onchange));
        self.elements
            .ddc_stopband_one_over_f
            .set_onchange(Some(ddc_onchange));
        // call the closure now to apply any preferences for the DDC
        ddc_onchange.call0(&JsValue::NULL)?;
        Ok(())
    }
}

// Alert
impl Ui {
    fn alert(&self, message: &str) -> Result<(), JsValue> {
        self.elements.alert_message.set_text_content(Some(message));
        self.elements.alert_dialog.show_modal()?;
        Ok(())
    }

    fn close_alert_onclick(&self) -> Closure<dyn Fn()> {
        let ui = self.clone();
        Closure::new(move || ui.elements.alert_dialog.close())
    }
}

// Settings
impl Ui {
    fn settings_button_onclick(&self) -> Closure<dyn Fn()> {
        let ui = self.clone();
        Closure::new(move || {
            if ui.elements.settings.open() {
                ui.elements.settings.close();
            } else {
                ui.elements.settings.show();
            }
        })
    }

    fn close_settings_onclick(&self) -> Closure<dyn Fn()> {
        let ui = self.clone();
        Closure::new(move || ui.elements.settings.close())
    }

    impl_tabs!(recording, ddc, waterfall);
}

// API methods
impl Ui {
    fn set_api_get_periodic(&self, interval_ms: i32) -> Result<(), JsValue> {
        let ui = self.clone();
        let handler = Closure::<dyn Fn() -> js_sys::Promise>::new(move || {
            let ui = ui.clone();
            future_to_promise(async move {
                ui.get_api_update_elements().await?;
                Ok(JsValue::NULL)
            })
        });
        let handler_ = handler.into_js_value();
        let handler: &js_sys::Function = handler_.unchecked_ref();
        // call handler immediately
        handler.call0(&JsValue::NULL)?;
        // call handler every interval_ms
        self.window
            .set_interval_with_callback_and_timeout_and_arguments_0(handler, interval_ms)?;
        Ok(())
    }

    async fn get_api_update_elements(&self) -> Result<(), JsValue> {
        let json = self.get_api().await?;
        self.api_state.replace(Some(json.clone()));
        self.update_ad9361_inactive_elements(&json.ad9361)?;
        self.update_ddc_inactive_elements(&json.ddc)?;
        self.update_spectrometer_inactive_elements(&json.spectrometer)?;
        self.update_waterfall_rate(&json.spectrometer);
        self.update_recorder_button(&json.recorder);
        self.update_recording_metadata_inactive_elements(&json.recording_metadata)?;
        self.update_recorder_inactive_elements(&json.recorder)?;

        // This potentially takes some time to complete, since it might have to
        // do a fetch call to PATCH the server time. We do this last.
        self.update_server_time(&json.time).await?;

        Ok(())
    }

    async fn get_api(&self) -> Result<maia_json::Api, JsValue> {
        let response = JsFuture::from(self.window.fetch_with_str(API_URL))
            .await?
            .dyn_into::<Response>()?;
        request::response_to_json(&response).await
    }
}

// AD9361 methods
impl Ui {
    /// Sets the value of the RX frequency.
    ///
    /// This is accomplished either by changing the DDC frequency when the DDC
    /// is the input of the waterfall and the frequency can still be changed, or
    /// by changing the AD9361 frequency otherwise.
    pub fn set_rx_frequency(&self, freq: u64) -> Result<(), JsValue> {
        let mut ad9361_freq = Some(freq);
        let state = self.api_state.borrow();
        let Some(state) = state.as_ref() else {
            return Err("set_rx_frequency: api_state not available yet".into());
        };
        if matches!(state.spectrometer.input, maia_json::SpectrometerInput::DDC) {
            // Change the DDC frequency if possible
            let samp_rate = state.ad9361.sampling_frequency as f64;
            let mut ddc_freq = freq as f64 - state.ad9361.rx_lo_frequency as f64;
            // Assume that 15% of the edges of the AD9361 spectrum is not usable
            // due to aliasing.
            const MARGIN: f64 = 0.5 * (1.0 - 0.15);
            let ddc_samp_rate = state.ddc.output_sampling_frequency;
            let limit = samp_rate * MARGIN - 0.5 * ddc_samp_rate;
            if ddc_freq.abs() > limit {
                ddc_freq = if ddc_freq < 0.0 { limit } else { -limit }.round();
                ad9361_freq = Some(u64::try_from(freq as i64 - ddc_freq as i64).unwrap());
            } else {
                ad9361_freq = None;
            }
            self.set_ddc_frequency(ddc_freq)?;
        }
        if let Some(freq) = ad9361_freq {
            // Change the AD9361 frequency
            self.elements.ad9361_rx_lo_frequency.set(&freq);
            self.elements
                .ad9361_rx_lo_frequency
                .onchange()
                .unwrap()
                .call0(&JsValue::NULL)?;
        }
        Ok(())
    }

    impl_section_custom!(
        ad9361,
        maia_json::Ad9361,
        maia_json::PatchAd9361,
        AD9361_URL,
        rx_lo_frequency,
        sampling_frequency,
        rx_rf_bandwidth,
        rx_gain,
        rx_gain_mode
    );
    impl_onchange_patch_modify_noop!(ad9361, maia_json::PatchAd9361);

    fn post_update_ad9361_elements(&self, json: &maia_json::Ad9361) -> Result<(), JsValue> {
        self.update_rx_gain_disabled_status(json);
        self.update_waterfall_ad9361(json)
    }

    fn post_patch_ad9361_update_elements(
        &self,
        json: &maia_json::PatchAd9361,
    ) -> Result<(), JsValue> {
        if json.sampling_frequency.is_some() {
            self.update_spectrometer_settings()?;
        }
        Ok(())
    }

    fn update_rx_gain_disabled_status(&self, json: &maia_json::Ad9361) {
        let disabled = match json.rx_gain_mode {
            maia_json::Ad9361GainMode::Manual => false,
            maia_json::Ad9361GainMode::FastAttack => true,
            maia_json::Ad9361GainMode::SlowAttack => true,
            maia_json::Ad9361GainMode::Hybrid => true,
        };
        self.elements.ad9361_rx_gain.set_disabled(disabled);
    }

    // Custom onchange function for the RX gain. This avoids trying to change
    // the gain when the AGC is not in manual mode, which would give an HTTP 500
    // error in the PATCH request.
    fn ad9361_rx_gain_onchange_manual(&self) -> Closure<dyn Fn() -> JsValue> {
        let closure = self.ad9361_rx_gain_onchange();
        let ui = self.clone();
        Closure::new(move || {
            let state = ui.api_state.borrow();
            let Some(state) = state.as_ref() else {
                return JsValue::NULL;
            };
            if !matches!(state.ad9361.rx_gain_mode, maia_json::Ad9361GainMode::Manual) {
                return JsValue::NULL;
            }
            // Run macro-generated closure to parse the entry value and make a FETCH request
            closure
                .as_ref()
                .unchecked_ref::<js_sys::Function>()
                .call0(&JsValue::NULL)
                .unwrap()
        })
    }
}

// DDC methods
impl Ui {
    impl_update_elements!(
        ddc,
        maia_json::DDCConfigSummary,
        frequency,
        decimation,
        output_sampling_frequency,
        max_input_sampling_frequency
    );
    impl_onchange!(ddc, maia_json::PatchDDCConfig, frequency);
    impl_onchange_patch_modify_noop!(ddc, maia_json::PatchDDCConfig);
    impl_patch!(
        ddc,
        maia_json::PatchDDCConfig,
        maia_json::DDCConfig,
        DDC_CONFIG_URL
    );
    impl_put!(
        ddc,
        maia_json::PutDDCDesign,
        maia_json::DDCConfig,
        DDC_DESIGN_URL
    );

    fn ddc_put_design_closure(&self) -> Closure<dyn Fn() -> JsValue> {
        let ui = self.clone();
        Closure::new(move || {
            if !ui.elements.ddc_frequency.report_validity()
                || !ui.elements.ddc_decimation.report_validity()
                || !ui.elements.ddc_passband_ripple.report_validity()
                || !ui.elements.ddc_stopband_attenuation_db.report_validity()
            {
                return JsValue::NULL;
            }
            let Some(frequency) = ui.elements.ddc_frequency.get() else {
                return JsValue::NULL;
            };
            let Some(decimation) = ui.elements.ddc_decimation.get() else {
                return JsValue::NULL;
            };
            // These calls can return None if the value cannot be parsed to the
            // appropriate type, in which case the entries will be missing from
            // the PUT request and maia-http will use default values.
            let transition_bandwidth = ui.elements.ddc_transition_bandwidth.get();
            let passband_ripple = ui.elements.ddc_passband_ripple.get();
            let stopband_attenuation_db = ui.elements.ddc_stopband_attenuation_db.get();
            let stopband_one_over_f = ui.elements.ddc_stopband_one_over_f.get();
            // try_borrow_mut prevents trying to update the
            // preferences as a consequence of the
            // Preferences::apply_client calling this closure
            if let Ok(mut prefs) = ui.preferences.try_borrow_mut() {
                if let Err(e) = prefs.update_ddc_decimation(&decimation) {
                    web_sys::console::error_1(&e);
                }
                if let Some(value) = transition_bandwidth {
                    if let Err(e) = prefs.update_ddc_transition_bandwidth(&value) {
                        web_sys::console::error_1(&e);
                    }
                }
                if let Some(value) = passband_ripple {
                    if let Err(e) = prefs.update_ddc_passband_ripple(&value) {
                        web_sys::console::error_1(&e);
                    }
                }
                if let Some(value) = stopband_attenuation_db {
                    if let Err(e) = prefs.update_ddc_stopband_attenuation_db(&value) {
                        web_sys::console::error_1(&e);
                    }
                }
                if let Some(value) = stopband_one_over_f {
                    if let Err(e) = prefs.update_ddc_stopband_one_over_f(&value) {
                        web_sys::console::error_1(&e);
                    }
                }
            }
            let put = maia_json::PutDDCDesign {
                frequency,
                decimation,
                transition_bandwidth,
                passband_ripple,
                stopband_attenuation_db,
                stopband_one_over_f,
            };
            let ui = ui.clone();
            future_to_promise(async move {
                request::ignore_request_failed(ui.put_ddc(&put).await)?;
                ui.update_spectrometer_settings()?;
                Ok(JsValue::NULL)
            })
            .into()
        })
    }

    fn post_update_ddc_elements(&self, json: &maia_json::DDCConfigSummary) -> Result<(), JsValue> {
        self.update_waterfall_ddc(json)
    }

    async fn patch_ddc_update_elements(
        &self,
        patch_json: &maia_json::PatchDDCConfig,
    ) -> Result<(), JsValue> {
        if let Some(json_output) = request::ignore_request_failed(self.patch_ddc(patch_json).await)?
        {
            let json = maia_json::DDCConfigSummary::from(json_output.clone());
            if let Some(state) = self.api_state.borrow_mut().as_mut() {
                state.ddc.clone_from(&json);
            }
            self.update_ddc_all_elements(&json)?;
        }
        Ok(())
    }

    /// Sets the DDC frequency.
    pub fn set_ddc_frequency(&self, frequency: f64) -> Result<(), JsValue> {
        self.elements.ddc_frequency.set(&frequency);
        self.elements
            .ddc_frequency
            .onchange()
            .unwrap()
            .call0(&JsValue::NULL)?;
        Ok(())
    }
}

// Recorder methods
impl Ui {
    impl_section!(
        recording_metadata,
        maia_json::RecordingMetadata,
        maia_json::PatchRecordingMetadata,
        RECORDING_METADATA_URL,
        filename,
        description,
        author
    );

    impl_section!(
        recorder,
        maia_json::Recorder,
        maia_json::PatchRecorder,
        RECORDER_URL,
        prepend_timestamp,
        mode,
        maximum_duration
    );

    fn update_recorder_button(&self, json: &maia_json::Recorder) {
        let text = match json.state {
            maia_json::RecorderState::Stopped => "Record",
            maia_json::RecorderState::Running => "Stop",
            maia_json::RecorderState::Stopping => "Stopping",
        };
        for button in [
            &self.elements.recorder_button,
            &self.elements.recorder_button_replica,
        ] {
            if button.inner_html() != text {
                button.set_text_content(Some(text));
                button.set_class_name(&format!("{}_button", text.to_lowercase()));
            }
        }
    }

    fn patch_recorder_promise(&self, patch: maia_json::PatchRecorder) -> JsValue {
        let ui = self.clone();
        future_to_promise(async move {
            if let Some(json_output) =
                request::ignore_request_failed(ui.patch_recorder(&patch).await)?
            {
                ui.update_recorder_button(&json_output);
            }
            Ok(JsValue::NULL)
        })
        .into()
    }

    fn recorder_button_onclick(&self) -> Closure<dyn Fn() -> JsValue> {
        let ui = self.clone();
        Closure::new(move || {
            let action = match ui.elements.recorder_button.text_content().as_deref() {
                Some("Record") => maia_json::RecorderStateChange::Start,
                Some("Stop") => maia_json::RecorderStateChange::Stop,
                Some("Stopping") => {
                    // ignore click
                    return JsValue::NULL;
                }
                content => {
                    web_sys::console::error_1(
                        &format!("recorder_button has unexpecte text_content: {content:?}").into(),
                    );
                    return JsValue::NULL;
                }
            };
            let patch = maia_json::PatchRecorder {
                state_change: Some(action),
                ..Default::default()
            };
            ui.patch_recorder_promise(patch)
        })
    }
}

// Spectrometer methods
impl Ui {
    impl_section_custom!(
        spectrometer,
        maia_json::Spectrometer,
        maia_json::PatchSpectrometer,
        SPECTROMETER_URL,
        input,
        output_sampling_frequency,
        mode
    );
    impl_post_patch_update_elements_noop!(spectrometer, maia_json::PatchSpectrometer);

    fn post_update_spectrometer_elements(
        &self,
        json: &maia_json::Spectrometer,
    ) -> Result<(), JsValue> {
        self.update_waterfall_spectrometer(json)
    }

    fn spectrometer_onchange_patch_modify(&self, json: &mut maia_json::PatchSpectrometer) {
        if json.input.is_some() {
            // add output_sampling_frequency to the patch to maintain this
            // parameter across the sample rate change
            if let Some(freq) = self
                .api_state
                .borrow()
                .as_ref()
                .map(|s| s.spectrometer.output_sampling_frequency)
            {
                // if the format of the element fails, there is not much we can
                // do
                json.output_sampling_frequency = Some(freq);
            }
        }
    }

    // This function fakes an onchange event for the spectrometer_rate in order
    // to update the spectrometer settings maintaining the current rate.
    fn update_spectrometer_settings(&self) -> Result<(), JsValue> {
        self.elements
            .spectrometer_output_sampling_frequency
            .onchange()
            .unwrap()
            .call0(&JsValue::NULL)?;
        Ok(())
    }
}

// Time methods
impl Ui {
    impl_patch!(time, maia_json::PatchTime, maia_json::Time, TIME_URL);

    async fn update_server_time(&self, json: &maia_json::Time) -> Result<(), JsValue> {
        let threshold = 1000.0; // update server time if off by more than 1 sec
        let milliseconds = js_sys::Date::now();
        if (milliseconds - json.time).abs() >= threshold {
            let patch = maia_json::PatchTime {
                time: Some(milliseconds),
            };
            request::ignore_request_failed(self.patch_time(&patch).await)?;
        }
        Ok(())
    }
}

// Waterfall methods
impl Ui {
    onchange_apply!(
        colormap_select,
        waterfall_min,
        waterfall_max,
        waterfall_show_waterfall,
        waterfall_show_spectrum,
        waterfall_show_ddc
    );

    fn colormap_select_apply(&self, value: colormap::Colormap) {
        let mut render_engine = self.render_engine.borrow_mut();
        self.waterfall
            .borrow()
            .load_colormap(&mut render_engine, value.colormap_as_slice())
            .unwrap();
    }

    fn waterfall_min_apply(&self, value: f32) {
        self.waterfall.borrow_mut().set_waterfall_min(value);
    }

    fn waterfall_max_apply(&self, value: f32) {
        self.waterfall.borrow_mut().set_waterfall_max(value);
    }

    fn waterfall_show_waterfall_apply(&self, value: bool) {
        self.waterfall.borrow_mut().set_waterfall_visible(value);
    }

    fn waterfall_show_spectrum_apply(&self, value: bool) {
        self.waterfall.borrow_mut().set_spectrum_visible(value);
    }

    fn waterfall_show_ddc_apply(&self, value: bool) {
        self.local_settings.borrow_mut().waterfall_show_ddc = value;
        let state = self.api_state.borrow();
        let Some(state) = state.as_ref() else {
            web_sys::console::error_1(
                &"waterfall_show_ddc_apply: api_state not available yet".into(),
            );
            return;
        };
        let input_is_ddc = matches!(state.spectrometer.input, maia_json::SpectrometerInput::DDC);
        self.waterfall
            .borrow_mut()
            .set_channel_visible(value && !input_is_ddc);
    }

    fn update_waterfall_ad9361(&self, json: &maia_json::Ad9361) -> Result<(), JsValue> {
        // updates only the frequency
        let mut waterfall = self.waterfall.borrow_mut();
        let samp_rate = waterfall.get_freq_samprate().1;
        let freq = json.rx_lo_frequency as f64 + self.waterfall_ddc_tuning();
        waterfall.set_freq_samprate(freq, samp_rate, &mut self.render_engine.borrow_mut())
    }

    fn waterfall_ddc_tuning(&self) -> f64 {
        let state = self.api_state.borrow();
        let Some(state) = state.as_ref() else {
            return 0.0;
        };
        if !matches!(state.spectrometer.input, maia_json::SpectrometerInput::DDC) {
            return 0.0;
        }
        state.ddc.frequency
    }

    fn update_waterfall_ddc(&self, json: &maia_json::DDCConfigSummary) -> Result<(), JsValue> {
        // updates the center frequency and channel frequency
        let mut waterfall = self.waterfall.borrow_mut();
        let state = self.api_state.borrow();
        let Some(state) = state.as_ref() else {
            return Err("update_waterfall_ddc: api_state not available yet".into());
        };
        let input_is_ddc = matches!(state.spectrometer.input, maia_json::SpectrometerInput::DDC);
        if input_is_ddc {
            // update the center frequency
            let samp_rate = waterfall.get_freq_samprate().1;
            let freq = state.ad9361.rx_lo_frequency as f64 + json.frequency;
            waterfall.set_freq_samprate(freq, samp_rate, &mut self.render_engine.borrow_mut())?;
        }
        // update the DDC channel settings
        let show_ddc = self.local_settings.borrow().waterfall_show_ddc;
        waterfall.set_channel_visible(show_ddc && !input_is_ddc);
        waterfall.set_channel_frequency(json.frequency);
        waterfall.set_channel_decimation(json.decimation);
        Ok(())
    }

    fn update_waterfall_spectrometer(&self, json: &maia_json::Spectrometer) -> Result<(), JsValue> {
        let mut waterfall = self.waterfall.borrow_mut();
        let state = self.api_state.borrow();
        let Some(state) = state.as_ref() else {
            return Err("update_waterfall_spectrometer: api_state not available yet".into());
        };
        let input_is_ddc = matches!(json.input, maia_json::SpectrometerInput::DDC);
        let ddc_tuning = if input_is_ddc {
            state.ddc.frequency
        } else {
            0.0
        };
        let freq = state.ad9361.rx_lo_frequency as f64 + ddc_tuning;
        waterfall.set_freq_samprate(
            freq,
            json.input_sampling_frequency,
            &mut self.render_engine.borrow_mut(),
        )?;
        let show_ddc = self.local_settings.borrow().waterfall_show_ddc;
        waterfall.set_channel_visible(show_ddc && !input_is_ddc);
        waterfall.set_channel_frequency(state.ddc.frequency);
        Ok(())
    }

    fn update_waterfall_rate(&self, json: &maia_json::Spectrometer) {
        self.waterfall
            .borrow_mut()
            .set_waterfall_update_rate(json.output_sampling_frequency as f32);
    }
}
