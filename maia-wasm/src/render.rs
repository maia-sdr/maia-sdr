//! Render engine.
//!
//! This module implements a WebGL2 render engine that is used to render the
//! waterfall. The render engine is organized around a [`RenderEngine`] to which
//! [`RenderObject`]'s are added.

use std::cell::Cell;
use std::rc::Rc;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlVertexArrayObject};

pub use engine::{
    CanvasDims, RenderEngine, TextsDimensions, Texture, TextureBuilder, TextureInternalFormat,
    TextureMagFilter, TextureMinFilter, TextureParameter, TextureWrap, VaoBuilder,
};
pub use uniform::{Uniform, UniformType, UniformValue};

mod engine;
mod uniform;

/// Render object.
///
/// Render objects describe a scene, and are added to the render engine using
/// [`RenderEngine::add_object`]. Render objects are drawn using the
/// `drawElements()` WebGL2 function.
pub struct RenderObject {
    /// Controls whether the object is enabled.
    ///
    /// If `enabled` is `false`, the object is not rendered.
    pub enabled: Rc<Cell<bool>>,
    /// WebGL2 program used to render the object.
    pub program: Rc<WebGlProgram>,
    /// VAO storing all the vertex arrays for the object.
    ///
    /// This VAO contains the element array as well as any arrays needed for the
    /// vertex shader.
    pub vao: Rc<WebGlVertexArrayObject>,
    /// Draw mode for the object.
    pub draw_mode: DrawMode,
    /// Number of elements to draw.
    ///
    /// This parameter is passed to `drawElements()`.
    pub draw_num_indices: Rc<Cell<u32>>,
    /// Offset in the element array buffer to use for drawing.
    ///
    /// Unlike in the `drawElements()` WebGL2 function, this field uses units of
    /// elements rather than bytes. The value is converted to bytes and passed
    /// to `drawElements()`.
    pub draw_offset_elements: Rc<Cell<usize>>,
    /// Uniforms used by the object.
    pub uniforms: Box<[Rc<dyn UniformValue>]>,
    /// Textures used by the object.
    pub textures: Box<[Texture]>,
}

/// Draw mode.
///
/// This enum lists the draw modes supported by WebGL2.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum DrawMode {
    /// Draw as points.
    Points = WebGl2RenderingContext::POINTS,
    /// Draw as a line strip.
    LineStrip = WebGl2RenderingContext::LINE_STRIP,
    /// Draw as a line loop.
    LineLoop = WebGl2RenderingContext::LINE_LOOP,
    /// Draw as lines.
    Lines = WebGl2RenderingContext::LINES,
    /// Draw as a triangle strip.
    TriangleStrip = WebGl2RenderingContext::TRIANGLE_STRIP,
    /// Draw as a triangle fan.
    TriangleFan = WebGl2RenderingContext::TRIANGLE_FAN,
    /// Draw as triangles.
    Triangles = WebGl2RenderingContext::TRIANGLES,
}

/// WebGL2 shader program source.
///
/// This contains the source for the vertex and fragment shaders of a WebGL2
/// program that is to be compiled.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ProgramSource<'a> {
    /// Source for the vertex shader.
    pub vertex_shader: &'a str,
    /// Source for the fragment shader.
    pub fragment_shader: &'a str,
}

pub mod texture_formats {
    //! WebGL2 internal texture formats.
    //!
    //! This module defines the formats that can be used with the
    //! [`TextureInternalFormat`](super::TextureInternalFormat) trait.

    pub use super::engine::{LuminanceAlpha, R16f, Rgb, Rgba};
}
