//! Maia SDR FPGA IP core.
//!
//! This module contains the userspace driver that interfaces with the FPGA IP
//! core.

use crate::rxbuffer::RxBuffer;
use crate::uio::{Mapping, Uio};
use anyhow::{Context, Result};
use std::cell::Cell;
use std::sync::Arc;
use tokio::sync::Notify;

/// Maia SDR FPGA IP core.
///
/// This struct represents the FPGA IP core and gives access to its registers
/// and DMA buffers.
#[derive(Debug)]
pub struct IpCore {
    registers: Registers,
    phys_addr: usize,
    spectrometer: Dma,
    // RAM-based cache for the number of spectrometer integrations. This is used
    // to speed up IpCore::spectrometer_number_integrations by avoiding to read
    // the FPGA register.
    spectrometer_integrations: Cell<u32>,
}

/// Interrupt waiter.
///
/// This is associated with an interrupt of a particular type and can be used by
/// a future to await until such an interrupt happens.
#[derive(Debug)]
pub struct InterruptWaiter {
    notify: Arc<Notify>,
}

/// Interrupt handler.
///
/// Receives the interrupts produced by the FPGA IP core and sends notifications
/// to the [`InterruptWaiter`]s. It is necessary to call
/// [`InterruptHandler::run`] in order to receive and process interrupts.
///
/// # Examples
///
/// This shows how to create an `InterruptHandler`, obtain an `InterruptWaiter`,
/// and wait for an interrupt, while running the `InterruptHandler` concurrently
/// in a Tokio task.
///
///
/// ```
/// # async fn f() -> Result<(), anyhow::Error> {
/// use maia_httpd::fpga::IpCore;
///
/// let (ip_core, interrupt_handler) = IpCore::take().await?;
/// let waiter = interrupt_handler.waiter_recorder();
/// tokio::spawn(async move { interrupt_handler.run() });
/// waiter.wait().await;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct InterruptHandler {
    uio: Uio,
    registers: Registers, // should only access registers.interrupts
    notify_spectrometer: Arc<Notify>,
    notify_recorder: Arc<Notify>,
}

#[derive(Debug)]
struct Dma {
    buffer: RxBuffer,
    last_written: Option<usize>,
    num_buffers_mask: usize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct Version {
    major: u8,
    minor: u8,
    bugfix: u8,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}.{}.{}", self.major, self.minor, self.bugfix)
    }
}

#[derive(Debug)]
struct Registers(Mapping);

impl std::ops::Deref for Registers {
    type Target = maia_pac::maia_sdr::RegisterBlock;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.0.addr() as *const maia_pac::maia_sdr::RegisterBlock) }
    }
}

unsafe impl Send for Registers {}

impl IpCore {
    /// Opens the FPGA IP core.
    ///
    /// This function can only be run successfully once in the lifetime of the
    /// process. If this function has returned `Ok` previously, all subsequent
    /// calls will return `Err`. This ensures that there is only a single
    /// [`IpCore`] object in the program.
    ///
    /// On success, the `IpCore` and the corresponding [`InterruptHandler`] are
    /// returned.
    pub async fn take() -> Result<(IpCore, InterruptHandler)> {
        let uio = Uio::from_name("maia-sdr")
            .await
            .context("failed to open maia-sdr UIO")?;
        let mapping = uio
            .map_mapping(0)
            .await
            .context("failed to map maia-sdr UIO")?;
        let phys_addr = uio.map_addr(0).await?;
        let spectrometer = Dma::new("maia-sdr-spectrometer")
            .await
            .context("failed to open maia-sdr-spectrometer DMA buffer")?;
        let interrupt_registers = Registers(mapping.clone());
        let ip_core = IpCore {
            registers: Registers(mapping),
            phys_addr,
            spectrometer,
            // This is initialized to the correct value below, after removing
            // the SDR reset.
            spectrometer_integrations: Cell::new(0),
        };

        ip_core.log_open().await?;
        ip_core.check_product_id()?;
        ip_core.set_sdr_reset(false);
        ip_core.spectrometer_integrations.set(
            ip_core
                .registers
                .spectrometer
                .read()
                .num_integrations()
                .bits()
                .into(),
        );
        let interrupt_handler = InterruptHandler::new(uio, interrupt_registers);
        Ok((ip_core, interrupt_handler))
    }

