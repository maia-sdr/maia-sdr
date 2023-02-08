//! Spectrometer.
//!
//! This module is used for the control of the spectrometer included in the Maia
//! SDR FPGA IP core.

use crate::fpga::{InterruptWaiter, IpCore};
use anyhow::Result;
use bytes::Bytes;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

// Used to obtain values in dB which are positive
const BASE_SCALE: f32 = 1e9;

/// Spectrometer.
///
/// This struct waits for interrupts from the spectrometer in the FPGA IP core,
/// reads the spectrum data, transforms it from `u64` to `f32` format, and sends
/// it (serialized into [`Bytes`]) into a [`tokio::sync::broadcast::Sender`].
#[derive(Debug)]
pub struct Spectrometer {
    ip_core: Arc<Mutex<IpCore>>,
    sender: broadcast::Sender<Bytes>,
    interrupt: InterruptWaiter,
    samp_rate: Arc<Mutex<f32>>,
}

/// Spectrometer sample rate setter.
///
/// This struct gives shared access to a setter for the spectrometer sample
/// rate. It is used to update the sample rate from other parts of the code.
#[derive(Debug, Clone)]
pub struct SpectrometerSampRate(Arc<Mutex<f32>>);

impl Spectrometer {
    /// Creates a new spectrometer struct.
    ///
    /// The `interrupt` parameter should correspond to the [`InterruptWaiter`]
    /// corresponding to the spectrometer. Each spectra received from the FPGA
    /// is sent to the `sender`.
    pub fn new(
        ip_core: Arc<Mutex<IpCore>>,
        interrupt: InterruptWaiter,
        sender: broadcast::Sender<Bytes>,
    ) -> Spectrometer {
        Spectrometer {
            ip_core,
            interrupt,
            sender,
            samp_rate: Arc::new(Mutex::new(0.0)),
        }
    }

    /// Returns a sample rate setter object.
    ///
    /// This returns a [`SpectrometerSampRate`] sample rate setter object that
    /// can be used to update the spectrometer sample rate from other objects.
    pub fn samp_rate_setter(&self) -> SpectrometerSampRate {
        SpectrometerSampRate(Arc::clone(&self.samp_rate))
    }

    /// Runs the spectrometer.
    ///
    /// This function only returns if there is an error. The function should be
    /// run concurrently with the rest of the application for the spectrometer
    /// to work.
    #[tracing::instrument(name = "spectrometer", skip_all)]
    pub async fn run(self) -> Result<()> {
        loop {
            self.interrupt.wait().await;
            let samp_rate = *self.samp_rate.lock().unwrap();
            let mut ip_core = self.ip_core.lock().unwrap();
            let num_integrations = ip_core.spectrometer_number_integrations() as f32;
            let scale = BASE_SCALE / (num_integrations * samp_rate);
            tracing::trace!(
                last_buffer = ip_core.spectrometer_last_buffer(),
                samp_rate,
                num_integrations,
                scale
            );
            // TODO: potential optimization: do not hold the mutex locked while
            // we iterate over the buffers.
            for buffer in ip_core.get_spectrometer_buffers() {
                // It is ok if send returns Err, because there might be
                // no receiver handles in this moment.
                let _ = self.sender.send(Self::buffer_u64_to_f32(buffer, scale));
            }
        }
    }

    fn buffer_u64_to_f32(buffer: &[u64], scale: f32) -> Bytes {
        // TODO: optimize using Neon
        buffer
            .iter()
            .flat_map(|&x| (x as f32 * scale).to_ne_bytes().into_iter())
            .collect()
    }
}

impl SpectrometerSampRate {
    /// Sets the spectrometer sample rate.
    ///
    /// Updates the spectrometer sample rate to the value give, in units of
    /// samples per second.
    pub fn set(&self, samp_rate: f32) {
        *self.0.lock().unwrap() = samp_rate;
    }
}
