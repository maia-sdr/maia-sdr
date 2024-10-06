/// UI macro: define UI elements.
///
/// This macro is used as a convenience to define a `struct` called `Elements`
/// that contains all the UI HTML elements. A constructor
/// `Elements::new(document: &Document) -> Result<Elements, JsValue>` is also
/// defined by this macro.
///
/// Each member in `Elements` is defined by its HTML id (which also gives the
/// name of the member), the [`web_sys`] type of the corresponding HTML element,
/// and the type to which it is transformed as a member of `Elements`. The
/// latter is either an [`InputElement`](crate::ui::input::InputElement) or an
/// [`Rc`](std::rc::Rc) wrapping the HTML element type. See the example below
/// for details about the syntax.
///
/// # Example
///
/// ```
/// use maia_wasm::{ui::input::{CheckboxInput, MHzPresentation, NumberInput},
///                 ui_elements};
/// use std::rc::Rc;
/// use web_sys::{Document, HtmlButtonElement, HtmlInputElement};
///
/// ui_elements! {
///     my_checkbox: HtmlInputElement => CheckboxInput,
///     my_button: HtmlButtonElement => Rc<HtmlButtonElement>,
///     my_frequency: HtmlInputElement => NumberInput<f32, MHzPresentation>,
/// }
///
/// fn main() -> Result<(), wasm_bindgen::JsValue> {
///     # // do not run the rest of the code during testing, as it will fail,
///     # // but still check that it compiles
///     # return Ok(());
///     let (_, document) = maia_wasm::get_window_and_document()?;
///     let elements = Elements::new(&document)?;
///
///     // elements.my_checkbox is a CheckboxInput
///     // elements.my_button is an Rc<HtmlButtonElement>
///     // etc.
///
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! ui_elements {
    {$($element:ident : $base_ty:ty => $transform_ty:ty),* $(,)?} => {
        #[derive(Clone)]
        struct Elements {
            $(
                $element: $transform_ty,
            )*
        }

        impl Elements {
            fn new(document: &web_sys::Document) -> Result<Elements, wasm_bindgen::JsValue> {
                use wasm_bindgen::JsCast;
                Ok(Elements {
                    $(
                        $element: std::rc::Rc::new(document
                                          .get_element_by_id(stringify!($element))
                                          .ok_or(concat!("failed to find ",
                                                         stringify!($element),
                                                         " element"))?
                                          .dyn_into::<$base_ty>()?)
                            .into(),
                    )*
                })
            }
        }
    }
}

/// UI macro: implement an `onchange` method that calls an `apply` function.
///
/// This macro implements an `onchange` method for an element called
/// `name`. The method is called `name_onchange`. It uses
/// [`InputElement::get`](crate::ui::input::InputElement::get) to obtain the
/// value of the element, updates the corresponding preferences item with this
/// value, and calls a `name_apply` with this value. The `name_apply` method
/// is defined by the user.
///
/// See [`onchange_apply_noprefs`](crate::onchange_apply_noprefs) for an example
/// of usage.
#[macro_export]
macro_rules! onchange_apply {
    ($($name:ident),*) => {
        paste::paste! {
            $(
                fn [<$name _onchange>](&self) -> wasm_bindgen::closure::Closure<dyn Fn()> {
                    use $crate::ui::input::InputElement;
                    let ui = self.clone();
                    wasm_bindgen::closure::Closure::new(move || {
                        let element = &ui.elements.$name;
                        if !element.report_validity() {
                            return;
                        }
                        if let Some(value) = element.get() {
                            ui.[<$name _apply>](value);
                            // try_borrow_mut prevents trying to update the
                            // preferences as a consequence of the
                            // Preferences::apply_client calling this closure
                            if let Ok(mut p) = ui.preferences.try_borrow_mut() {
                                if let Err(e) = p.[<update_ $name>](&value) {
                                    web_sys::console::error_1(&e);
                                }
                            }
                        } else {
                            ui.window
                                .alert_with_message(concat!("Invalid value for ",
                                                            stringify!($name)))
                                .unwrap();
                        }
                    })
                }
            )*
        }
    }
}

