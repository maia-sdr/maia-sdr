//! WebGL2 waterfall.
//!
//! This module contains the implementation of a WebGL2 waterfall using the
//! render engine contained in [`crate::render`].

use crate::render::{
    DrawMode, ProgramSource, RenderEngine, RenderObject, Texture, TextureMagFilter,
    TextureMinFilter, TextureParameter, TextureWrap, Uniform, UniformValue,
    texture_formats::{R16f, Rgb},
};
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{Performance, WebGlProgram, WebGlTexture, WebGlVertexArrayObject};

/// Waterfall.
///
/// This object is used to create and add a WebGL2 waterfall display to a
/// [`RenderEngine`] and to modify the parameters of the waterfall.
pub struct Waterfall {
    texture_map: Box<[f32]>,
    enables: Enables,
    uniforms: Uniforms,
    textures: Textures,
    programs: Programs,
    vaos: VAOs,
    performance: Performance,
    // State for rendering updates
    current_draw_line: usize,
    last_draw_line: usize,
    last_spectrum_timestamp: Option<f32>,
    waterfall_rate: Option<f32>,
    waterfall_wraps: usize,
    center_freq: f64,
    samp_rate: f64,
    // Auxiliary for frequency axis
    num_freqs: Vec<usize>,
    freq_radixes: Vec<u8>,
    freq_num_idx: Rc<Cell<u32>>,
    freq_num_idx_ticks: Rc<Cell<u32>>,
    zoom_levels: Vec<f32>,
    waterfall_min: f32,
    waterfall_max: f32,
}

#[derive(Default)]
struct Enables {
    waterfall: Rc<Cell<bool>>,
    spectrum_background: Rc<Cell<bool>>,
    spectrum: Rc<Cell<bool>>,
    frequency_labels: Rc<Cell<bool>>,
    frequency_ticks: Rc<Cell<bool>>,
    channel: Rc<Cell<bool>>,
}

struct Uniforms {
    time_translation: Rc<Uniform<f32>>,
    center_freq: Rc<Uniform<f32>>,
    zoom: Rc<Uniform<f32>>,
    waterfall_scale_add: Rc<Uniform<f32>>,
    waterfall_scale_add_floor: Rc<Uniform<f32>>,
    waterfall_scale_mult: Rc<Uniform<f32>>,
    waterfall_brightness: Rc<Uniform<f32>>,
    aspect_ratio: Rc<Uniform<f32>>,
    canvas_width: Rc<Uniform<f32>>,
    freq_labels_width: Rc<Uniform<f32>>,
    freq_labels_height: Rc<Uniform<f32>>,
    major_ticks_end: Rc<Uniform<i32>>,
    channel_freq: Rc<Uniform<f32>>,
    channel_width: Rc<Uniform<f32>>,
}

struct Textures {
    waterfall: Rc<WebGlTexture>,
    colormap: Rc<WebGlTexture>,
    text: Rc<WebGlTexture>,
}

struct Programs {
    frequency_labels: Rc<WebGlProgram>,
    frequency_ticks: Rc<WebGlProgram>,
}

#[derive(Default)]
struct VAOs {
    frequency_labels: Option<Rc<WebGlVertexArrayObject>>,
    frequency_ticks: Option<Rc<WebGlVertexArrayObject>>,
}

impl Waterfall {
    // number of indices for waterfall
    const NUM_INDICES: usize = 12;

    // number of indices for a rectangle
    const RECTANGLE_NUM_INDICES: usize = 6;

    const TEXTURE_WIDTH: usize = 4096;
    const TEXTURE_HEIGHT: usize = 512;

    const SPECTRUM_POINTS: usize = Self::TEXTURE_WIDTH;

    // horizontal divisions are spaced by 1 dB; these cover a 200 dB range,
    // which is more than enough
    const HORIZONTAL_DIVISIONS: usize = 200;

    // waterfall brightness when spectrum is visible
    const WATERFALL_BRIGHTNESS_WITH_SPECTRUM: f32 = 0.7;

    /// Creates a new waterfall, adding it to the [`RenderEngine`].
    ///
    /// The `performance` parameter should contain a performance object obtained
    /// with [`web_sys::Window::performance`].
    pub fn new(engine: &mut RenderEngine, performance: Performance) -> Result<Waterfall, JsValue> {
        let programs = Programs {
            frequency_labels: Self::frequency_labels_program(engine)?,
            frequency_ticks: Self::frequency_ticks_program(engine)?,
        };
        // These default values will be overwritten by the UI
        let samp_rate = 30.72e6;
        let center_freq = Self::actual_center_freq(2400e6, samp_rate);
        let mut w = Waterfall {
            texture_map: vec![0.0; Self::TEXTURE_WIDTH * Self::TEXTURE_HEIGHT].into_boxed_slice(),
            enables: Enables::default(),
            uniforms: Uniforms::new(),
            textures: Textures::new(engine)?,
            programs,
            vaos: VAOs::default(),
            performance,
            current_draw_line: Self::TEXTURE_HEIGHT - 1,
            last_draw_line: 0,
            waterfall_wraps: 0,
            last_spectrum_timestamp: None,
            waterfall_rate: None,
            center_freq,
            samp_rate,
            num_freqs: Vec::new(),
            freq_radixes: Vec::new(),
            zoom_levels: Vec::new(),
            freq_num_idx: Rc::new(Cell::new(0)),
            freq_num_idx_ticks: Rc::new(Cell::new(0)),
            waterfall_min: 35.0,
            waterfall_max: 85.0,
        };

        w.update_canvas_size(engine);
        w.update_waterfall_scale();
        w.load_waterfall(engine)?;
        w.load_colormap(engine, &crate::colormap::turbo::COLORMAP)?;
        let waterfall_object = w.waterfall_object(engine)?;
        engine.add_object(waterfall_object);
        let spectrum_background_object = w.spectrum_background_object(engine)?;
        engine.add_object(spectrum_background_object);
        let horizontal_divisions_object = w.horizontal_divisions_object(engine)?;
        engine.add_object(horizontal_divisions_object);
        let spectrum_object = w.spectrum_object(engine)?;
        engine.add_object(spectrum_object);
        let channel_object = w.channel_object(engine)?;
        engine.add_object(channel_object);
        let (frequency_labels_object, frequency_ticks_object) =
            w.frequency_labels_object(engine)?;
        engine.add_object(frequency_labels_object);
        engine.add_object(frequency_ticks_object);

        w.enables.waterfall.set(true);
        w.enables.frequency_labels.set(true);
        w.enables.frequency_ticks.set(true);

        Ok(w)
    }

