//! maia-wasm is part of Maia SDR. It is a web application that serves as the UI
//! of Maia SDR. It renders the waterfall using WebGL2 and gives a UI composed
//! of HTML elements that interacts with the maia-httpd RESTful API.

#![warn(missing_docs)]

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{Document, HtmlCanvasElement, Window};

use crate::render::RenderEngine;
use crate::ui::Ui;
use crate::waterfall::Waterfall;
use crate::waterfall_interaction::WaterfallInteraction;
use crate::websocket::WebSocketClient;

pub mod array_view;
pub mod colormap;
pub mod pointer;
pub mod render;
pub mod ui;
pub mod version;
pub mod waterfall;
pub mod waterfall_interaction;
pub mod websocket;

/// Initialize the wasm module.
///
/// This function is set to run as soon as the wasm module is instantiated. It
/// applies some settings that are needed for all kinds of usage of
/// `maia-wasm`. For instance, it sets a panic hook using the
/// [`console_error_panic_hook`] crate.
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    Ok(())
}

/// Starts the maia-wasm web application.
///
/// This function starts the maia-wasm application. It should be called from
/// JavaScript when the web page is loaded. It sets up all the objects and
/// callbacks that keep the application running.
#[wasm_bindgen]
pub fn maia_wasm_start() -> Result<(), JsValue> {
    let (window, document) = get_window_and_document()?;
    let canvas = Rc::new(
        document
            .get_element_by_id("canvas")
            .ok_or("unable to get #canvas element")?
            .dyn_into::<web_sys::HtmlCanvasElement>()?,
    );

    let (render_engine, waterfall, mut waterfall_interaction) =
        new_waterfall(&window, &document, &canvas)?;
    WebSocketClient::start(&window, Rc::clone(&waterfall))?;
    let ui = Ui::new(
        Rc::clone(&window),
        Rc::clone(&document),
        Rc::clone(&render_engine),
        Rc::clone(&waterfall),
    )?;
    waterfall_interaction.set_ui(ui);

    setup_render_loop(render_engine, waterfall);

    Ok(())
}

/// Returns the [`Window`] and [`Document`] objects.
///
/// These are returned inside an [`Rc`] so that their ownership can be shared.
pub fn get_window_and_document() -> Result<(Rc<Window>, Rc<Document>), JsValue> {
    let window = Rc::new(web_sys::window().ok_or("unable to get window")?);
    let document = Rc::new(window.document().ok_or("unable to get document")?);
    Ok((window, document))
}

/// Creates a [`Waterfall`] and associated objects.
///
/// This function creates a waterfall, the associated WebGL2 [`RenderEngine`],
/// and the [`WaterfallInteraction`] object, which is used to control the
/// waterfall based on user input.
#[allow(clippy::type_complexity)]
pub fn new_waterfall(
    window: &Rc<Window>,
    document: &Document,
    canvas: &Rc<HtmlCanvasElement>,
) -> Result<
    (
        Rc<RefCell<RenderEngine>>,
        Rc<RefCell<Waterfall>>,
        WaterfallInteraction,
    ),
    JsValue,
> {
    let render_engine = Rc::new(RefCell::new(RenderEngine::new(
        Rc::clone(canvas),
        Rc::clone(window),
        document,
    )?));
    let waterfall = Rc::new(RefCell::new(Waterfall::new(
        &mut render_engine.borrow_mut(),
        window.performance().ok_or("unable to get performance")?,
    )?));
    let waterfall_interaction = WaterfallInteraction::new(
        Rc::clone(window),
        Rc::clone(canvas),
        Rc::clone(&render_engine),
        Rc::clone(&waterfall),
    )?;
    Ok((render_engine, waterfall, waterfall_interaction))
}

/// Sets up a render loop for the waterfall.
///
/// This function sets up a render loop using `requestAnimationFrame()`. Each
/// time the the callback triggers, the waterfall is prepared for rendering and
/// the render engine is called. Then, the rendering of the next frame is
/// scheduled using `requestAnimationFrame()`.
pub fn setup_render_loop(
    render_engine: Rc<RefCell<RenderEngine>>,
    waterfall: Rc<RefCell<Waterfall>>,
) {
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::new(move |dt| {
        let mut render_engine = render_engine.borrow_mut();
        if let Err(e) = waterfall
            .borrow_mut()
            .prepare_render(&mut render_engine, dt)
        {
            web_sys::console::error_1(&e);
            return;
        }
        if let Err(e) = render_engine.render() {
            web_sys::console::error_1(&e);
            return;
        }
        // Schedule ourselves for another requestAnimationFrame callback.
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));
    // Initial requestAnimationFrame callback.
    request_animation_frame(g.borrow().as_ref().unwrap());
}

fn request_animation_frame(f: &Closure<dyn FnMut(f32)>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap();
}
