//! maia-httpd application.
//!
//! This module contains a top-level structure [`App`] that represents the whole
//! maia-httpd application.

use crate::{
    args::Args,
    fpga::{InterruptHandler, IpCore},
    httpd,
    iio::Ad9361,
    spectrometer::Spectrometer,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;

/// maia-httpd application.
///
/// This struct represents the maia-sdr application. It owns the different
/// objects of which the application is formed, and runs them concurrently.
#[derive(Debug)]
pub struct App {
    _ad9361: Arc<tokio::sync::Mutex<Ad9361>>,
    httpd: httpd::Server,
    interrupt_handler: InterruptHandler,
    spectrometer: Spectrometer,
}

impl App {
    /// Creates a new application.
    #[tracing::instrument(name = "App::new", level = "debug")]
    pub async fn new(args: &Args) -> Result<App> {
        let (ip_core, interrupt_handler) = IpCore::take().await?;
        let ip_core = Arc::new(std::sync::Mutex::new(ip_core));
        let ad9361 = Arc::new(tokio::sync::Mutex::new(Ad9361::new().await?));
        let (waterfall_sender, _) = broadcast::channel(16);
        let spectrometer = Spectrometer::new(
            Arc::clone(&ip_core),
            interrupt_handler.waiter_spectrometer(),
            waterfall_sender.clone(),
        );
        // Initialize spectrometer sample rate and mode
        let spectrometer_config = spectrometer.config();
        spectrometer_config.set_samp_rate_mode(
            ad9361.lock().await.get_sampling_frequency().await? as f32,
            ip_core.lock().unwrap().spectrometer_mode(),
        );
        let httpd = httpd::Server::new(
            &args.listen,
            Arc::clone(&ad9361),
            ip_core,
            spectrometer_config,
            interrupt_handler.waiter_recorder(),
            waterfall_sender,
        )
        .await?;
        Ok(App {
            _ad9361: ad9361,
            httpd,
            interrupt_handler,
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
            ret = self.spectrometer.run() => ret,
        }
    }
}