    /// Adds a spectrum line to the waterfall.
    ///
    /// This function updates the waterfall by adding a new spectrum line to
    /// it. The spectrum is given in linear power units.
    pub fn put_waterfall_spectrum(&mut self, spectrum_linear: &js_sys::Float32Array) {
        self.last_spectrum_timestamp = Some(self.performance.now() as f32);
        self.current_draw_line = (self.current_draw_line + 1) % Self::TEXTURE_HEIGHT;
        let line = self.current_draw_line;
        let spectrum_texture =
            &mut self.texture_map[line * Self::TEXTURE_WIDTH..(line + 1) * Self::TEXTURE_WIDTH];
        spectrum_linear.copy_to(spectrum_texture);
        // Convert to "dB". We don't include the 10.0 factor to save us a multiplication.
        // This will later be taken into account in the shader.
        for x in spectrum_texture.iter_mut() {
            // do not compute the log10 of pixels that are zero to avoid the
            // shader from having to handle -infty
            if *x != 0.0 {
                *x = x.log10();
            }
        }
    }

    /// Updates the waterfall for rendering.
    ///
    /// This function must be called before each call to
    /// [`RenderEngine::render`]. It updates and prepares the waterfall render
    /// objects for rendering. The vale of `dt` should be the timestamp given
    /// to the `request_animation_frame` callback in which this function is
    /// called. It is currently unused, since the waterfall scroll rate is
    /// determined by how often
    /// [`put_waterfall_spectrum`](Waterfall::put_waterfall_spectrum) is called.
    pub fn prepare_render(&mut self, engine: &mut RenderEngine, dt: f32) -> Result<(), JsValue> {
        let draw_lines_coarse = self.current_draw_line as f32;
        // Fine correction to draw_t_coarse for smooth animation interpolation
        // between waterfall lines. Only applied when we have the necessary data.
        let draw_lines_fine = match (self.last_spectrum_timestamp, self.waterfall_rate) {
            (Some(t0), Some(rate)) => {
                let elapsed_secs = (dt - t0) * 1e-3;
                let elapsed_lines = elapsed_secs * rate;
                // Gives a correction between -0.5 and +0.5 lines
                elapsed_lines.clamp(0.0, 1.0) - 0.5
            }
            _ => 0.0,
        };
        let draw_t = (draw_lines_coarse + draw_lines_fine) / Self::TEXTURE_HEIGHT as f32;
        // TODO use elapsed_ms to effect draw_t. This needs us to know the spectrometer rate.
        self.uniforms.time_translation.set_data(4.0 * draw_t);

        let end_draw = self.current_draw_line;
        let start_draw = if end_draw < self.last_draw_line {
            // wraps around
            let start_wrap = self.last_draw_line + 1;
            if start_wrap != Self::TEXTURE_HEIGHT {
                // Last render didn't finish the bottom of the texture. Update
                // it and load it.
                engine.texture_subimage::<R16f>(
                    &self.textures.waterfall,
                    &self.texture_map[start_wrap * Self::TEXTURE_WIDTH..],
                    0,
                    start_wrap,
                    Self::TEXTURE_WIDTH,
                    Self::TEXTURE_HEIGHT - start_wrap,
                )?;
            }
            self.waterfall_wraps += 1;
            0
        } else {
            self.last_draw_line + 1
        };

        if start_draw != end_draw + 1 {
            engine.texture_subimage::<R16f>(
                &self.textures.waterfall,
                &self.texture_map
                    [start_draw * Self::TEXTURE_WIDTH..(end_draw + 1) * Self::TEXTURE_WIDTH],
                0,
                start_draw,
                Self::TEXTURE_WIDTH,
                end_draw - start_draw + 1,
            )?;
        }

        self.last_draw_line = end_draw;

        Ok(())
    }

    /// Updates the waterfall according to the new dimensions of the canvas.
    ///
    /// This function should be called each time that the canvas size or the
    /// device pixel ratio changes, so that the waterfall can be update accordingly.
    pub fn resize_canvas(&mut self, engine: &mut RenderEngine) -> Result<(), JsValue> {
        // update frequency labels VAOs and texts texture
        self.frequency_labels_vao(engine)?;
        self.update_canvas_size(engine);
        Ok(())
    }

    fn update_canvas_size(&mut self, engine: &mut RenderEngine) {
        let dims = engine.canvas_dims().css_pixels();
        let aspect_ratio = f64::from(dims.0) / f64::from(dims.1);
        self.uniforms.aspect_ratio.set_data(aspect_ratio as f32);
        self.uniforms.canvas_width.set_data(dims.0 as f32);
    }

    /// Updates the waterfall with a new center frequency and sample rate.
    ///
    /// The center frequency and sample rate should be given in units of Hz and
    /// samples per second.
    pub fn set_freq_samprate(
        &mut self,
        center_freq: f64,
        samp_rate: f64,
        engine: &mut RenderEngine,
    ) -> Result<(), JsValue> {
        let center_freq = Self::actual_center_freq(center_freq, samp_rate);
        if center_freq != self.center_freq || samp_rate != self.samp_rate {
            self.center_freq = center_freq;
            self.samp_rate = samp_rate;
            // update frequency labels VAOs and texts texture
            self.frequency_labels_vao(engine)?;
        }
        Ok(())
    }

    fn actual_center_freq(center_freq: f64, samp_rate: f64) -> f64 {
        // Take note that the actual center_frequency in the waterfall is not
        // baseband DC, but rather the frequency between the DC FFT bin and one
        // bin to the left.
        let fft_bin_hz = samp_rate / Self::TEXTURE_WIDTH as f64;
        center_freq - 0.5 * fft_bin_hz
    }

    /// Returns the current waterfall center frequency and sample rate.
    ///
    /// The center frequency and sample rate are given in units of Hz and
    /// samples per second.
    pub fn get_freq_samprate(&self) -> (f64, f64) {
        let samp_rate = self.samp_rate;
        (
            Self::inv_actual_center_freq(self.center_freq, samp_rate),
            samp_rate,
        )
    }

