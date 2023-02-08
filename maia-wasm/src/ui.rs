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
    Document, HtmlButtonElement, HtmlDialogElement, HtmlInputElement, HtmlSelectElement, Response,
    Window,
};

use crate::render::RenderEngine;
use crate::waterfall::Waterfall;

use active::IsElementActive;
use input::{EnumInput, InputElement, NumberInput, TextInput};
use patch::json_patch;

mod active;
mod colormap;
mod input;
#[macro_use]
mod macros;
mod patch;
mod preferences;

const API_URL: &str = "/api";
const AD9361_URL: &str = "/api/ad9361";
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
    preferences: Rc<RefCell<preferences::Preferences>>,
    render_engine: Rc<RefCell<RenderEngine>>,
    waterfall: Rc<RefCell<Waterfall>>,
}

// Defines the 'struct Elements' and its constructor
ui_elements! {
    colormap_select: HtmlSelectElement => EnumInput<colormap::Colormap>,
    recorder_button: HtmlButtonElement => Rc<HtmlButtonElement>,
    recording_properties_button: HtmlButtonElement => Rc<HtmlButtonElement>,
    recording_dialog: HtmlDialogElement => Rc<HtmlDialogElement>,
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
    spectrometer_output_sampling_frequency: HtmlInputElement
        => NumberInput<f64, input::IntegerPresentation>,
    recording_metadata_filename: HtmlInputElement => TextInput,
    recording_metadata_description: HtmlInputElement => TextInput,
    recording_metadata_author: HtmlInputElement => TextInput,
    recorder_mode: HtmlSelectElement => EnumInput<maia_json::RecorderMode>,
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
            preferences,
            render_engine,
            waterfall,
        };
        ui.set_callbacks()?;
        ui.preferences.borrow().apply(&ui)?;
        Ok(ui)
    }

    fn set_callbacks(&self) -> Result<(), JsValue> {
        self.resize_canvas()();
        self.window
            .set_onresize(Some(self.onresize().into_js_value().unchecked_ref()));
        self.set_api_get_periodic(1000)?;

        set_on!(
            change,
            self,
            colormap_select,
            waterfall_min,
            waterfall_max,
            ad9361_rx_lo_frequency,
            ad9361_sampling_frequency,
            ad9361_rx_rf_bandwidth,
            ad9361_rx_gain,
            ad9361_rx_gain_mode,
            spectrometer_output_sampling_frequency,
            recording_metadata_filename,
            recording_metadata_description,
            recording_metadata_author,
            recorder_mode
        );

        set_on!(click, self, recorder_button, recording_properties_button);

        Ok(())
    }

    /// Sets the value of the RX LO frequency UI element.
    pub fn set_rx_lo_frequency(&self, freq: u64) -> Result<(), JsValue> {
        self.elements.ad9361_rx_lo_frequency.set(&freq);
        self.elements
            .ad9361_rx_lo_frequency
            .onchange()
            .unwrap()
            .call0(&JsValue::NULL)?;
        Ok(())
    }

    impl_section!(
        spectrometer,
        maia_json::Spectrometer,
        maia_json::PatchSpectrometer,
        SPECTROMETER_URL,
        output_sampling_frequency
    );

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
        mode
    );

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
        let handler = handler_.unchecked_ref();
        // call handler immediately
        self.window.set_timeout_with_callback(handler)?;
        // call handler every interval_ms
        self.window
            .set_interval_with_callback_and_timeout_and_arguments_0(handler, interval_ms)?;
        Ok(())
    }

    fn resize_canvas(&self) -> impl Fn() {
        let render_engine = Rc::clone(&self.render_engine);
        let waterfall = Rc::clone(&self.waterfall);
        move || {
            let mut engine = render_engine.borrow_mut();
            engine.resize_canvas().unwrap();
            waterfall.borrow_mut().resize_canvas(&mut engine).unwrap();
        }
    }

    fn onresize(&self) -> Closure<dyn Fn()> {
        Closure::new(self.resize_canvas())
    }

    fn colormap_select_onchange(&self) -> Closure<dyn Fn()> {
        let ui = self.clone();
        Closure::new(move || {
            let colormap = ui.elements.colormap_select.get().unwrap();
            let mut render_engine = ui.render_engine.borrow_mut();
            ui.waterfall
                .borrow()
                .load_colormap(&mut render_engine, colormap.colormap_as_slice())
                .unwrap();
            // try_borrow_mut prevents trying to update the preferences as a
            // consequence of the Preferences::apply_client calling this
            // function
            if let Ok(mut p) = ui.preferences.try_borrow_mut() {
                if let Err(e) = p.update_colormap_select(&colormap) {
                    web_sys::console::error_1(&e);
                }
            }
        })
    }

    waterfallminmax_onchange!(waterfall_min);
    waterfallminmax_onchange!(waterfall_max);

    async fn get_api_update_elements(&self) -> Result<(), JsValue> {
        let json = self.get_api().await?;
        self.update_ad9361_inactive_elements(&json.ad9361)?;
        self.update_spectrometer_inactive_elements(&json.spectrometer);
        self.update_waterfall_rate(&json.spectrometer);
        self.update_recorder_button(&json.recorder);
        self.update_recording_metadata_inactive_elements(&json.recording_metadata);
        self.update_recorder_inactive_elements(&json.recorder);

        // This potentially takes some time to complete, since it might have to
        // do a fetch call to PATCH the server time. We do this last.
        self.update_server_time(&json.time).await?;

        Ok(())
    }

    async fn get_api(&self) -> Result<maia_json::Api, JsValue> {
        let response = JsFuture::from(self.window.fetch_with_str(API_URL))
            .await?
            .dyn_into::<Response>()?;
        Self::response_to_json(response).await
    }

    async fn response_to_json<T>(response: Response) -> Result<T, JsValue>
    where
        for<'a> T: serde::Deserialize<'a>,
    {
        let text = JsFuture::from(response.text()?).await?;
        let json = serde_json::from_str(
            &text
                .as_string()
                .ok_or("unable to convert fetch text to string")?,
        )
        .map_err(|_| format!("unable to parse {} JSON", std::any::type_name::<T>()))?;
        Ok(json)
    }

    // The ad9361 is not implemented via impl_section! because it needs custom
    // update element functions that call update_waterfall_ad9361 and a custom
    // patch-update that calls the spectrometer onchange closure.
    async fn patch_ad9361_update_elements(
        &self,
        json: &maia_json::PatchAd9361,
    ) -> Result<(), JsValue> {
        let json_output = self.patch_ad9361(json).await?;
        self.update_ad9361_all_elements(&json_output)?;
        if json.sampling_frequency.is_some() {
            // The spectrometer needs to be updated also. To do this, we fake an
            // onchange event for the spectrometer_rate input element.
            self.elements
                .spectrometer_output_sampling_frequency
                .onchange()
                .unwrap()
                .call0(&JsValue::NULL)?;
        }
        Ok(())
    }

    fn update_ad9361_inactive_elements(&self, json: &maia_json::Ad9361) -> Result<(), JsValue> {
        set_values_if_inactive!(
            self,
            json,
            ad9361,
            rx_lo_frequency,
            sampling_frequency,
            rx_rf_bandwidth,
            rx_gain,
            rx_gain_mode
        );
        self.update_waterfall_ad9361(json)
    }

    fn update_ad9361_all_elements(&self, json: &maia_json::Ad9361) -> Result<(), JsValue> {
        set_values!(
            self,
            json,
            ad9361,
            rx_lo_frequency,
            sampling_frequency,
            rx_rf_bandwidth,
            rx_gain,
            rx_gain_mode
        );
        self.update_waterfall_ad9361(json)
    }

    fn update_waterfall_ad9361(&self, json: &maia_json::Ad9361) -> Result<(), JsValue> {
        self.waterfall.borrow_mut().set_freq_samprate(
            json.rx_lo_frequency as f64,
            f64::from(json.sampling_frequency),
            &mut self.render_engine.borrow_mut(),
        )
    }

    impl_patch!(
        ad9361,
        maia_json::PatchAd9361,
        maia_json::Ad9361,
        AD9361_URL
    );

    impl_onchange!(
        ad9361,
        maia_json::PatchAd9361,
        rx_lo_frequency,
        sampling_frequency,
        rx_rf_bandwidth,
        rx_gain,
        rx_gain_mode
    );

    fn update_recorder_button(&self, json: &maia_json::Recorder) {
        let text = match json.state {
            maia_json::RecorderState::Stopped => "Record",
            maia_json::RecorderState::Running => "Stop",
        };
        let button = &self.elements.recorder_button;
        if button.inner_html() != text {
            button.set_inner_html(text);
            button.set_class_name(&format!("{}_button", text.to_lowercase()));
        }
    }

    fn patch_recorder_promise(&self, patch: maia_json::PatchRecorder) -> JsValue {
        let ui = self.clone();
        future_to_promise(async move {
            let patch = patch;
            let json = ui.patch_recorder(&patch).await?;
            ui.update_recorder_button(&json);
            Ok(JsValue::NULL)
        })
        .into()
    }

    fn recorder_button_onclick(&self) -> Closure<dyn Fn() -> JsValue> {
        let ui = self.clone();
        Closure::new(move || {
            let action = match ui.elements.recorder_button.inner_html().as_str() {
                "Record" => maia_json::RecorderStateChange::Start,
                "Stop" => maia_json::RecorderStateChange::Stop,
                _ => return JsValue::NULL,
            };
            let patch = maia_json::PatchRecorder {
                state_change: Some(action),
                ..Default::default()
            };
            ui.patch_recorder_promise(patch)
        })
    }

    fn recording_properties_button_onclick(&self) -> Closure<dyn Fn()> {
        let ui = self.clone();
        Closure::new(move || {
            ui.elements.recording_dialog.show_modal().unwrap();
        })
    }

    fn update_waterfall_rate(&self, json: &maia_json::Spectrometer) {
        self.waterfall
            .borrow_mut()
            .set_waterfall_update_rate(json.output_sampling_frequency as f32);
    }

    impl_patch!(time, maia_json::PatchTime, maia_json::Time, TIME_URL);

    async fn update_server_time(&self, json: &maia_json::Time) -> Result<(), JsValue> {
        let threshold = 1000.0; // update server time if off by more than 1 sec
        let milliseconds = js_sys::Date::now();
        if (milliseconds - json.time).abs() >= threshold {
            let patch = maia_json::PatchTime {
                time: Some(milliseconds),
            };
            self.patch_time(&patch).await?;
        }
        Ok(())
    }

    // fn update_server_preferences(&self, json: &maia_json::Api) -> Result<(), JsValue> {
    //     let mut p = self.preferences.borrow_mut();
    //     p.update_ad9361_rx_lo_frequency(json.ad9361.rx_lo_frequency)?;
    //     p.update_ad9361_sampling_frequency(json.ad9361.
    // }
}