    fn version(&self) -> Version {
        let version = self.registers.version.read();
        Version {
            major: version.major().bits(),
            minor: version.minor().bits(),
            bugfix: version.bugfix().bits(),
        }
    }

    fn check_product_id(&self) -> Result<()> {
        const PRODUCT_ID: &[u8; 4] = b"maia";
        let product_id = unsafe {
            std::slice::from_raw_parts(self.registers.0.addr() as *const u8, PRODUCT_ID.len())
        };
        if product_id != PRODUCT_ID {
            anyhow::bail!("wrong product ID {:#02x?}", product_id);
        }
        Ok(())
    }

    fn set_sdr_reset(&self, value: bool) {
        self.registers
            .control
            .modify(|_, w| w.sdr_reset().bit(value))
    }

    async fn log_open(&self) -> Result<()> {
        tracing::info!(
            "opened Maia SDR IP core version {} at physical address {:#08x}",
            self.version(),
            self.phys_addr
        );
        Ok(())
    }

    /// Gives the value of the last buffer register of the spectrometer.
    ///
    /// This register indicates the index of the last buffer to which the
    /// spectrometer has written.
    pub fn spectrometer_last_buffer(&self) -> usize {
        self.registers
            .spectrometer
            .read()
            .last_buffer()
            .bits()
            .into()
    }

    /// Gives the value of the number of integrations register of the spectrometer.
    ///
    /// This register indicates how many FFTs are non-coherently accumulated by
    /// the spectrometer.
    ///
    /// Note: [`IpCore`] caches in RAM the value of this register every time
    /// that it is updated, so calls to this function are very fast because the
    /// FPGA register doesn't need to be accessed.
    pub fn spectrometer_number_integrations(&self) -> u32 {
        self.spectrometer_integrations.get()
    }

    /// Sets the value of the number of integrations register of the spectrometer.
    ///
    /// See [`IpCore::spectrometer_number_integrations`].
    pub fn set_spectrometer_number_integrations(&self, value: u32) -> Result<()> {
        let width = maia_pac::maia_sdr::spectrometer::NUM_INTEGRATIONS_W::<0>::WIDTH;
        if !(1..1 << width).contains(&value) {
            anyhow::bail!("invalid number of integrations: {}", value);
        }
        unsafe {
            self.registers
                .spectrometer
                .modify(|_, w| w.num_integrations().bits(value as _))
        };
        self.spectrometer_integrations.set(value);
        Ok(())
    }

    /// Returns the new buffers that have been written by the spectrometer.
    ///
    /// This function returns an iterator that iterates over the buffers to
    /// which the spectrometter has written since the last call to
    /// `get_spectrometer_buffers`.
    pub fn get_spectrometer_buffers(&mut self) -> impl Iterator<Item = &[u64]> {
        self.spectrometer
            .get_new_buffers(self.spectrometer_last_buffer())
            .map(|buff| unsafe {
                std::slice::from_raw_parts(
                    buff.as_ptr() as *const u64,
                    buff.len() / std::mem::size_of::<u64>(),
                )
            })
    }

    /// Gives the value of the recorder mode register of the recorder.
    ///
    /// This register is used to select 8-bit mode or 12-bit mode.
    pub fn recorder_mode(&self) -> maia_json::RecorderMode {
        match self.registers.recorder_control.read().mode_8bit().bit() {
            true => maia_json::RecorderMode::IQ8bit,
            false => maia_json::RecorderMode::IQ12bit,
        }
    }

    /// Sets the value of the recorder mode register of the recorder.
    ///
    /// See [`IpCore::recorder_mode`].
    pub fn set_recorder_mode(&self, mode: maia_json::RecorderMode) {
        let mode_8bit = match mode {
            maia_json::RecorderMode::IQ8bit => true,
            maia_json::RecorderMode::IQ12bit => false,
        };
        self.registers
            .recorder_control
            .modify(|_, w| w.mode_8bit().bit(mode_8bit));
    }