    fn inv_actual_center_freq(center_freq: f64, samp_rate: f64) -> f64 {
        // inverse of acqtual_center_freq
        let fft_bin_hz = samp_rate / Self::TEXTURE_WIDTH as f64;
        center_freq + 0.5 * fft_bin_hz
    }

    /// Returns whether the waterfall is visible.
    pub fn is_waterfall_visible(&self) -> bool {
        self.enables.waterfall.get()
    }

    /// Sets whether the waterfall is visible.
    ///
    /// By default the waterfall is visible.
    pub fn set_waterfall_visible(&self, visible: bool) {
        self.enables.waterfall.set(visible);
        self.enables.spectrum_background.set(!visible);
    }

    /// Returns whether the spectrum is visible.
    pub fn is_spectrum_visible(&self) -> bool {
        self.enables.spectrum.get()
    }

    /// Sets whether the spectrum is visible.
    ///
    /// By default the spectrum is not visible.
    pub fn set_spectrum_visible(&self, visible: bool) {
        self.enables.spectrum.set(visible);
        // darken waterfall slightly if the spectrum is visible to make the
        // spectrum more clear
        self.uniforms.waterfall_brightness.set_data(if visible {
            Self::WATERFALL_BRIGHTNESS_WITH_SPECTRUM
        } else {
            1.0
        });
    }

    /// Returns whether the DDC channel is visible in the waterfall.
    pub fn is_channel_visible(&self) -> bool {
        self.enables.channel.get()
    }

    /// Sets whether the DDC channel is visible in the waterfall.
    ///
    /// By default the channel is not visible.
    pub fn set_channel_visible(&self, visible: bool) {
        self.enables.channel.set(visible);
    }

    /// Returns the frequency of the DDC channel in the waterfall.
    ///
    /// The frequency is given in Hz of offset with respect to the waterfall
    /// center frequency.
    pub fn get_channel_frequency(&self) -> f64 {
        0.5 * self.uniforms.channel_freq.get_data() as f64 * self.samp_rate
    }

    /// Sets the frequency of the DDC channel in the waterfall.
    ///
    /// This function shall be called when the DDC frequency changes.  The
    /// `frequency` is given in Hz of offset with respect to the waterfall
    /// center frequency.
    ///
    /// This funciton must also be called when the sample rate is changed using
    /// `[Self::set_freq_samprate]`.
    pub fn set_channel_frequency(&mut self, frequency: f64) {
        // The range for frequency is [-1, 1], so we need to multiply by 2.
        let frequency = 2.0 * frequency / self.samp_rate;
        self.uniforms.channel_freq.set_data(frequency as f32);
    }

    /// Sets the decimation factor of the DDC channel in the waterfall.
    ///
    /// This function shall be called when the DDC decimation factor changes.
    pub fn set_channel_decimation(&mut self, decimation: u32) {
        self.uniforms
            .channel_width
            .set_data(f64::from(decimation).recip() as f32);
    }

    fn waterfall_object(&self, engine: &mut RenderEngine) -> Result<RenderObject, JsValue> {
        let program = Self::waterfall_program(engine)?;
        let vao = self.waterfall_vao(engine, &program)?;
        Ok(RenderObject {
            enabled: Rc::clone(&self.enables.waterfall),
            program,
            vao,
            draw_mode: DrawMode::Triangles,
            draw_num_indices: Rc::new(Cell::new(Self::NUM_INDICES as u32)),
            draw_offset_elements: Rc::new(Cell::new(0)),
            uniforms: self.uniforms.waterfall_uniforms(),
            textures: self.textures.waterfall_textures(),
        })
    }

    fn spectrum_background_object(
        &self,
        engine: &mut RenderEngine,
    ) -> Result<RenderObject, JsValue> {
        let program = Self::spectrum_background_program(engine)?;
        let vao = self.rectangle_vao(engine, &program)?;
        Ok(RenderObject {
            enabled: Rc::clone(&self.enables.spectrum_background),
            program,
            vao,
            draw_mode: DrawMode::Triangles,
            draw_num_indices: Rc::new(Cell::new(Self::RECTANGLE_NUM_INDICES as u32)),
            draw_offset_elements: Rc::new(Cell::new(0)),
            uniforms: Box::new([]),
            textures: Box::new([]),
        })
    }

    fn spectrum_object(&self, engine: &mut RenderEngine) -> Result<RenderObject, JsValue> {
        let program = Self::spectrum_program(engine)?;
        let vao = self.spectrum_vao(engine, &program)?;
        Ok(RenderObject {
            enabled: Rc::clone(&self.enables.spectrum),
            program,
            vao,
            draw_mode: DrawMode::Triangles,
            draw_num_indices: Rc::new(Cell::new(6 * (Self::SPECTRUM_POINTS - 1) as u32)),
            draw_offset_elements: Rc::new(Cell::new(0)),
            uniforms: self.uniforms.spectrum_uniforms(),
            textures: self.textures.spectrum_textures(),
        })
    }

    fn frequency_labels_object(
        &mut self,
        engine: &mut RenderEngine,
    ) -> Result<(RenderObject, RenderObject), JsValue> {
        let (vao_labels, vao_ticks) = self.frequency_labels_vao(engine)?;

        let object_labels = RenderObject {
            enabled: Rc::clone(&self.enables.frequency_labels),
            program: Rc::clone(&self.programs.frequency_labels),
            vao: vao_labels,
            draw_mode: DrawMode::Triangles,
            draw_num_indices: Rc::clone(&self.freq_num_idx),
            draw_offset_elements: Rc::new(Cell::new(0)),
            uniforms: self.uniforms.frequency_labels_uniforms(),
            textures: self.textures.text_textures(),
        };
        let object_ticks = RenderObject {
            enabled: Rc::clone(&self.enables.frequency_ticks),
            program: Rc::clone(&self.programs.frequency_ticks),
            vao: vao_ticks,
            draw_mode: DrawMode::Lines,
            draw_num_indices: Rc::clone(&self.freq_num_idx_ticks),
            draw_offset_elements: Rc::new(Cell::new(0)),
            uniforms: self.uniforms.frequency_ticks_uniforms(),
            textures: Box::new([]),
        };
        Ok((object_labels, object_ticks))
    }

