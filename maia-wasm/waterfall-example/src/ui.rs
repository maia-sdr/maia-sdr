use maia_wasm::{
    onchange_apply_noprefs,
    render::RenderEngine,
    set_on,
    ui::{
        colormap::Colormap,
        input::{CheckboxInput, EnumInput, NumberInput},
    },
    ui_elements,
    waterfall::Waterfall,
};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsValue;
use web_sys::{Document, HtmlInputElement, HtmlSelectElement, Window};

#[derive(Clone)]
pub struct Ui {
    window: Rc<Window>,
    elements: Elements,
    render_engine: Rc<RefCell<RenderEngine>>,
    waterfall: Rc<RefCell<Waterfall>>,
}

// Defines the 'struct Elements' and its constructor
ui_elements! {
    colormap_select: HtmlSelectElement => EnumInput<Colormap>,
    waterfall_show_waterfall: HtmlInputElement => CheckboxInput,
    waterfall_show_spectrum: HtmlInputElement => CheckboxInput,
    waterfall_min: HtmlInputElement => NumberInput<f32>,
    waterfall_max: HtmlInputElement => NumberInput<f32>,
}

impl Ui {
    pub fn new(
        window: Rc<Window>,
        document: &Document,
        render_engine: Rc<RefCell<RenderEngine>>,
        waterfall: Rc<RefCell<Waterfall>>,
    ) -> Result<Ui, JsValue> {
        let elements = Elements::new(document)?;
        let ui = Ui {
            window,
            elements,
            render_engine,
            waterfall,
        };
        ui.set_callbacks()?;
        Ok(ui)
    }

    fn set_callbacks(&self) -> Result<(), JsValue> {
        set_on!(
            change,
            self,
            colormap_select,
            waterfall_show_waterfall,
            waterfall_show_spectrum,
            waterfall_min,
            waterfall_max
        );

        Ok(())
    }

    onchange_apply_noprefs!(
        colormap_select,
        waterfall_show_waterfall,
        waterfall_show_spectrum,
        waterfall_min,
        waterfall_max
    );

    fn waterfall_show_waterfall_apply(&self, value: bool) {
        self.waterfall.borrow_mut().set_waterfall_visible(value);
    }

    fn waterfall_show_spectrum_apply(&self, value: bool) {
        self.waterfall.borrow_mut().set_spectrum_visible(value);
    }

    fn colormap_select_apply(&self, value: Colormap) {
        let mut render_engine = self.render_engine.borrow_mut();
        self.waterfall
            .borrow()
            .load_colormap(&mut render_engine, value.colormap_as_slice())
            .unwrap();
    }

    fn waterfall_min_apply(&self, value: f32) {
        self.waterfall.borrow_mut().set_waterfall_min(value);
    }

    fn waterfall_max_apply(&self, value: f32) {
        self.waterfall.borrow_mut().set_waterfall_max(value);
    }
}
