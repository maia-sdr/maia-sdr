//! WebSocket client for waterfall data.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CloseEvent, MessageEvent, WebSocket, Window};

use crate::waterfall::Waterfall;

/// WebSocket client for waterfall data.
///
/// Implements a WebSocket client that receives messages containing waterfall
/// data and submits the data to the waterfall by calling
/// [Waterfall::put_waterfall_spectrum].
pub struct WebSocketClient {}

struct WebSocketData {
    url: String,
    // Closure that handles onmessage
    onmessage: JsValue,
    // Closure that handles onclose. It is inside a RefCell<Option<>> because
    // the closure is self-referential, in the sense that to try a reconnection,
    // the onclose closure needs access to the onclose closure, in order to
    // assign it to the onclose of the new websocket.
    onclose: RefCell<Option<JsValue>>,
}

impl WebSocketClient {
    /// Starts the WebSocket client.
    ///
    /// The client is given shared mutable access to the [`Waterfall`].
    ///
    /// This function creates and registers the appropriate on-message handler
    /// for the WebSocket client. No further interaction with the
    /// `WebSocketClient` returned by this function is needed and it can be
    /// dropped immediately.
    pub fn start(window: &Window, waterfall: Rc<RefCell<Waterfall>>) -> Result<(), JsValue> {
        let location = window.location();
        let protocol = if location.protocol()? == "https:" {
            "wss"
        } else {
            "ws"
        };
        let hostname = location.hostname()?;
        let port = location.port()?;
        let data = Rc::new(WebSocketData {
            url: format!("{protocol}://{hostname}:{port}/waterfall"),
            onmessage: onmessage(waterfall).into_js_value(),
            onclose: RefCell::new(None),
        });
        data.setup_onclose();
        // initiate first connection
        data.connect()?;
        Ok(())
    }
}

fn onmessage(waterfall: Rc<RefCell<Waterfall>>) -> Closure<dyn Fn(MessageEvent)> {
    Closure::new(move |event: MessageEvent| {
        let data = match event.data().dyn_into::<js_sys::ArrayBuffer>() {
            Ok(x) => x,
            Err(e) => {
                web_sys::console::error_1(&e);
                return;
            }
        };
        waterfall
            .borrow_mut()
            .put_waterfall_spectrum(&js_sys::Float32Array::new(&data));
    })
}

impl WebSocketData {
    fn connect(&self) -> Result<(), JsValue> {
        let ws = WebSocket::new(&self.url)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        ws.set_onmessage(Some(self.onmessage.unchecked_ref()));
        // by this point onclose shouldn't be None
        ws.set_onclose(Some(
            self.onclose.borrow().as_ref().unwrap().unchecked_ref(),
        ));
        Ok(())
    }

    fn setup_onclose(self: &Rc<Self>) {
        let data = Rc::clone(self);
        let closure = Closure::<dyn Fn(CloseEvent)>::new(move |_: CloseEvent| {
            data.connect().unwrap();
        });
        *self.onclose.borrow_mut() = Some(closure.into_js_value());
    }
}
