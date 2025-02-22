use super::CanvasDims;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

pub struct TextRender {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
}

/// Rendered texts dimensions.
///
/// This struct contains the dimensions describing how a list of strings of text
/// has been rendered into a texture.
pub struct TextsDimensions {
    /// Texture coordinates for the bounding box of each string.
    ///
    /// This vector is the concatenation of 4 texture coordinates for each
    /// string of text that has been rendered. These 4 coordinates consist of 8
    /// floats which give the coordinates (in texture coordinates units) for the
    /// bounding box of the corresponding text string in the rendered texture.
    pub texture_coordinates: Vec<f32>,
    /// Width for the bounding box of each string.
    ///
    /// This gives the width of the bounding box of each of the rendered
    /// strings, using screen coordinates.
    pub text_width: f32,
    /// Height for the bounding box of each string.
    ///
    /// This gives the height of the bounding box of each of the rendered
    /// strings, using screen coordinates.
    pub text_height: f32,
}

impl TextRender {
    pub fn new(document: &web_sys::Document) -> Result<TextRender, JsValue> {
        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        let context = canvas
            .get_context("2d")?
            .ok_or("unable to get 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()?;
        Ok(TextRender { canvas, context })
    }

    pub fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }

    pub fn text_width(&self, text: &str, dims: CanvasDims, height_px: u32) -> Result<f32, JsValue> {
        self.set_font(height_px);
        let width_px = self.context.measure_text(text)?.width();
        let width_relative = 2.0 * width_px as f32 / dims.width as f32;
        Ok(width_relative)
    }

    fn set_font(&self, height_px: u32) {
        self.context.set_font(&format!("bold {height_px}px sans"))
    }

    pub fn render(
        &self,
        texts: &[String],
        dims: CanvasDims,
        height_px: u32,
    ) -> Result<TextsDimensions, JsValue> {
        // Find maximum width over all the texts
        self.set_font(height_px);
        let mut max = None;
        for text in texts.iter() {
            let w = self.context.measure_text(text)?.width();
            max = match (max, w) {
                (Some(z), w) if z >= w => Some(z),
                _ => Some(w),
            };
        }
        let width_px = max.ok_or("no texts specified")?.ceil() as u32;

        // Add some pixels of vertical margin to prevent pieces of texts
        // from showing in the labels for other texts
        let height_px_margin = height_px + 2;
        let margin = 0.5 * (1.0 - height_px as f32 / height_px_margin as f32);

        // Set 2D canvas dimensions to contain all the texts
        let n = (texts.len() as f32 * width_px as f32 / height_px as f32)
            .sqrt()
            .round() as usize;
        let m = texts.len().div_ceil(n);
        let total_height_px = height_px_margin * n as u32;
        let total_width_px = width_px * m as u32;
        self.canvas.set_width(total_width_px);
        self.canvas.set_height(total_height_px);

        self.context.set_text_align("center");
        self.context.set_text_baseline("middle");
        // Setting the font again is needed after resizing the canvas.
        self.set_font(height_px);
        self.context
            .clear_rect(0.0, 0.0, total_width_px as f64, total_height_px as f64);
        self.context.set_fill_style_str("white");

        // Render each text and calculate its texture coordinates. Each text
        // gets 4 2D coordinates, given by the corners of its bounding
        // rectangle.
        let mut texture_coords = Vec::with_capacity(8 * texts.len());
        for (j, text) in texts.iter().enumerate() {
            let b = j / n;
            let a = j - b * n;
            self.context.fill_text(
                text,
                (b as f64 + 0.5) * width_px as f64,
                (a as f64 + 0.5) * height_px_margin as f64,
            )?;
            texture_coords.push(b as f32 / m as f32);
            texture_coords.push(((a + 1) as f32 - margin) / n as f32);
            texture_coords.push((b + 1) as f32 / m as f32);
            texture_coords.push(((a + 1) as f32 - margin) / n as f32);
            texture_coords.push(b as f32 / m as f32);
            texture_coords.push((a as f32 + margin) / n as f32);
            texture_coords.push((b + 1) as f32 / m as f32);
            texture_coords.push((a as f32 + margin) / n as f32);
        }

        let width_relative = 2.0 * width_px as f32 / dims.width as f32;
        let height_relative = 2.0 * height_px as f32 / dims.height as f32;
        Ok(TextsDimensions {
            texture_coordinates: texture_coords,
            text_width: width_relative,
            text_height: height_relative,
        })
    }
}