    fn horizontal_divisions_object(
        &mut self,
        engine: &mut RenderEngine,
    ) -> Result<RenderObject, JsValue> {
        let program = Self::horizontal_divisions_program(engine)?;
        let vao = self.horizontal_divisions_vao(engine, &program)?;
        Ok(RenderObject {
            enabled: Rc::clone(&self.enables.spectrum),
            program,
            vao,
            draw_mode: DrawMode::Lines,
            draw_num_indices: Rc::new(Cell::new(2 * Self::HORIZONTAL_DIVISIONS as u32)),
            draw_offset_elements: Rc::new(Cell::new(0)),
            uniforms: self.uniforms.horizontal_divisions_uniforms(),
            textures: Box::new([]),
        })
    }

    fn channel_object(&self, engine: &mut RenderEngine) -> Result<RenderObject, JsValue> {
        let program = Self::channel_program(engine)?;
        let vao = self.rectangle_vao(engine, &program)?;
        Ok(RenderObject {
            enabled: Rc::clone(&self.enables.channel),
            program,
            vao,
            draw_mode: DrawMode::Triangles,
            draw_num_indices: Rc::new(Cell::new(Self::RECTANGLE_NUM_INDICES as u32)),
            draw_offset_elements: Rc::new(Cell::new(0)),
            uniforms: self.uniforms.channel_uniforms(),
            textures: Box::new([]),
        })
    }

    fn waterfall_program(engine: &RenderEngine) -> Result<Rc<WebGlProgram>, JsValue> {
        let source = ProgramSource {
            vertex_shader: r#"#version 300 es
        in vec2 aPosition;
        in vec2 aTextureCoordinates;
        uniform float uTimeTranslation;
        uniform float uCenterFreq;
        uniform float uZoom;
        out vec2 vTextureCoordinates;
        void main() {
            gl_Position = vec4(uZoom * (aPosition.x - uCenterFreq),
                               aPosition.y + uTimeTranslation,
                               0.0, 1.0);
            vTextureCoordinates = aTextureCoordinates;
        }"#,
            fragment_shader: r#"#version 300 es
        precision highp float;
        in vec2 vTextureCoordinates;
        uniform sampler2D uSampler;
        uniform sampler2D uColormapSampler;
        uniform float uWaterfallScaleAdd;
        uniform float uWaterfallScaleMult;
        uniform float uWaterfallBrightness;
        out vec4 color;
        void main() {
            float power = texture(uSampler, vTextureCoordinates).x;
            float normalizedPower = uWaterfallScaleMult * (power + uWaterfallScaleAdd);
            color = texture(uColormapSampler, vec2(normalizedPower, 0.0))
                    * vec4(vec3(uWaterfallBrightness), 1.0);
        }"#,
        };
        engine.make_program(source)
    }

    fn spectrum_background_program(engine: &RenderEngine) -> Result<Rc<WebGlProgram>, JsValue> {
        let source = ProgramSource {
            vertex_shader: r#"#version 300 es
        in vec2 aPosition;
        void main() {
            gl_Position = vec4(aPosition.xy, 0.0, 1.0);
        }"#,
            fragment_shader: r#"#version 300 es
        precision highp float;
        out vec4 color;
        void main() {
            color = vec4(vec3(0.1333), 1.0);
        }"#,
        };
        engine.make_program(source)
    }

    fn spectrum_program(engine: &RenderEngine) -> Result<Rc<WebGlProgram>, JsValue> {
        let source = ProgramSource {
            vertex_shader: &format!(
                r#"#version 300 es
        in vec2 aPosition;
        uniform sampler2D uSampler;
        uniform float uTimeTranslation;
        uniform float uCenterFreq;
        uniform float uZoom;
        uniform float uWaterfallScaleAdd;
        uniform float uWaterfallScaleMult;
        uniform float uAspectRatio;
        uniform float uCanvasWidth;
        out float vSignedDistance;
        void main() {{
            vec2 texturePosition = vec2(0.5 * (aPosition.x + 1.0), 0.25 * uTimeTranslation);
            float delta = 1.0 / {0:.3};
            vec2 textureNeighLeft = vec2(texturePosition.x - delta, texturePosition.y);
            vec2 textureNeighRight = vec2(texturePosition.x + delta, texturePosition.y);
            float power = texture(uSampler, texturePosition).x;
            float powerLeft = texture(uSampler, textureNeighLeft).x;
            float powerRight = texture(uSampler, textureNeighRight).x;
            float normalizedPower = 2.0 * uWaterfallScaleMult * (power + uWaterfallScaleAdd) - 1.0;
            float normalizedPowerLeft = 2.0 * uWaterfallScaleMult * (powerLeft + uWaterfallScaleAdd) - 1.0;
            float normalizedPowerRight = 2.0 * uWaterfallScaleMult * (powerRight + uWaterfallScaleAdd) - 1.0;

            float deltaScreen = uZoom * 2.0 / {0:.3} * uAspectRatio;
            vec2 leftNormal = normalize(vec2(normalizedPowerLeft - normalizedPower, deltaScreen));
            vec2 rightNormal = normalize(vec2(normalizedPower - normalizedPowerRight, deltaScreen));
            vec2 normal = normalize(leftNormal + rightNormal);
            float maxMiter = 10.0;
            float miter = min(maxMiter, sqrt(2.0 / (1.0 + dot(leftNormal, rightNormal))));

            vec2 position = vec2(uZoom * (aPosition.x - uCenterFreq), normalizedPower);
            float thickness = 2.0;
            vec2 positionExpand = position + thickness / uCanvasWidth * aPosition.y * miter * normal * vec2(1.0, uAspectRatio);
            gl_Position = vec4(positionExpand, 0.0, 1.0);
            vSignedDistance = aPosition.y;
        }}"#,
                (Self::TEXTURE_WIDTH - 1) as f32
            ),
            fragment_shader: r#"#version 300 es
        precision highp float;
        in float vSignedDistance;
        out vec4 color;
        void main() {
            float alpha = 1.0 - vSignedDistance * vSignedDistance;
            color = vec4(alpha);
        }"#,
        };
        engine.make_program(source)
    }

