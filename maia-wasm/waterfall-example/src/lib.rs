use maia_wasm::waterfall::Waterfall;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

mod ui;
use ui::Ui;

const NFFT: usize = 4096;

#[wasm_bindgen]
pub fn make_waterfall(canvas: &str) -> Result<(), JsValue> {
    setup_waterfall(canvas, false)
}

#[wasm_bindgen]
pub fn make_waterfall_with_ui(canvas: &str) -> Result<(), JsValue> {
    setup_waterfall(canvas, true)
}

fn setup_waterfall(canvas: &str, with_ui: bool) -> Result<(), JsValue> {
    let (window, document) = maia_wasm::get_window_and_document()?;
    let canvas = Rc::new(
        document
            .get_element_by_id(canvas)
            .ok_or(&format!("unable to get {canvas} canvas element"))?
            .dyn_into::<web_sys::HtmlCanvasElement>()?,
    );
    let (render_engine, waterfall, _) = maia_wasm::new_waterfall(&window, &document, &canvas)?;

    if with_ui {
        Ui::new(
            Rc::clone(&window),
            &document,
            Rc::clone(&render_engine),
            Rc::clone(&waterfall),
        )?;
    }

    let center_freq = 915e6;
    let samp_rate = 960e3;
    // An averaging of 8 and FFT size of 4096 were used to construct the
    // waterfall data
    let waterfall_averaging = 8;
    let waterfall_rate = samp_rate / ((NFFT * waterfall_averaging) as f64);
    {
        let mut waterfall = waterfall.borrow_mut();
        waterfall.set_freq_samprate(center_freq, samp_rate, &mut render_engine.borrow_mut())?;
        waterfall.set_waterfall_min(25.0);
        waterfall.set_waterfall_max(95.0);
        waterfall.set_waterfall_update_rate(waterfall_rate as f32);
    }

    let mut generator = WaterfallGenerator::new();
    let handler = Closure::<dyn FnMut()>::new({
        let waterfall = Rc::clone(&waterfall);
        move || {
            generator.put_line(&mut waterfall.borrow_mut());
        }
    });
    let interval_ms = (1000.0 / waterfall_rate).round() as i32;
    window.set_interval_with_callback_and_timeout_and_arguments_0(
        handler.into_js_value().unchecked_ref(),
        interval_ms,
    )?;

    maia_wasm::setup_render_loop(render_engine, waterfall);
    Ok(())
}

// We generate waterfall lines by reading a JPEG file that is embedded in the wasm file

const WATERFALL_JPEG: &[u8; 888519] = include_bytes!("waterfall.jpg");
const WATERFALL_LINES: usize = 3955;

struct WaterfallGenerator {
    data: Box<[f32]>,
    current_line: usize,
}

impl WaterfallGenerator {
    fn new() -> WaterfallGenerator {
        let mut decoder = jpeg_decoder::Decoder::new(&WATERFALL_JPEG[..]);
        let pixels = decoder.decode().expect("failed to decode waterfall JPEG");
        let data = pixels
            .into_iter()
            .map(|x| {
                // Scale from 0-255 JPEG pixel data to dB units. The dB range in
                // the waterfall is 67.7 dB. The range in the JPEG is the full
                // 0-255 range. We arbitrarily set the minimum waterfall power
                // to 20 dB.
                let db = 67.7 * f32::from(x) / 255.0 + 20.0;
                // convert dB to linear power units
                10.0_f32.powf(0.1 * db)
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        WaterfallGenerator {
            data,
            current_line: 0,
        }
    }

    fn put_line(&mut self, waterfall: &mut Waterfall) {
        let line = &self.data[NFFT * self.current_line..NFFT * (self.current_line + 1)];
        self.current_line += 1;
        if self.current_line == WATERFALL_LINES {
            self.current_line = 0;
        }
        // Safety: the view into self.data is always dropped before self.data
        unsafe {
            let line = js_sys::Float32Array::view(line);
            waterfall.put_waterfall_spectrum(&line);
        }
    }
}
