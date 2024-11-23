//! maia-httpd application.
//!
//! This module contains a top-level structure [`App`] that represents the whole
//! maia-httpd application and a structure [`AppState`] that contains the
//! application state.

use crate::{
    args::Args,
    fpga::{InterruptHandler, IpCore},
    httpd::{self, RecorderFinishWaiter, RecorderState},
    iio::Ad9361,
    spectrometer::{Spectrometer, SpectrometerConfig},
};
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// maia-httpd application.
///
/// This struct represents the maia-sdr application. It owns the different
/// objects of which the application is formed, and runs them concurrently.
#[derive(Debug)]
pub struct App {
    httpd: httpd::Server,
    interrupt_handler: InterruptHandler,
    recorder_finish: RecorderFinishWaiter,
    spectrometer: Spectrometer,
}

impl App {
    /// Creates a new application.
    #[tracing::instrument(name = "App::new", level = "debug")]
    pub async fn new(args: &Args) -> Result<App> {
        // Initialize and build application state
        let (ip_core, interrupt_handler) = IpCore::take().await?;
        let ip_core = std::sync::Mutex::new(ip_core);
        let ad9361 = tokio::sync::Mutex::new(Ad9361::new().await?);
        let recorder = RecorderState::new(&ad9361, &ip_core).await?;
        let state = AppState(Arc::new(State {
            ad9361,
            ip_core,
            geolocation: std::sync::Mutex::new(None),
            recorder,
            spectrometer_config: Default::default(),
        }));
        // Initialize spectrometer sample rate and mode
        state.spectrometer_config().set_samp_rate_mode(
            state.ad9361().lock().await.get_sampling_frequency().await? as f32,
            state.ip_core().lock().unwrap().spectrometer_mode(),
        );

        // Build application objects

        let (waterfall_sender, _) = broadcast::channel(16);
        let spectrometer = Spectrometer::new(
            state.clone(),
            interrupt_handler.waiter_spectrometer(),
            waterfall_sender.clone(),
        );

        let recorder_finish =
            RecorderFinishWaiter::new(state.clone(), interrupt_handler.waiter_recorder());

        let httpd = httpd::Server::new(
            args.listen,
            args.listen_https,
            args.ssl_cert.as_ref(),
            args.ssl_key.as_ref(),
            args.ca_cert.as_ref(),
            state,
            waterfall_sender,
        )
        .await?;

        Ok(App {
            httpd,
            interrupt_handler,
            recorder_finish,
            spectrometer,
        })
    }

    /// Runs the application.
    ///
    /// This only returns if one of the objects that form the application fails.
    #[tracing::instrument(name = "App::run", level = "debug", skip_all)]
    pub async fn run(self) -> Result<()> {
        tokio::select! {
            ret = self.httpd.run() => ret,
            ret = self.interrupt_handler.run() => ret,
            ret = self.recorder_finish.run() => ret,
            ret = self.spectrometer.run() => ret,
        }
    }
}

/// Application state.
///
/// This struct contains the application state that needs to be shared between
/// different modules, such as different Axum handlers in the HTTP server. The
/// struct behaves as an `Arc<...>`. It is cheaply clonable and clones represent
/// a reference to a shared object.
#[derive(Debug, Clone)]
pub struct AppState(Arc<State>);

#[derive(Debug)]
struct State {
    ad9361: tokio::sync::Mutex<Ad9361>,
    ip_core: Mutex<IpCore>,
    geolocation: Mutex<Option<maia_json::Geolocation>>,
    recorder: RecorderState,
    spectrometer_config: SpectrometerConfig,
}

impl AppState {
    /// Gives access to the [`Ad9361`] object of the application.
    pub fn ad9361(&self) -> &tokio::sync::Mutex<Ad9361> {
        &self.0.ad9361
    }

    /// Gives access to the [`IpCore`] object of the application.
    pub fn ip_core(&self) -> &Mutex<IpCore> {
        &self.0.ip_core
    }

    /// Gives access to the current geolocation of the device.
    ///
    /// The geolocation is `None` if it has never been set or if it has been
    /// cleared, or a valid [`Geolocation`](maia_json::Geolocation) otherwise.
    pub fn geolocation(&self) -> &Mutex<Option<maia_json::Geolocation>> {
        &self.0.geolocation
    }

    /// Gives access to the [`RecorderState`] object of the application.
    pub fn recorder(&self) -> &RecorderState {
        &self.0.recorder
    }

    /// Gives access to the [`SpectrometerConfig`] object of the application.
    pub fn spectrometer_config(&self) -> &SpectrometerConfig {
        &self.0.spectrometer_config
    }

    /// Returns the AD9361 sampling frequency.
    pub async fn ad9361_samp_rate(&self) -> Result<f64> {
        Ok(self.ad9361().lock().await.get_sampling_frequency().await? as f64)
    }
}
