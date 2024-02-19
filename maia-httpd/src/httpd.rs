//! HTTP server.
//!
//! This module contains the HTTP server of maia-httpd, which is a web server
//! implemented using [`axum`].

use crate::{
    fpga::{InterruptWaiter, IpCore},
    iio::Ad9361,
    spectrometer::SpectrometerConfig,
};
use anyhow::Result;
use axum::{routing::get, Router};
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

mod ad9361;
mod api;
mod iqengine;
mod recording;
mod spectrometer;
mod time;
mod websocket;
mod zeros;

/// HTTP server.
///
/// This HTTP server is the core of the functionality of maia-httpd. Most
/// operations are performed as response to an HTTP request handled by this
/// server.
#[derive(Debug)]
pub struct Server {
    server: axum::serve::Serve<Router, Router>,
}

impl Server {
    /// Creates a new HTTP server.
    ///
    /// The `address` parameter gives the address in which the server will
    /// listen. The `ad9361` and `ip_core` parameters give the server shared
    /// access to the AD9361 device and the Maia SDR FPGA IP core. The
    /// `spectrometer_samp_rate` parameter gives shared access to update the
    /// sample rate of the spectrometer. The `waiter_recorder` is the interrupt
    /// waiter for the IQ recorder, which is contolled by the HTTP server. The
    /// `waterfall_sender` is used to obtain waterfall channel receivers for the
    /// websocket server.
    ///
    /// After calling this function, the server needs to be run by calling
    /// [`Server::run`].
    pub async fn new(
        address: &std::net::SocketAddr,
        ad9361: Arc<tokio::sync::Mutex<Ad9361>>,
        ip_core: Arc<std::sync::Mutex<IpCore>>,
        spectrometer_config: SpectrometerConfig,
        waiter_recorder: InterruptWaiter,
        waterfall_sender: broadcast::Sender<Bytes>,
    ) -> Result<Server> {
        let recorder =
            recording::Recorder::new(Arc::clone(&ad9361), Arc::clone(&ip_core), waiter_recorder)
                .await?;
        let spectrometer = spectrometer::State {
            ip_core,
            ad9361: Arc::clone(&ad9361),
            spectrometer_config,
        };
        let api = api::Api::new(Arc::clone(&ad9361), spectrometer.clone(), recorder.clone());

        let app = Router::new()
            .route("/api", get(api::get_api).with_state(api))
            .route(
                "/api/ad9361",
                get(ad9361::get_ad9361)
                    .put(ad9361::put_ad9361)
                    .patch(ad9361::patch_ad9361)
                    .with_state(ad9361),
            )
            .route(
                "/api/spectrometer",
                get(spectrometer::get_spectrometer)
                    .patch(spectrometer::patch_spectrometer)
                    .with_state(spectrometer),
            )
            .route(
                "/api/recorder",
                get(recording::get_recorder)
                    .patch(recording::patch_recorder)
                    .with_state(recorder.clone()),
            )
            .route(
                "/api/recording/metadata",
                get(recording::get_recording_metadata)
                    .put(recording::put_recording_metadata)
                    .patch(recording::patch_recording_metadata)
                    .with_state(recorder.clone()),
            )
            .route(
                "/api/time",
                get(time::get_time)
                    .put(time::put_time)
                    .patch(time::patch_time),
            )
            .route(
                "/recording",
                get(recording::get_recording).with_state(recorder.clone()),
            )
            .route(
                "/waterfall",
                get(websocket::handler).with_state(waterfall_sender),
            )
            .route("/zeros", get(zeros::get_zeros)) // used for benchmarking
            // IQEngine viewer for IQ recording
            .route_service(
                "/view/api/maiasdr/maiasdr/recording",
                ServeFile::new("iqengine/index.html"),
            )
            .route(
                "/api/datasources/maiasdr/maiasdr/recording/meta",
                get(recording::iqengine::meta).with_state(recorder.clone()),
            )
            .route(
                "/api/datasources/maiasdr/maiasdr/recording/iq-data",
                get(recording::iqengine::iq_data).with_state(recorder.clone()),
            )
            .route(
                "/api/datasources/maiasdr/maiasdr/recording/minimap-data",
                get(recording::iqengine::minimap_data).with_state(recorder),
            )
            .route("/assets/:filename", get(iqengine::serve_assets))
            .fallback_service(ServeDir::new("."));
        tracing::info!(%address, "starting HTTP server");
        let listener = tokio::net::TcpListener::bind(address).await?;
        let server = axum::serve(listener, app.layer(TraceLayer::new_for_http()));
        Ok(Server { server })
    }

    /// Runs the HTTP server.
    ///
    /// This only returns if there is a fatal error.
    pub async fn run(self) -> Result<()> {
        Ok(self.server.await?)
    }
}

mod json_error {
    use anyhow::Error;
    use axum::{
        http::StatusCode,
        response::{IntoResponse, Response},
    };
    use serde::Serialize;

    #[derive(Serialize, Debug, Clone, Eq, PartialEq)]
    pub struct JsonError {
        http_status_code: u16,
        error_description: String,
    }

    impl JsonError {
        pub fn from_error(status_code: StatusCode, error: Error) -> JsonError {
            JsonError {
                http_status_code: status_code.as_u16(),
                error_description: format!("{error:#}"),
            }
        }

        pub fn server_error(error: Error) -> JsonError {
            JsonError::from_error(StatusCode::INTERNAL_SERVER_ERROR, error)
        }
    }

    impl IntoResponse for JsonError {
        fn into_response(self) -> Response {
            let status_code = StatusCode::from_u16(self.http_status_code).unwrap();
            let json = serde_json::to_string(&self).unwrap();
            (status_code, json).into_response()
        }
    }
}