    fn frequency_ticks_program(engine: &RenderEngine) -> Result<Rc<WebGlProgram>, JsValue> {
        let source = ProgramSource {
            vertex_shader: r#"#version 300 es
        in vec2 aPosition;
        uniform float uCenterFreq;
        uniform float uZoom;
        uniform int uMajorTicksEnd;
        void main() {
            bool majorTick = gl_VertexID < uMajorTicksEnd;
            bool tickStart = (gl_VertexID & 1) == 0;
            float majorTickOffset = majorTick && !tickStart ? 0.02 : 0.0;
            gl_Position = vec4(uZoom * (aPosition.x - uCenterFreq),
                               aPosition.y + majorTickOffset,
                               0.0, 1.0);
        }"#,
            fragment_shader: r#"#version 300 es
        precision highp float;
        out vec4 color;
        void main() {
            color = vec4(1.0);
        }"#,
        };
        engine.make_program(source)
    }

    fn frequency_labels_program(engine: &RenderEngine) -> Result<Rc<WebGlProgram>, JsValue> {
        let source = ProgramSource {
            vertex_shader: r#"#version 300 es
        in vec2 aPosition;
        in vec2 aTextureCoordinates;
        uniform float uCenterFreq;
        uniform float uZoom;
        uniform float uLabelWidth;
        uniform float uLabelHeight;
        out vec2 vTextureCoordinates;
        void main() {
            float side_offset = (float(gl_VertexID & 1) - 0.5) * uLabelWidth;
            float vertical_offset = (gl_VertexID & 2) != 0 ? uLabelHeight : 0.0;
            float center = uZoom * (aPosition.x - uCenterFreq);
            gl_Position = vec4(center + side_offset,
                               aPosition.y + vertical_offset,
                               0.0, 1.0);
            vTextureCoordinates = aTextureCoordinates;
        }"#,
            fragment_shader: r#"#version 300 es
        precision highp float;
        in vec2 vTextureCoordinates;
        uniform sampler2D uSampler;
        out vec4 color;
        void main() {
            color = texture(uSampler, vTextureCoordinates);
        }"#,
        };
        engine.make_program(source)
    }

    fn horizontal_divisions_program(engine: &RenderEngine) -> Result<Rc<WebGlProgram>, JsValue> {
        let source = ProgramSource {
            vertex_shader: r#"#version 300 es
        in vec2 aPosition;
        uniform float uWaterfallScaleAdd;
        uniform float uWaterfallScaleAddFloor;
        uniform float uWaterfallScaleMult;
        out float vAlpha;
        void main() {
            bool majorDivision = (gl_VertexID >> 1) % 10 == 0;
            vAlpha = float(majorDivision) * 0.4 + 0.4;
            // power is in units of 10 dB, because the conversion to power does not include
            // the 10 factor in the 10*log10 formula
            //
            // subtracting uWaterfallScaleAddFloor + 1.0 ensures that power >= 0 for most of
            // the horizontal divisions, so that few of them are hidden below the lower edge
            // of the screen
            float power = aPosition.y - uWaterfallScaleAddFloor - 1.0;
            float normalizedPower = 2.0 * uWaterfallScaleMult * (power + uWaterfallScaleAdd) - 1.0;
            gl_Position = vec4(aPosition.x, normalizedPower, 0.0, 1.0);
        }"#,
            fragment_shader: r#"#version 300 es
        precision highp float;
        in float vAlpha;
        out vec4 color;
        void main() {
            color = vec4(vAlpha);
        }"#,
        };
        engine.make_program(source)
    }

    fn channel_program(engine: &RenderEngine) -> Result<Rc<WebGlProgram>, JsValue> {
        let source = ProgramSource {
            vertex_shader: r#"#version 300 es
        in vec2 aPosition;
        uniform float uCenterFreq;
        uniform float uZoom;
        uniform float uChannelFreq;
        uniform float uChannelWidth;
        void main() {
            gl_Position = vec4(
                uZoom * (aPosition.x * uChannelWidth + uChannelFreq - uCenterFreq),
                aPosition.y, 0.0, 1.0);
        }"#,
            fragment_shader: r#"#version 300 es
        precision highp float;
        out vec4 color;
        void main() {
            color = vec4(0.0, 0.0, 0.0, 0.33);
        }"#,
        };

        engine.make_program(source)
    }

    fn waterfall_vao(
        &self,
        engine: &mut RenderEngine,
        program: &WebGlProgram,
    ) -> Result<Rc<WebGlVertexArrayObject>, JsValue> {
        let vertices: [f32; 16] = [
            -1.0, -1.0, // A
            1.0, -1.0, // B
            -1.0, -5.0, // C
            1.0, -5.0, // D
            -1.0, 1.0, // E
            1.0, 1.0, // F
            -1.0, -1.0, // G
            1.0, -1.0, // H
        ];
        let indices: [u16; Self::NUM_INDICES] = [
            0, 1, 2, // ABC
            1, 2, 3, // BCD
            4, 5, 6, // EFG
            5, 6, 7, // FGH
        ];
        let texture_coordinates: [f32; 16] = [
            0.0, 0.0, // A
            1.0, 0.0, // B
            0.0, 1.0, // C
            1.0, 1.0, // D
            0.0, 0.5, // E
            1.0, 0.5, // F
            0.0, 1.0, // G
            1.0, 1.0, // H
        ];
        let vao = engine
            .create_vao()?
            .create_array_buffer(program, "aPosition", 2, &vertices)?
            .create_array_buffer(program, "aTextureCoordinates", 2, &texture_coordinates)?
            .create_element_array_buffer(&indices)?
            .build();
        Ok(vao)
    }

    fn spectrum_vao(
        &self,
        engine: &mut RenderEngine,
        program: &WebGlProgram,
    ) -> Result<Rc<WebGlVertexArrayObject>, JsValue> {
        let step = 2.0 / (Self::SPECTRUM_POINTS - 1) as f64;
        let vertices: Vec<f32> = (0..Self::SPECTRUM_POINTS)
            .flat_map(|j| {
                let x = (-1.0 + step * j as f64) as f32;
                [x, -1.0, x, 1.0]
            })
            .collect();
        let indices: Vec<u16> = (0..2 * u16::try_from(Self::SPECTRUM_POINTS).unwrap() - 2)
            .flat_map(|j| [j, j + 1, j + 2])
            .collect();
        let vao = engine
            .create_vao()?
            .create_array_buffer(program, "aPosition", 2, &vertices)?
            .create_element_array_buffer(&indices)?
            .build();
        Ok(vao)
    }

