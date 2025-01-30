use super::RenderEngine;
use crate::array_view::ArrayView;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlTexture};

/// WebGL2 texture.
///
/// An opaque object describing a WebGL2 texture. It pairs a WebGL2 texture
/// object (`WebGlTexture`) together with the identifier of its corresponding
/// sampler.
#[derive(Debug)]
pub struct Texture {
    texture: Rc<WebGlTexture>,
    sampler: String,
}

impl Texture {
    /// Creates a new texture.
    pub fn new(sampler: String, texture: Rc<WebGlTexture>) -> Texture {
        Texture { sampler, texture }
    }

    /// Returns the texture object.
    pub fn texture(&self) -> &Rc<WebGlTexture> {
        &self.texture
    }

    /// Returns the sampler identifier.
    pub fn sampler(&self) -> &str {
        &self.sampler
    }
}

/// WebGL2 texture builder.
///
/// This builder object is used to create a new WebGL2 texture with a given set
/// of parameters.
pub struct TextureBuilder<'a> {
    engine: &'a mut RenderEngine,
    texture: Rc<WebGlTexture>,
}

impl TextureBuilder<'_> {
    pub(super) fn new(engine: &mut RenderEngine) -> Result<TextureBuilder<'_>, JsValue> {
        let texture = Rc::new(
            engine
                .gl
                .create_texture()
                .ok_or("failed to create texture")?,
        );
        engine.bind_texture(&texture);
        Ok(TextureBuilder { engine, texture })
    }

    /// Applies a parameter to the texture.
    pub fn set_parameter(self, parameter: TextureParameter) -> Self {
        self.engine.gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            parameter.parameter_code(),
            parameter.value_code(),
        );
        self
    }

    /// Builds and returns the new texture.
    pub fn build(self) -> Rc<WebGlTexture> {
        self.texture
    }
}

/// Texture parameter.
///
/// This enum describes all the parameters that can be applied to a WebGL2 texture.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TextureParameter {
    /// Magnification filter.
    MagFilter(TextureMagFilter),
    /// Minification filter.
    MinFilter(TextureMinFilter),
    /// Wrap-S.
    WrapS(TextureWrap),
    /// Wrap-T.
    WrapT(TextureWrap),
    /// Wrap-R.
    WrapR(TextureWrap),
}

impl TextureParameter {
    fn parameter_code(&self) -> u32 {
        match self {
            TextureParameter::MagFilter(_) => WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            TextureParameter::MinFilter(_) => WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            TextureParameter::WrapS(_) => WebGl2RenderingContext::TEXTURE_WRAP_S,
            TextureParameter::WrapT(_) => WebGl2RenderingContext::TEXTURE_WRAP_T,
            TextureParameter::WrapR(_) => WebGl2RenderingContext::TEXTURE_WRAP_R,
        }
    }

    fn value_code(&self) -> i32 {
        match *self {
            TextureParameter::MagFilter(a) => a as i32,
            TextureParameter::MinFilter(a) => a as i32,
            TextureParameter::WrapS(a) => a as i32,
            TextureParameter::WrapT(a) => a as i32,
            TextureParameter::WrapR(a) => a as i32,
        }
    }
}

/// Magnification filter.
///
/// This enum lists the magnification filters supported by WebGL2.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Default)]
#[repr(u32)]
pub enum TextureMagFilter {
    /// Linear filtering.
    #[default]
    Linear = WebGl2RenderingContext::LINEAR,
    /// Nearest-neighbor filtering.
    Nearest = WebGl2RenderingContext::NEAREST,
}

/// Minification filter.
///
/// This enum lists the minification filters supported by WebGL2.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Default)]
#[repr(u32)]
pub enum TextureMinFilter {
    /// Linear filtering.
    Linear = WebGl2RenderingContext::LINEAR,
    /// Nearest-neighbor filtering.
    Nearest = WebGl2RenderingContext::NEAREST,
    /// Nearest-neighbor filtering using the nearest mipmap.
    NearestMipmapNearest = WebGl2RenderingContext::NEAREST_MIPMAP_NEAREST,
    /// Linear filtering using the nearest mipmap.
    LinearMipmapNearest = WebGl2RenderingContext::LINEAR_MIPMAP_NEAREST,
    /// Nearest-neighbor filtering using linear interpolation between mipmaps.
    #[default]
    NearestMipmapLinear = WebGl2RenderingContext::NEAREST_MIPMAP_LINEAR,
    /// Linear filtering using linear interpolation between mipmaps.
    LinearMipmapLinear = WebGl2RenderingContext::LINEAR_MIPMAP_LINEAR,
}

/// Texture wrapping.
///
/// This enum lists the texture wrapping settings supported by WebGL2.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Default)]
#[repr(u32)]
pub enum TextureWrap {
    /// Repeat.
    #[default]
    Repeat = WebGl2RenderingContext::REPEAT,
    /// Clamp to edge.
    ClampToEdge = WebGl2RenderingContext::CLAMP_TO_EDGE,
    /// Mirrored repeat.
    MirroredRepeat = WebGl2RenderingContext::MIRRORED_REPEAT,
}

/// WebGL2 texture internal format.
///
/// This trait gives idiomatic Rust usage of WebGL2 texture formats. The trait
/// gives the WebGL2 constants that list the corresponding internal format and
/// format, and a Rust type that can be converted to a JS type, using the
/// [`ArrayView`] trait.
pub trait TextureInternalFormat {
    /// WebGL2 constant that indicates the internal format.
    const INTERNAL_FORMAT: i32;
    /// WebGL2 constant that indicates the format.
    const FORMAT: u32;
    /// Native Rust type corresponding to this format.
    type T: ArrayView;
}

