use super::{ProgramSource, RenderObject};
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, WebGlProgram, WebGlShader, WebGlTexture,
    WebGlVertexArrayObject, Window,
};

use text::TextRender;
pub use text::TextsDimensions;
pub use texture::{
    LuminanceAlpha, R16f, Rgb, Rgba, Texture, TextureBuilder, TextureInternalFormat,
    TextureMagFilter, TextureMinFilter, TextureParameter, TextureWrap,
};
pub use vao::VaoBuilder;

// There are some modules defined below

/// Render engine.
///
/// The render engine is the main object used for rendering. [`RenderObject`]'s
/// are added to the engine using [`RenderEngine::add_object`]. A call to
/// [`RenderEngine::render`] renders the scene.
///
/// The render engine also gives additional functionality, such as creation and
/// modification of textures and VAOs, and rendering of text to a texture.
pub struct RenderEngine {
    canvas: Rc<HtmlCanvasElement>,
    window: Rc<Window>,
    canvas_dims: CanvasDims,
    gl: WebGl2RenderingContext,
    current: Current,
    objects: Vec<RenderObject>,
    text_render: TextRender,
}

#[derive(Debug)]
struct Current {
    program: Option<Rc<WebGlProgram>>,
    vao: Option<Rc<WebGlVertexArrayObject>>,
    textures: Textures,
}

#[derive(Debug)]
struct Textures {
    textures: Box<[Option<Rc<WebGlTexture>>]>,
    write_pointer: usize,
}

/// Canvas dimensions.
///
/// This structure holds the canvas dimensions and can perform some calculations
/// involving device pixels and CSS pixels.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CanvasDims {
    // These are in CSS pixels
    width: u32,
    height: u32,
    // This is used to obtain device pixels
    device_pixel_ratio: f64,
}

impl CanvasDims {
    fn from_canvas_and_window(canvas: &HtmlCanvasElement, window: &web_sys::Window) -> CanvasDims {
        CanvasDims {
            width: canvas.client_width() as u32,
            height: canvas.client_height() as u32,
            device_pixel_ratio: window.device_pixel_ratio(),
        }
    }

    /// Returns the canvas dimensions in device pixels.
    pub fn device_pixels(&self) -> (u32, u32) {
        (
            (self.width as f64 * self.device_pixel_ratio).round() as u32,
            (self.height as f64 * self.device_pixel_ratio).round() as u32,
        )
    }

    /// Returns the canvas dimensions in CSS pixels.
    pub fn css_pixels(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn set_viewport(&self, gl: &WebGl2RenderingContext) {
        let (w, h) = self.device_pixels();
        gl.viewport(0, 0, w as i32, h as i32);
    }

    fn set_canvas(&self, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
        let (w, h) = self.device_pixels();
        canvas.set_width(w);
        canvas.set_height(h);
        Ok(())
    }
}

impl Textures {
    fn new(gl: &WebGl2RenderingContext) -> Result<Textures, JsValue> {
        let num_textures =
            gl.get_parameter(WebGl2RenderingContext::MAX_COMBINED_TEXTURE_IMAGE_UNITS)?
                .as_f64()
                .ok_or("MAX_COMBINED_TEXTURE_IMAGE_UNITS is not an number")? as usize;
        let textures = vec![None; num_textures].into_boxed_slice();
        Ok(Textures {
            textures,
            write_pointer: 0,
        })
    }

    fn find_texture_unit(&self, texture: &Rc<WebGlTexture>) -> Option<i32> {
        self.textures
            .iter()
            .enumerate()
            .find_map(|(j, tex)| match tex {
                Some(tex) if Rc::ptr_eq(tex, texture) => Some(j as i32),
                _ => None,
            })
    }

    fn load_texture(&mut self, gl: &WebGl2RenderingContext, texture: &Rc<WebGlTexture>) -> i32 {
        let n = self.write_pointer;
        self.write_pointer = (self.write_pointer + 1) % self.textures.len();
        self.textures[n].replace(Rc::clone(texture));
        gl.active_texture(WebGl2RenderingContext::TEXTURE0 + n as u32);
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(texture));
        n as i32
    }

    fn bind_texture(&mut self, gl: &WebGl2RenderingContext, texture: &Rc<WebGlTexture>) {
        if let Some(texture_unit) = self.find_texture_unit(texture) {
            // Texture is already bound to a texture unit. We only need to
            // activate the texture unit.
            gl.active_texture(WebGl2RenderingContext::TEXTURE0 + texture_unit as u32);
        } else {
            // Texture is not bound to any texture unit. We "load" it, which
            // leaves it bound to the current active texture unit.
            self.load_texture(gl, texture);
        }
    }
}

impl Current {
    fn new(gl: &WebGl2RenderingContext) -> Result<Current, JsValue> {
        Ok(Current {
            program: None,
            vao: None,
            textures: Textures::new(gl)?,
        })
    }
}