/// UI macro: implement an `onchange` method that calls an `apply` method without
/// updating the preferences.
///
/// This macro is similar to [`onchange_apply`], but it does not update the
/// preferences.
///
/// # Example
///
/// ```
/// use maia_wasm::{onchange_apply_noprefs, set_on, ui_elements,
///                 ui::input::TextInput};
/// use std::rc::Rc;
/// use wasm_bindgen::JsValue;
/// use web_sys::{Document, HtmlInputElement, Window};
///
/// #[derive(Clone)]
/// struct Ui {
///     window: Rc<Window>,
///     elements: Elements,
/// }
///
/// ui_elements! {
///     my_text_field: HtmlInputElement => TextInput,
/// }
///
/// impl Ui {
///     fn new(window: Rc<Window>, document: &Document) -> Result<Ui, JsValue> {
///         let elements = Elements::new(document)?;
///         let ui = Ui { window, elements };
///         ui.set_callbacks();
///         Ok(ui)
///     }
///
///     fn set_callbacks(&self) -> Result<(), JsValue> {
///         set_on!(change, self, my_text_field);
///         Ok(())
///     }
///
///     onchange_apply_noprefs!(my_text_field);
///
///     fn my_text_field_apply(&self, value: String) {
///         // do something with the value
///         self.window.alert_with_message(&format!("got my_text_field = {value}"));
///     }
/// }
/// ```
#[macro_export]
macro_rules! onchange_apply_noprefs {
    ($($name:ident),*) => {
        paste::paste! {
            $(
                fn [<$name _onchange>](&self) -> wasm_bindgen::closure::Closure<dyn Fn()> {
                    use $crate::ui::input::InputElement;
                    let ui = self.clone();
                    wasm_bindgen::closure::Closure::new(move || {
                        let element = &ui.elements.$name;
                        if !element.report_validity() {
                            return;
                        }
                        if let Some(value) = element.get() {
                            ui.[<$name _apply>](value);
                        } else {
                            ui.window
                                .alert_with_message(concat!("Invalid value for ",
                                                            stringify!($name)))
                                .unwrap();
                        }
                    })
                }
            )*
        }
    }
}

/// UI macro: set event callback for UI elements.
///
/// Given an `event` (for instance `change` or `click`) and assuming that there
/// are methods called `element_onevent` for each element, this macro sets the
/// event callbacks of each of the elements to the closures returned by these
/// methods.
///
/// # Example
///
/// ```
/// use maia_wasm::{onchange_apply_noprefs, set_on, ui_elements,
///                 ui::input::{InputElement, TextInput}};
/// use std::rc::Rc;
/// use wasm_bindgen::{closure::Closure, JsCast, JsValue};
/// use web_sys::{Document, HtmlButtonElement, HtmlInputElement, Window};
///
/// #[derive(Clone)]
/// struct Ui {
///     window: Rc<Window>,
///     elements: Elements,
/// }
///
/// ui_elements! {
///     my_text_field_a: HtmlInputElement => TextInput,
///     my_text_field_b: HtmlInputElement => TextInput,
///     my_button_a: HtmlButtonElement => Rc<HtmlButtonElement>,
///     my_button_b: HtmlButtonElement => Rc<HtmlButtonElement>,
/// }
///
/// impl Ui {
///     fn new(window: Rc<Window>, document: &Document) -> Result<Ui, JsValue> {
///         let elements = Elements::new(document)?;
///         let ui = Ui { window, elements };
///         ui.set_callbacks();
///         Ok(ui)
///     }
///
///     fn set_callbacks(&self) -> Result<(), JsValue> {
///         set_on!(change, self, my_text_field_a, my_text_field_b);
///         set_on!(click, self, my_button_a, my_button_b);
///         Ok(())
///     }
///
///    fn my_text_field_a_onchange(&self) -> Closure<dyn Fn()> {
///        let element = self.elements.my_text_field_a.clone();
///        let window = self.window.clone();
///        Closure::new(move || {
///            if let Some(text) = element.get() {
///                window.alert_with_message(&format!("my_text_field_a changed: value = {text}"));
///            }
///        })
///    }
///
///    fn my_text_field_b_onchange(&self) -> Closure<dyn Fn()> {
///        let element = self.elements.my_text_field_b.clone();
///        let window = self.window.clone();
///        Closure::new(move || {
///            if let Some(text) = element.get() {
///                window.alert_with_message(&format!("my_text_field_b changed: value = {text}"));
///            }
///        })
///    }
///
///    fn my_button_a_onclick(&self) -> Closure<dyn Fn()> {
///        let window = self.window.clone();
///        Closure::new(move || {
///            window.alert_with_message("my_button_a has been clicked");
///        })
///    }
///
///    fn my_button_b_onclick(&self) -> Closure<dyn Fn()> {
///        let window = self.window.clone();
///        Closure::new(move || {
///            window.alert_with_message("my_button_b has been clicked");
///        })
///    }
/// }
/// ```
#[macro_export]
macro_rules! set_on {
    ($event:ident, $self:expr, $($element:ident),*) => {
        paste::paste! {
            $(
                $self.elements.$element.[<set_on $event>](Some(
                    wasm_bindgen::JsCast::unchecked_ref(
                        &$self.[<$element _on $event>]()
                        .into_js_value())
                ));
            )*
        }
    }
}