    /// Starts a recording.
    ///
    /// The recording will end when the recording DMA buffer is exhausted or
    /// when [`IpCore::recorder_stop`] is called.
    pub fn recorder_start(&self) {
        tracing::info!("starting recorder");
        self.registers
            .recorder_control
            .modify(|_, w| w.start().set_bit());
    }

    /// Stops a recording.
    ///
    /// This stops a currently running recording.
    pub fn recorder_stop(&self) {
        tracing::info!("stopping recorder");
        self.registers
            .recorder_control
            .modify(|_, w| w.stop().set_bit());
    }

    /// Gives the value of the next address register of the recorder.
    ///
    /// This register indicates the next physical address to which the recorder
    /// would have written if it had not stopped. It can be used to calculate
    /// the size of the recording.
    pub fn recorder_next_address(&self) -> usize {
        usize::try_from(self.registers.recorder_next_address.read().bits()).unwrap()
    }
}

macro_rules! impl_interrupt_handler {
    ($($interrupt:ident),*) => {
        paste::paste! {
            fn new(uio: Uio, registers: Registers) -> InterruptHandler {
                InterruptHandler {
                    uio,
                    registers,
                    $(
                        [<notify_ $interrupt>]: Arc::new(Notify::new()),
                    )*
                }
            }

            async fn wait_and_notify(&mut self) -> Result<()> {
                self.uio.irq_enable().await?;
                self.uio.irq_wait().await?;
                let interrupts = self.registers.interrupts.read();
                $(
                    if interrupts.$interrupt().bit() {
                        self.[<notify_ $interrupt>].notify_one();
                    }
                )*;
                Ok(())
            }

            $(
                #[doc = concat!("Returns a waiter for the ", stringify!($interrupt), " interrupt.")]
                pub fn [<waiter_ $interrupt>](&self) -> InterruptWaiter {
                    InterruptWaiter {
                        notify: Arc::clone(&self.[<notify_ $interrupt>]),
                    }
                }
            )*
        }
    }
}

impl InterruptHandler {
    /// Runs the interrupt handler.
    ///
    /// This function only returns if there is an error.
    ///
    /// The function must be run concurrently with the rest of the application
    /// so that interrupts can be received and notifications can be sent to the
    /// waiters.
    pub async fn run(mut self) -> Result<()> {
        loop {
            self.wait_and_notify().await?;
        }
    }

    impl_interrupt_handler!(spectrometer, recorder);
}

impl InterruptWaiter {
    /// Waits for an interrupt.
    ///
    /// Awaiting on the future returned by this function will only return when
    /// the interrupt is received.
    pub fn wait(&self) -> impl std::future::Future<Output = ()> + '_ {
        self.notify.notified()
    }
}

impl Dma {
    async fn new(name: &str) -> Result<Dma> {
        let buffer = RxBuffer::new(name)
            .await
            .context("failed to open rxbuffer DMA buffer")?;
        let num_buffers = buffer.num_buffers();
        if !num_buffers.is_power_of_two() {
            anyhow::bail!("num_buffers is not a power of 2");
        }
        Ok(Dma {
            buffer,
            last_written: None,
            num_buffers_mask: num_buffers - 1,
        })
    }

    fn get_new_buffers(&mut self, last_written: usize) -> impl Iterator<Item = &[u8]> {
        let start = match self.last_written {
            Some(n) => n + 1,
            None => last_written + 1, // this yields an empty iterator
        };
        self.last_written = Some(last_written);
        let end = (last_written + 1) & self.num_buffers_mask;

        (start..)
            .into_iter()
            .map(|n| n & self.num_buffers_mask)
            .take_while(move |&n| n != end)
            .map(|n| {
                self.buffer.cache_invalidate(n).unwrap();
                self.buffer.buffer_as_slice(n)
            })
    }
}
