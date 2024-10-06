//! Input fields.
//!
//! This module defines the trait [`InputElement`] and some structs which
//! implement it. They are used to manipulate HTML input elements in an
//! idiomatic way from Rust.

use std::ops::Deref;
use std::rc::Rc;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlSpanElement};

/// HTML input element.
///
/// This trait models an abstract HTML input element whose value can can be get
/// and setted. The `type T` corresponds to the Rust type of the value of the
/// input element. The generic type `E` corresponds to the HTML input element
/// type from [`web_sys`].
///
/// This trait is intended to be implement for types that behave like a wrapper
/// over [`Rc<E>`].
pub trait InputElement<E>: Deref<Target = E> + From<Rc<E>> + Clone {
    /// Rust type corresponding to the value of the input element.
    type T;

    /// Gets the value of the input element.
    ///
    /// This function attempts to convert the contents of the input element into
    /// a Rust value of type `T`. If the conversion is successful, the value is
    /// returned inside a `Some`. If the format of the contents of the input
    /// element is not correct and a Rust value cannot be obtained, `None` is
    /// returned.
    fn get(&self) -> Option<Self::T>;

    /// Sets the value of the input element.
    ///
    /// This function sets the contents of the input element to match that of
    /// the Rust value given in the `value` argument. After the input element is
    /// set, its contents might not match exactly the Rust value, for instance
    /// due to rounding to represent a floating point number with fewer decimals
    /// in a text field.
    fn set(&self, value: &Self::T);
}

/// Number presentation.
///
/// This trait defines how to format numbers in a text field.
///
/// The `SCALE` corresponds to the units that are used in the text field. The
/// value of the text field is multiplied by `SCALE` to give a Rust value, and
/// likewise a Rust value is divided by the `SCALE` to compute the value that is
/// displayed in the text field. For example, if the Rust value has units of Hz
/// but the text field displays the value in units of MHz, the `SCALE` is `1e6`.
///
/// The `RESOLUTION` is used to limit the number of digits that are used in the
/// text field when displaying the value. This is done by rounding the Rust
/// value to the nearest multiple of `RESOLUTION` before displaying it on the
/// text field. For example, if the Rust value has units of Hz but the text
/// field has a resolution of kHz, the `RESOLUTION` is `Some(1e3)`. If the
/// `RESOLUTION` is `None`, then no rounding is done and the exact Rust value is
/// displayed in the field.
pub trait NumberPresentation: Clone {
    /// Scale of the text field.
    ///
    /// This indicates how many Rust value units a text field unit consists of.
    const SCALE: f64;
    /// Resolution of the text field.
    ///
    /// This indicates how many Rust value units correspond to the least
    /// significant digit of the text field.
    const RESOLUTION: Option<f64>;
}

macro_rules! presentation {
    ($t:ident, $scale:expr, $resolution:expr, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
        pub struct $t {}

        impl NumberPresentation for $t {
            const SCALE: f64 = $scale;
            const RESOLUTION: Option<f64> = $resolution;
        }
    };
}

presentation!(
    DefaultPresentation,
    1.0,
    None,
    "Default number presentation.\n\n\
     The Rust value and the text field use the same units, \
     and there is no resolution rounding."
);
presentation!(
    IntegerPresentation,
    1.0,
    Some(1.0),
    "Integer number presentation.\n\n\
     The Rust value and the text field use the same units, \
     but the text field is rounded to always display an integer."
);
presentation!(
    MHzPresentation,
    1e6,
    Some(1e3),
    "MHz presentation.\n\n\
     The Rust value has units of Hz, and the text field has units of MHz. \
     The text field is rounded to the nearest integer kHz (three decimal digits).\n\n\
     This is often used to display frequencies such as LO frequency in the UI."
);
presentation!(
    KHzPresentation,
    1e3,
    Some(1.0),
    "kHz presentation.\n\n\
     The Rust value has units of Hz, and the text field has units of kHz. \
     The text field is rounded to the nearest integer Hz (three decimal digits).\n\n\
     This is often used to display small frequency up to a few MHz in the UI."
);

