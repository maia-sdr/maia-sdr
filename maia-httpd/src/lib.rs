//! maia-httpd is part of Maia SDR. It is a web server that controls the Maia
//! SDR FPGA IP core and the AD9361 RFIC. It provides a RESTful API for these
//! elements, and serves the web application `maia-wasm`, which functions as the
//! UI of Maia SDR. Waterfall data is streamed to clients in real time using
//! WebSockets.

#![warn(missing_docs)]

pub mod app;
pub mod args;
pub mod ddc;
pub mod fpga;
pub mod httpd;
pub mod iio;
pub mod rxbuffer;
pub mod sigmf;
pub mod spectrometer;
pub mod uio;