    fn frequency_labels_vao(
        &mut self,
        engine: &mut RenderEngine,
    ) -> Result<(Rc<WebGlVertexArrayObject>, Rc<WebGlVertexArrayObject>), JsValue> {
        // Measure the width of a frequency label to determine the width of the
        // bounding box for the labels. We use 0000.000 as a "template label", since
        // we don't really know what labels we will use yet.
        const TEXT_HEIGHT_PX: u32 = 16;
        let boundingbox_margin_factor = 1.1;
        let width_boundingbox = boundingbox_margin_factor
            * engine.text_renderer_text_width("0000.000", TEXT_HEIGHT_PX)?;
        let mut max_depth_labels = 4;
        let mut max_depth = max_depth_labels + 2;

        let s = (self.samp_rate * 0.5 * width_boundingbox as f64).log10();
        let s2 = s.ceil();
        let s3 = s2 - 2.0_f64.log10();
        let (mut step, mut radix5) = if s3 >= s {
            (10.0_f64.powf(s3), true)
        } else {
            (10.0_f64.powf(s2), false)
        };
        let minfreq = self.center_freq - 0.5 * self.samp_rate;
        let maxfreq = self.center_freq + 0.5 * self.samp_rate;
        let start = (minfreq / step).floor() as i32 - 1;
        let stop = (maxfreq / step).ceil() as i32 + 1;
        let mut freqs = (start..=stop).map(|k| k as f64 * step).collect::<Vec<_>>();
        let mut nfreqs = Vec::with_capacity(max_depth + 1);
        nfreqs.push(freqs.len());
        let mut freq_radixes = Vec::with_capacity(max_depth);
        let step_factor = 0.5 * width_boundingbox as f64 * self.samp_rate;
        let mut zoom_levels = vec![(step_factor / step) as f32];
        for depth in 0..max_depth {
            step /= if radix5 { 5.0 } else { 2.0 };
            freq_radixes.push(if radix5 { 5 } else { 2 });
            for j in 0..freqs.len() {
                let f = freqs[j];
                if radix5 {
                    for &mult in &[-2.0, -1.0, 1.0, 2.0] {
                        freqs.push(f + mult * step);
                    }
                } else {
                    if j == 0 {
                        freqs.push(f - step);
                    }
                    freqs.push(f + step);
                }
            }
            radix5 = !radix5;
            nfreqs.push(freqs.len());
            if depth < max_depth_labels - 1 {
                zoom_levels.push((step_factor / step) as f32);
            }
        }

        // cull frequencies outside passband
        let freqs_all = freqs;
        let mut freqs = Vec::with_capacity(freqs_all.len());
        let mut j = 0;
        for nf in nfreqs.iter_mut() {
            while j < *nf {
                if freqs_all[j] > minfreq && freqs_all[j] < maxfreq {
                    freqs.push(freqs_all[j]);
                }
                j += 1;
            }
            *nf = freqs.len();
        }
        drop(freqs_all);

        // We need to have 2 vertices per frequency for the ticks, and we cannot
        // have more than 1 << 16 vertices, since we index them with a u16.
        //
        // Limit depth of frequencies to guarantee that this happens. Typically,
        // no limiting needs to be done. The limiting is only used at high resolutions.
        if 2 * freqs.len() > (1 << 16) {
            let (depth, ndepth) = nfreqs
                .iter()
                .copied()
                .enumerate()
                .filter(|&(_, n)| 2 * n <= 1 << 16)
                .next_back()
                .unwrap();
            freqs.truncate(ndepth);
            nfreqs.truncate(depth + 1);
            max_depth = depth;
            assert_eq!(nfreqs.len(), max_depth + 1);
            max_depth_labels = if max_depth > 2 { max_depth - 2 } else { 1 };
            zoom_levels.truncate(max_depth_labels);
        }
        assert!(2 * freqs.len() <= (1 << 16));

        let freqs_labels = &freqs[..nfreqs[max_depth_labels - 1]];
        // We need to have 4 vertices per frequency label for the labels, and we
        // cannot have more than 1 << 16 vertices, since we index them with a
        // u16.
        assert!(4 * freqs_labels.len() <= (1 << 16));

        let y = -0.96;
        let vertices_labels = freqs_labels
            .iter()
            .flat_map(|f| {
                let x = (2.0 * (f - self.center_freq) / self.samp_rate) as f32;
                [x, y, x, y, x, y, x, y]
            })
            .collect::<Vec<f32>>();

        let vertices_ticks = freqs
            .iter()
            .flat_map(|f| {
                let x = (2.0 * (f - self.center_freq) / self.samp_rate) as f32;
                [x, -1.0, x, -0.98]
            })
            .collect::<Vec<f32>>();

        let indices_labels = freqs_labels
            .iter()
            .enumerate()
            .flat_map(|(j, _)| {
                let a = 4 * j as u16;
                [a, a + 1, a + 2, a + 1, a + 2, a + 3]
            })
            .collect::<Vec<u16>>();

        let indices_ticks = (0..vertices_ticks.len())
            .map(|x| x as u16)
            .collect::<Vec<u16>>();

        let texture_texts = freqs_labels
            .iter()
            .map(|f| format!("{:.03}", f * 1e-6))
            .collect::<Vec<_>>();
        let texts_dimensions =
            engine.render_texts_to_texture(&self.textures.text, &texture_texts, TEXT_HEIGHT_PX)?;

        let vao_labels = match self.vaos.frequency_labels.take() {
            Some(vao) => engine.modify_vao(vao),
            None => engine.create_vao()?,
        }
        .create_array_buffer(
            &self.programs.frequency_labels,
            "aPosition",
            2,
            &vertices_labels,
        )?
        .create_array_buffer(
            &self.programs.frequency_labels,
            "aTextureCoordinates",
            2,
            &texts_dimensions.texture_coordinates,
        )?
        .create_element_array_buffer(&indices_labels)?
        .build();
        self.vaos.frequency_labels = Some(Rc::clone(&vao_labels));

        let vao_ticks = match self.vaos.frequency_ticks.take() {
            Some(vao) => engine.modify_vao(vao),
            None => engine.create_vao()?,
        }
        .create_array_buffer(
            &self.programs.frequency_ticks,
            "aPosition",
            2,
            &vertices_ticks,
        )?
        .create_element_array_buffer(&indices_ticks)?
        .build();
        self.vaos.frequency_ticks = Some(Rc::clone(&vao_ticks));

        self.num_freqs = nfreqs;
        self.freq_radixes = freq_radixes;
        self.zoom_levels = zoom_levels;
        // Update zoom-related variables.
        self.set_zoom(self.get_zoom());
        self.uniforms
            .freq_labels_width
            .set_data(texts_dimensions.text_width);
        self.uniforms
            .freq_labels_height
            .set_data(texts_dimensions.text_height);

        Ok((vao_labels, vao_ticks))
    }