// This is called by impl_patch and impl_put and not to be called directly.
#[doc(hidden)]
#[macro_export]
macro_rules! impl_request {
    ($name:ident, $request_json:ty, $get_json:ty, $url:expr, $method_ident:ident, $method:expr) => {
        paste::paste! {
            async fn [<$method_ident _ $name>](&self, json: &$request_json) -> Result<$get_json, $crate::ui::request::RequestError> {
                use wasm_bindgen::JsCast;
                let method = $method;
                let request = $crate::ui::request::json_request($url, json, method)?;
                let response = wasm_bindgen_futures::JsFuture::from(self.window.fetch_with_request(&request))
                    .await?
                    .dyn_into::<web_sys::Response>()?;
                if !response.ok() {
                    let status = response.status();
                    let error: maia_json::Error = $crate::ui::request::response_to_json(&response).await?;
                    match error.suggested_action {
                        maia_json::ErrorAction::Ignore => {}
                        maia_json::ErrorAction::Log =>
                            web_sys::console::error_1(&format!(
                                "{method} request failed with HTTP code {status}. \
                                 Error description: {}", error.error_description).into()),
                        maia_json::ErrorAction::Alert => {
                            web_sys::console::error_1(&format!(
                                "{method} request failed with HTTP code {status}. \
                                 UI alert suggested. Error description: {}", error.error_description).into());
                            self.alert(&error.error_description)?;
                        }
                    }
                    return Err($crate::ui::request::RequestError::RequestFailed(error));
                }
                Ok($crate::ui::request::response_to_json(&response).await?)
            }
        }
    };
}

