//! maia-wasm is part of Maia SDR. It is a web application that serves as the UI
//! of Maia SDR. It renders the waterfall using WebGL2 and gives a UI composed
//! of HTML elements that interacts with the maia-httpd RESTful API.

#![warn(missing_docs)]

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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
pub mod waterfall;
pub mod waterfall_interaction;
pub mod websocket;

/// Starts the web application.
///
/// This is the main web application function. It is called when the web page is
/// loaded, and it set up all the objects and callbacks that keep the web
/// application running.
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    let window = Rc::new(web_sys::window().ok_or("unable to get window")?);
    let document = Rc::new(window.document().ok_or("unable to get document")?);
    let canvas = Rc::new(
        document
            .get_element_by_id("canvas")
            .ok_or("unable to get #canvas element")?
            .dyn_into::<web_sys::HtmlCanvasElement>()?,
    );
    canvas.style().set_property("cursor", "crosshair")?;

    let render_engine = Rc::new(RefCell::new(RenderEngine::new(
        Rc::clone(&canvas),
        Rc::clone(&window),
        &document,
    )?));
    let waterfall = Rc::new(RefCell::new(Waterfall::new(
        &mut render_engine.borrow_mut(),
        window.performance().ok_or("unable to get performance")?,
    )?));
    WebSocketClient::start(&window, Rc::clone(&waterfall))?;
    let ui = Ui::new(
        Rc::clone(&window),
        Rc::clone(&document),
        Rc::clone(&render_engine),
        Rc::clone(&waterfall),
    )?;
    let waterfall_interaction =
        WaterfallInteraction::new(Rc::clone(&render_engine), canvas, ui, Rc::clone(&waterfall));
    waterfall_interaction.set_callbacks();

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

    Ok(())
}

fn request_animation_frame(f: &Closure<dyn FnMut(f32)>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap();
}
