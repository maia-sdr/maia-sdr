use crate::array_view::ArrayView;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlVertexArrayObject};

use super::RenderEngine;

/// WebGL2 VAO builder.
///
/// This builder object is used to create a new Vertex Array Object or modify an
/// existing one.
pub struct VaoBuilder<'a> {
    engine: &'a mut RenderEngine,
    vao: Rc<WebGlVertexArrayObject>,
}

impl VaoBuilder<'_> {
    pub(super) fn new(engine: &mut RenderEngine) -> Result<VaoBuilder<'_>, JsValue> {
        let vao = Rc::new(
            engine
                .gl
                .create_vertex_array()
                .ok_or("failed to create VAO")?,
        );
        Ok(Self::modify_vao(engine, vao))
    }

    pub(super) fn modify_vao(
        engine: &mut RenderEngine,
        vao: Rc<WebGlVertexArrayObject>,
    ) -> VaoBuilder<'_> {
        engine.bind_vertex_array(&vao);
        VaoBuilder { engine, vao }
    }

    /// Adds or modifies an array buffer to the VAO.
    ///
    /// This function creates a WebGL2 buffer, fills it with the array
    /// `contents`, and associates it with the VAO being built or modified,
    /// associating it to a given `attribute` in a WebGL2 `program`.
    pub fn create_array_buffer<T: ArrayView>(
        self,
        program: &WebGlProgram,
        attribute: &str,
        size: i32,
        contents: &[T],
    ) -> Result<Self, JsValue> {
        let attribute_location = match self.engine.gl.get_attrib_location(program, attribute) {
            x if x >= 0 => Ok(x as u32),
            _ => Err("failed to get attribute location"),
        }?;
        self.engine
            .gl
            .enable_vertex_attrib_array(attribute_location);
        self.create_and_fill_buffer(WebGl2RenderingContext::ARRAY_BUFFER, contents)?;
        let normalized = false;
        let stride = 0;
        let offset = 0;
        self.engine.gl.vertex_attrib_pointer_with_i32(
            attribute_location,
            size,
            T::GL_TYPE,
            normalized,
            stride,
            offset,
        );
        Ok(self)
    }

    /// Adds or modifies an element array buffer to the VAO.
    ///
    /// This function creates a WebGL2 buffer, fills it with the array
    /// `contents`, and associates it with the VAO as an element array buffer.
    pub fn create_element_array_buffer(self, contents: &[u16]) -> Result<Self, JsValue> {
        self.create_and_fill_buffer(WebGl2RenderingContext::ELEMENT_ARRAY_BUFFER, contents)?;
        Ok(self)
    }

    fn create_and_fill_buffer<T: ArrayView>(
        &self,
        target: u32,
        contents: &[T],
    ) -> Result<WebGlBuffer, JsValue> {
        let buffer = self
            .engine
            .gl
            .create_buffer()
            .ok_or("failed to create_buffer")?;
        self.engine.gl.bind_buffer(target, Some(&buffer));
        unsafe {
            let view = T::view(contents);
            self.engine.gl.buffer_data_with_array_buffer_view(
                target,
                &view,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }
        Ok(buffer)
    }

    /// Builds the VAO.
    ///
    /// Finishes the construction of the VAO, returning the VAO object.
    pub fn build(self) -> Rc<WebGlVertexArrayObject> {
        self.vao
    }
}
