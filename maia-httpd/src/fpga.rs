//! Maia SDR FPGA IP core.
//!
//! This module contains the userspace driver that interfaces with the FPGA IP
//! core.

use crate::rxbuffer::RxBuffer;
use crate::uio::{Mapping, Uio};
use anyhow::{Context, Result};
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
    // RAM-based cache for the number of spectrometer integrations and
    // mode. These are used to speed up IpCore::spectrometer_number_integrations
    // and IpCore::spectrometer_mode by avoiding to read the FPGA register.
    spectrometer_integrations: u32,
    spectrometer_mode: maia_json::SpectrometerMode,
    spectrometer_input: maia_json::SpectrometerInput,
    // RAM-based cache for DDC configuration
    ddc_config: maia_json::PutDDCConfig,
    ddc_enabled: bool,
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

fn default_ddc_config() -> maia_json::PutDDCConfig {
    // this design can be calculated quickly and it is good for sample rates
    // above 61.44 Msps
    let input_samp_rate = 61.44e6;
    crate::ddc::make_design(
        &maia_json::PutDDCDesign {
            frequency: 0.0,
            decimation: 20,
            transition_bandwidth: None,
            passband_ripple: None,
            stopband_attenuation_db: None,
            stopband_one_over_f: None,
        },
        input_samp_rate,
    )
    .unwrap()
}

