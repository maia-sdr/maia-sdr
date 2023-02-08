use std::cell::Cell;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlUniformLocation};

/// WebGL2 uniform.
///
/// This associates an identifier for a WebGL2 uniform with a data value that
/// can be accessed and modified using inner mutability.
///
/// Usually, the type `T` would be [`Copy`].
pub struct Uniform<T> {
    name: String,
    data: Cell<T>,
}

impl<T> Uniform<T> {
    /// Creates a new WebGL2 uniform.
    ///
    /// The `name` corresponds to the identifier of the uniform, and `data`
    /// gives its initial value.
    pub fn new(name: String, data: T) -> Uniform<T> {
        Uniform {
            name,
            data: Cell::new(data),
        }
    }

    /// Returns the name (identifier) of the uniform.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Modifies the data of the uniform.
    ///
    /// This function sets the value of the uniform to `value`.
    pub fn set_data(&self, value: T) {
        self.data.set(value)
    }
}

impl<T: Copy> Uniform<T> {
    /// Returns the data of the uniform.
    ///
    /// This function returns a copy of the value of the uniform.
    pub fn get_data(&self) -> T {
        self.data.get()
    }
}

/// Trait that abstracts WebGL2 uniforms.
///
/// This trait is implemented by objects that represent a WebGL2 uniform and its
/// value, and which know how to set the value of the uniform if given a WebGL2
/// program with such uniform.
pub trait UniformValue {
    /// Set the value of the uniform.
    ///
    /// If the `program` contains the uniform represented by `self`, this
    /// function sets the value of the uniform to the value stored by `self`. If
    /// the program does not contain the uniform, this function does nothing.
    fn set_uniform(&self, gl: &WebGl2RenderingContext, program: &WebGlProgram);
}

impl<T: UniformType + Copy> UniformValue for Uniform<T> {
    fn set_uniform(&self, gl: &WebGl2RenderingContext, program: &WebGlProgram) {
        if let Some(location) = gl.get_uniform_location(program, self.name()) {
            self.get_data().uniform(gl, Some(&location))
        }
    }
}

/// Trait that links native Rust types with WebGL2 uniform types.
pub trait UniformType {
    /// Sets the value of the uniform.
    ///
    /// This function sets the value of the WebGL2 uniform in `location` to the
    /// value of `self` using one of the `uniform{1,2,3,4}{f,i,ui}` WebGL2
    /// functions as appropriate.
    fn uniform(&self, gl: &WebGl2RenderingContext, location: Option<&WebGlUniformLocation>);
}

macro_rules! impl_uniform {
    ($t:ty, $fun:ident, $sel:ident, $($things:expr),+) => {
        #[doc = concat!("Uniform type corresponding to `", stringify!($fun), "`.")]
	impl UniformType for $t {
	    fn uniform(&$sel, gl: &WebGl2RenderingContext, location: Option<&WebGlUniformLocation>) {
		gl.$fun(location, $($things,)+)
	    }
	}
    }
}

impl_uniform!(f32, uniform1f, self, *self);
impl_uniform!(i32, uniform1i, self, *self);
impl_uniform!(u32, uniform1ui, self, *self);
impl_uniform!((f32, f32), uniform2f, self, self.0, self.1);
impl_uniform!((i32, i32), uniform2i, self, self.0, self.1);
impl_uniform!((u32, u32), uniform2ui, self, self.0, self.1);
impl_uniform!((f32, f32, f32), uniform3f, self, self.0, self.1, self.2);
impl_uniform!((i32, i32, i32), uniform3i, self, self.0, self.1, self.2);
impl_uniform!((u32, u32, u32), uniform3ui, self, self.0, self.1, self.2);
impl_uniform!(
    (f32, f32, f32, f32),
    uniform4f,
    self,
    self.0,
    self.1,
    self.2,
    self.3
);
impl_uniform!(
    (i32, i32, i32, i32),
    uniform4i,
    self,
    self.0,
    self.1,
    self.2,
    self.3
);
impl_uniform!(
    (u32, u32, u32, u32),
    uniform4ui,
    self,
    self.0,
    self.1,
    self.2,
    self.3
);