// This module is only used to make the RenderEngine methods defined here appear
// in the docs before those defined in the texture module.
mod render_engine {
    use super::*;

    impl RenderEngine {
        /// Creates a new render engine.
        ///
        /// The `canvas` is the HTML canvas element that will be used for the
        /// render output.
        pub fn new(
            canvas: Rc<HtmlCanvasElement>,
            window: Rc<Window>,
            document: &web_sys::Document,
        ) -> Result<RenderEngine, JsValue> {
            let gl = canvas
                .get_context("webgl2")?
                .ok_or("unable to get webgl2 context")?
                .dyn_into::<WebGl2RenderingContext>()?;
            let gl_attrs = gl
                .get_context_attributes()
                .ok_or("unable to get webgl2 context attributes")?;
            gl_attrs.set_alpha(false);
            gl_attrs.set_antialias(true);
            gl_attrs.set_power_preference(web_sys::WebGlPowerPreference::LowPower);
            let canvas_dims = CanvasDims::from_canvas_and_window(&canvas, &window);
            let current = Current::new(&gl)?;

            // We use pre-multiplied alpha to obtain correct results with bilinear
            // interpolation. See
            // http://www.realtimerendering.com/blog/gpus-prefer-premultiplication/
            gl.enable(WebGl2RenderingContext::BLEND);
            gl.blend_func(
                WebGl2RenderingContext::ONE,
                WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
            );

            Ok(RenderEngine {
                canvas,
                window,
                canvas_dims,
                gl,
                current,
                objects: Vec::new(),
                text_render: TextRender::new(document)?,
            })
        }

        /// Adds a render object to the scene.
        pub fn add_object(&mut self, object: RenderObject) {
            self.objects.push(object);
        }

        /// Renders the scene to the canvas.
        ///
        /// The scene is formed by the objects that have been previously added
        /// with [`RenderEngine::add_object`].
        pub fn render(&mut self) -> Result<(), JsValue> {
            for object in &self.objects {
                if object.enabled.get() {
                    self.current.draw(&self.gl, object)?;
                }
            }
            Ok(())
        }

        /// Compiles a WebGL2 program.
        ///
        /// This function compiles the vertex and fragment shaders given in
        /// `source` and links them as a program.
        pub fn make_program(&self, source: ProgramSource<'_>) -> Result<Rc<WebGlProgram>, JsValue> {
            self.link_program(
                &self
                    .compile_shader(WebGl2RenderingContext::VERTEX_SHADER, source.vertex_shader)?,
                &self.compile_shader(
                    WebGl2RenderingContext::FRAGMENT_SHADER,
                    source.fragment_shader,
                )?,
            )
            .map(Rc::new)
        }

        fn compile_shader(&self, shader_type: u32, source: &str) -> Result<WebGlShader, JsValue> {
            let shader = self
                .gl
                .create_shader(shader_type)
                .ok_or("failed to create shader")?;
            self.gl.shader_source(&shader, source);
            self.gl.compile_shader(&shader);
            if self
                .gl
                .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
                .as_bool()
                .unwrap_or(false)
            {
                Ok(shader)
            } else {
                Err(self
                    .gl
                    .get_shader_info_log(&shader)
                    .map(|x| JsValue::from(&x))
                    .unwrap_or_else(|| "unknown error creating shader".into()))
            }
        }

        fn link_program(
            &self,
            vertex_shader: &WebGlShader,
            fragment_shader: &WebGlShader,
        ) -> Result<WebGlProgram, JsValue> {
            let program = self.gl.create_program().ok_or("unable to create program")?;
            self.gl.attach_shader(&program, vertex_shader);
            self.gl.attach_shader(&program, fragment_shader);
            self.gl.link_program(&program);
            if self
                .gl
                .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
                .as_bool()
                .unwrap_or(false)
            {
                Ok(program)
            } else {
                Err(self
                    .gl
                    .get_program_info_log(&program)
                    .map(|x| JsValue::from(&x))
                    .unwrap_or_else(|| "unknown error linking prgram".into()))
            }
        }

        /// Creates a new VAO.
        ///
        /// This function creates a new Vertex Array Object. The VAO is
        /// constructed using a [`VaoBuilder`].
        pub fn create_vao(&mut self) -> Result<VaoBuilder<'_>, JsValue> {
            VaoBuilder::new(self)
        }