/// UI macro: implements a method to send a PATCH request.
///
/// Given a `name`, this macro implements a `patch_name` method that sends a
/// PATCH request to a `url`. The response of the PATCH is then parsed as JSON,
/// and the resulting Rust value is returned.
///
/// The patch method signature is `async fn patch_name(&self, json:
/// &$request_json) -> Result<$get_json, RequestError>`.
///
/// # Example
///
/// ```
/// use maia_wasm::{impl_patch, ui::request::ignore_request_failed};
/// use serde::{Deserialize, Serialize};
/// use std::rc::Rc;
/// use wasm_bindgen::JsValue;
/// use web_sys::Window;
///
/// // An object defined by a REST API. This corresponds to the GET method.
/// #[derive(Debug, Serialize, Deserialize)]
/// struct MyObject {
///     my_value: u64,
/// }
///
/// // An object defined by a REST API. This corresponds to the PATCH method.
/// #[derive(Debug, Serialize, Deserialize)]
/// struct PatchMyObject {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     my_value: Option<u64>,
/// }
///
/// #[derive(Clone)]
/// struct Ui {
///     window: Rc<Window>,
/// }
///
/// impl Ui {
///     fn new(window: Rc<Window>) -> Ui {
///         Ui { window }
///     }
///
///     // it is necessary to define an alert method like this
///     // to show errors to the user
///     fn alert(&self, message: &str) -> Result<(), JsValue> {
///         self.window.alert_with_message(message);
///         Ok(())
///     }
///
///     impl_patch!(my_object, PatchMyObject, MyObject, "/my_object");
/// }
///
/// async fn example() -> Result<(), JsValue> {
///     # // do not run the rest of the code during testing, as it will fail,
///     # // but still check that it compiles
///     # return Ok(());
///     let (window, _) = maia_wasm::get_window_and_document()?;
///     let ui = Ui::new(Rc::clone(&window));
///
///     let patch = PatchMyObject { my_value: Some(42) };
///     // server errors are already handled by patch_my_object by calling
///     // Ui::alert, so we can ignore them here.
///     if let Some(result) = ignore_request_failed(ui.patch_my_object(&patch).await)? {
///         window.alert_with_message(&format!("request result: {result:?}"));
///     }
///
///     Ok(())
/// }
#[macro_export]
macro_rules! impl_patch {
    ($name:ident, $patch_json:ty, $get_json:ty, $url:expr) => {
        $crate::impl_request!($name, $patch_json, $get_json, $url, patch, "PATCH");
    };
}

/// UI macro: implements a method to send a PUT request.
///
/// Given a `name`, this macro implements a `put_name` method that sends a
/// PUT request to a `url`. The response of the PUT is then parsed as JSON,
/// and the resulting Rust value is returned.
///
/// The patch method signature is `async fn put_name(&self, json:
/// &$request_json) -> Result<$get_json, RequestError>`.
///
/// This macro is very similar to [`impl_patch`]. See its documentation for an
/// example.
#[macro_export]
macro_rules! impl_put {
    ($name:ident, $put_json:ty, $get_json:ty, $url:expr) => {
        $crate::impl_request!($name, $put_json, $get_json, $url, put, "PUT");
    };
}

// This macro is not to be called directly by the user. It must only be called
// through impl_update_elements and similar macros.
#[doc(hidden)]
#[macro_export]
macro_rules! set_values_if_inactive {
    ($self:expr, $source:expr, $section:ident, $($element:ident),*) => {
        use $crate::ui::{active::IsElementActive, input::InputElement};
        let mut preferences = $self.preferences.borrow_mut();
        paste::paste!{
            $(
                // A checkbox HtmlInputElement is always considered inactive,
                // because the user interaction with it is limited to clicking
                // (rather than typing). Therefore, we update it regardless of
                // whether it has focus.
                if !$self.document.is_element_active(stringify!([<$section _ $element>]))
                    || std::any::Any::type_id(&$self.elements.[<$section _ $element>])
                    == std::any::TypeId::of::<$crate::ui::input::CheckboxInput>() {
                    $self.elements.[<$section _ $element>].set(&$source.$element);
                }
                if let Err(e) = preferences.[<update_ $section _ $element>](&$source.$element) {
                    web_sys::console::error_1(&e);
                }
            )*
        }
    }
}

// This macro is not to be called directly by the user. It must only be called
// through impl_update_elements and similar macros.
#[doc(hidden)]
#[macro_export]
macro_rules! set_values {
    ($self:expr, $source:expr, $section:ident, $($element:ident),*) => {
        use $crate::ui::input::InputElement;
        let mut preferences = $self.preferences.borrow_mut();
        paste::paste! {
            $(
                $self.elements.[<$section _ $element>].set(&$source.$element);
                if let Err(e) = preferences.[<update_ $section _ $element>](&$source.$element) {
                    web_sys::console::error_1(&e);
                }
            )*
        }
    }
}