macro_rules! impl_set_ddc_fir {
    ($func:ident, $addr_offset:expr, $do_fold:expr, $decimation_reg:ident, $op_reg:ident, $odd_reg:ident) => {
        fn $func(
            &self,
            coefficients: &[i32],
            decimation: usize,
            input_samp_rate: f64,
        ) -> Result<()> {
            use crate::ddc::constants;

            const MIN_COEFF: i32 = -(1 << (constants::COEFFICIENT_BITS - 1));
            const MAX_COEFF: i32 = (1 << (constants::COEFFICIENT_BITS - 1)) - 1;
            if coefficients
                .iter()
                .any(|c| !(MIN_COEFF..=MAX_COEFF).contains(c))
            {
                anyhow::bail!("FIR coefficient out of range");
            }
            if !(2..=constants::MAX_DECIMATION).contains(&decimation) {
                anyhow::bail!("decimation out of range");
            }
            if coefficients.is_empty() {
                anyhow::bail!("no coefficients specified");
            }
            // Pretend that the coefficient list length is divisible by decimation
            // by "virtually" extending the list with zeros.
            let operations = coefficients.len().div_ceil(decimation);
            let odd_operations = operations % 2 == 1;
            let operations = if $do_fold {
                operations.div_ceil(2)
            } else {
                operations
            };
            if operations > constants::MAX_OPERATIONS {
                anyhow::bail!("coefficient list too long (too many operations)");
            }
            if operations as f64 * input_samp_rate > constants::CLOCK_FREQUENCY {
                anyhow::bail!(
                    "coefficient list too long (too many operations for input sample rate)"
                )
            }

            // See test_fir.py in maia-hdl for FIR addressing details
            const ADDR_OFFSET: usize = $addr_offset;
            const NUM_ADDR: usize = if $do_fold {
                constants::MAX_COEFFICIENTS_4DSP
            } else {
                constants::MAX_COEFFICIENTS_2DSP
            };
            if operations * decimation > NUM_ADDR {
                anyhow::bail!("coefficient list too long (does not fit in BRAM)");
            }
            for addr in 0..NUM_ADDR {
                let (off, fold) = if $do_fold && addr >= NUM_ADDR / 2 {
                    (1, NUM_ADDR / 2)
                } else {
                    (0, 0)
                };
                let k = (addr - fold) / operations;
                let coeff = if k >= decimation {
                    0
                } else {
                    let j = (addr - fold) % operations;
                    const FOLD_MULT: usize = if $do_fold { 2 } else { 1 };
                    let n = (FOLD_MULT * j + off) * decimation + (decimation - 1 - k);
                    // map_or is used to "virtually" extend the coefficient list
                    // length to a multiple of decimation.
                    coefficients.get(n).map_or(0, |c| *c)
                };
                // TODO: these can be write() instead of modify() if the SVD includes reset values
                let waddr = u16::try_from(addr + ADDR_OFFSET).unwrap();
                self.registers
                    .ddc_coeff_addr()
                    .modify(|_, w| unsafe { w.coeff_waddr().bits(waddr) });
                self.registers.ddc_coeff().modify(|_, w| unsafe {
                    w.coeff_wren().bit(true).coeff_wdata().bits(coeff as u32)
                });
            }

            let dec = u8::try_from(decimation).unwrap();
            self.registers
                .ddc_decimation()
                .modify(|_, w| unsafe { w.$decimation_reg().bits(dec) });
            let opm1 = u8::try_from(operations - 1).unwrap();
            self.registers.ddc_control().modify(|_, w| unsafe {
                let w = w.$op_reg().bits(opm1);
                if $do_fold {
                    w.$odd_reg().bit(odd_operations)
                } else {
                    w
                }
            });

            Ok(())
        }
    };
}

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
        let mut ip_core = IpCore {
            registers: Registers(mapping),
            phys_addr,
            spectrometer,
            // These are initialized to the correct value below, after removing
            // the SDR reset.
            spectrometer_input: maia_json::SpectrometerInput::AD9361,
            spectrometer_integrations: 0,
            spectrometer_mode: maia_json::SpectrometerMode::Average,
            ddc_config: default_ddc_config(),
            ddc_enabled: false,
        };

        ip_core.log_open().await?;
        ip_core.check_product_id()?;
        ip_core.set_sdr_reset(false);
        ip_core.spectrometer_integrations = ip_core
            .registers
            .spectrometer()
            .read()
            .num_integrations()
            .bits()
            .into();
        // this also modifies the DDC enable
        ip_core
            .set_spectrometer_input(
                if ip_core.registers.spectrometer().read().use_ddc_out().bit() {
                    maia_json::SpectrometerInput::DDC
                } else {
                    maia_json::SpectrometerInput::AD9361
                },
                // fake the input sample rate so that this never fails
                0.0,
            )
            .unwrap();
        ip_core.spectrometer_mode = if ip_core.registers.spectrometer().read().peak_detect().bit() {
            maia_json::SpectrometerMode::PeakDetect
        } else {
            maia_json::SpectrometerMode::Average
        };
        ip_core.set_ddc_config(&default_ddc_config(), 0.0).unwrap();
        let interrupt_handler = InterruptHandler::new(uio, interrupt_registers);
        Ok((ip_core, interrupt_handler))
    }

    fn version_struct(&self) -> Version {
        let version = self.registers.version().read();
        Version {
            major: version.major().bits(),
            minor: version.minor().bits(),
            bugfix: version.bugfix().bits(),
        }
    }

    /// Gives the version of the IP core as a `String`.
    pub fn version(&self) -> String {
        format!("{}", self.version_struct())
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
            .control()
            .modify(|_, w| w.sdr_reset().bit(value));
    }

    async fn log_open(&self) -> Result<()> {
        tracing::info!(
            "opened Maia SDR IP core version {} at physical address {:#08x}",
            self.version_struct(),
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
            .spectrometer()
            .read()
            .last_buffer()
            .bits()
            .into()
    }

    /// Gives the signal that is used as an input to the spectrometer.
    pub fn spectrometer_input(&self) -> maia_json::SpectrometerInput {
        self.spectrometer_input
    }

    /// Returns the frequency offset associated with the input to the
    /// spectrometer.
    ///
    /// This offset is relative to the AD9361 RX LO frequency. The offset is
    /// zero if the input is the AD9361, or the DDC frequency if the input is
    /// the DDC.
    pub fn spectrometer_input_frequency_offset(&self) -> f64 {
        match self.spectrometer_input() {
            maia_json::SpectrometerInput::AD9361 => 0.0,
            maia_json::SpectrometerInput::DDC => self.ddc_frequency(),
        }
    }

    /// Returns the decimation factor associated with the input to the
    /// spectrometer.
    ///
    /// This decimation factor relates the AD9361 sample rate to the sample rate
    /// used by the input of the spectrometer. The decimation factor is 1 if the
    /// input is the AD9361, or the DDC decimation if the input is the DDC.
    pub fn spectrometer_input_decimation(&self) -> usize {
        match self.spectrometer_input() {
            maia_json::SpectrometerInput::AD9361 => 1,
            maia_json::SpectrometerInput::DDC => self.ddc_decimation(),
        }
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
        self.spectrometer_integrations
    }

    /// Returns the current spectrometer mode.
    ///
    /// This register indicates whether the spectrometer is running in average
    /// power mode or in peak detect mode.
    ///
    /// Note: [`IpCore`] caches in RAM the value of this register every time
    /// that it is updated, so calls to this function are very fast because the
    /// FPGA register doesn't need to be accessed.
    pub fn spectrometer_mode(&self) -> maia_json::SpectrometerMode {
        self.spectrometer_mode
    }

    /// Sets the spectrometer input.
    ///
    /// This sets the signal that is used as an input for the spectrometer. The
    /// function can fail if the DDC output is selected but the current DDC
    /// configuration cannot run with the current input sample rate, as given in
    /// the `input_samp_rate` argument.
    pub fn set_spectrometer_input(
        &mut self,
        input: maia_json::SpectrometerInput,
        input_samp_freq: f64,
    ) -> Result<()> {
        let use_ddc = matches!(input, maia_json::SpectrometerInput::DDC);
        if use_ddc {
            let max_samp_freq = self
                .ddc_config_summary(input_samp_freq)
                .max_input_sampling_frequency;
            if input_samp_freq > max_samp_freq {
                anyhow::bail!(
                    "cannot set spectrometer input to DDC: \
                     current DDC input sampling frequency {input_samp_freq} is greater than \
                     maximum DDC sample rate {max_samp_freq}"
                );
            }
        }
        self.registers
            .spectrometer()
            .modify(|_, w| w.use_ddc_out().bit(use_ddc));
        self.set_ddc_enable(use_ddc);
        self.spectrometer_input = input;
        Ok(())
    }

    /// Sets the value of the number of integrations register of the spectrometer.
    ///
    /// See [`IpCore::spectrometer_number_integrations`].
    pub fn set_spectrometer_number_integrations(&mut self, value: u32) -> Result<()> {
        const WIDTH: u8 = maia_pac::maia_sdr::spectrometer::NumIntegrationsW::<
            maia_pac::maia_sdr::spectrometer::SpectrometerSpec,
        >::WIDTH;
        if !(1..1 << WIDTH).contains(&value) {
            anyhow::bail!("invalid number of integrations: {}", value);
        }
        unsafe {
            self.registers.spectrometer().modify(|r, w| {
                // if reducing the number of integrations, use the abort bit
                // to force the current integration to stop
                let abort = u32::from(r.num_integrations().bits()) > value;
                w.num_integrations().bits(value as _).abort().bit(abort)
            })
        };
        self.spectrometer_integrations = value;
        Ok(())
    }

    /// Sets the spectrometer mode.
    ///
    /// See [`IpCore::spectrometer_mode`].
    pub fn set_spectrometer_mode(&mut self, mode: maia_json::SpectrometerMode) {
        let peak_detect = match mode {
            maia_json::SpectrometerMode::Average => false,
            maia_json::SpectrometerMode::PeakDetect => true,
        };
        self.registers
            .spectrometer()
            .modify(|_, w| w.peak_detect().bit(peak_detect));
        self.spectrometer_mode = mode;
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

    fn set_ddc_enable(&mut self, enable: bool) {
        self.registers
            .ddc_control()
            .modify(|_, w| w.enable_input().bit(enable));
        self.ddc_enabled = enable;
    }

    /// Gives the current configuration of the DDC.
    ///
    /// The `input_sampling_frequency` parameter indicates the sampling
    /// frequency of the source connected to the DDC input (typically the
    /// AD9361).
    pub fn ddc_config(&self, input_sampling_frequency: f64) -> maia_json::DDCConfig {
        let summary = self.ddc_config_summary(input_sampling_frequency);
        maia_json::DDCConfig {
            enabled: summary.enabled,
            frequency: summary.frequency,
            decimation: summary.decimation,
            input_sampling_frequency: summary.input_sampling_frequency,
            output_sampling_frequency: summary.output_sampling_frequency,
            max_input_sampling_frequency: summary.max_input_sampling_frequency,
            fir1: self.ddc_config.fir1.clone(),
            fir2: self.ddc_config.fir2.clone(),
            fir3: self.ddc_config.fir3.clone(),
        }
    }

    /// Gives a summary of the current configuration of the DDC.
    ///
    /// The summary does not include the FIR filter taps.
    ///
    /// The `input_sampling_frequency` parameter indicates the sampling
    /// frequency of the source connected to the DDC input (typically the
    /// AD9361).
    pub fn ddc_config_summary(&self, input_sampling_frequency: f64) -> maia_json::DDCConfigSummary {
        use crate::ddc::constants;

        let n = self.ddc_config.fir1.coefficients.len();
        let d = usize::try_from(self.ddc_config.fir1.decimation).unwrap();
        let operations = n.div_ceil(d).div_ceil(2);
        let mut max_input_sampling_frequency = constants::CLOCK_FREQUENCY / operations as f64;
        let mut decimation = d;

        if let Some(fir) = &self.ddc_config.fir2 {
            let n = fir.coefficients.len();
            let d = usize::try_from(fir.decimation).unwrap();
            let operations = n.div_ceil(d);
            max_input_sampling_frequency = max_input_sampling_frequency
                .min(constants::CLOCK_FREQUENCY * decimation as f64 / operations as f64);
            decimation *= d;
        }

        if let Some(fir) = &self.ddc_config.fir3 {
            let n = fir.coefficients.len();
            let d = usize::try_from(fir.decimation).unwrap();
            let operations = n.div_ceil(d).div_ceil(2);
            max_input_sampling_frequency = max_input_sampling_frequency
                .min(constants::CLOCK_FREQUENCY * decimation as f64 / operations as f64);
            decimation *= d;
        }
        maia_json::DDCConfigSummary {
            enabled: self.ddc_enabled,
            frequency: self.ddc_frequency(),
            decimation: u32::try_from(decimation).unwrap(),
            input_sampling_frequency,
            output_sampling_frequency: input_sampling_frequency / decimation as f64,
            max_input_sampling_frequency,
        }
    }

    /// Sets the configuration of the DDC.
    ///
    /// Setting the DDC configuration can fail if the parameters are out of
    /// range for the capabilities of the DDC (for instance, if too many FIR
    /// coefficients have been specified). If setting the configuration fails
    /// mid-way, this function tries to revert to the previous configuration in
    /// order to leave the DDC with a consistent configuration.
    ///
    /// This `input_samp_rate` parameter indicates the sample rate at the input
    /// of the DDC in samples per second. It is used to check if the FPGA DSPs
    /// can do enough multiplications per sample as required by the FIR filters.
    pub fn set_ddc_config(
        &mut self,
        config: &maia_json::PutDDCConfig,
        input_samp_rate: f64,
    ) -> Result<()> {
        if let Err(e) = self.try_set_ddc_config(config, input_samp_rate) {
            // revert DDC config; this should not fail, since the
            // configuration was previously set successfully
            if let Err(err) = self.try_set_ddc_config(&self.ddc_config, input_samp_rate) {
                tracing::error!("error reverting DDC configuration: {err}");
            }
            Err(e)
        } else {
            // save DDC config
            self.ddc_config.clone_from(config);
            Ok(())
        }
    }

    fn try_set_ddc_config(
        &self,
        config: &maia_json::PutDDCConfig,
        input_samp_rate: f64,
    ) -> Result<()> {
        let mut input_samp_rate = input_samp_rate;
        self.try_set_ddc_frequency(config.frequency, input_samp_rate)
            .context("failed to configure DDC frequency")?;
        self.set_ddc_fir1(
            &config.fir1.coefficients,
            usize::try_from(config.fir1.decimation).unwrap(),
            input_samp_rate,
        )
        .context("failed to configure fir1")?;
        input_samp_rate /= config.fir1.decimation as f64;
        if let Some(config) = &config.fir2 {
            self.set_ddc_fir2(
                &config.coefficients,
                usize::try_from(config.decimation).unwrap(),
                input_samp_rate,
            )
            .context("failed to configure fir2")?;
            input_samp_rate /= config.decimation as f64;
        }
        if let Some(config) = &config.fir3 {
            self.set_ddc_fir3(
                &config.coefficients,
                usize::try_from(config.decimation).unwrap(),
                input_samp_rate,
            )
            .context("failed to configure fir3")?;
        }
        self.registers.ddc_control().modify(|_, w| {
            w.bypass2()
                .bit(config.fir2.is_none())
                .bypass3()
                .bit(config.fir3.is_none())
        });
        Ok(())
    }

    /// Gets the mixer frequency of the DDC.
    ///
    /// The frequency is given in units of Hz.
    pub fn ddc_frequency(&self) -> f64 {
        self.ddc_config.frequency
    }

    /// Sets the mixer frequency of the DDC.
    ///
    /// The `frequency` is given in units of Hz.
    pub fn set_ddc_frequency(&mut self, frequency: f64, input_samp_rate: f64) -> Result<()> {
        self.try_set_ddc_frequency(frequency, input_samp_rate)?;
        // update configuration cache if we succeeded
        self.ddc_config.frequency = frequency;
        Ok(())
    }

    fn try_set_ddc_frequency(&self, frequency: f64, input_samp_rate: f64) -> Result<()> {
        if !(-0.5 * input_samp_rate..=0.5 * input_samp_rate).contains(&frequency) {
            anyhow::bail!(
                "frequency {frequency} is out of range with input sample rate {input_samp_rate}"
            );
        }
        let cycles_per_sample = frequency / input_samp_rate;
        const NCO_WIDTH: usize = 28;
        let scale = (1 << NCO_WIDTH) as f64;
        let nco_freq = (cycles_per_sample * scale).round() as i32;
        // TODO: this could be write instead of modify if the register
        // had declared its reset value
        self.registers
            .ddc_frequency()
            .modify(|_, w| unsafe { w.frequency().bits(nco_freq as u32) });
        Ok(())
    }

    impl_set_ddc_fir!(
        set_ddc_fir1,
        0,
        true,
        decimation1,
        operations_minus_one1,
        odd_operations1
    );
    // we need an odd_operations field for fir2, even though it doesn't have its
    // own and it isn't touched
    impl_set_ddc_fir!(
        set_ddc_fir2,
        256,
        false,
        decimation2,
        operations_minus_one2,
        odd_operations1
    );
    impl_set_ddc_fir!(
        set_ddc_fir3,
        512,
        true,
        decimation3,
        operations_minus_one3,
        odd_operations3
    );

    /// Gives the decimation factor set in the DDC.
    pub fn ddc_decimation(&self) -> usize {
        let mut decimation = usize::try_from(self.ddc_config.fir1.decimation).unwrap();
        if let Some(config) = &self.ddc_config.fir2 {
            decimation *= usize::try_from(config.decimation).unwrap();
        }
        if let Some(config) = &self.ddc_config.fir3 {
            decimation *= usize::try_from(config.decimation).unwrap();
        }
        decimation
    }

    /// Returns the frequency offset associated with the input to the
    /// recorder.
    ///
    /// This offset is relative to the AD9361 RX LO frequency. The offset is
    /// zero if the input is the AD9361, or the DDC frequency if the input is
    /// the DDC.
    pub fn recorder_input_frequency_offset(&self) -> f64 {
        // currently the recorder shares the same input as the spectrometer
        self.spectrometer_input_frequency_offset()
    }

    /// Returns the decimation factor associated with the input to the
    /// recorder.
    ///
    /// This decimation factor relates the AD9361 sample rate to the sample rate
    /// used by the input of the recorder. The decimation factor is 1 if the
    /// input is the AD9361, or the DDC decimation if the input is the DDC.
    pub fn recorder_input_decimation(&self) -> usize {
        // currently the recorder shares the same input as the spectrometer
        self.spectrometer_input_decimation()
    }

    /// Gives the value of the recorder mode register of the recorder.
    ///
    /// This register is used to select 8-bit mode or 12-bit mode.
    pub fn recorder_mode(&self) -> Result<maia_json::RecorderMode> {
        Ok(
            match self.registers.recorder_control().read().mode().bits() {
                0 => maia_json::RecorderMode::IQ16bit,
                1 => maia_json::RecorderMode::IQ12bit,
                2 => maia_json::RecorderMode::IQ8bit,
                _ => anyhow::bail!("invalid recorder mode value"),
            },
        )
    }

    /// Sets the value of the recorder mode register of the recorder.
    ///
    /// See [`IpCore::recorder_mode`].
    pub fn set_recorder_mode(&self, mode: maia_json::RecorderMode) {
        let mode = match mode {
            maia_json::RecorderMode::IQ16bit => 0,
            maia_json::RecorderMode::IQ12bit => 1,
            maia_json::RecorderMode::IQ8bit => 2,
        };
        self.registers
            .recorder_control()
            .modify(|_, w| unsafe { w.mode().bits(mode) });
    }

    /// Starts a recording.
    ///
    /// The recording will end when the recording DMA buffer is exhausted or
    /// when [`IpCore::recorder_stop`] is called.
    pub fn recorder_start(&self) {
        tracing::info!("starting recorder");
        self.registers
            .recorder_control()
            .modify(|_, w| w.start().set_bit());
    }

    /// Stops a recording.
    ///
    /// This stops a currently running recording.
    pub fn recorder_stop(&self) {
        tracing::info!("stopping recorder");
        self.registers
            .recorder_control()
            .modify(|_, w| w.stop().set_bit());
    }

    /// Gives the value of the next address register of the recorder.
    ///
    /// This register indicates the next physical address to which the recorder
    /// would have written if it had not stopped. It can be used to calculate
    /// the size of the recording.
    pub fn recorder_next_address(&self) -> usize {
        usize::try_from(self.registers.recorder_next_address().read().bits()).unwrap()
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
                let interrupts = self.registers.interrupts().read();
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
            .map(|n| n & self.num_buffers_mask)
            .take_while(move |&n| n != end)
            .map(|n| {
                self.buffer.cache_invalidate(n).unwrap();
                self.buffer.buffer_as_slice(n)
            })
    }
}
