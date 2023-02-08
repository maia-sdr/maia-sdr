use std::ops::Deref;
use std::rc::Rc;
use web_sys::{HtmlInputElement, HtmlSelectElement};

pub trait InputElement<E>: Deref<Target = E> + From<Rc<E>> + Clone {
    type T;

    fn get(&self) -> Option<Self::T>;
    fn set(&self, value: &Self::T);
}

pub trait NumberPresentation: Clone {
    const SCALE: f64;
    const RESOLUTION: Option<f64>;
}

macro_rules! presentation {
    ($t:ident, $scale:expr, $resolution:expr) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
        pub struct $t {}

        impl NumberPresentation for $t {
            const SCALE: f64 = $scale;
            const RESOLUTION: Option<f64> = $resolution;
        }
    };
}

presentation!(DefaultPresentation, 1.0, None);
presentation!(IntegerPresentation, 1.0, Some(1.0));
presentation!(MHzPresentation, 1e6, Some(1e3));

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
            impl<P> InputElement<HtmlInputElement> for NumberInput<$t, P>
            where
                P: NumberPresentation,
            {
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

impl<E: std::str::FromStr + std::string::ToString> InputElement<HtmlSelectElement>
    for EnumInput<E>
{
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