    fn horizontal_divisions_vao(
        &self,
        engine: &mut RenderEngine,
        program: &WebGlProgram,
    ) -> Result<Rc<WebGlVertexArrayObject>, JsValue> {
        // 1 dB separation in the divisions. The power is in units of 10 dB, so
        // we space the y coordinates by 0.1
        let vertices: Vec<f32> = (0..Self::HORIZONTAL_DIVISIONS)
            .flat_map(|j| [-1.0, 0.1 * j as f32, 1.0, 0.1 * j as f32])
            .collect();
        let indices: Vec<u16> = (0..vertices.len()).map(|x| x as u16).collect();
        let vao = engine
            .create_vao()?
            .create_array_buffer(program, "aPosition", 2, &vertices)?
            .create_element_array_buffer(&indices)?
            .build();
        Ok(vao)
    }

    fn rectangle_vao(
        &self,
        engine: &mut RenderEngine,
        program: &WebGlProgram,
    ) -> Result<Rc<WebGlVertexArrayObject>, JsValue> {
        let vertices: [f32; 8] = [
            -1.0, -1.0, // A
            1.0, -1.0, // B
            1.0, 1.0, // C
            -1.0, 1.0, // D
        ];
        let indices: [u16; Self::RECTANGLE_NUM_INDICES] = [
            0, 1, 2, // ABC
            2, 3, 0, // CDA
        ];
        let vao = engine
            .create_vao()?
            .create_array_buffer(program, "aPosition", 2, &vertices)?
            .create_element_array_buffer(&indices)?
            .build();
        Ok(vao)
    }

    /// Loads a new colormap for the waterfall.
    ///
    /// The `colormap` is given as a slice whose length is a multiple of 3 and
    /// contains the concatenation of the RGB values of the list of colors that
    /// defines the colormap (typically, 256 colors are used for the colormap,
    /// so the length of the colormap slice is `3 * 256`).
    pub fn load_colormap(&self, engine: &mut RenderEngine, colormap: &[u8]) -> Result<(), JsValue> {
        self.textures.load_colormap(engine, colormap)
    }

    fn load_waterfall(&self, engine: &mut RenderEngine) -> Result<(), JsValue> {
        engine.texture_image::<R16f>(
            &self.textures.waterfall,
            &self.texture_map,
            Self::TEXTURE_WIDTH,
            Self::TEXTURE_HEIGHT,
        )
    }

    /// Sets the zoom level of the waterfall.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.uniforms.zoom.set_data(zoom);
        // TODO: improve search algorithm
        let mut k = 0;
        for (j, &z) in self.zoom_levels.iter().enumerate() {
            if z <= zoom {
                k = j;
            } else {
                break;
            }
        }
        self.freq_num_idx.set(6 * self.num_freqs[k] as u32);
        let next = if self.freq_radixes[k] == 2 { k + 1 } else { k };
        self.freq_num_idx_ticks
            .set(2 * self.num_freqs[next + 1] as u32);
        self.uniforms
            .major_ticks_end
            .set_data(2 * self.num_freqs[next] as i32);
    }

    /// Returns the current zoom level of the waterfall.
    pub fn get_zoom(&self) -> f32 {
        self.uniforms.zoom.get_data()
    }

    /// Sets the center frequency of the waterfall.
    ///
    /// This function is used when dragging the waterfall to scroll in
    /// frequency. The `frequency` does not use physical units, but rather has a
    /// value between -1 and 1 that corresponds to screen coordinates.
    pub fn set_center_frequency(&mut self, frequency: f32) {
        self.uniforms.center_freq.set_data(frequency);
    }

    /// Returns the current center frequency of the waterfall.
    ///
    /// The frequency is defined as in the
    /// [`set_center_frequency`](Waterfall::set_center_frequency) function.
    pub fn get_center_frequency(&self) -> f32 {
        self.uniforms.center_freq.get_data()
    }

    /// Sets the waterfall minimum power value.
    ///
    /// The minimum value is used to scale the colormap. The `value` is in dB
    /// units.
    pub fn set_waterfall_min(&mut self, value: f32) {
        self.waterfall_min = value;
        self.update_waterfall_scale();
    }

    /// Sets the waterfall maximum power value.
    ///
    /// The maximum value is used to scale the colormap. The `value` is in dB
    /// units.
    pub fn set_waterfall_max(&mut self, value: f32) {
        self.waterfall_max = value;
        self.update_waterfall_scale();
    }

    /// Returns the value of the uniform associated with the DDC channel
    /// frequency.
    pub fn get_channel_frequency_uniform(&self) -> f32 {
        self.uniforms.channel_freq.get_data()
    }

    /// Returns the value of the uniform associated with the DDC channel
    /// width.
    pub fn get_channel_width_uniform(&self) -> f32 {
        self.uniforms.channel_width.get_data()
    }

    fn update_waterfall_scale(&mut self) {
        let waterfall_scale_add = -self.waterfall_min * 0.1;
        self.uniforms
            .waterfall_scale_add
            .set_data(waterfall_scale_add);
        self.uniforms
            .waterfall_scale_add_floor
            .set_data(waterfall_scale_add.floor());
        self.uniforms
            .waterfall_scale_mult
            .set_data(10.0 / (self.waterfall_max - self.waterfall_min));
    }

    /// Sets the waterfall update rate.
    ///
    /// The waterfall update rate is used for smooth animation interpolation
    /// between waterfall lines. If the rate is not set, smooth animation
    /// interpolation is not used.
    ///
    /// The rate is indicated in Hz (updates per second).
    pub fn set_waterfall_update_rate(&mut self, rate: f32) {
        self.waterfall_rate = Some(rate);
    }
}