        /// Modifies an existing VAO.
        ///
        /// This function returns a [`VaoBuilder`] that can modify the existing
        /// Vertex Array Object `vao`.
        pub fn modify_vao(&mut self, vao: Rc<WebGlVertexArrayObject>) -> VaoBuilder<'_> {
            VaoBuilder::modify_vao(self, vao)
        }

        /// Creates a new texture.
        ///
        /// The texture is constructed using a [`TextureBuilder`].
        pub fn create_texture(&mut self) -> Result<TextureBuilder<'_>, JsValue> {
            TextureBuilder::new(self)
        }

        /// Returns the current canvas dimensions.
        pub fn canvas_dims(&self) -> CanvasDims {
            self.canvas_dims
        }

        /// Resizes the canvas.
        ///
        /// Resizes the canvas according to the current dimensions of the HTML
        /// canvas element and the device pixel ratio. This function should be
        /// called whenever any of these parameters change, in order to update
        /// the render engine accordingly.
        pub fn resize_canvas(&mut self) -> Result<(), JsValue> {
            self.canvas_dims = CanvasDims::from_canvas_and_window(&self.canvas, &self.window);
            self.canvas_dims.set_canvas(&self.canvas)?;
            self.canvas_dims.set_viewport(&self.gl);
            Ok(())
        }

        /// Renders a series of texts into a texture.
        ///
        /// Given a slice of text strings, this function uses an auxiliarly HTML
        /// canvas element (which does not form part of the document) to render
        /// all of these strings into the canvas and load the resulting image
        /// into a texture. The information about the coordinates of the
        /// bounding boxes of each text is return, allowing parts of the texture
        /// to be used to display each text.
        ///
        /// A fixed text height in pixels, `text_height_px`, is used to set the
        /// size of the font and of the image loaded in the texture.
        pub fn render_texts_to_texture(
            &mut self,
            texture: &Rc<WebGlTexture>,
            texts: &[String],
            text_height_px: u32,
        ) -> Result<TextsDimensions, JsValue> {
            let dimensions = self
                .text_render
                .render(texts, self.canvas_dims, text_height_px)?;
            self.texture_from_text_render::<LuminanceAlpha>(texture)?;
            Ok(dimensions)
        }

        /// Returns the corresponding text width for a given string of text.
        ///
        /// The string of text is measured with a text height of `height_px`
        /// pixels, and the width in screen coordinates is returned.
        ///
        /// This function can be used to find out how much screen space a given
        /// text will occupy before the text is even rendered.
        pub fn text_renderer_text_width(&self, text: &str, height_px: u32) -> Result<f32, JsValue> {
            self.text_render
                .text_width(text, self.canvas_dims, height_px)
        }

        #[allow(dead_code)]
        fn use_program(&mut self, program: &Rc<WebGlProgram>) {
            self.current.use_program(&self.gl, program)
        }

        pub(super) fn bind_vertex_array(&mut self, vao: &Rc<WebGlVertexArrayObject>) {
            self.current.bind_vertex_array(&self.gl, vao)
        }

        pub(super) fn bind_texture(&mut self, texture: &Rc<WebGlTexture>) {
            self.current.textures.bind_texture(&self.gl, texture)
        }
    }
}

mod text;
mod texture;
mod vao;

impl Current {
    fn use_program(&mut self, gl: &WebGl2RenderingContext, program: &Rc<WebGlProgram>) {
        gl.use_program(Some(program));
        self.program.replace(Rc::clone(program));
    }

    fn bind_vertex_array(&mut self, gl: &WebGl2RenderingContext, vao: &Rc<WebGlVertexArrayObject>) {
        gl.bind_vertex_array(Some(vao));
        self.vao.replace(Rc::clone(vao));
    }

    fn texture_unit(&mut self, gl: &WebGl2RenderingContext, texture: &Rc<WebGlTexture>) -> i32 {
        self.textures
            .find_texture_unit(texture)
            .unwrap_or_else(|| self.textures.load_texture(gl, texture))
    }

    fn draw(&mut self, gl: &WebGl2RenderingContext, object: &RenderObject) -> Result<(), JsValue> {
        if !self
            .program
            .as_ref()
            .is_some_and(|p| Rc::ptr_eq(p, &object.program))
        {
            // Program doesn't match. Load new program.
            self.use_program(gl, &object.program);
        }

        if !self
            .vao
            .as_ref()
            .is_some_and(|vao| Rc::ptr_eq(vao, &object.vao))
        {
            // VAO doesn't match. Load new VAO.
            self.bind_vertex_array(gl, &object.vao);
        }

        for uniform in object.uniforms.iter() {
            uniform.set_uniform(gl, &object.program);
        }

        for texture in object.textures.iter() {
            let texture_unit = self.texture_unit(gl, texture.texture());
            let sampler_location = gl
                .get_uniform_location(&object.program, texture.sampler())
                .ok_or("sampler uniform location not found")?;
            gl.uniform1i(Some(&sampler_location), texture_unit);
        }

        gl.draw_elements_with_i32(
            object.draw_mode as u32,
            object.draw_num_indices.get() as i32,
            WebGl2RenderingContext::UNSIGNED_SHORT,
            (object.draw_offset_elements.get() * std::mem::size_of::<u16>()) as i32,
        );

        Ok(())
    }
}
