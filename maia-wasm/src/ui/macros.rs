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

macro_rules! waterfallminmax_onchange {
    ($minmax:ident) => {
        paste::paste! {
            fn [<$minmax _onchange>](&self) -> Closure<dyn Fn()> {
                let ui = self.clone();
                Closure::new(move || {
                    let element = &ui.elements.$minmax;
                    if !element.report_validity() {
                        return;
                    }
                    if let Some(value) = element.get() {
                        ui.waterfall.borrow_mut().[<set_ $minmax>](value);
                        // try_borrow_mut prevents trying to update the
                        // preferences as a consequence of the
                        // Preferences::apply_client calling this closure
                        if let Ok(mut p) = ui.preferences.try_borrow_mut() {
                            if let Err(e) = p.[<update_ $minmax>](&value) {
                                web_sys::console::error_1(&e);
                            }
                        }
                    } else {
                        ui.window
                            .alert_with_message(concat!("Invalid value for ",
                                                        stringify!($minmax)))
                            .unwrap();
                    }
                })
            }
        }
    };
}

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

macro_rules! impl_patch {
    ($name:ident, $patch_json:ty, $get_json:ty, $url:expr) => {
        paste::paste! {
            async fn [<patch_ $name>](&self, json: &$patch_json) -> Result<$get_json, crate::ui::patch::PatchError> {
                let request = json_patch($url, json)?;
                let response = JsFuture::from(self.window.fetch_with_request(&request))
                    .await?
                    .dyn_into::<Response>()?;
                if !response.ok() {
                    let status = response.status();
                    let text = crate::ui::patch::response_to_string(&response).await?;
                    web_sys::console::error_1(&format!("PATCH request failed with HTTP code {status}. \
                                                        Server returned: {text}").into());
                    return Err(crate::ui::patch::PatchError::RequestFailed(text));
                }
                Ok(crate::ui::patch::response_to_json(&response).await?)
            }
        }
    };
}

macro_rules! set_values_if_inactive {
    ($self:expr, $source:expr, $section:ident, $($element:ident),*) => {
        let mut preferences = $self.preferences.borrow_mut();
        paste::paste!{
            $(
                if !$self.document.is_element_active(stringify!([<$section _ $element>])) {
                    $self.elements.[<$section _ $element>].set(&$source.$element);
                }
                if let Err(e) = preferences.[<update_ $section _ $element>](&$source.$element) {
                    web_sys::console::error_1(&e);
                }
            )*
        }
    }
}

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

macro_rules! impl_update_elements {
    ($name:ident, $json:ty, $($element:ident),*) => {
        paste::paste! {
            fn [<update_ $name _inactive_elements>](&self, json: &$json) {
                set_values_if_inactive!(
                    self,
                    json,
                    $name,
                    $(
                        $element
                    ),*
                );
            }

            fn [<update_ $name _all_elements>](&self, json: &$json) {
                set_values!(
                    self,
                    json,
                    $name,
                    $(
                        $element
                    ),*
                );
            }
        }
    }
}

macro_rules! impl_section {
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
                match self.[<patch_ $name>](json).await {
                    Ok(json_output) => self.[<update_ $name _all_elements>](&json_output),
                    Err(crate::ui::patch::PatchError::RequestFailed(_)) => {
                        // The error has already been logged by patch_$name, so we do nothing
                        // and return Ok(()) so that the promise doesn't fail.
                    }
                    Err(crate::ui::patch::PatchError::OtherError(err)) => {
                        // Unhandled error. Make the promise fail (eventually) with this error.
                        return Err(err);
                    }
                }
                Ok(())
            }
        }
    }
}

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
                        let patch = $patch_json { $element: Some(value), ..Default::default() };
                        let ui = ui.clone();
                        future_to_promise(async move {
                            let patch = patch;
                            ui.[<patch_ $name _update_elements>](&patch).await?;
                            Ok(JsValue::NULL)
                        }).into()
                    })
                }
            )*
        }
    }
}
