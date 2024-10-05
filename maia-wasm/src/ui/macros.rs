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
            fn new(document: &Document) -> Result<Elements, JsValue> {
                Ok(Elements {
                    $(
                        $element: Rc::new(document
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

#[macro_export]
macro_rules! onchange_apply {
    ($($name:ident),*) => {
        paste::paste! {
            $(
                fn [<$name _onchange>](&self) -> Closure<dyn Fn()> {
                    let ui = self.clone();
                    Closure::new(move || {
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

#[macro_export]
macro_rules! onchange_apply_noprefs {
    ($($name:ident),*) => {
        paste::paste! {
            $(
                fn [<$name _onchange>](&self) -> Closure<dyn Fn()> {
                    let ui = self.clone();
                    Closure::new(move || {
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

#[macro_export]
macro_rules! set_on {
    ($event:ident, $self:expr, $($element:ident),*) => {
        paste::paste! {
            $(
                $self.elements.$element.[<set_on $event>](Some(
                    $self.[<$element _on $event>]()
                        .into_js_value()
                        .unchecked_ref(),
                ));
            )*
        }
    }
}

#[macro_export]
macro_rules! impl_request {
    ($name:ident, $request_json:ty, $get_json:ty, $url:expr, $method_ident:ident, $method:expr) => {
        paste::paste! {
            async fn [<$method_ident _ $name>](&self, json: &$request_json) -> Result<$get_json, $crate::ui::request::RequestError> {
                let method = $method;
                let request = $crate::ui::request::json_request($url, json, method)?;
                let response = JsFuture::from(self.window.fetch_with_request(&request))
                    .await?
                    .dyn_into::<Response>()?;
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

#[macro_export]
macro_rules! impl_patch {
    ($name:ident, $patch_json:ty, $get_json:ty, $url:expr) => {
        impl_request!($name, $patch_json, $get_json, $url, patch, "PATCH");
    };
}

#[macro_export]
macro_rules! impl_put {
    ($name:ident, $put_json:ty, $get_json:ty, $url:expr) => {
        impl_request!($name, $put_json, $get_json, $url, put, "PUT");
    };
}

#[macro_export]
macro_rules! set_values_if_inactive {
    ($self:expr, $source:expr, $section:ident, $($element:ident),*) => {
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

#[macro_export]
macro_rules! set_values {
    ($self:expr, $source:expr, $section:ident, $($element:ident),*) => {
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

#[macro_export]
macro_rules! impl_update_elements {
    ($name:ident, $json:ty, $($element:ident),*) => {
        paste::paste! {
            fn [<update_ $name _inactive_elements>](&self, json: &$json) -> Result<(), JsValue> {
                set_values_if_inactive!(
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
                set_values!(
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

#[macro_export]
macro_rules! impl_onchange_patch_modify_noop {
    ($name:ident, $patch_json:ty) => {
        paste::paste! {
            fn [<$name _onchange_patch_modify>](&self, _json: &mut $patch_json) {}
        }
    };
}

#[macro_export]
macro_rules! impl_section {
    ($name:ident, $json:ty, $patch_json:ty, $url:expr, $($element:ident),*) => {
        impl_post_update_noop!($name, $json);
        impl_post_patch_update_elements_noop!($name, $patch_json);
        impl_onchange_patch_modify_noop!($name, $patch_json);
        impl_section_custom!($name, $json, $patch_json, $url, $($element),*);
    }
}

#[macro_export]
macro_rules! impl_section_custom {
    ($name:ident, $json:ty, $patch_json:ty, $url:expr, $($element:ident),*) => {
        impl_patch!($name, $patch_json, $json, $url);

        impl_update_elements!(
            $name,
            $json,
            $(
                $element
            ),*
        );

        impl_onchange!(
            $name,
            $patch_json,
            $(
                $element
            ),*
        );

        paste::paste! {
            async fn [<patch_ $name _update_elements>](&self, json: &$patch_json) -> Result<(), JsValue> {
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

#[macro_export]
macro_rules! impl_onchange {
    ($name:ident, $patch_json:ty, $($element:ident),*) => {
        paste::paste! {
            $(
                fn [<$name _ $element _onchange>](&self) -> Closure<dyn Fn() -> JsValue> {
                    let ui = self.clone();
                    Closure::new(move || {
                        if !ui.elements.[<$name _ $element>].report_validity() {
                            return JsValue::NULL;
                        }
                        let Some(value) = ui.elements.[<$name _ $element>].get() else {
                            // TODO: decide what to do in this case, since it passes validity
                            ui.window
                                .alert_with_message(concat!(
                                    "Invalid value for ", stringify!([<$name _ $element>])))
                                .unwrap();
                            return JsValue::NULL;
                        };
                        #[allow(clippy::needless_update)]
                        let mut patch = $patch_json { $element: Some(value), ..Default::default() };
                        ui.[<$name _onchange_patch_modify>](&mut patch);
                        let ui = ui.clone();
                        future_to_promise(async move {
                            ui.[<patch_ $name _update_elements>](&patch).await?;
                            Ok(JsValue::NULL)
                        }).into()
                    })
                }
            )*
        }
    }
}

#[macro_export]
macro_rules! impl_tabs {
    ($($element:ident),*) => {
        paste::paste! {
            fn hide_all_tab_panels(&self) -> Result<(), JsValue> {
                $(
                    self.elements.[<$element _panel>].class_list().add_1("hidden")?;
                    self.elements.[<$element _tab>].set_attribute("aria-selected", "false")?;
                )*
                Ok(())
            }

            $(
                fn [<$element _tab_onclick>](&self) -> Closure<dyn Fn()> {
                    let ui = self.clone();
                    Closure::new(move || {
                        ui.hide_all_tab_panels().unwrap();
                        ui.elements.[<$element _panel>].class_list().remove_1("hidden").unwrap();
                        ui.elements.[<$element _tab>].set_attribute("aria-selected", "true").unwrap();
                    })
                }
            )*
        }
    }
}