/// Number input.
///
/// This struct behaves as a wrapper over `Rc<HtmlInputElement>` and implements
/// the [`InputElement`] trait. The type generic `T` corresponds to the Rust
/// type used for the value of the input element. The type generic `P` is the
/// [`NumberPresentation`] that is used.
#[derive(Clone)]
pub struct NumberInput<T, P = DefaultPresentation> {
    element: Rc<HtmlInputElement>,
    _phantom: std::marker::PhantomData<(T, P)>,
}

impl<T, P> From<Rc<HtmlInputElement>> for NumberInput<T, P> {
    fn from(element: Rc<HtmlInputElement>) -> NumberInput<T, P> {
        NumberInput {
            element,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, P> Deref for NumberInput<T, P> {
    type Target = HtmlInputElement;

    fn deref(&self) -> &HtmlInputElement {
        &self.element
    }
}

macro_rules! number_input_set {
    ($t:ty) => {
        fn set(&self, value: &$t) {
            let value = value.clone() as f64;
            let value = if let Some(resolution) = P::RESOLUTION {
                (value / resolution).round() * resolution
            } else {
                value
            };
            self.element.set_value_as_number(value / P::SCALE);
        }
    };
}

macro_rules! number_input_int {
    ($($t:ty),*) => {
        $(
            impl<P: NumberPresentation> InputElement<HtmlInputElement> for NumberInput<$t, P> {
                type T = $t;

                fn get(&self) -> Option<$t> {
                    let x = (self.element.value_as_number() * P::SCALE).round() as i64;
                    <$t>::try_from(x).ok()
                }

                number_input_set!($t);
            }
        )*
    }
}

macro_rules! number_input_float {
    ($($t:ty),*) => {
        $(
            impl<P: NumberPresentation> InputElement<HtmlInputElement> for NumberInput<$t, P> {
                type T = $t;

                fn get(&self) -> Option<$t> {
                    Some((self.element.value_as_number() * P::SCALE) as $t)
                }

                number_input_set!($t);
            }
        )*
    }
}

number_input_int!(u64, u32);
number_input_float!(f64, f32);

/// Number input.
///
/// This struct behaves as a wrapper over `Rc<HtmlSpanElement>` and implements
/// the [`InputElement`] trait. The type generic `T` corresponds to the Rust
/// type used for the value of the input element. The type generic `P` is the
/// [`NumberPresentation`] that is used.
#[derive(Clone)]
pub struct NumberSpan<T, P = DefaultPresentation> {
    element: Rc<HtmlSpanElement>,
    _phantom: std::marker::PhantomData<(T, P)>,
}

impl<T, P> From<Rc<HtmlSpanElement>> for NumberSpan<T, P> {
    fn from(element: Rc<HtmlSpanElement>) -> NumberSpan<T, P> {
        NumberSpan {
            element,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, P> Deref for NumberSpan<T, P> {
    type Target = HtmlSpanElement;

    fn deref(&self) -> &HtmlSpanElement {
        &self.element
    }
}

macro_rules! number_span_set {
    ($t:ty) => {
        fn set(&self, value: &$t) {
            let value = value.clone() as f64;
            let value = if let Some(resolution) = P::RESOLUTION {
                (value / resolution).round() * resolution
            } else {
                value
            };
            let value = value / P::SCALE;
            self.element.set_text_content(Some(&value.to_string()));
        }
    };
}

macro_rules! number_span_int {
    ($($t:ty),*) => {
        $(
            impl<P: NumberPresentation> InputElement<HtmlSpanElement> for NumberSpan<$t, P> {
                type T = $t;

                fn get(&self) -> Option<$t> {
                    let value: f64 = self.element.text_content()?.parse().ok()?;
                    let value = (value * P::SCALE).round() as i64;
                    <$t>::try_from(value).ok()
                }

                number_span_set!($t);
            }
        )*
    }
}

macro_rules! number_span_float {
    ($($t:ty),*) => {
        $(
            impl<P: NumberPresentation> InputElement<HtmlSpanElement> for NumberSpan<$t, P> {
                type T = $t;

                fn get(&self) -> Option<$t> {
                    let value: f64 = self.element.text_content()?.parse().ok()?;
                    Some((value * P::SCALE) as $t)
                }

                number_span_set!($t);
            }
        )*
    }
}

number_span_int!(u64, u32);
number_span_float!(f64, f32);

/// Text input.
///
/// This struct behaves as a wrapper over `Rc<HtmlInputElement>` and implements
/// the [`InputElement`] trait. It gives access to the text field contents as
/// Rust [`String`]s.
#[derive(Clone)]
pub struct TextInput {
    element: Rc<HtmlInputElement>,
}

impl From<Rc<HtmlInputElement>> for TextInput {
    fn from(element: Rc<HtmlInputElement>) -> TextInput {
        TextInput { element }
    }
}

impl Deref for TextInput {
    type Target = HtmlInputElement;

    fn deref(&self) -> &HtmlInputElement {
        &self.element
    }
}

impl InputElement<HtmlInputElement> for TextInput {
    type T = String;

    fn get(&self) -> Option<String> {
        Some(self.element.value())
    }

    fn set(&self, value: &String) {
        self.element.set_value(value)
    }
}

/// Enum input.
///
/// This struct behaves as a wrapper over `Rc<HtmlSelectElement>` and implements
/// the [`InputElement`] trait. It maps an HTML select element to a Rust `enum`
/// type `E`. The implementations of [`FromStr`](std::str::FromStr) and
/// [`ToString`] of the `enum` `E` are used to convert between Rust values and
/// the strings used by the HTML select.
pub struct EnumInput<E> {
    element: Rc<HtmlSelectElement>,
    _phantom: std::marker::PhantomData<E>,
}

impl<E> Clone for EnumInput<E> {
    fn clone(&self) -> Self {
        EnumInput {
            element: Rc::clone(&self.element),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E> From<Rc<HtmlSelectElement>> for EnumInput<E> {
    fn from(element: Rc<HtmlSelectElement>) -> EnumInput<E> {
        EnumInput {
            element,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E> Deref for EnumInput<E> {
    type Target = HtmlSelectElement;

    fn deref(&self) -> &HtmlSelectElement {
        &self.element
    }
}

impl<E: std::str::FromStr + ToString> InputElement<HtmlSelectElement> for EnumInput<E> {
    type T = E;

    fn get(&self) -> Option<E> {
        Some(match self.element.value().parse() {
            Ok(x) => x,
            Err(_) => panic!("could not parse HtmlSelectElement value"),
        })
    }

    fn set(&self, value: &E) {
        self.element.set_value(&value.to_string());
    }
}

/// Checkbox input.
///
/// This struct behaves as a wrapper over `Rc<HtmlInputElement>` and implements
/// the [`InputElement`] trait. It should be used with checkbox input elements,
/// and it gives access to the checkbox value as a Rust [`bool`].
#[derive(Clone)]
pub struct CheckboxInput {
    element: Rc<HtmlInputElement>,
}

impl From<Rc<HtmlInputElement>> for CheckboxInput {
    fn from(element: Rc<HtmlInputElement>) -> CheckboxInput {
        CheckboxInput { element }
    }
}

impl Deref for CheckboxInput {
    type Target = HtmlInputElement;

    fn deref(&self) -> &HtmlInputElement {
        &self.element
    }
}

impl InputElement<HtmlInputElement> for CheckboxInput {
    type T = bool;

    fn get(&self) -> Option<bool> {
        Some(self.element.checked())
    }

    fn set(&self, &value: &bool) {
        self.element.set_checked(value)
    }
}