/// UI macro: implements methods to update UI elements.
///
/// Given a `name`, this macro implements the methods
/// `update_name_inactive_elements` and `update_name_all_elements`. Both methods
/// have the signature `fn ...(&self, json: &$json) -> Result<(), JsValue>`.
///
/// These methods are intended to be called when a JSON response has been
/// received, in order to keep the values of the UI elements synchronized with
/// their server-side values.
///
/// The difference between the `_inactive_elements` and `_all_elements` methods
/// is that the former does not update active elements, in order to avoid
/// overriding the input that the user might be typing in a field.
///
/// The functions call a `post_update_name_elements` method with the same
/// signature, which can implement any custom functionality. If no custom
/// functionality is needed, a dummy method that does nothing can be implemented
/// with the [`impl_post_update_noop`](crate::impl_post_update_noop) macro.
///
/// # Example
///
/// ```
/// use maia_wasm::{impl_dummy_preferences, impl_post_update_noop, impl_update_elements,
///                 ui_elements, ui::input::NumberInput};
/// use serde::{Deserialize, Serialize};
/// use std::{cell::RefCell, rc::Rc};
/// use wasm_bindgen::JsValue;
/// use web_sys::{Document, HtmlInputElement, Window};
///
/// // An object defined by a REST API. This corresponds to the GET method.
/// #[derive(Debug, Serialize, Deserialize)]
/// struct MyObject {
///     my_integer: u64,
///     my_float: f32,
/// }
///
/// #[derive(Clone)]
/// struct Ui {
///     window: Rc<Window>,
///     document: Rc<Document>,
///     elements: Elements,
///     preferences: Rc<RefCell<Preferences>>,
/// }
///
/// ui_elements! {
///     my_object_my_integer: HtmlInputElement => NumberInput<u64>,
///     my_object_my_float: HtmlInputElement => NumberInput<f32>,
/// }
///
/// // Dummy Preferences struct. This is needed because update_elements
/// // keeps the preferences in sync, so it calls methods in the Preferences.
/// struct Preferences {}
/// impl_dummy_preferences!(
///     my_object_my_integer: u64,
///     my_object_my_float: f32,
/// );
///
/// impl Ui {
///     fn new(window: Rc<Window>, document: Rc<Document>) -> Result<Ui, JsValue> {
///         let elements = Elements::new(&document)?;
///         let preferences = Rc::new(RefCell::new(Preferences {}));
///         let ui = Ui { window, document, elements, preferences };
///         Ok(ui)
///     }
///
///     impl_update_elements!(my_object, MyObject, my_integer, my_float);
///     impl_post_update_noop!(my_object, MyObject);
/// }
///
/// fn main() -> Result<(), JsValue> {
///     # // do not run the rest of the code during testing, as it will fail,
///     # // but still check that it compiles
///     # return Ok(());
///     let (window, document) = maia_wasm::get_window_and_document()?;
///     let ui = Ui::new(window, document)?;
///
///     // assume that the following data has come from a REST API GET
///     let response = MyObject { my_integer: 42, my_float: 3.14 };
///     ui.update_my_object_all_elements(&response);
///
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! impl_update_elements {
    ($name:ident, $json:ty, $($element:ident),*) => {
        paste::paste! {
            fn [<update_ $name _inactive_elements>](&self, json: &$json) -> Result<(), JsValue> {
                $crate::set_values_if_inactive!(
                    self,
                    json,
                    $name,
                    $(
                        $element
                    ),*
                );
                self.[<post_update_ $name _elements>](json)
            }

            #[allow(dead_code)]
            fn [<update_ $name _all_elements>](&self, json: &$json) -> Result<(), JsValue> {
                $crate::set_values!(
                    self,
                    json,
                    $name,
                    $(
                        $element
                    ),*
                );
                self.[<post_update_ $name _elements>](json)
            }
        }
    }
}

/// UI macro: implements a dummy `post_update_name_elements` method.
///
/// See [`impl_update_elements`] for more details.
#[macro_export]
macro_rules! impl_post_update_noop {
    ($name:ident, $json:ty) => {
        paste::paste! {
            fn [<post_update_ $name _elements>](&self, _json: &$json) -> Result<(), JsValue> {
                Ok(())
            }
        }
    };
}

/// UI macro: implements a dummy `post_patch_name_update_elements` method.
///
/// See [`impl_section_custom`](crate::impl_section_custom) for more details.
#[macro_export]
macro_rules! impl_post_patch_update_elements_noop {
    ($name:ident, $patch_json:ty) => {
        paste::paste! {
            fn [<post_patch_ $name _update_elements>](&self, _json: &$patch_json) -> Result<(), JsValue> {
                Ok(())
            }
        }
    }
}

