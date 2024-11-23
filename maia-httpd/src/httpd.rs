//! HTTP server.
//!
//! This module contains the HTTP server of maia-httpd, which is a web server
//! implemented using [`axum`].

use crate::app::AppState;
use anyhow::Result;
use axum::{
    routing::{get, put},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use bytes::Bytes;
use std::{net::SocketAddr, path::Path};
use tokio::sync::broadcast;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

mod ad9361;
mod api;
mod ddc;
mod geolocation;
mod iqengine;
mod recording;
mod spectrometer;
mod time;
mod version;
mod websocket;
mod zeros;

pub use recording::{RecorderFinishWaiter, RecorderState};

/// HTTP server.
///
/// This HTTP server is the core of the functionality of maia-httpd. Most
/// operations are performed as response to an HTTP request handled by this
/// server.
#[derive(Debug)]
pub struct Server {
    http_server: axum_server::Server,
    https_server: Option<axum_server::Server<axum_server::tls_rustls::RustlsAcceptor>>,
    app: Router,
}

impl Server {
    /// Creates a new HTTP server.
    ///
    /// The `http_address` parameter gives the address in which the server will
    /// listen using HTTP. The `https_address` parameter gives the address in
    /// which the server will listen using HTTPS. The `ad9361` and `ip_core`
    /// parameters give the server shared access to the AD9361 device and the
    /// Maia SDR FPGA IP core. The `spectrometer_samp_rate` parameter gives
    /// shared access to update the sample rate of the spectrometer. The
    /// `waiter_recorder` is the interrupt waiter for the IQ recorder, which is
    /// contolled by the HTTP server. The `waterfall_sender` is used to obtain
    /// waterfall channel receivers for the websocket server.
    ///
    /// After calling this function, the server needs to be run by calling
    /// [`Server::run`].
    pub async fn new(
        http_address: SocketAddr,
        https_address: SocketAddr,
        ssl_cert: Option<impl AsRef<Path>>,
        ssl_key: Option<impl AsRef<Path>>,
        ca_cert: Option<impl AsRef<Path>>,
        state: AppState,
        waterfall_sender: broadcast::Sender<Bytes>,
    ) -> Result<Server> {
        let mut app = Router::new()
            // all the following routes have .with_state(state)
            .route("/api", get(api::get_api))
            .route(
                "/api/ad9361",
                get(ad9361::get_ad9361)
                    .put(ad9361::put_ad9361)
                    .patch(ad9361::patch_ad9361),
            )
            .route(
                "/api/spectrometer",
                get(spectrometer::get_spectrometer).patch(spectrometer::patch_spectrometer),
            )
            .route(
                "/api/ddc/config",
                get(ddc::get_ddc_config)
                    .put(ddc::put_ddc_config)
                    .patch(ddc::patch_ddc_config),
            )
            .route("/api/ddc/design", put(ddc::put_ddc_design))
            .route(
                "/api/geolocation",
                get(geolocation::get_geolocation).put(geolocation::put_geolocation),
            )
            .route(
                "/api/recorder",
                get(recording::get_recorder).patch(recording::patch_recorder),
            )
            .route(
                "/api/recording/metadata",
                get(recording::get_recording_metadata)
                    .put(recording::put_recording_metadata)
                    .patch(recording::patch_recording_metadata),
            )
            .route("/recording", get(recording::get_recording))
            .route("/version", get(version::get_version))
            // IQEngine viewer for IQ recording
            .route(
                "/api/datasources/maiasdr/maiasdr/recording/meta",
                get(recording::iqengine::meta),
            )
            .route(
                "/api/datasources/maiasdr/maiasdr/recording/iq-data",
                get(recording::iqengine::iq_data),
            )
            .route(
                "/api/datasources/maiasdr/maiasdr/recording/minimap-data",
                get(recording::iqengine::minimap_data),
            )
            .with_state(state)
            // the following routes have another (or no) state
            .route(
                "/api/time",
                get(time::get_time)
                    .put(time::put_time)
                    .patch(time::patch_time),
            )
            .route(
                "/waterfall",
                get(websocket::handler).with_state(waterfall_sender),
            )
            .route("/zeros", get(zeros::get_zeros)); // used for benchmarking
        if let Some(ca_cert) = &ca_cert {
            // Maia SDR CA certificate
            app = app.route_service("/ca.crt", ServeFile::new(ca_cert));
        }
        let app = app
            // IQEngine viewer for IQ recording
            .route_service(
                "/view/api/maiasdr/maiasdr/recording",
                ServeFile::new("iqengine/index.html"),
            )
            .route("/assets/:filename", get(iqengine::serve_assets))
            .fallback_service(ServeDir::new("."))
            .layer(TraceLayer::new_for_http());
        tracing::info!(%http_address, "starting HTTP server");
        let http_server = axum_server::bind(http_address);
        tracing::info!(%https_address, "starting HTTPS server");
        let https_server = match (&ssl_cert, &ssl_key) {
            (Some(ssl_cert), Some(ssl_key)) => Some(axum_server::bind_rustls(
                https_address,
                RustlsConfig::from_pem_file(ssl_cert, ssl_key).await?,
            )),
            _ => None,
        };
        Ok(Server {
            http_server,
            https_server,
            app,
        })
    }

    /// Runs the HTTP server.
    ///
    /// This only returns if there is a fatal error.
    pub async fn run(self) -> Result<()> {
        let http_server = self.http_server.serve(self.app.clone().into_make_service());
        if let Some(https_server) = self.https_server {
            let https_server = https_server.serve(self.app.into_make_service());
            Ok(tokio::select! {
                ret = http_server => ret,
                ret = https_server => ret,
            }?)
        } else {
            Ok(http_server.await?)
        }
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
    pub struct JsonError(maia_json::Error);

    impl JsonError {
        pub fn from_error<E: Into<Error>>(
            error: E,
            status_code: StatusCode,
            suggested_action: maia_json::ErrorAction,
        ) -> JsonError {
            let error: Error = error.into();
            JsonError(maia_json::Error {
                http_status_code: status_code.as_u16(),
                error_description: format!("{error:#}"),
                suggested_action,
            })
        }

        pub fn client_error_alert<E: Into<Error>>(error: E) -> JsonError {
            JsonError::from_error(
                error,
                StatusCode::BAD_REQUEST,
                maia_json::ErrorAction::Alert,
            )
        }

        pub fn client_error<E: Into<Error>>(error: E) -> JsonError {
            JsonError::from_error(error, StatusCode::BAD_REQUEST, maia_json::ErrorAction::Log)
        }

        pub fn server_error<E: Into<Error>>(error: E) -> JsonError {
            JsonError::from_error(
                error,
                StatusCode::INTERNAL_SERVER_ERROR,
                maia_json::ErrorAction::Log,
            )
        }
    }

    impl IntoResponse for JsonError {
        fn into_response(self) -> Response {
            let status_code = StatusCode::from_u16(self.0.http_status_code).unwrap();
            let json = serde_json::to_string(&self.0).unwrap();
            (status_code, json).into_response()
        }
    }
}