impl Textures {
    fn new(engine: &mut RenderEngine) -> Result<Textures, JsValue> {
        // We do not use mipmaps for the waterfall texture, to avoid having to
        // regenerate the mipmap every time that a small piece of the texture is
        // updated.
        let waterfall = engine
            .create_texture()?
            .set_parameter(TextureParameter::MagFilter(TextureMagFilter::Linear))
            .set_parameter(TextureParameter::MinFilter(TextureMinFilter::Linear))
            .set_parameter(TextureParameter::WrapS(TextureWrap::ClampToEdge))
            .set_parameter(TextureParameter::WrapT(TextureWrap::ClampToEdge))
            .build();

        let colormap = engine
            .create_texture()?
            .set_parameter(TextureParameter::MagFilter(TextureMagFilter::Linear))
            .set_parameter(TextureParameter::MinFilter(
                TextureMinFilter::LinearMipmapLinear,
            ))
            .set_parameter(TextureParameter::WrapS(TextureWrap::ClampToEdge))
            .set_parameter(TextureParameter::WrapT(TextureWrap::ClampToEdge))
            .build();

        let text = engine
            .create_texture()?
            .set_parameter(TextureParameter::MagFilter(TextureMagFilter::Linear))
            .set_parameter(TextureParameter::MinFilter(TextureMinFilter::Linear))
            .set_parameter(TextureParameter::WrapS(TextureWrap::ClampToEdge))
            .set_parameter(TextureParameter::WrapT(TextureWrap::ClampToEdge))
            .build();

        Ok(Textures {
            waterfall,
            colormap,
            text,
        })
    }

    fn load_colormap(&self, engine: &mut RenderEngine, colormap: &[u8]) -> Result<(), JsValue> {
        engine.texture_image::<Rgb>(&self.colormap, colormap, colormap.len() / 3, 1)?;
        engine.generate_mipmap(&self.colormap);
        Ok(())
    }

    fn waterfall_textures(&self) -> Box<[Texture]> {
        Box::new([
            Texture::new(String::from("uSampler"), Rc::clone(&self.waterfall)),
            Texture::new(String::from("uColormapSampler"), Rc::clone(&self.colormap)),
        ])
    }

    fn spectrum_textures(&self) -> Box<[Texture]> {
        Box::new([Texture::new(
            String::from("uSampler"),
            Rc::clone(&self.waterfall),
        )])
    }

    fn text_textures(&self) -> Box<[Texture]> {
        Box::new([Texture::new(
            String::from("uSampler"),
            Rc::clone(&self.text),
        )])
    }
}

impl Uniforms {
    fn new() -> Uniforms {
        Uniforms {
            time_translation: Rc::new(Uniform::new(String::from("uTimeTranslation"), 0.0)),
            center_freq: Rc::new(Uniform::new(String::from("uCenterFreq"), 0.0)),
            zoom: Rc::new(Uniform::new(String::from("uZoom"), 1.0)),
            waterfall_scale_add: Rc::new(Uniform::new(String::from("uWaterfallScaleAdd"), 0.0)),
            waterfall_scale_add_floor: Rc::new(Uniform::new(
                String::from("uWaterfallScaleAddFloor"),
                0.0,
            )),
            waterfall_scale_mult: Rc::new(Uniform::new(String::from("uWaterfallScaleMult"), 0.0)),
            waterfall_brightness: Rc::new(Uniform::new(String::from("uWaterfallBrightness"), 1.0)),
            aspect_ratio: Rc::new(Uniform::new(String::from("uAspectRatio"), 0.0)),
            canvas_width: Rc::new(Uniform::new(String::from("uCanvasWidth"), 0.0)),
            freq_labels_width: Rc::new(Uniform::new(
                String::from("uLabelWidth"),
                Default::default(),
            )),
            freq_labels_height: Rc::new(Uniform::new(
                String::from("uLabelHeight"),
                Default::default(),
            )),
            major_ticks_end: Rc::new(Uniform::new(
                String::from("uMajorTicksEnd"),
                Default::default(),
            )),
            channel_freq: Rc::new(Uniform::new(String::from("uChannelFreq"), 0.0)),
            channel_width: Rc::new(Uniform::new(String::from("uChannelWidth"), 0.1)),
        }
    }

    fn waterfall_uniforms(&self) -> Box<[Rc<dyn UniformValue>]> {
        Box::new([
            Rc::clone(&self.time_translation) as _,
            Rc::clone(&self.center_freq) as _,
            Rc::clone(&self.zoom) as _,
            Rc::clone(&self.waterfall_scale_add) as _,
            Rc::clone(&self.waterfall_scale_mult) as _,
            Rc::clone(&self.waterfall_brightness) as _,
        ])
    }

    fn spectrum_uniforms(&self) -> Box<[Rc<dyn UniformValue>]> {
        Box::new([
            Rc::clone(&self.time_translation) as _,
            Rc::clone(&self.center_freq) as _,
            Rc::clone(&self.zoom) as _,
            Rc::clone(&self.waterfall_scale_add) as _,
            Rc::clone(&self.waterfall_scale_mult) as _,
            Rc::clone(&self.aspect_ratio) as _,
            Rc::clone(&self.canvas_width) as _,
        ])
    }

    fn frequency_ticks_uniforms(&self) -> Box<[Rc<dyn UniformValue>]> {
        Box::new([
            Rc::clone(&self.center_freq) as _,
            Rc::clone(&self.zoom) as _,
            Rc::clone(&self.major_ticks_end) as _,
        ])
    }

    fn frequency_labels_uniforms(&self) -> Box<[Rc<dyn UniformValue>]> {
        Box::new([
            Rc::clone(&self.center_freq) as _,
            Rc::clone(&self.zoom) as _,
            Rc::clone(&self.freq_labels_width) as _,
            Rc::clone(&self.freq_labels_height) as _,
        ])
    }

    fn horizontal_divisions_uniforms(&self) -> Box<[Rc<dyn UniformValue>]> {
        Box::new([
            Rc::clone(&self.waterfall_scale_add) as _,
            Rc::clone(&self.waterfall_scale_add_floor) as _,
            Rc::clone(&self.waterfall_scale_mult) as _,
        ])
    }

    fn channel_uniforms(&self) -> Box<[Rc<dyn UniformValue>]> {
        Box::new([
            Rc::clone(&self.center_freq) as _,
            Rc::clone(&self.zoom) as _,
            Rc::clone(&self.channel_freq) as _,
            Rc::clone(&self.channel_width) as _,
        ])
    }
}

impl Default for Uniforms {
    fn default() -> Uniforms {
        Uniforms::new()
    }
}