/// UI macro: implements a dummy `name_onchange_patch_modify` method.
///
/// See [`impl_section_custom`](crate::impl_section_custom) for more details.
#[macro_export]
macro_rules! impl_onchange_patch_modify_noop {
    ($name:ident, $patch_json:ty) => {
        paste::paste! {
            fn [<$name _onchange_patch_modify>](&self, _json: &mut $patch_json) {}
        }
    };
}

/// UI macro: implements a REST API section with default functionality.
///
/// This UI macro calls [`impl_section_custom`](crate::impl_section_custom) and
/// it additionally calls the following macros to define dummy user-provided
/// functions that do nothing:
///
/// - [`impl_post_update_noop`](crate::impl_post_update_noop), which implements
///   a dummy `post_update_name_elements` method.
///
/// - [`impl_post_patch_update_elements_noop`], which implements a dummy
///   `post_patch_name_update_elements` method.
///
/// - [`impl_onchange_patch_modify_noop`], which implements a dummy
///   `name_onchange_patch_modify` method.
///
/// If custom functionality needs to be used in any of these methods,
/// `impl_section_custom` needs to be used instead of `impl_section`. The
/// `impl_*_noop` macros can still be called to implement the methods that do
/// not need custom functionality.
///
/// # Example
///
/// # Example
///
/// ```
/// use maia_wasm::{impl_dummy_preferences, impl_section, set_on, ui_elements, ui::input::NumberInput};
/// use serde::{Deserialize, Serialize};
/// use std::{cell::RefCell, rc::Rc};
/// use wasm_bindgen::JsValue;
/// use web_sys::{Document, HtmlInputElement, Window};
///
/// // An object defined by a REST API. This corresponds to the GET method.
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct MyObject {
///     my_integer: u64,
///     my_float: f32,
/// }
///
/// // An object defined by a REST API. This corresponds to the PATCH method.
/// #[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// struct PatchMyObject {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     my_integer: Option<u64>,
///     #[serde(skip_serializing_if = "Option::is_none")]
///     my_float: Option<f32>,
/// }
///
/// #[derive(Clone)]
/// struct Ui {
///     window: Rc<Window>,
///     document: Rc<Document>,
///     elements: Elements,
///     // the api_state is optional; it could be defined
///     // as Rc<RefCell<Option<()>> and set to contain None
///     api_state: Rc<RefCell<Option<ApiState>>>,
///     preferences: Rc<RefCell<Preferences>>,
/// }
///
/// ui_elements! {
///     my_object_my_integer: HtmlInputElement => NumberInput<u64>,
///     my_object_my_float: HtmlInputElement => NumberInput<f32>,
/// }
///
/// struct ApiState {
///     my_object: MyObject,
/// }
///
/// // Dummy Preferences struct. This is needed because update_elements
/// // keeps the preferences in sync, so it calls methods in the Preferences.
/// struct Preferences {}
/// impl_dummy_preferences!(
///     my_object_my_integer: u64,
///     my_object_my_float: f32,
/// );
///
/// impl Ui {
///     fn new(window: Rc<Window>, document: Rc<Document>) -> Result<Ui, JsValue> {
///         let elements = Elements::new(&document)?;
///         let preferences = Rc::new(RefCell::new(Preferences {}));
///         let api_state = Rc::new(RefCell::new(Some(ApiState {
///             my_object: MyObject { my_integer: 0, my_float: 0.0 },
///         })));
///         let ui = Ui { window, document, elements, api_state, preferences };
///         ui.set_callbacks();
///         Ok(ui)
///     }
///
///     fn set_callbacks(&self) -> Result<(), JsValue> {
///         set_on!(change,
///                 self,
///                 my_object_my_integer,
///                 my_object_my_float);
///         Ok(())
///     }
///
///     // it is necessary to define an alert method like this
///     // to show errors to the user
///     fn alert(&self, message: &str) -> Result<(), JsValue> {
///         self.window.alert_with_message(message);
///         Ok(())
///     }
///
///     impl_section!(my_object,
///                   MyObject,
///                   PatchMyObject,
///                   "/my_object",
///                   my_integer,
///                   my_float);
/// }
///
/// ```
#[macro_export]
macro_rules! impl_section {
    ($name:ident, $json:ty, $patch_json:ty, $url:expr, $($element:ident),*) => {
        $crate::impl_post_update_noop!($name, $json);
        $crate::impl_post_patch_update_elements_noop!($name, $patch_json);
        $crate::impl_onchange_patch_modify_noop!($name, $patch_json);
        $crate::impl_section_custom!($name, $json, $patch_json, $url, $($element),*);
    }
}

/// UI macro: implements a REST API section using custom functionality.
///
/// This UI macro is used to implement the interaction between the UI and a
/// section of the REST API using some custom functions. See [`impl_section`]
/// for the equivalent without custom functions.
///
/// This macro includes the following:
///
/// - [`impl_patch`] call to implement a method that sends a PATCH request.
///
/// - [`impl_update_elements`] call to implement methods to update the UI elements.
///
/// - [`impl_onchange`](crate::impl_onchange) call to implement `onchange`
///   methods for each element that perform a PATCH request and update all the UI
///   elements of this section using the result.
///
/// - Implements a `patch_name_update_elements` of signature `async fn
///   patch_name_update_elements(&self, json: &$patch_json) -> Result<(),
///   JsValue>` that updates the `api_state` member of the UI with the `json`
///   data, calls the `update_name_all_elements` method to update the contents of
///   UI elements to match the server-side state, and calls the user-defined
///   `post_patch_name_update_elements` with the `json` to perform any required
///   user-defined functionality.
///
/// See [`impl_section`] for an example.
#[macro_export]
macro_rules! impl_section_custom {
    ($name:ident, $json:ty, $patch_json:ty, $url:expr, $($element:ident),*) => {
        $crate::impl_patch!($name, $patch_json, $json, $url);

        $crate::impl_update_elements!(
            $name,
            $json,
            $(
                $element
            ),*
        );

        $crate::impl_onchange!(
            $name,
            $patch_json,
            $(
                $element
            ),*
        );

        paste::paste! {
            async fn [<patch_ $name _update_elements>](&self, json: &$patch_json) -> Result<(), wasm_bindgen::JsValue> {
                if let Some(json_output) = $crate::ui::request::ignore_request_failed(self.[<patch_ $name>](json).await)? {
                    if let Some(state) = self.api_state.borrow_mut().as_mut() {
                        state.$name.clone_from(&json_output);
                    }
                    self.[<update_ $name _all_elements>](&json_output)?;
                    self.[<post_patch_ $name _update_elements>](json)?;
                }
                Ok(())
            }
        }
    }
}

/// UI macro: implements a `_onchange` methods that perform a PATCH request.
///
/// Given a `name` and several `element`s, this macro implements
/// `name_element_onchange` methods with signature `fn
/// name_element_onchange(&self) -> Closure<dyn Fn() -> JsValue>`, whose return
/// value can be used as the `onchange` closure for each of the elements. The
/// closure obtains the Rust value of the element using
/// [`InputElement::get`](crate::ui::input::InputElement::get), creates a PATCH
/// object of type `$patch_json` that contains `element` set to a `Some`
/// containing the obtained value and the remaining members set to `None` (their
/// [`Default::default`] value), passes the `$patch_json` object to a
/// user-defined `name_onchange_patch_modify` method that can modify the value
/// of this object, and finally calls and awaits `patch_name_update_elements`,
/// which performs the PATCH request and updates the UI elements using the
/// response.
#[macro_export]
macro_rules! impl_onchange {
    ($name:ident, $patch_json:ty, $($element:ident),*) => {
        paste::paste! {
            $(
                fn [<$name _ $element _onchange>](&self) -> wasm_bindgen::closure::Closure<dyn Fn() -> wasm_bindgen::JsValue> {
                    use $crate::ui::input::InputElement;
                    let ui = self.clone();
                    wasm_bindgen::closure::Closure::new(move || {
                        if !ui.elements.[<$name _ $element>].report_validity() {
                            return wasm_bindgen::JsValue::NULL;
                        }
                        let Some(value) = ui.elements.[<$name _ $element>].get() else {
                            // TODO: decide what to do in this case, since it passes validity
                            ui.window
                                .alert_with_message(concat!(
                                    "Invalid value for ", stringify!([<$name _ $element>])))
                                .unwrap();
                            return wasm_bindgen::JsValue::NULL;
                        };
                        #[allow(clippy::needless_update)]
                        let mut patch = $patch_json { $element: Some(value), ..Default::default() };
                        ui.[<$name _onchange_patch_modify>](&mut patch);
                        let ui = ui.clone();
                        wasm_bindgen_futures::future_to_promise(async move {
                            ui.[<patch_ $name _update_elements>](&patch).await?;
                            Ok(wasm_bindgen::JsValue::NULL)
                        }).into()
                    })
                }
            )*
        }
    }
}

/// UI macro: implements methods for an interface with tabs.
///
/// This macro implements a method `hide_all_tab_panels` with signature `fn
/// hide_all_tab_panels(&self) -> Result<(), JsValue>` that sets, for each
/// `element` given in the macro call, the corresponding `element_panel` to
/// hidden by adding `hidden` to its class list, and the corresponding
/// `element_tab` to deselected by seting its `aria-selected` attribute to
/// `false`. For each `element`, it implements an `element_tab_onclick` method
/// of signature `fn element_tab_onclick(&self) -> Closure<dyn Fn()>`. The
/// closure returned by this method is a suitable `onclick` callback for the tab
/// element. It calls the `hide_all_tab_panels` method and then selects the tab
/// corresponding to `element` by removing the `hidden` class from the
/// `element_panel` and setting the `arial-selected` attribute to `true` in the
/// `element_tab`.
///
/// # Example
///
/// ```
/// use maia_wasm::{impl_tabs, set_on, ui_elements};
/// use std::rc::Rc;
/// use wasm_bindgen::JsValue;
/// use web_sys::{Document, HtmlButtonElement, HtmlElement, Window};
///
/// #[derive(Clone)]
/// struct Ui {
///     window: Rc<Window>,
///     elements: Elements,
/// }
///
/// ui_elements! {
///     a_tab: HtmlButtonElement => Rc<HtmlButtonElement>,
///     a_panel: HtmlElement => Rc<HtmlElement>,
///     b_tab: HtmlButtonElement => Rc<HtmlButtonElement>,
///     b_panel: HtmlElement => Rc<HtmlElement>,
/// }
///
/// impl Ui {
///     fn new(window: Rc<Window>, document: &Document) -> Result<Ui, JsValue> {
///         let elements = Elements::new(document)?;
///         let ui = Ui { window, elements };
///         ui.set_callbacks();
///         Ok(ui)
///     }
///
///     fn set_callbacks(&self) -> Result<(), JsValue> {
///         set_on!(click, self, a_tab, b_tab);
///         Ok(())
///     }
///
///     impl_tabs!(a, b);
/// }
#[macro_export]
macro_rules! impl_tabs {
    ($($element:ident),*) => {
        paste::paste! {
            fn hide_all_tab_panels(&self) -> Result<(), wasm_bindgen::JsValue> {
                $(
                    self.elements.[<$element _panel>].class_list().add_1("hidden")?;
                    self.elements.[<$element _tab>].set_attribute("aria-selected", "false")?;
                )*
                Ok(())
            }

            $(
                fn [<$element _tab_onclick>](&self) -> wasm_bindgen::closure::Closure<dyn Fn()> {
                    let ui = self.clone();
                    wasm_bindgen::closure::Closure::new(move || {
                        ui.hide_all_tab_panels().unwrap();
                        ui.elements.[<$element _panel>].class_list().remove_1("hidden").unwrap();
                        ui.elements.[<$element _tab>].set_attribute("aria-selected", "true").unwrap();
                    })
                }
            )*
        }
    }
}