macro_rules! new_format {
    ($doc:expr, $type:ident, $int:expr, $fmt:expr, $t:ty) => {
        #[doc = $doc]
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
        pub struct $type {}

        impl TextureInternalFormat for $type {
            const INTERNAL_FORMAT: i32 = ($int) as i32;
            const FORMAT: u32 = $fmt;
            type T = $t;
        }
    };
}

new_format!(
    r#"RGB texture internal format.

This uses `u8` as the native Rust type and the `RGB` WebGL2 format as
internal format and format."#,
    Rgb,
    WebGl2RenderingContext::RGB,
    WebGl2RenderingContext::RGB,
    u8
);
new_format!(
    r#"RGBA texture internal format.

This uses `u8` as the native Rust type and the `RGBA` WebGL2 format as
internal format and format."#,
    Rgba,
    WebGl2RenderingContext::RGBA,
    WebGl2RenderingContext::RGBA,
    u8
);
new_format!(
    r#"Luminance alpha texture internal format.

This uses `u8` as the native Rust type and the `LUMINANCE_ALPHA` WebGl2
format as internal format and format."#,
    LuminanceAlpha,
    WebGl2RenderingContext::LUMINANCE_ALPHA,
    WebGl2RenderingContext::LUMINANCE_ALPHA,
    u8
);
new_format!(
    r#"R16F texture internal format.

This uses `f32` as the native Rust type, the `R16F` WebGL2 format as
internal format, and the `RED` WebGL2 format as format."#,
    R16f,
    WebGl2RenderingContext::R16F,
    WebGl2RenderingContext::RED,
    f32
);

impl RenderEngine {
    /// Loads a texture with an image.
    ///
    /// The `image` is described by a slice of elements whose type is the native
    /// Rust type corresponding to the WebGL2 texture internal format `F`. Such
    /// internal format, and its corresponding format are used to load the image
    /// into the texture.
    ///
    /// The `width` and `height` parameters indicate the dimensions of the image
    /// in pixels.
    pub fn texture_image<F: TextureInternalFormat>(
        &mut self,
        texture: &Rc<WebGlTexture>,
        image: &[F::T],
        width: usize,
        height: usize,
    ) -> Result<(), JsValue> {
        self.bind_texture(texture);
        unsafe {
            let view = F::T::view(image);
            let level = 0;
            let border = 0;
            self.gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
		WebGl2RenderingContext::TEXTURE_2D,
		level,
		F::INTERNAL_FORMAT,
		width as i32,
		height as i32,
		border,
		F::FORMAT,
		F::T::GL_TYPE,
		Some(&view)
		    )?;
        };
        Ok(())
    }

    /// Loads a portion of a texture with an image.
    ///
    /// The `image` is described in the same way as in
    /// [`texture_image`](RenderEngine::texture_image), together with its
    /// corresponding `width` and `height` dimensions.
    ///
    /// The `xoffset` and `yoffset` give the offsets in pixels that indicate in
    /// which portion of the texture the image data should be loaded.
    pub fn texture_subimage<F: TextureInternalFormat>(
        &mut self,
        texture: &Rc<WebGlTexture>,
        image: &[F::T],
        xoffset: usize,
        yoffset: usize,
        width: usize,
        height: usize,
    ) -> Result<(), JsValue> {
        self.bind_texture(texture);
        unsafe {
            let view = F::T::view(image);
            let level = 0;
            self.gl
                .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                    WebGl2RenderingContext::TEXTURE_2D,
                    level,
                    xoffset as i32,
                    yoffset as i32,
                    width as i32,
                    height as i32,
                    F::FORMAT,
                    F::T::GL_TYPE,
                    Some(&view),
                )?;
        };
        Ok(())
    }

    pub(super) fn texture_from_text_render<F: TextureInternalFormat>(
        &mut self,
        texture: &Rc<WebGlTexture>,
    ) -> Result<(), JsValue> {
        self.bind_texture(texture);
        let level = 0;

        // See https://registry.khronos.org/webgl/specs/1.0/index.html#PIXEL_STORAGE_PARAMETERS
        //
        // UNPACK_PREMULTIPLY_ALPHA_WEBGL of type boolean
        //
        // If set, then during any subsequent calls to texImage2D or
        // texSubImage2D, the alpha channel of the source data, if present, is
        // multiplied into the color channels during the data transfer. The
        // initial value is false. Any non-zero value is interpreted as true.
        //
        // We do this here and un-do it afterwards because according to Firefox
        // "Alpha-premult and y-flip are deprecated for non-DOM Element uploads"
        // (which affects the textures loaded from a JS Array.
        self.gl
            .pixel_storei(WebGl2RenderingContext::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1);

        self.gl
            .tex_image_2d_with_u32_and_u32_and_html_canvas_element(
                WebGl2RenderingContext::TEXTURE_2D,
                level,
                F::INTERNAL_FORMAT,
                F::FORMAT,
                F::T::GL_TYPE,
                self.text_render.canvas(),
            )?;

        self.gl
            .pixel_storei(WebGl2RenderingContext::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 0);

        Ok(())
    }

    /// Generate mipmap.
    ///
    /// This function generates the mipmap from a given texture.
    pub fn generate_mipmap(&mut self, texture: &Rc<WebGlTexture>) {
        self.bind_texture(texture);
        self.gl.generate_mipmap(WebGl2RenderingContext::TEXTURE_2D);
    }
}
